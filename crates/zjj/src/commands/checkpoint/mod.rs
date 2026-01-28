//! Checkpoint command - save and restore full session state snapshots
//!
//! Provides atomic save/restore of all session state, enabling rollback
//! to known-good configurations.

use anyhow::{Context, Result};
use chrono::TimeZone;
use serde::Serialize;
use sqlx::Row;
use zjj_core::OutputFormat;

use crate::{commands::get_session_db, db::SessionDb};

// ── Types ────────────────────────────────────────────────────────────

/// CLI arguments for the checkpoint command
pub struct CheckpointArgs {
    pub action: CheckpointAction,
    pub format: OutputFormat,
}

/// Which checkpoint action to perform
pub enum CheckpointAction {
    Create { description: Option<String> },
    Restore { checkpoint_id: String },
    List,
}

/// Response from checkpoint operations
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum CheckpointResponse {
    Created { checkpoint_id: String },
    Restored { checkpoint_id: String },
    List { checkpoints: Vec<CheckpointInfo> },
}

/// Information about a single checkpoint
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub created_at: String,
    pub session_count: usize,
    pub description: Option<String>,
}

// ── Schema ───────────────────────────────────────────────────────────

const CHECKPOINT_SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id TEXT UNIQUE NOT NULL,
    description TEXT,
    session_count INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS checkpoint_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id TEXT NOT NULL,
    session_name TEXT NOT NULL,
    status TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    branch TEXT,
    metadata TEXT,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(checkpoint_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id ON checkpoint_sessions(checkpoint_id);
";

// ── Public entry point ───────────────────────────────────────────────

/// Run the checkpoint command
pub fn run(args: &CheckpointArgs) -> Result<()> {
    let db = get_session_db()?;
    ensure_checkpoint_tables(&db)?;

    let response = match &args.action {
        CheckpointAction::Create { description } => create_checkpoint(&db, description.as_deref()),
        CheckpointAction::Restore { checkpoint_id } => restore_checkpoint(&db, checkpoint_id),
        CheckpointAction::List => list_checkpoints(&db),
    }?;

    output_response(&response, args.format)
}

// ── Implementation ───────────────────────────────────────────────────

fn ensure_checkpoint_tables(db: &SessionDb) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
    rt.block_on(async {
        let pool = db.pool();
        sqlx::query(CHECKPOINT_SCHEMA)
            .execute(pool)
            .await
            .map(|_| ())
            .context("Failed to create checkpoint tables")
    })
}

fn create_checkpoint(db: &SessionDb, description: Option<&str>) -> Result<CheckpointResponse> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
    rt.block_on(async {
        let pool = db.pool();
        let sessions = db.list(None).await.map_err(|e| anyhow::anyhow!("{e}"))?;

        let checkpoint_id = generate_checkpoint_id();
        let session_count =
            i64::try_from(sessions.len()).unwrap_or(i64::MAX);

        sqlx::query(
            "INSERT INTO checkpoints (checkpoint_id, description, session_count) VALUES (?, ?, ?)",
        )
        .bind(&checkpoint_id)
        .bind(description)
        .bind(session_count)
        .execute(pool)
        .await
        .context("Failed to insert checkpoint")?;

        for session in &sessions {
            let metadata_json = session
                .metadata
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .context("Failed to serialize session metadata")?;

            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path, branch, metadata)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&checkpoint_id)
            .bind(&session.name)
            .bind(session.status.to_string())
            .bind(&session.workspace_path)
            .bind(&session.branch)
            .bind(&metadata_json)
            .execute(pool)
            .await
            .context("Failed to insert checkpoint session")?;
        }

        Ok(CheckpointResponse::Created { checkpoint_id })
    })
}

fn restore_checkpoint(db: &SessionDb, checkpoint_id: &str) -> Result<CheckpointResponse> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
    rt.block_on(async {
        let pool = db.pool();

        // Verify checkpoint exists
        let exists: bool = sqlx::query("SELECT 1 FROM checkpoints WHERE checkpoint_id = ?")
            .bind(checkpoint_id)
            .fetch_optional(pool)
            .await
            .context("Failed to query checkpoint")?
            .is_some();

        if !exists {
            anyhow::bail!("Checkpoint '{checkpoint_id}' not found");
        }

        // Fetch saved sessions
        let rows = sqlx::query(
            "SELECT session_name, status, workspace_path, branch, metadata
             FROM checkpoint_sessions WHERE checkpoint_id = ?",
        )
        .bind(checkpoint_id)
        .fetch_all(pool)
        .await
        .context("Failed to fetch checkpoint sessions")?;

        // Atomic restore: delete all current sessions, then re-insert from checkpoint
        // Use a transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to begin transaction")?;

        sqlx::query("DELETE FROM sessions")
            .execute(&mut *tx)
            .await
            .context("Failed to clear sessions for restore")?;

        for row in &rows {
            let name: String = row.try_get("session_name").context("Missing session_name")?;
            let status: String = row.try_get("status").context("Missing status")?;
            let workspace_path: String =
                row.try_get("workspace_path").context("Missing workspace_path")?;
            let branch: Option<String> = row.try_get("branch").context("Missing branch")?;
            let metadata: Option<String> = row.try_get("metadata").context("Missing metadata")?;

            sqlx::query(
                "INSERT INTO sessions (name, status, workspace_path, branch, metadata, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))",
            )
            .bind(&name)
            .bind(&status)
            .bind(&workspace_path)
            .bind(&branch)
            .bind(&metadata)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to restore session '{name}'"))?;
        }

        tx.commit().await.context("Failed to commit restore transaction")?;

        Ok(CheckpointResponse::Restored {
            checkpoint_id: checkpoint_id.to_string(),
        })
    })
}

fn list_checkpoints(db: &SessionDb) -> Result<CheckpointResponse> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create runtime")?;
    rt.block_on(async {
        let pool = db.pool();

        let rows = sqlx::query(
            "SELECT checkpoint_id, created_at, session_count, description
             FROM checkpoints ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .context("Failed to list checkpoints")?;

        let checkpoints = rows
            .iter()
            .map(|row| {
                let id: String = row
                    .try_get("checkpoint_id")
                    .map_err(|e| anyhow::anyhow!("Missing checkpoint_id: {e}"))?;
                let created_at_ts: i64 = row
                    .try_get("created_at")
                    .map_err(|e| anyhow::anyhow!("Missing created_at: {e}"))?;
                let session_count: i64 = row
                    .try_get("session_count")
                    .map_err(|e| anyhow::anyhow!("Missing session_count: {e}"))?;
                let description: Option<String> = row
                    .try_get("description")
                    .map_err(|e| anyhow::anyhow!("Missing description: {e}"))?;

                let created_at = chrono::Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                    .unwrap_or_else(|| created_at_ts.to_string());

                let count = usize::try_from(session_count).unwrap_or(0);

                Ok(CheckpointInfo {
                    id,
                    created_at,
                    session_count: count,
                    description,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(CheckpointResponse::List { checkpoints })
    })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn generate_checkpoint_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("chk-{now:x}")
}

fn output_response(response: &CheckpointResponse, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let json = serde_json::to_string_pretty(response)
            .context("Failed to serialize checkpoint response")?;
        println!("{json}");
    } else {
        match response {
            CheckpointResponse::Created { checkpoint_id } => {
                println!("Checkpoint created: {checkpoint_id}");
            }
            CheckpointResponse::Restored { checkpoint_id } => {
                println!("Restored to checkpoint: {checkpoint_id}");
            }
            CheckpointResponse::List { checkpoints } => {
                if checkpoints.is_empty() {
                    println!("No checkpoints found.");
                } else {
                    println!(
                        "{:<20} {:<28} {:>8}  Description",
                        "ID", "Created", "Sessions"
                    );
                    println!("{}", "-".repeat(72));
                    for cp in checkpoints {
                        let desc = cp.description.as_deref().unwrap_or("");
                        println!(
                            "{:<20} {:<28} {:>8}  {}",
                            cp.id, cp.created_at, cp.session_count, desc
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

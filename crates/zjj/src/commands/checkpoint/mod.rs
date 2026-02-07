//! Checkpoint command - save and restore full session state snapshots
//!
//! Provides atomic save/restore of all session state, enabling rollback
//! to known-good configurations.

use anyhow::{Context, Result};
use chrono::TimeZone;
use futures::{StreamExt, TryStreamExt};
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
pub async fn run(args: &CheckpointArgs) -> Result<()> {
    let db = get_session_db().await?;
    ensure_checkpoint_tables(&db).await?;

    let response = match &args.action {
        CheckpointAction::Create { description } => {
            create_checkpoint(&db, description.as_deref()).await
        }
        CheckpointAction::Restore { checkpoint_id } => restore_checkpoint(&db, checkpoint_id).await,
        CheckpointAction::List => list_checkpoints(&db).await,
    }?;

    output_response(&response, args.format)
}

// ── Implementation ───────────────────────────────────────────────────

async fn ensure_checkpoint_tables(db: &SessionDb) -> Result<()> {
    let pool = db.pool();
    sqlx::query(CHECKPOINT_SCHEMA)
        .execute(pool)
        .await
        .map(|_| ())
        .context("Failed to create checkpoint tables")
}

async fn create_checkpoint(
    db: &SessionDb,
    description: Option<&str>,
) -> Result<CheckpointResponse> {
    let pool = db.pool();
    let sessions = db.list(None).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    let checkpoint_id = generate_checkpoint_id()?;

    sqlx::query("INSERT INTO checkpoints (checkpoint_id, description) VALUES (?, ?)")
        .bind(&checkpoint_id)
        .bind(description)
        .execute(pool)
        .await
        .context("Failed to insert checkpoint")?;

    futures::stream::iter(sessions)
        .map(Ok::<crate::session::Session, anyhow::Error>)
        .try_for_each(|session| {
            let pool = pool.clone();
            let checkpoint_id = checkpoint_id.clone();
            async move {
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
                .execute(&pool)
                .await
                .context("Failed to insert checkpoint session")?;
                Ok::<(), anyhow::Error>(())
            }
        })
        .await?;

    Ok(CheckpointResponse::Created { checkpoint_id })
}

async fn restore_checkpoint(db: &SessionDb, checkpoint_id: &str) -> Result<CheckpointResponse> {
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

    let tx = futures::stream::iter(rows)
        .map(Ok::<sqlx::sqlite::SqliteRow, anyhow::Error>)
        .try_fold(tx, |mut tx, row| async move {
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
            Ok::<_, anyhow::Error>(tx)
        })
        .await?;

    tx.commit()
        .await
        .context("Failed to commit restore transaction")?;

    Ok(CheckpointResponse::Restored {
        checkpoint_id: checkpoint_id.to_string(),
    })
}

async fn list_checkpoints(db: &SessionDb) -> Result<CheckpointResponse> {
    let pool = db.pool();

    let rows = sqlx::query(
        "SELECT checkpoint_id, created_at, description
         FROM checkpoints ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list checkpoints")?;

    let checkpoints = futures::stream::iter(rows)
        .map(Ok::<sqlx::sqlite::SqliteRow, anyhow::Error>)
        .and_then(|row| {
            let pool = pool.clone();
            async move {
                let id: String = row
                    .try_get("checkpoint_id")
                    .context("Missing checkpoint_id")?;
                let created_at_ts: i64 = row.try_get("created_at").context("Missing created_at")?;
                let description: Option<String> =
                    row.try_get("description").context("Missing description")?;

                // Query actual session count from checkpoint_sessions table
                let count_row: Option<(i64,)> = sqlx::query_as(
                    "SELECT COUNT(*) FROM checkpoint_sessions WHERE checkpoint_id = ?",
                )
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .context("Failed to count checkpoint sessions")?;

                let session_count = count_row.map_or(0, |(c,)| c);
                let count = usize::try_from(session_count)
                    .map_err(|_| anyhow::anyhow!("Session count out of range: {session_count}"))?;

                let created_at = chrono::Utc
                    .timestamp_opt(created_at_ts, 0)
                    .single()
                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                    .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {created_at_ts}"))?;

                Ok(CheckpointInfo {
                    id,
                    created_at,
                    session_count: count,
                    description,
                })
            }
        })
        .try_collect::<Vec<_>>()
        .await?;

    Ok(CheckpointResponse::List { checkpoints })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn generate_checkpoint_id() -> Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("System time before UNIX epoch")?
        .as_millis();
    Ok(format!("chk-{now:x}"))
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
                        let desc = cp.description.as_deref().map_or("", |s| s);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Type Structure Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_action_create_with_description() {
        let action = CheckpointAction::Create {
            description: Some("test checkpoint".to_string()),
        };
        assert!(matches!(
            action,
            CheckpointAction::Create { description } if description == Some("test checkpoint".to_string())
        ));
    }

    #[test]
    fn test_checkpoint_action_create_without_description() {
        let action = CheckpointAction::Create { description: None };
        assert!(matches!(
            action,
            CheckpointAction::Create { description } if description.is_none()
        ));
    }

    #[test]
    fn test_checkpoint_action_restore() {
        let action = CheckpointAction::Restore {
            checkpoint_id: "chk-abc123".to_string(),
        };
        assert!(matches!(
            action,
            CheckpointAction::Restore { checkpoint_id } if checkpoint_id == "chk-abc123"
        ));
    }

    #[test]
    fn test_checkpoint_action_list() {
        let action = CheckpointAction::List;
        assert!(matches!(action, CheckpointAction::List));
    }

    // ── Response Serialization Tests ─────────────────────────────────────

    #[test]
    fn test_checkpoint_response_created_serialization() {
        let response = CheckpointResponse::Created {
            checkpoint_id: "chk-abc123".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("Created"));
        assert!(json_str.contains("chk-abc123"));
    }

    #[test]
    fn test_checkpoint_response_restored_serialization() {
        let response = CheckpointResponse::Restored {
            checkpoint_id: "chk-def456".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("Restored"));
        assert!(json_str.contains("chk-def456"));
    }

    #[test]
    fn test_checkpoint_response_list_empty_serialization() {
        let response = CheckpointResponse::List {
            checkpoints: vec![],
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("List"));
        assert!(json_str.contains("checkpoints"));
    }

    #[test]
    fn test_checkpoint_response_list_with_checkpoints_serialization() {
        let response = CheckpointResponse::List {
            checkpoints: vec![
                CheckpointInfo {
                    id: "chk-1".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    session_count: 3,
                    description: Some("first checkpoint".to_string()),
                },
                CheckpointInfo {
                    id: "chk-2".to_string(),
                    created_at: "2024-01-02T00:00:00Z".to_string(),
                    session_count: 5,
                    description: None,
                },
            ],
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok(), "serialization should succeed");
        let Ok(json_str) = json else { return };
        assert!(json_str.contains("chk-1"));
        assert!(json_str.contains("chk-2"));
        assert!(json_str.contains("first checkpoint"));
        assert!(json_str.contains("\"session_count\":3"));
        assert!(json_str.contains("\"session_count\":5"));
    }

    // ── CheckpointInfo Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_info_with_description() {
        let info = CheckpointInfo {
            id: "chk-test".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 10,
            description: Some("Test description".to_string()),
        };
        assert_eq!(info.id, "chk-test");
        assert_eq!(info.created_at, "2024-06-15T10:30:00Z");
        assert_eq!(info.session_count, 10);
        assert_eq!(info.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_checkpoint_info_without_description() {
        let info = CheckpointInfo {
            id: "chk-test2".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 0,
            description: None,
        };
        assert!(info.description.is_none());
        assert_eq!(info.session_count, 0);
    }

    #[test]
    fn test_checkpoint_info_clone() {
        let info = CheckpointInfo {
            id: "chk-orig".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            session_count: 5,
            description: Some("original".to_string()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.created_at, info.created_at);
        assert_eq!(cloned.session_count, info.session_count);
        assert_eq!(cloned.description, info.description);
    }

    // ── CheckpointArgs Tests ─────────────────────────────────────────────

    #[test]
    fn test_checkpoint_args_create() {
        let args = CheckpointArgs {
            action: CheckpointAction::Create {
                description: Some("new checkpoint".to_string()),
            },
            format: OutputFormat::Human,
        };
        assert!(matches!(args.action, CheckpointAction::Create { .. }));
        assert!(!args.format.is_json());
    }

    #[test]
    fn test_checkpoint_args_list_json() {
        let args = CheckpointArgs {
            action: CheckpointAction::List,
            format: OutputFormat::Json,
        };
        assert!(matches!(args.action, CheckpointAction::List));
        assert!(args.format.is_json());
    }

    // ── Helper Function Tests ────────────────────────────────────────────

    #[test]
    fn test_generate_checkpoint_id_format() {
        let id_result = generate_checkpoint_id();
        assert!(id_result.is_ok(), "id generation should succeed");
        let Ok(id) = id_result else { return };
        assert!(id.starts_with("chk-"));
        // Should be a valid hex string after the prefix
        let hex_part = &id[4..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_checkpoint_id_uniqueness() {
        let Ok(id1) = generate_checkpoint_id() else {
            return;
        };
        // Sleep briefly to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));
        let Ok(id2) = generate_checkpoint_id() else {
            return;
        };
        // IDs should be different (based on timestamp)
        assert_ne!(id1, id2);
    }

    // ── Schema Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_checkpoint_schema_contains_required_tables() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoint_sessions"));
    }

    #[test]
    fn test_checkpoint_schema_has_foreign_key() {
        assert!(CHECKPOINT_SCHEMA.contains("FOREIGN KEY (checkpoint_id)"));
        assert!(CHECKPOINT_SCHEMA.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_checkpoint_schema_has_index() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id"));
    }

    #[test]
    fn test_checkpoint_schema_checkpoints_columns() {
        assert!(CHECKPOINT_SCHEMA.contains("checkpoint_id TEXT UNIQUE NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("description TEXT"));
        assert!(CHECKPOINT_SCHEMA.contains("created_at INTEGER NOT NULL"));
    }

    #[test]
    fn test_checkpoint_schema_sessions_columns() {
        assert!(CHECKPOINT_SCHEMA.contains("session_name TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("status TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("workspace_path TEXT NOT NULL"));
        assert!(CHECKPOINT_SCHEMA.contains("branch TEXT"));
        assert!(CHECKPOINT_SCHEMA.contains("metadata TEXT"));
    }
}

//! Checkpoint command - save and restore full session state snapshots
//!
//! Provides atomic save/restore of all session state, enabling rollback
//! to known-good configurations.

use std::{collections::BTreeSet, str::FromStr};

use anyhow::{Context, Result};
use chrono::TimeZone;
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use sqlx::Row;
use zjj_core::OutputFormat;

use crate::{
    commands::get_session_db,
    db::SessionDb,
    session::{validate_session_name, SessionStatus},
};

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

CREATE TABLE IF NOT EXISTS checkpoint_schema_meta (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    version INTEGER NOT NULL
);
";

const CHECKPOINT_SCHEMA_VERSION: i64 = 2;

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
        .context("Failed to create checkpoint tables")?;

    ensure_checkpoint_schema(pool).await
}

async fn ensure_checkpoint_schema(pool: &sqlx::SqlitePool) -> Result<()> {
    recover_interrupted_checkpoint_migration(pool).await?;

    let current_version: Option<i64> =
        sqlx::query("SELECT version FROM checkpoint_schema_meta WHERE id = 1")
            .fetch_optional(pool)
            .await
            .context("Failed to read checkpoint schema version")?
            .map(|row| row.try_get("version"))
            .transpose()
            .context("Failed to parse checkpoint schema version")?;

    if let Some(version) = current_version {
        if version > CHECKPOINT_SCHEMA_VERSION {
            anyhow::bail!(
                "Checkpoint schema version mismatch: database has version {version}, but zjj expects version {CHECKPOINT_SCHEMA_VERSION}"
            );
        }
    }

    let needs_legacy_fk_migration = has_legacy_fk_to_sessions(pool).await?;

    if needs_legacy_fk_migration {
        migrate_legacy_checkpoint_fk(pool).await?;
    }

    sqlx::query(
        "INSERT INTO checkpoint_schema_meta (id, version) VALUES (1, ?)
         ON CONFLICT(id) DO UPDATE SET version = excluded.version",
    )
    .bind(CHECKPOINT_SCHEMA_VERSION)
    .execute(pool)
    .await
    .context("Failed to set checkpoint schema version")?;

    Ok(())
}

async fn table_exists(pool: &sqlx::SqlitePool, table_name: &str) -> Result<bool> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ? LIMIT 1")
            .bind(table_name)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("Failed to inspect sqlite_master for table '{table_name}'"))?;

    Ok(row.is_some())
}

async fn recover_interrupted_checkpoint_migration(pool: &sqlx::SqlitePool) -> Result<()> {
    let old_exists = table_exists(pool, "checkpoints_old").await?;
    if !old_exists {
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin interrupted checkpoint migration recovery")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            checkpoint_id TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to ensure checkpoints table during migration recovery")?;

    sqlx::query(
        "INSERT INTO checkpoints (id, checkpoint_id, description, created_at)
         SELECT id, checkpoint_id, description, created_at FROM checkpoints_old
         WHERE NOT EXISTS (
             SELECT 1 FROM checkpoints c WHERE c.checkpoint_id = checkpoints_old.checkpoint_id
         )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to replay checkpoint rows from checkpoints_old")?;

    sqlx::query("DROP TABLE checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to drop checkpoints_old during migration recovery")?;

    tx.commit()
        .await
        .context("Failed to commit interrupted checkpoint migration recovery")
}

async fn has_legacy_fk_to_sessions(pool: &sqlx::SqlitePool) -> Result<bool> {
    let checkpoint_fks = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
        .fetch_all(pool)
        .await
        .context("Failed to inspect checkpoints foreign keys")?;

    Ok(checkpoint_fks.iter().any(|row| {
        let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
        table_name == "sessions"
    }))
}

async fn migrate_legacy_checkpoint_fk(pool: &sqlx::SqlitePool) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .context("Failed to begin checkpoint schema migration")?;

    sqlx::query("ALTER TABLE checkpoints RENAME TO checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to rename legacy checkpoints table")?;

    sqlx::query(
        "CREATE TABLE checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            checkpoint_id TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to create migrated checkpoints table")?;

    sqlx::query(
        "INSERT INTO checkpoints (id, checkpoint_id, description, created_at)
         SELECT id, checkpoint_id, description, created_at FROM checkpoints_old",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to copy checkpoint rows during migration")?;

    sqlx::query("DROP TABLE checkpoints_old")
        .execute(&mut *tx)
        .await
        .context("Failed to drop legacy checkpoints table")?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_checkpoint_sessions_id ON checkpoint_sessions(checkpoint_id)")
        .execute(&mut *tx)
        .await
        .context("Failed to ensure checkpoint index")?;

    tx.commit()
        .await
        .context("Failed to commit checkpoint schema migration")
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

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Verify checkpoint exists
    let exists: bool = sqlx::query("SELECT 1 FROM checkpoints WHERE checkpoint_id = ?")
        .bind(checkpoint_id)
        .fetch_optional(&mut *tx)
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
    .fetch_all(&mut *tx)
    .await
    .context("Failed to fetch checkpoint sessions")?;

    let invalid_names = rows.iter().try_fold(Vec::new(), |mut names, row| {
        let name: String = row
            .try_get("session_name")
            .context("Missing session_name while validating checkpoint rows")?;
        if validate_session_name(&name).is_err() {
            names.push(name);
        }
        Ok::<Vec<String>, anyhow::Error>(names)
    })?;

    let duplicate_names = rows
        .iter()
        .try_fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut seen, mut duplicates), row| {
                let name: String = row
                    .try_get("session_name")
                    .context("Missing session_name while checking duplicate checkpoint rows")?;
                if !seen.insert(name.clone()) {
                    duplicates.insert(name);
                }
                Ok::<(BTreeSet<String>, BTreeSet<String>), anyhow::Error>((seen, duplicates))
            },
        )?
        .1
        .into_iter()
        .collect::<Vec<_>>();

    let invalid_statuses = rows.iter().try_fold(Vec::new(), |mut statuses, row| {
        let status: String = row
            .try_get("status")
            .context("Missing status while validating checkpoint rows")?;
        if SessionStatus::from_str(&status).is_err() {
            statuses.push(status);
        }
        Ok::<Vec<String>, anyhow::Error>(statuses)
    })?;

    if !invalid_names.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains invalid session names; restore aborted before deleting sessions: {}",
            checkpoint_id,
            invalid_names.join(", ")
        );
    }

    if !duplicate_names.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains duplicate session names; restore aborted before deleting sessions: {}",
            checkpoint_id,
            duplicate_names.join(", ")
        );
    }

    if !invalid_statuses.is_empty() {
        anyhow::bail!(
            "Checkpoint '{}' contains invalid statuses; restore aborted: {}",
            checkpoint_id,
            invalid_statuses.join(", ")
        );
    }

    // FIX: Removed DELETE FROM sessions to prevent data loss during restore
    // Now using INSERT OR REPLACE to preserve existing sessions not in checkpoint

    // Track statistics for reporting
    let (mut total_sessions, mut restored_count) = (0usize, 0usize);

    for row in rows {
        total_sessions += 1;

        let name: String = row
            .try_get("session_name")
            .context("Missing session_name")?;
        let status: String = row.try_get("status").context("Missing status")?;
        let workspace_path: String = row
            .try_get("workspace_path")
            .context("Missing workspace_path")?;
        let branch: Option<String> = row.try_get("branch").context("Missing branch")?;
        let metadata: Option<String> = row.try_get("metadata").context("Missing metadata")?;

        // FIX: Use INSERT with ON CONFLICT to handle both new and existing sessions
        // This preserves existing sessions that aren't in the checkpoint
        sqlx::query(
            "INSERT INTO sessions (name, status, workspace_path, branch, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))
             ON CONFLICT(name) DO UPDATE SET
                status = excluded.status,
                workspace_path = excluded.workspace_path,
                branch = excluded.branch,
                metadata = excluded.metadata,
                updated_at = strftime('%s', 'now')",
        )
        .bind(&name)
        .bind(&status)
        .bind(&workspace_path)
        .bind(&branch)
        .bind(&metadata)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("Failed to restore session '{name}'"))?;
        restored_count += 1;
    }

    tx.commit()
        .await
        .context("Failed to commit restore transaction")?;

    debug_assert_eq!(restored_count, total_sessions);

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
    fn test_checkpoint_schema_has_version_table() {
        assert!(CHECKPOINT_SCHEMA.contains("CREATE TABLE IF NOT EXISTS checkpoint_schema_meta"));
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

    // ── Session Name Validation Tests (zjj-3xuo) ───────────────────────────

    #[tokio::test]
    async fn test_restore_checkpoint_with_valid_session_names() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with valid session names
        let checkpoint_id = "chk-test-valid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        // Insert sessions with valid names
        let valid_sessions = vec![
            ("my-session", "active", "/path/to/my-session"),
            ("feature-auth", "active", "/path/to/feature-auth"),
            ("test_123", "paused", "/path/to/test_123"),
        ];

        for (name, status, path) in valid_sessions {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .execute(db.pool())
            .await
            .expect("Failed to insert session");
        }

        // Restore the checkpoint
        let result = restore_checkpoint(&db, checkpoint_id).await;

        // Should succeed
        assert!(result.is_ok(), "Restore with valid names should succeed");

        // Verify all sessions were restored
        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(sessions.len(), 3, "All 3 valid sessions should be restored");
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_invalid_session_names_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with both valid and invalid session names
        let checkpoint_id = "chk-test-invalid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        // Insert a mix of valid and invalid session names
        let test_cases = vec![
            ("valid-session", "active", "/path/to/valid"), // Valid
            ("", "active", "/path/to/empty"),              // Invalid: empty
            ("  ", "active", "/path/to/space"),            // Invalid: whitespace
            ("../../etc/passwd", "active", "/path/to/traverse"), // Invalid: path traversal
            ("feature-auth", "active", "/path/to/feature"), // Valid
            ("\t", "active", "/path/to/tab"),              // Invalid: tab
            ("123-starts-with-digit", "active", "/path/to/digit"), // Invalid: starts with digit
        ];

        for (name, status, path) in test_cases {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .execute(db.pool())
            .await
            .expect("Failed to insert session");
        }

        // Seed an existing valid session; restore must not delete it on validation failure
        db.create("existing-safe", "/path/to/existing")
            .await
            .expect("Failed to seed existing session");

        // Restore the checkpoint
        let result = restore_checkpoint(&db, checkpoint_id).await;

        // Should fail before deleting current sessions
        assert!(
            result.is_err(),
            "Restore should fail with invalid checkpoint names"
        );

        // Verify existing sessions remain untouched
        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(
            sessions.len(),
            1,
            "Restore failure should not wipe existing sessions"
        );
        assert_eq!(sessions[0].name, "existing-safe");
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_only_invalid_names_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with only invalid session names
        let checkpoint_id = "chk-test-all-invalid";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        // Insert only invalid session names
        let invalid_sessions = vec![
            ("", "active", "/path/to/empty"),
            ("  ", "active", "/path/to/space"),
            ("../../etc/passwd", "active", "/path/to/traverse"),
        ];

        for (name, status, path) in invalid_sessions {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind(name)
            .bind(status)
            .bind(path)
            .execute(db.pool())
            .await
            .expect("Failed to insert session");
        }

        // Seed an existing valid session; restore must not delete it on validation failure
        db.create("existing-safe", "/path/to/existing")
            .await
            .expect("Failed to seed existing session");

        // Restore the checkpoint
        let result = restore_checkpoint(&db, checkpoint_id).await;

        // Should fail before deleting current sessions
        assert!(
            result.is_err(),
            "Restore should fail when checkpoint contains only invalid names"
        );

        // Verify existing sessions remain untouched
        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(
            sessions.len(),
            1,
            "Restore failure should not wipe existing sessions"
        );
        assert_eq!(sessions[0].name, "existing-safe");
    }

    #[tokio::test]
    async fn test_restore_checkpoint_preserves_valid_session_data() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        // Ensure checkpoint tables exist
        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        // Create a checkpoint with detailed session data
        let checkpoint_id = "chk-test-preserve";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        // Insert session with all fields
        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path, branch, metadata)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(checkpoint_id)
        .bind("my-session")
        .bind("active")
        .bind("/workspace/path")
        .bind("main")
        .bind(r#"{"bead_id": "zjj-abc123"}"#)
        .execute(db.pool())
        .await
        .expect("Failed to insert session");

        // Restore the checkpoint
        restore_checkpoint(&db, checkpoint_id)
            .await
            .expect("Restore should succeed");

        // Verify the session was restored with all data intact
        let session = db
            .get("my-session")
            .await
            .expect("Failed to get session")
            .expect("Session should exist");

        assert_eq!(session.name, "my-session");
        assert_eq!(session.workspace_path, "/workspace/path");
        assert_eq!(session.branch, Some("main".to_string()));
        assert!(session.metadata.is_some());
    }

    #[tokio::test]
    async fn test_restore_checkpoint_with_invalid_names_preserves_all_existing_sessions() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        db.create("keep-one", "/path/to/keep-one")
            .await
            .expect("Failed to seed keep-one");
        db.create("keep-two", "/path/to/keep-two")
            .await
            .expect("Failed to seed keep-two");

        let checkpoint_id = "chk-test-invalid-preserve-all";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
             VALUES (?, ?, ?, ?)",
        )
        .bind(checkpoint_id)
        .bind("../../etc/passwd")
        .bind("active")
        .bind("/path/to/invalid")
        .execute(db.pool())
        .await
        .expect("Failed to insert invalid checkpoint session");

        let result = restore_checkpoint(&db, checkpoint_id).await;
        assert!(
            result.is_err(),
            "Restore should fail on invalid session names"
        );

        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(sessions.len(), 2, "All existing sessions must remain");
        assert!(sessions.iter().any(|session| session.name == "keep-one"));
        assert!(sessions.iter().any(|session| session.name == "keep-two"));
    }

    #[tokio::test]
    async fn test_restore_checkpoint_duplicate_names_rolls_back_without_data_loss() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        db.create("existing-safe", "/path/to/existing")
            .await
            .expect("Failed to seed existing session");

        let checkpoint_id = "chk-test-duplicate-rollback";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        for path in ["/path/to/one", "/path/to/two"] {
            sqlx::query(
                "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(checkpoint_id)
            .bind("dup-session")
            .bind("active")
            .bind(path)
            .execute(db.pool())
            .await
            .expect("Failed to insert duplicate checkpoint session rows");
        }

        let result = restore_checkpoint(&db, checkpoint_id).await;
        assert!(
            result.is_err(),
            "Restore should fail on duplicate session inserts"
        );
        let error_message = result.err().map_or_else(String::new, |err| err.to_string());
        assert!(
            error_message.contains("duplicate session names"),
            "Restore should fail with duplicate preflight validation error"
        );

        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(
            sessions.len(),
            1,
            "Rollback must preserve pre-restore sessions"
        );
        assert_eq!(sessions[0].name, "existing-safe");
    }

    #[tokio::test]
    async fn test_restore_checkpoint_invalid_status_aborts_without_delete() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to create checkpoint tables");

        db.create("existing-safe", "/path/to/existing")
            .await
            .expect("Failed to seed existing session");

        let checkpoint_id = "chk-test-invalid-status";
        sqlx::query("INSERT INTO checkpoints (checkpoint_id) VALUES (?)")
            .bind(checkpoint_id)
            .execute(db.pool())
            .await
            .expect("Failed to create checkpoint");

        sqlx::query(
            "INSERT INTO checkpoint_sessions (checkpoint_id, session_name, status, workspace_path)
             VALUES (?, ?, ?, ?)",
        )
        .bind(checkpoint_id)
        .bind("restored-session")
        .bind("bad-status")
        .bind("/path/to/restored")
        .execute(db.pool())
        .await
        .expect("Failed to insert invalid checkpoint status");

        let result = restore_checkpoint(&db, checkpoint_id).await;
        assert!(result.is_err(), "Restore should fail on invalid status");

        let sessions = db.list(None).await.expect("Failed to list sessions");
        assert_eq!(sessions.len(), 1, "Existing session must remain untouched");
        assert_eq!(sessions[0].name, "existing-safe");
    }

    #[tokio::test]
    async fn test_legacy_checkpoint_schema_migration_removes_session_fk() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        sqlx::query("DROP TABLE IF EXISTS checkpoint_sessions")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoint_sessions");
        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoints");

        sqlx::query(
            "CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                session_id INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(db.pool())
        .await
        .expect("Failed to create legacy checkpoints table");

        ensure_checkpoint_tables(&db)
            .await
            .expect("Failed to run checkpoint schema ensure/migration");

        let fk_rows = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
            .fetch_all(db.pool())
            .await
            .expect("Failed to read checkpoint foreign keys");

        let has_session_fk = fk_rows.iter().any(|row| {
            let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
            table_name == "sessions"
        });

        assert!(
            !has_session_fk,
            "Legacy FK to sessions should be removed by migration"
        );

        let version: i64 = sqlx::query("SELECT version FROM checkpoint_schema_meta WHERE id = 1")
            .fetch_one(db.pool())
            .await
            .expect("Failed to read checkpoint schema version")
            .try_get("version")
            .expect("Failed to parse checkpoint schema version");

        assert_eq!(
            version, CHECKPOINT_SCHEMA_VERSION,
            "Checkpoint schema version should be updated after migration"
        );
    }

    #[tokio::test]
    async fn test_checkpoint_schema_migration_is_idempotent_across_repeated_runs() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        sqlx::query("DROP TABLE IF EXISTS checkpoint_sessions")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoint_sessions");
        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoints");

        sqlx::query(
            "CREATE TABLE checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                session_id INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
        )
        .execute(db.pool())
        .await
        .expect("Failed to create legacy checkpoints table");

        ensure_checkpoint_tables(&db)
            .await
            .expect("First migration run should succeed");
        ensure_checkpoint_tables(&db)
            .await
            .expect("Second migration run should be idempotent");

        let fk_rows = sqlx::query("PRAGMA foreign_key_list(checkpoints)")
            .fetch_all(db.pool())
            .await
            .expect("Failed to read checkpoint foreign keys");

        let has_session_fk = fk_rows.iter().any(|row| {
            let table_name: String = row.try_get("table").map_or(String::new(), |value| value);
            table_name == "sessions"
        });
        assert!(
            !has_session_fk,
            "session FK should stay removed after re-run"
        );
    }

    #[tokio::test]
    async fn test_checkpoint_schema_recovery_from_interrupted_rename_copy_drop_window() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path)
            .await
            .expect("Failed to create database");

        sqlx::query("DROP TABLE IF EXISTS checkpoints")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoints");
        sqlx::query("DROP TABLE IF EXISTS checkpoints_old")
            .execute(db.pool())
            .await
            .expect("Failed to drop checkpoints_old");

        sqlx::query(
            "CREATE TABLE checkpoints_old (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                checkpoint_id TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(db.pool())
        .await
        .expect("Failed to create checkpoints_old");

        sqlx::query(
            "INSERT INTO checkpoints_old (checkpoint_id, description, created_at)
             VALUES ('chk-interrupted', 'from-old', strftime('%s', 'now'))",
        )
        .execute(db.pool())
        .await
        .expect("Failed to seed checkpoints_old");

        ensure_checkpoint_tables(&db)
            .await
            .expect("Recovery should succeed for interrupted migration window");

        let restored: Option<(String,)> = sqlx::query_as(
            "SELECT description FROM checkpoints WHERE checkpoint_id = 'chk-interrupted'",
        )
        .fetch_optional(db.pool())
        .await
        .expect("Failed to query restored checkpoint");
        assert_eq!(
            restored.map(|row| row.0),
            Some("from-old".to_string()),
            "Interrupted migration data should be replayed into checkpoints"
        );

        let old_exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'checkpoints_old' LIMIT 1",
        )
        .fetch_optional(db.pool())
        .await
        .expect("Failed to check checkpoints_old presence");
        assert!(
            old_exists.is_none(),
            "checkpoints_old should be dropped after recovery"
        );
    }
}

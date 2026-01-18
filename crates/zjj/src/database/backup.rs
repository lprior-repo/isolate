//! Backup and restore operations for session database

use std::path::Path;

use sqlx::SqlitePool;
use zjj_core::{Error, Result};

use super::{query::query_sessions, schema};
use crate::session::Session;

/// Trait for backup and restore operations
#[allow(async_fn_in_trait)]
pub trait BackupOps {
    /// Get reference to the connection pool
    fn pool(&self) -> &SqlitePool;

    /// Create a backup of the database
    ///
    /// # Errors
    ///
    /// Returns error if backup cannot be written
    async fn backup(&self, backup_path: &Path) -> Result<()> {
        query_sessions(self.pool(), None)
            .await
            .and_then(|sessions| serialize_sessions(&sessions))
            .and_then(|json| write_backup(backup_path, &json))
    }

    /// Restore database from a backup file
    ///
    /// # Errors
    ///
    /// Returns error if backup is invalid or restore fails
    async fn restore(&self, backup_path: &Path) -> Result<()> {
        let json = read_backup(backup_path)?;
        let sessions = deserialize_sessions(&json)?;
        rebuild_database(self.pool(), sessions).await
    }
}

/// Verify integrity of a backup file
///
/// # Errors
///
/// Returns error if backup file is invalid
pub fn verify_backup(backup_path: &Path) -> Result<usize> {
    read_backup(backup_path)
        .and_then(|json| deserialize_sessions(&json))
        .map(|sessions| sessions.len())
}

// === PURE FUNCTIONS (Functional Core) ===

/// Serialize sessions to JSON
fn serialize_sessions(sessions: &[Session]) -> Result<String> {
    serde_json::to_string_pretty(sessions)
        .map_err(|e| Error::parse_error(format!("Failed to serialize sessions: {e}")))
}

/// Deserialize sessions from JSON
fn deserialize_sessions(json: &str) -> Result<Vec<Session>> {
    serde_json::from_str(json)
        .map_err(|e| Error::parse_error(format!("Failed to parse backup file: {e}")))
}

/// Write backup to file
fn write_backup(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .map_err(|e| Error::io_error(format!("Failed to write backup file: {e}")))
}

/// Read backup from file
fn read_backup(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::io_error(format!("Failed to read backup file: {e}")))
}

// === IMPERATIVE SHELL (Database Side Effects) ===

/// Rebuild database from sessions list (ATOMIC via transaction)
///
/// CRITICAL FIX: This function now wraps all operations in a single transaction
/// to prevent data loss if any step fails. Previously, if the insert step failed
/// after dropping the table, all data would be permanently lost.
async fn rebuild_database(pool: &SqlitePool, sessions: Vec<Session>) -> Result<()> {
    // Begin transaction for atomic rebuild (fixes non-atomic restore bug)
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| Error::database_error(format!("Failed to begin rebuild transaction: {e}")))?;

    // Step 1: Drop existing schema
    schema::drop_existing_schema_tx(&mut tx).await?;

    // Step 2: Recreate schema
    schema::init_schema_tx(&mut tx).await?;

    // Step 3: Insert all sessions
    insert_all_sessions_tx(&mut tx, sessions).await?;

    // Commit transaction - all or nothing
    tx.commit()
        .await
        .map_err(|e| Error::database_error(format!("Failed to commit rebuild transaction: {e}")))?;

    Ok(())
}

/// Insert all sessions into database (transaction version)
async fn insert_all_sessions_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    sessions: Vec<Session>,
) -> Result<()> {
    // Sequential async iteration required for database transaction safety
    // Cannot use functional patterns: async/await with ? early return in transaction
    for session in sessions {
        insert_session_from_backup_tx(tx, &session).await?;
    }
    Ok(())
}

/// Insert a session from backup (transaction version)
async fn insert_session_from_backup_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    session: &Session,
) -> Result<()> {
    let metadata_json = session
        .metadata
        .as_ref()
        .map(|m| {
            serde_json::to_string(m)
                .map_err(|e| Error::parse_error(format!("Failed to serialize metadata: {e}")))
        })
        .transpose()?;

    sqlx::query(
        "INSERT INTO sessions (name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&session.name)
    .bind(session.status.to_string())
    .bind(&session.workspace_path)
    .bind(&session.branch)
    .bind(session.created_at.cast_signed())
    .bind(session.updated_at.cast_signed())
    .bind(session.last_synced.map(u64::cast_signed))
    .bind(metadata_json)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(|e| Error::database_error(format!("Failed to insert session during rebuild: {e}")))
}

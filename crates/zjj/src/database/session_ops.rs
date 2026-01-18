//! CRUD operations for session management

use sqlx::SqlitePool;
use zjj_core::{Error, Result};

use super::query::{build_session, get_current_timestamp, query_session_by_name, query_sessions};
use crate::session::{Session, SessionStatus, SessionUpdate};

/// Trait for session database operations
#[allow(async_fn_in_trait)]
pub trait SessionOps {
    /// Get reference to the connection pool
    fn pool(&self) -> &SqlitePool;

    /// Create a new session
    ///
    /// # Errors
    ///
    /// Returns error if session name already exists or database operation fails
    async fn create(&self, name: &str, workspace_path: &str) -> Result<Session> {
        let now = get_current_timestamp()?;
        let status = SessionStatus::Creating;

        insert_session(self.pool(), name, &status, workspace_path, now)
            .await
            .map(|id| build_session(id, name, status, workspace_path, now))
    }

    /// Get a session by name
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    async fn get(&self, name: &str) -> Result<Option<Session>> {
        query_session_by_name(self.pool(), name).await
    }

    /// Update an existing session
    ///
    /// # Errors
    ///
    /// Returns error if database update fails
    async fn update(&self, name: &str, update: SessionUpdate) -> Result<()> {
        update_session(self.pool(), name, update).await
    }

    /// Delete a session by name
    ///
    /// Returns `true` if session was deleted, `false` if it didn't exist
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn delete(&self, name: &str) -> Result<bool> {
        delete_session(self.pool(), name).await
    }

    /// List all sessions, optionally filtered by status
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    async fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        query_sessions(self.pool(), status_filter).await
    }
}

/// Insert a new session into database
async fn insert_session(
    pool: &SqlitePool,
    name: &str,
    status: &SessionStatus,
    workspace_path: &str,
    timestamp: u64,
) -> Result<i64> {
    sqlx::query(
        "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(status.to_string())
    .bind(workspace_path)
    .bind(timestamp.cast_signed())
    .bind(timestamp.cast_signed())
    .execute(pool)
    .await
    .map(|result| result.last_insert_rowid())
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("unique") {
            Error::database_error(format!("Session '{name}' already exists"))
        } else {
            Error::database_error(format!("Failed to create session: {e}"))
        }
    })
}

/// Update a session in the database
async fn update_session(pool: &SqlitePool, name: &str, update: SessionUpdate) -> Result<()> {
    let updates = build_update_clauses(&update)?;

    if updates.is_empty() {
        return Ok(());
    }

    let (sql, values) = build_update_query(updates, name);
    execute_update(pool, &sql, values).await
}

/// Build update clauses from `SessionUpdate`
fn build_update_clauses(update: &SessionUpdate) -> Result<Vec<(&'static str, String)>> {
    // Add metadata clause if present (can error, so handled separately)
    let metadata_clause = update
        .metadata
        .as_ref()
        .map(|m| {
            serde_json::to_string(m)
                .map(|json| ("metadata", json))
                .map_err(|e| Error::parse_error(format!("Failed to serialize metadata: {e}")))
        })
        .transpose()?;

    // Combine base clauses with optional metadata clause without intermediate collection
    Ok([
        update.status.as_ref().map(|s| ("status", s.to_string())),
        update.branch.as_deref().map(|b| ("branch", b.to_string())),
        update.last_synced.map(|ls| ("last_synced", ls.to_string())),
    ]
    .into_iter()
    .flatten()
    .chain(metadata_clause)
    .collect())
}

/// Build SQL UPDATE query from clauses (consumes clauses to avoid cloning values)
fn build_update_query(clauses: Vec<(&str, String)>, name: &str) -> (String, Vec<String>) {
    let set_clauses: Vec<String> = clauses
        .iter()
        .map(|(field, _)| format!("{field} = ?"))
        .collect();

    let sql = format!(
        "UPDATE sessions SET {} WHERE name = ?",
        set_clauses.join(", ")
    );

    // Consume clauses to extract values without cloning, append name at end
    let values: Vec<String> = clauses
        .into_iter()
        .map(|(_, value)| value)
        .chain(std::iter::once(name.to_string()))
        .collect();

    (sql, values)
}

/// Execute UPDATE query
async fn execute_update(pool: &SqlitePool, sql: &str, values: Vec<String>) -> Result<()> {
    // Iterative builder pattern acceptable: sqlx query builder from external library
    // Functional fold would work but this is clearer for builder pattern
    let mut query = sqlx::query(sql);
    for value in values {
        query = query.bind(value);
    }

    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| Error::database_error(format!("Failed to update session: {e}")))
}

/// Delete a session from the database
async fn delete_session(pool: &SqlitePool, name: &str) -> Result<bool> {
    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .map_err(|e| Error::database_error(format!("Failed to delete session: {e}")))
}

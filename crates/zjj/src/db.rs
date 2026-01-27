//! Database operations for session persistence using SQLx
//!
//! This module provides async SQLite-based persistence with:
//! - Connection pooling (no Arc<Mutex<>>)
//! - Zero unwraps, zero panics
//! - Simple embedded schema (no migration files)
//! - Pure functional patterns with Railway-Oriented Programming

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{path::Path, str::FromStr, time::SystemTime};

use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use zjj_core::{Error, Result};

use crate::session::{Session, SessionStatus, SessionUpdate};

/// Database schema as SQL string - executed once on init
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_name ON sessions(name);

CREATE TRIGGER IF NOT EXISTS update_timestamp
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;
"#;

/// Database wrapper for session storage with connection pooling
#[derive(Clone)]
pub struct SessionDb {
    pool: SqlitePool,
}

impl SessionDb {
    /// Open or create a session database at the given path
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if:
    /// - Database file cannot be opened
    /// - Schema initialization fails
    /// - Database is corrupted
    pub async fn open(path: &Path) -> Result<Self> {
        Self::open_internal(path, false).await
    }

    /// Create or open database (for init command only)
    pub async fn create_or_open(path: &Path) -> Result<Self> {
        Self::open_internal(path, true).await
    }

    async fn open_internal(path: &Path, allow_create: bool) -> Result<Self> {
        validate_database_path(path, allow_create)?;

        let db_url = format!("sqlite:{}", path.display());

        let pool = create_connection_pool(&db_url).await?;
        init_schema(&pool).await?;
        Ok(Self { pool })
    }

    /// Create a new session
    ///
    /// # Errors
    ///
    /// Returns error if session name already exists or database operation fails
    pub async fn create(&self, name: &str, workspace_path: &str) -> Result<Session> {
        let now = get_current_timestamp()?;
        let status = SessionStatus::Creating;

        insert_session(&self.pool, name, &status, workspace_path, now)
            .await
            .map(|id| build_session(id, name, status, workspace_path, now))
    }

    /// Get a session by name
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get(&self, name: &str) -> Result<Option<Session>> {
        query_session_by_name(&self.pool, name).await
    }

    /// Update an existing session
    ///
    /// # Errors
    ///
    /// Returns error if database update fails
    pub async fn update(&self, name: &str, update: SessionUpdate) -> Result<()> {
        update_session(&self.pool, name, update).await
    }

    /// Delete a session by name
    ///
    /// Returns `true` if session was deleted, `false` if it didn't exist
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn delete(&self, name: &str) -> Result<bool> {
        delete_session(&self.pool, name).await
    }

    /// List all sessions, optionally filtered by status
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        query_sessions(&self.pool, status_filter).await
    }

    /// Create a backup of the database
    ///
    /// # Errors
    ///
    /// Returns error if backup cannot be written
    pub async fn backup(&self, backup_path: &Path) -> Result<()> {
        self.list(None)
            .await
            .and_then(|sessions| serialize_sessions(&sessions))
            .and_then(|json| write_backup(backup_path, &json))
    }

    /// Restore database from a backup file
    ///
    /// # Errors
    ///
    /// Returns error if backup is invalid or restore fails
    pub async fn restore(&self, backup_path: &Path) -> Result<()> {
        let json = read_backup(backup_path)?;
        let sessions = deserialize_sessions(&json)?;
        rebuild_database(&self.pool, sessions).await
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

    /// Rebuild database from a list of sessions
    ///
    /// # Errors
    ///
    /// Returns error if rebuild fails
    pub async fn rebuild_from_sessions(&self, sessions: Vec<Session>) -> Result<()> {
        rebuild_database(&self.pool, sessions).await
    }

    // === BLOCKING WRAPPERS ===
    // These provide synchronous versions of async methods for use in
    // non-async contexts (like CLI commands). Each creates a runtime
    // and blocks on the corresponding async method.

    /// Blocking version of [`open`](Self::open)
    pub fn open_blocking(path: &Path) -> Result<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(Self::open(path))
    }

    /// Blocking version of [`create_or_open`](Self::create_or_open)
    pub fn create_or_open_blocking(path: &Path) -> Result<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(Self::create_or_open(path))
    }

    /// Blocking version of [`create`](Self::create)
    pub fn create_blocking(&self, name: &str, workspace_path: &str) -> Result<Session> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.create(name, workspace_path))
    }

    /// Blocking version of [`get`](Self::get)
    pub fn get_blocking(&self, name: &str) -> Result<Option<Session>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.get(name))
    }

    /// Blocking version of [`update`](Self::update)
    pub fn update_blocking(&self, name: &str, update: SessionUpdate) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.update(name, update))
    }

    /// Blocking version of [`delete`](Self::delete)
    pub fn delete_blocking(&self, name: &str) -> Result<bool> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.delete(name))
    }

    /// Blocking version of [`list`](Self::list)
    pub fn list_blocking(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.list(status_filter))
    }

    /// Blocking version of [`backup`](Self::backup)
    pub fn backup_blocking(&self, backup_path: &Path) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.backup(backup_path))
    }

    /// Blocking version of [`restore`](Self::restore)
    pub fn restore_blocking(&self, backup_path: &Path) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.restore(backup_path))
    }

    /// Blocking version of [`rebuild_from_sessions`](Self::rebuild_from_sessions)
    pub fn rebuild_from_sessions_blocking(&self, sessions: Vec<Session>) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Unknown(format!("Failed to create runtime: {e}")))?;
        rt.block_on(self.rebuild_from_sessions(sessions))
    }
}

// === PURE FUNCTIONS (Functional Core) ===

/// Validate database path preconditions
fn validate_database_path(path: &Path, allow_create: bool) -> Result<()> {
    let exists = path.exists();

    if !exists && !allow_create {
        return Err(Error::DatabaseError(format!(
            "Database file does not exist: {}\n\nRun 'jjz init' to initialize ZJZ.",
            path.display()
        )));
    }

    if exists {
        validate_existing_file(path)?;
    }

    Ok(())
}

/// Validate existing database file is not empty
fn validate_existing_file(path: &Path) -> Result<()> {
    std::fs::metadata(path)
        .map_err(|e| Error::DatabaseError(format!("Failed to read database metadata: {e}")))
        .and_then(|metadata| {
            if metadata.len() == 0 {
                Err(Error::DatabaseError(format!(
                    "Database file is empty or corrupted: {}\n\nRun 'jjz init' to reinitialize.",
                    path.display()
                )))
            } else {
                Ok(())
            }
        })
}

/// Get current Unix timestamp
fn get_current_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| Error::Unknown(format!("System time error: {e}")))
}

/// Build a Session struct from components
fn build_session(
    id: i64,
    name: &str,
    status: SessionStatus,
    workspace_path: &str,
    timestamp: u64,
) -> Session {
    Session {
        id: Some(id),
        name: name.to_string(),
        status,
        workspace_path: workspace_path.to_string(),
        zellij_tab: format!("jjz:{name}"),
        branch: None,
        created_at: timestamp,
        updated_at: timestamp,
        last_synced: None,
        metadata: None,
    }
}

/// Serialize sessions to JSON
fn serialize_sessions(sessions: &[Session]) -> Result<String> {
    serde_json::to_string_pretty(sessions)
        .map_err(|e| Error::ParseError(format!("Failed to serialize sessions: {e}")))
}

/// Deserialize sessions from JSON
fn deserialize_sessions(json: &str) -> Result<Vec<Session>> {
    serde_json::from_str(json)
        .map_err(|e| Error::ParseError(format!("Failed to parse backup file: {e}")))
}

/// Write backup to file
fn write_backup(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .map_err(|e| Error::IoError(format!("Failed to write backup file: {e}")))
}

/// Read backup from file
fn read_backup(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::IoError(format!("Failed to read backup file: {e}")))
}

// === IMPERATIVE SHELL (Database Side Effects) ===

/// Create SQLite connection pool
async fn create_connection_pool(db_url: &str) -> Result<SqlitePool> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect(db_url)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to connect to database: {e}")))
}

/// Initialize database schema
async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| Error::DatabaseError(format!("Failed to initialize schema: {e}")))
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
    .bind(timestamp as i64)
    .bind(timestamp as i64)
    .execute(pool)
    .await
    .map(|result| result.last_insert_rowid())
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("unique") {
            Error::DatabaseError(format!("Session '{name}' already exists"))
        } else {
            Error::DatabaseError(format!("Failed to create session: {e}"))
        }
    })
}

/// Query a session by name
async fn query_session_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Session>> {
    sqlx::query(
        "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
         FROM sessions WHERE name = ?"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::DatabaseError(format!("Failed to query session: {e}")))
    .and_then(|opt_row| match opt_row {
        Some(row) => parse_session_row(row).map(Some),
        None => Ok(None),
    })
}

/// Query all sessions with optional status filter
async fn query_sessions(
    pool: &SqlitePool,
    status_filter: Option<SessionStatus>,
) -> Result<Vec<Session>> {
    let rows = match status_filter {
        Some(status) => {
            sqlx::query(
                "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE status = ? ORDER BY created_at"
            )
            .bind(status.to_string())
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions ORDER BY created_at"
            )
            .fetch_all(pool)
            .await
        }
    }.map_err(|e| Error::DatabaseError(format!("Failed to query sessions: {e}")))?;

    rows.into_iter().map(parse_session_row).collect()
}

/// Parse a database row into a Session
fn parse_session_row(row: sqlx::sqlite::SqliteRow) -> Result<Session> {
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::DatabaseError(format!("Failed to read id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| Error::DatabaseError(format!("Failed to read name: {e}")))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| Error::DatabaseError(format!("Failed to read status: {e}")))?;
    let status = SessionStatus::from_str(&status_str)?;
    let workspace_path: String = row
        .try_get("workspace_path")
        .map_err(|e| Error::DatabaseError(format!("Failed to read workspace_path: {e}")))?;
    let branch: Option<String> = row
        .try_get("branch")
        .map_err(|e| Error::DatabaseError(format!("Failed to read branch: {e}")))?;
    let created_at: i64 = row
        .try_get("created_at")
        .map_err(|e| Error::DatabaseError(format!("Failed to read created_at: {e}")))?;
    let updated_at: i64 = row
        .try_get("updated_at")
        .map_err(|e| Error::DatabaseError(format!("Failed to read updated_at: {e}")))?;
    let last_synced: Option<i64> = row
        .try_get("last_synced")
        .map_err(|e| Error::DatabaseError(format!("Failed to read last_synced: {e}")))?;
    let metadata_str: Option<String> = row
        .try_get("metadata")
        .map_err(|e| Error::DatabaseError(format!("Failed to read metadata: {e}")))?;

    let metadata = metadata_str
        .map(|s| {
            serde_json::from_str(&s)
                .map_err(|e| Error::ParseError(format!("Invalid metadata JSON: {e}")))
        })
        .transpose()?;

    Ok(Session {
        id: Some(id),
        name: name.clone(),
        status,
        workspace_path,
        zellij_tab: format!("jjz:{name}"),
        branch,
        created_at: created_at as u64,
        updated_at: updated_at as u64,
        last_synced: last_synced.map(|v| v as u64),
        metadata,
    })
}

/// Update a session in the database
async fn update_session(pool: &SqlitePool, name: &str, update: SessionUpdate) -> Result<()> {
    let updates = build_update_clauses(&update)?;

    if updates.is_empty() {
        return Ok(());
    }

    let (sql, values) = build_update_query(&updates, name);
    execute_update(pool, &sql, values).await
}

/// Build update clauses from SessionUpdate
fn build_update_clauses(update: &SessionUpdate) -> Result<Vec<(&'static str, String)>> {
    let mut clauses = Vec::new();

    if let Some(ref status) = update.status {
        clauses.push(("status", status.to_string()));
    }

    if let Some(ref branch) = update.branch {
        clauses.push(("branch", branch.clone()));
    }

    if let Some(last_synced) = update.last_synced {
        clauses.push(("last_synced", last_synced.to_string()));
    }

    if let Some(ref metadata) = update.metadata {
        let json_str = serde_json::to_string(metadata)
            .map_err(|e| Error::ParseError(format!("Failed to serialize metadata: {e}")))?;
        clauses.push(("metadata", json_str));
    }

    Ok(clauses)
}

/// Build SQL UPDATE query from clauses
fn build_update_query(clauses: &[(&str, String)], name: &str) -> (String, Vec<String>) {
    let set_clauses: Vec<String> = clauses
        .iter()
        .map(|(field, _)| format!("{field} = ?"))
        .collect();

    let sql = format!(
        "UPDATE sessions SET {} WHERE name = ?",
        set_clauses.join(", ")
    );

    let mut values: Vec<String> = clauses.iter().map(|(_, value)| value.clone()).collect();
    values.push(name.to_string());

    (sql, values)
}

/// Execute UPDATE query
async fn execute_update(pool: &SqlitePool, sql: &str, values: Vec<String>) -> Result<()> {
    let mut query = sqlx::query(sql);
    for value in values {
        query = query.bind(value);
    }

    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| Error::DatabaseError(format!("Failed to update session: {e}")))
}

/// Delete a session from the database
async fn delete_session(pool: &SqlitePool, name: &str) -> Result<bool> {
    sqlx::query("DELETE FROM sessions WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .map_err(|e| Error::DatabaseError(format!("Failed to delete session: {e}")))
}

/// Rebuild database from sessions list
async fn rebuild_database(pool: &SqlitePool, sessions: Vec<Session>) -> Result<()> {
    drop_existing_schema(pool).await?;
    init_schema(pool).await?;
    insert_all_sessions(pool, sessions).await
}

/// Drop existing database schema
async fn drop_existing_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS sessions")
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to drop sessions table: {e}")))?;

    sqlx::query("DROP TRIGGER IF EXISTS update_timestamp")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| Error::DatabaseError(format!("Failed to drop update trigger: {e}")))
}

/// Insert all sessions into database
async fn insert_all_sessions(pool: &SqlitePool, sessions: Vec<Session>) -> Result<()> {
    for session in sessions {
        insert_session_from_backup(pool, &session).await?;
    }
    Ok(())
}

/// Insert a session from backup
async fn insert_session_from_backup(pool: &SqlitePool, session: &Session) -> Result<()> {
    let metadata_json = session
        .metadata
        .as_ref()
        .map(|m| {
            serde_json::to_string(m)
                .map_err(|e| Error::ParseError(format!("Failed to serialize metadata: {e}")))
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
    .bind(session.created_at as i64)
    .bind(session.updated_at as i64)
    .bind(session.last_synced.map(|v| v as i64))
    .bind(metadata_json)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| Error::DatabaseError(format!("Failed to insert session during rebuild: {e}")))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_create_session_success() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;
        let session = db.create("test-session", "/workspace").await?;

        assert_eq!(session.name, "test-session");
        assert_eq!(session.status, SessionStatus::Creating);
        assert_eq!(session.workspace_path, "/workspace");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;
        let created = db.create("test", "/path").await?;

        let retrieved = db.get("test").await?;
        assert!(retrieved.is_some());

        let session = retrieved.ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(session.name, created.name);
        Ok(())
    }

    #[tokio::test]
    async fn test_unique_constraint_enforced() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;
        let _session1 = db.create("test", "/path1").await?;

        let result = db.create("test", "/path2").await;
        assert!(result.is_err());

        if let Err(Error::DatabaseError(msg)) = result {
            assert!(msg.contains("already exists"));
        } else {
            return Err(Error::Unknown("Expected DatabaseError".to_string()));
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_backup_restore_roundtrip() -> Result<()> {
        let (db1, dir) = setup_test_db().await?;

        db1.create("session1", "/path1").await?;
        db1.create("session2", "/path2").await?;

        let backup_path = dir.path().join("backup.json");
        db1.backup(&backup_path).await?;

        let db2_path = dir.path().join("restored.db");
        let db2 = SessionDb::create_or_open(&db2_path).await?;
        db2.restore(&backup_path).await?;

        let sessions1 = db1.list(None).await?;
        let sessions2 = db2.list(None).await?;

        assert_eq!(sessions1.len(), sessions2.len());
        Ok(())
    }
}

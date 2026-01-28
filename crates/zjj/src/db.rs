//! Database operations for session persistence using `SQLx`
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

use num_traits::cast::ToPrimitive;
use sqlx::{Row, SqlitePool};
use zjj_core::{log_recovery, should_log_recovery, Error, RecoveryPolicy, Result};

use crate::session::{Session, SessionStatus, SessionUpdate};

const CURRENT_SCHEMA_VERSION: i64 = 1;

/// Database schema as SQL string - executed once on init
const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY CHECK(version = 1)
);

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
";

/// Database wrapper for session storage with connection pooling
#[derive(Clone)]
pub struct SessionDb {
    pool: SqlitePool,
}

impl SessionDb {
    /// Get a reference to the underlying connection pool
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

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
        // Ensure parent directory exists for create operations
        if let Some(parent) = path.parent() {
            if !parent.exists() && allow_create {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Error::IoError(format!("Failed to create parent directory: {e}"))
                })?;
            }
        }

        // Convert path to string for use in connection string
        let path_str = path.to_str().ok_or_else(|| {
            Error::DatabaseError("Database path contains invalid UTF-8".to_string())
        })?;

        // Check recovery policy for connection mode
        let policy = get_recovery_policy();
        let mode = if policy == RecoveryPolicy::FailFast {
            "rw" // No auto-create, fail on missing/corrupt
        } else {
            "rwc" // Auto-create and recover (existing behavior)
        };

        // Pre-flight check: Detect permission issues BEFORE SQLite tries to open
        // This prevents silent recovery from permission-denied scenarios
        if path.exists() {
            use std::fs::File;
            if let Err(e) = File::open(path) {
                return Err(Error::DatabaseError(format!(
                    "Database file is not accessible: {e}\n\n\
                     Run 'zjj doctor' to diagnose, or fix permissions manually:\n\
                     chmod 644 {}",
                    path.display()
                )));
            }
        }

        // SQLx connection string with mode parameter
        let db_url = if path.is_absolute() {
            format!("sqlite:///{path_str}?mode={mode}")
        } else {
            format!("sqlite:{path_str}?mode={mode}")
        };

        // Try to open the database, with auto-recovery for missing/corrupted files
        let pool = match create_connection_pool(&db_url).await {
            Ok(p) => p,
            Err(e) => {
                // Connection failed - check if we can recover
                match can_recover_database(path, allow_create) {
                    Ok(()) => {
                        recover_database(path)?;
                        create_connection_pool(&db_url).await?
                    }
                    Err(recovery_err) => {
                        return Err(Error::DatabaseError(format!(
                            "{e}\n\nRecovery check failed: {recovery_err}"
                        )));
                    }
                }
            }
        };

        // Try to initialize schema, with recovery for corrupted databases
        match init_schema(&pool).await {
            Ok(()) => {
                check_schema_version(&pool).await?;
                Ok(Self { pool })
            }
            Err(e) => {
                // Schema init failed - likely corrupted database
                match can_recover_database(path, allow_create) {
                    Ok(()) => {
                        recover_database(path)?;
                        let new_pool = create_connection_pool(&db_url).await?;
                        init_schema(&new_pool).await?;
                        check_schema_version(&new_pool).await?;
                        Ok(Self { pool: new_pool })
                    }
                    Err(recovery_err) => Err(Error::DatabaseError(format!(
                        "{e}\n\nRecovery check failed: {recovery_err}"
                    ))),
                }
            }
        }
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
}

// === PURE FUNCTIONS (Functional Core) ===

/// Get the current recovery policy from environment variables
fn get_recovery_policy() -> RecoveryPolicy {
    // Check ZJJ_STRICT flag first (highest priority)
    if std::env::var("ZJJ_STRICT").is_ok() {
        return RecoveryPolicy::FailFast;
    }

    // Check ZJJ_RECOVERY_POLICY env var
    if let Ok(policy_str) = std::env::var("ZJJ_RECOVERY_POLICY") {
        return policy_str.parse().unwrap_or(RecoveryPolicy::Warn);
    }

    // Default to warn policy
    RecoveryPolicy::Warn
}

/// Check if database can be auto-recovered
///
/// Recovery is allowed when:
/// - The database file exists (corruption recovery)
/// - The parent directory exists (missing file recovery)
fn can_recover_database(path: &Path, allow_create: bool) -> Result<()> {
    // Check if we can access and read the file before allowing recovery
    // This prevents recovery when DB is inaccessible (chmod 000, permission denied, etc.)
    if path.exists() {
        match path.metadata() {
            Ok(_) => {
                // File exists, check if readable
                use std::fs::File;
                match File::open(path) {
                    Ok(_) => {
                        // File is accessible, recovery is allowed
                        return Ok(());
                    }
                    Err(e) => {
                        // File exists but not readable (permission denied, etc.)
                        // Don't allow recovery - will be handled by doctor's read-only check
                        return Err(Error::DatabaseError(format!(
                            "Database file is not accessible: {e}",
                        )));
                    }
                }
            }
            Err(e) => {
                // Can't even read metadata, permission denied
                return Err(Error::DatabaseError(format!(
                    "Cannot access database metadata: {e}"
                )));
            }
        }
    }

    // If file doesn't exist, check if parent directory exists
    // (meaning zjj was previously initialized)
    if let Some(parent) = path.parent() {
        if parent.exists() {
            return Ok(());
        }
    }

    // Otherwise, only allow creation if explicitly requested
    if allow_create {
        Ok(())
    } else {
        Err(Error::DatabaseError(format!(
            "Database file does not exist: {}\n\nRun 'zjj init' to initialize.",
            path.display()
        )))
    }
}

/// Recover database by removing corrupted/missing file
///
/// This is safe because the database is a cache of session state,
/// not the source of truth. Sessions can be reconstructed from JJ workspaces.
///
/// Behavior depends on recovery policy:
/// - FailFast: Returns error without recovering
/// - Warn: Logs warning, then recovers
/// - Silent: Recovers without warning (old behavior)
fn recover_database(path: &Path) -> Result<()> {
    let policy = get_recovery_policy();
    let should_log = should_log_recovery();

    match policy {
        RecoveryPolicy::FailFast => {
            return Err(Error::DatabaseError(format!(
                "Database corruption detected: {}\n\n\
                 Recovery is disabled in strict mode (--strict or ZJJ_STRICT=1).\n\n\
                 To recover, either:\n\
                 - Remove --strict flag\n\
                 - Run 'zjj doctor --fix'\n\
                 - Manually delete the database and run 'zjj init'",
                path.display()
            )));
        }
        RecoveryPolicy::Warn => {
            eprintln!("⚠  Database corruption detected: {}", path.display());
            eprintln!("   Recovering by recreating database file...");

            if should_log {
                let log_msg = format!(
                    "Database corruption detected at: {}. Recovered by recreating database.",
                    path.display()
                );
                log_recovery(&log_msg).ok();
            }
        }
        RecoveryPolicy::Silent => {
            if should_log {
                let log_msg = format!(
                    "Database corruption detected at: {}. Recovered silently.",
                    path.display()
                );
                log_recovery(&log_msg).ok();
            }
        }
    }

    // Remove corrupted file if it exists
    // Do NOT modify file permissions - respect user-set permissions
    // Only log error if removal fails - don't attempt chmod
    if path.exists() {
        match std::fs::remove_file(path) {
            Ok(()) => {
                // Successfully removed corrupted file
                // New database will be created with default permissions on next DB open
            }
            Err(e) => {
                // Failed to remove - log error and return it
                // Don't attempt chmod - preserve user permissions
                log_recovery(&format!(
                    "Failed to remove corrupted database {}: {e}",
                    path.display()
                ))
                .ok();
                return Err(Error::IoError(format!(
                    "Failed to remove corrupted database: {e}"
                )));
            }
        };
    }

    Ok(())
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
        zellij_tab: format!("zjj:{name}"),
        branch: None,
        created_at: timestamp,
        updated_at: timestamp,
        last_synced: None,
        metadata: None,
    }
}

// === IMPERATIVE SHELL (Database Side Effects) ===

/// Create `SQLite` connection pool
async fn create_connection_pool(db_url: &str) -> Result<SqlitePool> {
    SqlitePool::connect(db_url)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to connect to database: {e}")))
}

/// Initialize database schema
async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to initialize schema: {e}")))?;

    sqlx::query("INSERT OR IGNORE INTO schema_version (version) VALUES (?)")
        .bind(CURRENT_SCHEMA_VERSION)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to set schema version: {e}")))?;

    Ok(())
}

/// Check database schema version matches expected
async fn check_schema_version(pool: &SqlitePool) -> Result<()> {
    let version: Option<i64> = sqlx::query("SELECT version FROM schema_version")
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to read schema version: {e}")))?
        .map(|row| {
            row.try_get("version")
                .map_err(|e| Error::DatabaseError(format!("Failed to parse schema version: {e}")))
        })
        .transpose()?;

    match version {
        Some(v) if v == CURRENT_SCHEMA_VERSION => Ok(()),
        Some(v) => Err(Error::DatabaseError(format!(
            "Schema version mismatch: database has version {v}, but zjj expects version {CURRENT_SCHEMA_VERSION}\n\n\
             The database may have been created by a different version of zjj.\n\n\
             To reset: rm .zjj/state.db && zjj init"
        ))),
        None => Err(Error::DatabaseError(format!(
            "Schema version not found in database. The database may be corrupted.\n\n\
             To reset: rm .zjj/state.db && zjj init"
        ))),
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
    .bind(timestamp.to_i64().unwrap_or(i64::MAX))
    .bind(timestamp.to_i64().unwrap_or(i64::MAX))
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
    .and_then(|opt_row| opt_row.map(parse_session_row).transpose())
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

/// Parse a database row into a `Session`
#[allow(clippy::needless_pass_by_value)]
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
        zellij_tab: format!("zjj:{name}"),
        branch,
        created_at: created_at.cast_unsigned(),
        updated_at: updated_at.cast_unsigned(),
        last_synced: last_synced.map(i64::cast_unsigned),
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

/// Build update clauses from `SessionUpdate`
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
}

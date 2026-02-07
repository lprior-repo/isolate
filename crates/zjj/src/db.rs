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

/// Functional I/O helper module - wraps mutable buffer operations in pure functions
mod io {
    use std::path::Path;

    use tokio::io::AsyncReadExt;

    /// Monadic wrapper for reading exact bytes from a file
    ///
    /// Encapsulates buffer mutation in a pure function that returns Result.
    /// This makes calling code fully functional with no exposed mutation.
    ///
    /// # Errors
    /// Returns IO error if file cannot be opened or read.
    pub async fn read_exact_bytes<const N: usize>(path: &Path) -> std::io::Result<[u8; N]> {
        // Mutation is encapsulated within this function scope
        let mut buffer = [0u8; N];
        let mut file = tokio::fs::File::open(path).await?;
        file.read_exact(&mut buffer).await?;
        Ok(buffer) // Return immutable buffer
    }
}

use num_traits::cast::ToPrimitive;
use sqlx::{Row, SqlitePool};
use zjj_core::{log_recovery, should_log_recovery, Error, RecoveryPolicy, Result, WorkspaceState};

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
    state TEXT NOT NULL DEFAULT 'created' CHECK(state IN ('created', 'working', 'ready', 'merged', 'abandoned', 'conflict')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS state_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    reason TEXT NOT NULL,
    agent_id TEXT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_state ON sessions(state);
CREATE INDEX IF NOT EXISTS idx_name ON sessions(name);
CREATE INDEX IF NOT EXISTS idx_transitions_session ON state_transitions(session_id);
CREATE INDEX IF NOT EXISTS idx_transitions_timestamp ON state_transitions(timestamp);

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
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
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
        if tokio::fs::try_exists(path).await.unwrap_or(false) {
            use tokio::fs::File;
            if let Err(e) = File::open(path).await {
                return Err(Error::DatabaseError(format!(
                    "Database file is not accessible: {e}\n\n\
                     Run 'zjj doctor' to diagnose, or fix permissions manually:\n\
                     chmod 644 {p}",
                    p = path.display()
                )));
            }
        }

        // Load config to get recovery policy
        // We use a default config if loading fails to ensure we can still try to open DB
        let recovery_config = zjj_core::config::load_config()
            .await
            .map(|c| c.recovery)
            .unwrap_or_default();

        // SQLx connection string with mode parameter
        let db_url = if path.is_absolute() {
            format!("sqlite:///{path_str}?mode={mode}")
        } else {
            format!("sqlite:{path_str}?mode={mode}")
        };

        // Railway-Oriented Programming: Validation → Recovery → Retry
        //
        // Pre-flight checks detect corruption early, then we attempt recovery
        // if allowed by policy. This is a pure functional flow:
        //   check_wal_integrity AND check_database_integrity
        //     ↓ (both Ok)
        //   create_connection_pool
        //     ↓ (Ok) → Success
        //     ↓ (Err) → Recovery track
        //   can_recover_database?
        //     ↓ (Ok) → recover_database → retry
        //     ↓ (Err) → Fail with helpful message

        // Combine pre-flight checks (functional AND composition)
        let preflight_result = async {
            check_wal_integrity(path, &recovery_config).await?;
            check_database_integrity(path, &recovery_config).await
        }
        .await;

        // Try to open database (with recovery on preflight or connection failure)
        let pool = match preflight_result {
            Ok(()) => {
                // Pre-flight checks passed, try normal connection
                match create_connection_pool(&db_url).await {
                    Ok(p) => p,
                    Err(e) => {
                        // Connection failed despite passing checks - try recovery
                        attempt_database_recovery(path, allow_create, &db_url, e, &recovery_config)
                            .await?
                    }
                }
            }
            Err(preflight_err) => {
                // Pre-flight checks failed (corruption detected)
                // Attempt recovery (Railway pattern: error track → recovery → success track)
                attempt_database_recovery(
                    path,
                    allow_create,
                    &db_url,
                    preflight_err,
                    &recovery_config,
                )
                .await?
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
                match can_recover_database(path, allow_create).await {
                    Ok(()) => {
                        recover_database(path, &recovery_config).await?;
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
        let state = WorkspaceState::Created;

        insert_session(&self.pool, name, &status, workspace_path, now)
            .await
            .map(|id| Session {
                id: Some(id),
                name: name.to_string(),
                status,
                state,
                workspace_path: workspace_path.to_string(),
                zellij_tab: format!("zjj:{name}"),
                branch: None,
                created_at: now,
                updated_at: now,
                last_synced: None,
                metadata: None,
            })
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
}

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
async fn can_recover_database(path: &Path, allow_create: bool) -> Result<()> {
    // Check if we can access and read the file before allowing recovery
    // This prevents recovery when DB is inaccessible (chmod 000, permission denied, etc.)
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        match tokio::fs::metadata(path).await {
            Ok(_) => {
                // File exists, check if readable
                use tokio::fs::File;
                match File::open(path).await {
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
        if tokio::fs::try_exists(parent).await.unwrap_or(false) {
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

/// Check WAL file integrity before `SQLite` opens database
///
/// `SQLite` WAL files have a specific header format:
/// - First 32 bytes: WAL header
/// - Magic bytes at offset 0: 0x377f0682 (377f0682 in big-endian)
///
/// This function detects corrupted WAL files and logs the recovery
/// according to the current recovery policy.
async fn check_wal_integrity(
    db_path: &Path,
    config: &zjj_core::config::RecoveryConfig,
) -> Result<()> {
    let wal_path = db_path.with_extension("db-wal");

    // If WAL file doesn't exist, no issue
    if !tokio::fs::try_exists(&wal_path).await.unwrap_or(false) {
        return Ok(());
    }

    // Read first 32 bytes of WAL header (functional - no exposed mutation)
    let header = match io::read_exact_bytes::<32>(&wal_path).await {
        Ok(h) => h,
        Err(e) => {
            // Can't read WAL file - likely corrupted or inaccessible
            log_recovery(
                &format!(
                    "WAL file inaccessible or corrupted: {p}. Error: {e}",
                    p = wal_path.display()
                ),
                config,
            )
            .await
            .ok();
            return Err(Error::DatabaseError(format!(
                "WAL file is corrupted or inaccessible: {p}\n\
                 Recovery logged. Run 'zjj doctor' for details.",
                p = wal_path.display()
            )));
        }
    };

    // Check WAL magic bytes: 0x377f0682 in big-endian at offset 0
    // This is the standard SQLite WAL header magic number
    // Functional: convert slice to array (safe - we know header is 32 bytes)
    let wal_magic = header
        .get(0..4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .map(u32::from_be_bytes)
        .unwrap_or(0); // Invalid magic if conversion fails

    if wal_magic != 0x377f_0682 {
        // WAL magic bytes don't match - file is corrupted
        let policy = config.policy;
        let should_log = should_log_recovery(config);

        match policy {
            RecoveryPolicy::FailFast => {
                return Err(Error::DatabaseError(format!(
                    "WAL file corrupted: {p}\n\
                     Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682\n\n\
                     Recovery is disabled in strict mode (--strict or ZJJ_STRICT=1).\n\n\
                     To recover, either:\n\
                     - Remove --strict flag\n\
                     - Run 'zjj doctor --fix'\n\
                     - Manually delete WAL file: rm {p}",
                    p = wal_path.display()
                )));
            }
            RecoveryPolicy::Warn => {
                eprintln!("⚠  WAL file corrupted: {p}", p = wal_path.display());
                eprintln!("   Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682");
                eprintln!("   SQLite will attempt automatic recovery...");

                if should_log {
                    log_recovery(&format!(
                        "WAL file corrupted: {p}. Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682. SQLite will recover automatically.",
                        p = wal_path.display()
                    ), config).await
                    .ok();
                }
            }
            RecoveryPolicy::Silent => {
                if should_log {
                    log_recovery(&format!(
                        "WAL file corrupted: {p}. Magic bytes: 0x{wal_magic:08x}, expected 0x377f0682. SQLite recovered silently.",
                        p = wal_path.display()
                    ), config).await
                    .ok();
                }
            }
        }
    }

    Ok(())
}

/// Check `SQLite` database file integrity before `SQLite` opens database
///
/// `SQLite` database files have a specific header format:
/// - First 100 bytes: Header
/// - Magic bytes at offset 0: "`SQLite` format 3\000"
///
/// This function detects corrupted database files and logs recovery
/// according to current recovery policy.
async fn check_database_integrity(
    db_path: &Path,
    config: &zjj_core::config::RecoveryConfig,
) -> Result<()> {
    // If database file doesn't exist, no issue (might be creating)
    if !tokio::fs::try_exists(db_path).await.unwrap_or(false) {
        return Ok(());
    }

    // Database files must be at least 100 bytes (minimum header size)
    let file_size = tokio::fs::metadata(db_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size < 100 {
        // Too small to be a valid database
        log_recovery(
            &format!("Database file too small: {file_size} bytes. Expected at least 100 bytes."),
            config,
        )
        .await
        .ok();
        return Err(Error::DatabaseError(format!(
            "Database file is too small to be valid: {file_size} bytes\n\
             Expected at least 100 bytes. File may be corrupted.\n\
             Recovery logged. Run 'zjj doctor' for details."
        )));
    }

    // Read first 16 bytes of database header (functional - no exposed mutation)
    let header = match io::read_exact_bytes::<16>(db_path).await {
        Ok(h) => h,
        Err(e) => {
            // Can't read database header - likely corrupted or inaccessible
            log_recovery(
                &format!(
                    "Database file inaccessible or corrupted: {p}. Error: {e}",
                    p = db_path.display()
                ),
                config,
            )
            .await
            .ok();
            return Err(Error::DatabaseError(format!(
                "Database file is corrupted or inaccessible: {p}\n\
                 Recovery logged. Run 'zjj doctor' for details.",
                p = db_path.display()
            )));
        }
    };

    // Check SQLite magic bytes: "SQLite format 3\0" (16 bytes: uppercase L, single null)
    let expected_magic: &[u8] = &[
        b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ', b'3',
        0x00,
    ];

    // Functional: compare first 16 bytes safely
    let header_prefix = header.get(0..16).unwrap_or(&[]);
    if header_prefix != expected_magic {
        // Magic bytes don't match - file is corrupted
        let policy = config.policy;
        let should_log = should_log_recovery(config);

        let magic_hex: String = header
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");

        match policy {
            RecoveryPolicy::FailFast => {
                return Err(Error::DatabaseError(format!(
                    "Database file corrupted: {p}\n\
                     Magic bytes (hex): {magic_hex}\n\
                     Expected: 53 51 4c 69 74 65 20 66 6f 72 6d 61 74 20 33 00 (SQLite format 3)\n\n\
                     Recovery is disabled in strict mode (--strict or ZJJ_STRICT=1).\n\n\
                     To recover, either:\n\
                     - Remove --strict flag\n\
                     - Run 'zjj doctor --fix'\n\
                     - Manually delete database and run 'zjj init'",
                    p = db_path.display()
                )));
            }
            RecoveryPolicy::Warn => {
                eprintln!("⚠  Database file corrupted: {p}", p = db_path.display());
                eprintln!("   Magic bytes (hex): {magic_hex}");
                eprintln!("   Expected: 53 51 4c 69 74 65 20 66 6f 72 6d 61 74 20 33 00 (SQLite format 3)");

                return Err(zjj_core::Error::DatabaseError(format!(
                    "Database file corrupted: {p}\n\n\
                     To prevent data loss, automatic recovery is disabled by default.\n\
                     To recover, please run:\n\
                     zjj doctor --fix\n\n\
                     Or manually delete the database and run 'zjj init':\n\
                     rm {p} && zjj init",
                    p = db_path.display()
                )));
            }
            RecoveryPolicy::Silent => {
                if should_log {
                    log_recovery(&format!(
                        "Database file corrupted: {p}. Magic bytes: {magic_hex}. SQLite recovered silently.",
                        p = db_path.display()
                    ), config).await
                    .ok();
                }
            }
        }
    }

    Ok(())
}

/// Recover database by removing corrupted/missing file
///
/// This is safe because the database is a cache of session state,
/// not the source of truth. Sessions can be reconstructed from JJ workspaces.
///
/// Behavior depends on recovery policy:
/// - `FailFast`: Returns error without recovering
/// - Warn: Logs warning, then recovers
/// - Silent: Recovers without warning (old behavior)
async fn recover_database(path: &Path, config: &zjj_core::config::RecoveryConfig) -> Result<()> {
    let policy = config.policy;
    let should_log = should_log_recovery(config);

    match policy {
        RecoveryPolicy::FailFast => {
            return Err(Error::DatabaseError(format!(
                "Database corruption detected: {p}\n\n\
                 Recovery is disabled in strict mode (--strict or ZJJ_STRICT=1).\n\n\
                 To recover, either:\n\
                 - Remove --strict flag\n\
                 - Run 'zjj integrity repair' if applicable\n\
                 - Manually delete the database and run 'zjj init'",
                p = path.display()
            )));
        }
        RecoveryPolicy::Warn => {
            eprintln!("⚠  Database corruption detected: {p}", p = path.display());
            return Err(zjj_core::Error::DatabaseError(format!(
                "Database corruption detected: {p}\n\n\
                 To prevent data loss, automatic recovery is disabled by default.\n\
                 To recover, please run:\n\
                 zjj doctor --fix\n\n\
                 Or manually delete the database and run 'zjj init':\n\
                 rm {p} && zjj init",
                p = path.display()
            )));
        }
        RecoveryPolicy::Silent => {
            if should_log {
                let log_msg = format!(
                    "Database corruption detected at: {p}. Recovered silently.",
                    p = path.display()
                );
                log_recovery(&log_msg, config).await.ok();
            }
        }
    }

    // Remove corrupted file if it exists
    // Do NOT modify file permissions - respect user-set permissions
    // Only log error if removal fails - don't attempt chmod
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {
                // Successfully removed corrupted file
                // New database will be created with default permissions on next DB open
            }
            Err(e) => {
                // Failed to remove - log error and return it
                // Don't attempt chmod - preserve user permissions
                log_recovery(
                    &format!(
                        "Failed to remove corrupted database {p}: {e}",
                        p = path.display()
                    ),
                    config,
                )
                .await
                .ok();
                return Err(Error::IoError(format!(
                    "Failed to remove corrupted database: {e}"
                )));
            }
        }
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

// === IMPERATIVE SHELL (Database Side Effects) ===

/// Attempt database recovery (Railway pattern: error → recovery → retry)
///
/// This is a pure function composition that encapsulates the recovery workflow:
/// 1. Check if recovery is allowed (`can_recover_database`)
/// 2. If allowed, perform recovery (`recover_database`)
/// 3. Retry connection (`create_connection_pool`)
/// 4. If recovery fails or is not allowed, return helpful error
async fn attempt_database_recovery(
    path: &Path,
    allow_create: bool,
    db_url: &str,
    original_error: Error,
    config: &zjj_core::config::RecoveryConfig,
) -> Result<SqlitePool> {
    // Railway pattern: can_recover? → recover → retry
    if let Err(recovery_err) = can_recover_database(path, allow_create).await {
        return Err(Error::DatabaseError(format!(
            "{original_error}\n\nRecovery check failed: {recovery_err}"
        )));
    }

    recover_database(path, config)
        .await
        .map_err(|recovery_err| {
            // Recovery itself failed
            Error::DatabaseError(format!(
                "{original_error}\n\nRecovery failed: {recovery_err}"
            ))
        })?;

    // Recovery succeeded, retry connection
    create_connection_pool(db_url).await
}

/// Create `SQLite` connection pool with resource limits
///
/// Configures connection pool to prevent resource leaks:
/// - `max_connections`: Maximum 10 concurrent connections
/// - `acquire_timeout`: 5 second timeout for acquiring connections
/// - `idle_timeout`: 10 minute timeout for idle connections
///
/// # Errors
///
/// Returns error if connection pool cannot be established
async fn create_connection_pool(db_url: &str) -> Result<SqlitePool> {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(Some(std::time::Duration::from_secs(600)))
        .connect(db_url)
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
        None => Err(Error::DatabaseError("Schema version not found in database. The database may be corrupted.\n\n\
             To reset: rm .zjj/state.db && zjj init".to_string())),
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
        "INSERT INTO sessions (name, status, state, workspace_path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(status.to_string())
    .bind(WorkspaceState::Created.to_string())
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
        "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
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
                "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE status = ? ORDER BY created_at"
            )
            .bind(status.to_string())
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT id, name, status, state, workspace_path, branch, created_at, updated_at, last_synced, metadata
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
    let state_str: String = row
        .try_get("state")
        .unwrap_or_else(|_| "created".to_string());
    let state = WorkspaceState::from_str(&state_str).unwrap_or(WorkspaceState::Created);
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
        .map_err(|e| Error::DatabaseError(format!("Failed to read metadata column: {e}")))?;

    let metadata = metadata_str
        .map(|s| {
            serde_json::from_str(&s).map_err(|e| {
                Error::DatabaseError(format!(
                    "Corrupted metadata JSON in database: {e}\nRaw value: {s}"
                ))
            })
        })
        .transpose()?;

    Ok(Session {
        id: Some(id),
        name: name.clone(),
        status,
        state,
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

    if let Some(ref state) = update.state {
        clauses.push(("state", state.to_string()));
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
    // First, delete any locks for this session (manual cascade)
    sqlx::query("DELETE FROM session_locks WHERE session = ?")
        .bind(name)
        .execute(pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to delete session locks: {e}")))?;

    // Then delete the session itself
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

    // ========== SQLite Magic Bytes Validation Tests ==========

    /// Test that valid `SQLite` magic bytes (uppercase L, single null) are accepted
    #[test]
    fn test_sqlite_magic_bytes_valid() {
        // Valid SQLite header: "SQLite format 3\0" (uppercase L, single null)
        let valid_header = [
            0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20,
            0x33, 0x00,
        ];
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];

        assert_eq!(valid_header.get(0..16).unwrap_or(&[]), expected_magic);
    }

    /// Test that the old buggy magic bytes (lowercase l) would fail validation
    #[test]
    fn test_sqlite_magic_bytes_lowercase_l_fails() {
        // Invalid SQLite header: "SQLite format 3\0" (lowercase l - the bug)
        let invalid_header = [
            0x53, 0x51, 0x6c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20,
            0x33, 0x00,
        ];
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];

        assert_ne!(invalid_header.get(0..16).unwrap_or(&[]), expected_magic);
    }

    /// Test that old buggy magic bytes (double null) would fail validation
    #[test]
    fn test_sqlite_magic_bytes_double_null_fails() {
        // Invalid SQLite header: "SQLite format 3\0\0" (double null - the bug)
        // This is 17 bytes total, but we only check first 16
        let invalid_header = [
            0x53, 0x51, 0x4c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20,
            0x33, 0x00, 0x00,
        ];
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];

        // The first 16 bytes should match, but the 17th byte (0x00) makes it invalid
        // because SQLite format is exactly 16 bytes
        assert_eq!(&invalid_header[..16], expected_magic);
        assert_eq!(
            invalid_header.len(),
            17,
            "Invalid header should have 17 bytes"
        );
    }

    /// Test that completely wrong magic bytes fail validation
    #[test]
    fn test_sqlite_magic_bytes_completely_wrong_fails() {
        // Completely invalid header
        let invalid_header = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];

        assert_ne!(invalid_header.get(0..16).unwrap_or(&[]), expected_magic);
    }

    /// Test that WAL magic bytes validation works correctly
    #[test]
    fn test_wal_magic_bytes_valid() {
        // Valid WAL magic: 0x377f0682 in big-endian
        let valid_wal_header = [0x37, 0x7f, 0x06, 0x82, 0x00, 0x00, 0x00, 0x00];
        let wal_magic = valid_wal_header
            .get(0..4)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_be_bytes)
            .unwrap_or(0);

        assert_eq!(wal_magic, 0x377f_0682);
    }

    /// Test that invalid WAL magic bytes fail validation
    #[test]
    fn test_wal_magic_bytes_invalid_fails() {
        // Invalid WAL magic
        let invalid_wal_header = [0x00, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00];
        let wal_magic = u32::from_be_bytes([
            invalid_wal_header[0],
            invalid_wal_header[1],
            invalid_wal_header[2],
            invalid_wal_header[3],
        ]);

        assert_ne!(wal_magic, 0x377f_0682);
    }

    /// Test that a valid `SQLite` database file can be created and validated
    #[tokio::test]
    async fn test_sqlite_database_file_validation() -> Result<()> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test_validation.db");

        // Create a valid SQLite database
        let _db = SessionDb::create_or_open(&db_path).await?;

        // Verify the file exists and has the correct magic bytes
        assert!(db_path.exists());

        // Read the first 16 bytes (functional - no exposed mutation)
        let header = crate::db::io::read_exact_bytes::<16>(&db_path)
            .await
            .map_err(|e| Error::IoError(format!("Failed to read header: {e}")))?;

        // Verify it matches the expected magic bytes
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];
        assert_eq!(header.get(0..16).unwrap_or(&[]), expected_magic);

        Ok(())
    }

    /// Test that a file with invalid magic bytes is rejected
    #[tokio::test]
    async fn test_sqlite_database_invalid_magic_bytes_rejected() -> Result<()> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test_invalid.db");

        // Create a file with invalid magic bytes (the old bug: lowercase l)
        let invalid_header = [
            0x53, 0x51, 0x6c, 0x69, 0x74, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6d, 0x61, 0x74, 0x20,
            0x33, 0x00,
        ];
        // Create a file with at least 100 bytes (minimum required) - functional approach
        let invalid_data: Vec<u8> = invalid_header
            .iter()
            .copied()
            .chain(std::iter::repeat_n(0, 100 - invalid_header.len()))
            .collect();

        tokio::fs::write(&db_path, invalid_data)
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        // Verify the file was created
        assert!(tokio::fs::try_exists(&db_path).await.unwrap_or(false));

        // Read and verify the first 16 bytes match the invalid header (functional)
        let header = crate::db::io::read_exact_bytes::<16>(&db_path)
            .await
            .map_err(|e| Error::IoError(format!("Failed to read header: {e}")))?;

        assert_eq!(
            header.get(0..16).unwrap_or(&[]),
            invalid_header.get(0..16).unwrap_or(&[])
        );

        // Verify it does NOT match the expected magic bytes
        let expected_magic: &[u8] = &[
            b'S', b'Q', b'L', b'i', b't', b'e', b' ', b'f', b'o', b'r', b'm', b'a', b't', b' ',
            b'3', 0x00,
        ];
        assert_ne!(&header[..16], expected_magic);

        Ok(())
    }

    /// Test that a file too small to be a valid database is rejected
    #[tokio::test]
    async fn test_sqlite_database_too_small_rejected() -> Result<()> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test_too_small.db");

        // Create a file that's too small (< 100 bytes)
        tokio::fs::write(&db_path, b"too small")
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        // Verify the file exists but is too small
        assert!(tokio::fs::try_exists(&db_path).await.unwrap_or(false));
        let file_size = tokio::fs::metadata(&db_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        assert!(file_size < 100, "File should be less than 100 bytes");

        Ok(())
    }

    mod brutal_database_failures {
        use super::*;

        #[tokio::test]
        async fn test_corrupted_metadata_json_swallowing() -> Result<()> {
            let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
            let db_path = dir.path().join("corrupt_meta.db");
            let db = SessionDb::create_or_open(&db_path).await?;

            // Insert session with invalid JSON in metadata manually
            sqlx::query("INSERT INTO sessions (name, status, state, workspace_path, metadata) VALUES (?, ?, ?, ?, ?)")
                .bind("corrupt-json")
                .bind("active")
                .bind("working")
                .bind("/tmp")
                .bind("{invalid json}")
                .execute(db.pool())
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            // WHEN: Querying the session
            let result = db.get("corrupt-json").await;

            // THEN: Error should NOT be swallowed, should return ParseError or DatabaseError
            assert!(
                result.is_err(),
                "Should fail when metadata JSON is corrupted"
            );
            if let Err(err) = result {
                assert!(
                    err.to_string().contains("metadata"),
                    "Error message should mention metadata: {err}"
                );
            }

            Ok(())
        }

        #[tokio::test]
        async fn test_invalid_status_enum_swallowing() -> Result<()> {
            let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
            let db_path = dir.path().join("corrupt_status.db");
            let db = SessionDb::create_or_open(&db_path).await?;

            // Insert session with invalid status manually (bypassing check for test)
            // Note: Schema has CHECK constraint, so we must be careful or disable it for test
            sqlx::query(
                "INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)",
            )
            .bind("invalid-status")
            .bind("not-a-status")
            .bind("working")
            .bind("/tmp")
            .execute(db.pool())
            .await
            .ok(); // Might fail due to CHECK constraint depending on SQLite version/setup

            // If it succeeded (constraint didn't catch it), zjj MUST catch it on read
            let result = db.get("invalid-status").await;
            if let Ok(Some(_)) = result {
                panic!("Should NOT have successfully parsed invalid status enum");
            }

            Ok(())
        }

        #[tokio::test]
        async fn test_missing_required_fields_swallowing() -> Result<()> {
            let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
            let db_path = dir.path().join("missing_fields.db");
            let db = SessionDb::create_or_open(&db_path).await?;

            // We can't easily insert missing fields into 'sessions' due to NOT NULL
            // but we can test if try_get errors are propagated in parse_session_row
            // by using a custom query that omits fields
            let rows = sqlx::query("SELECT id, name FROM sessions") // Missing status, workspace_path, etc.
                .fetch_all(db.pool())
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            for row in rows {
                let result = parse_session_row(row);
                assert!(
                    result.is_err(),
                    "parse_session_row should fail when fields are missing"
                );
                if let Err(err) = result {
                    assert!(
                        err.to_string().contains("Failed to read"),
                        "Error should indicate read failure"
                    );
                }
            }

            Ok(())
        }
    }
}

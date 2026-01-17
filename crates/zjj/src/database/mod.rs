//! Database operations for session persistence using `SQLx`
//!
//! This module provides async SQLite-based persistence with:
//! - Connection pooling (no Arc<Mutex<>>)
//! - Zero unwraps, zero panics
//! - Simple embedded schema (no migration files)
//! - Pure functional patterns with Railway-Oriented Programming

use std::path::Path;

use sqlx::SqlitePool;
use zjj_core::Result;

use crate::session::{Session, SessionStatus, SessionUpdate};

mod backup;
mod query;
mod schema;
mod session_ops;
mod validation;

pub use backup::BackupOps;
pub use session_ops::SessionOps;

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
        validation::validate_database_path(path, allow_create)?;

        let db_url = if allow_create {
            format!("sqlite:{}?mode=rwc", path.display())
        } else {
            format!("sqlite:{}", path.display())
        };

        let pool = schema::create_connection_pool(&db_url).await?;
        schema::init_schema(&pool).await?;
        Ok(Self { pool })
    }
}

// Implement session operations trait
impl SessionOps for SessionDb {
    fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// Implement backup operations trait
impl BackupOps for SessionDb {
    fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// Forward public methods to trait implementations
impl SessionDb {
    /// Create a new session
    ///
    /// # Errors
    ///
    /// Returns error if session name already exists or database operation fails
    pub async fn create(&self, name: &str, workspace_path: &str) -> Result<Session> {
        <Self as SessionOps>::create(self, name, workspace_path).await
    }

    /// Get a session by name
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get(&self, name: &str) -> Result<Option<Session>> {
        <Self as SessionOps>::get(self, name).await
    }

    /// Update an existing session
    ///
    /// # Errors
    ///
    /// Returns error if database update fails
    pub async fn update(&self, name: &str, update: SessionUpdate) -> Result<()> {
        <Self as SessionOps>::update(self, name, update).await
    }

    /// Delete a session by name
    ///
    /// Returns `true` if session was deleted, `false` if it didn't exist
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn delete(&self, name: &str) -> Result<bool> {
        <Self as SessionOps>::delete(self, name).await
    }

    /// List all sessions, optionally filtered by status
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        <Self as SessionOps>::list(self, status_filter).await
    }

    /// Create a backup of the database
    ///
    /// # Errors
    ///
    /// Returns error if backup cannot be written
    pub async fn backup(&self, backup_path: &Path) -> Result<()> {
        <Self as BackupOps>::backup(self, backup_path).await
    }

    /// Restore database from a backup file
    ///
    /// # Errors
    ///
    /// Returns error if backup is invalid or restore fails
    pub async fn restore(&self, backup_path: &Path) -> Result<()> {
        <Self as BackupOps>::restore(self, backup_path).await
    }

    /// Verify integrity of a backup file
    ///
    /// # Errors
    ///
    /// Returns error if backup file is invalid
    pub fn verify_backup(backup_path: &Path) -> Result<usize> {
        backup::verify_backup(backup_path)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use tokio_test::block_on;
    use zjj_core::Error;

    use super::*;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[test]
    fn test_create_session_success() -> Result<()> {
        block_on(async {
            let (db, _dir) = setup_test_db().await?;
            let session = db.create("test-session", "/workspace").await?;

            assert_eq!(session.name, "test-session");
            assert_eq!(session.status, SessionStatus::Creating);
            assert_eq!(session.workspace_path, "/workspace");
            Ok(())
        })
    }

    #[test]
    fn test_get_session_exists() -> Result<()> {
        block_on(async {
            let (db, _dir) = setup_test_db().await?;
            let created = db.create("test", "/path").await?;

            let retrieved = db.get("test").await?;
            assert!(retrieved.is_some());

            let session = retrieved.ok_or_else(|| Error::NotFound("session".into()))?;
            assert_eq!(session.name, created.name);
            Ok(())
        })
    }

    #[test]
    fn test_unique_constraint_enforced() -> Result<()> {
        block_on(async {
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
        })
    }

    #[test]
    fn test_backup_restore_roundtrip() -> Result<()> {
        block_on(async {
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
        })
    }
}

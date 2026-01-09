//! Database operations for session persistence

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::session::Session;

/// Database wrapper for session storage
pub struct SessionDb {
    conn: Connection,
}

impl SessionDb {
    /// Open or create a session database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database")?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS sessions (
                    name TEXT PRIMARY KEY,
                    workspace_path TEXT NOT NULL,
                    zellij_tab TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )",
                [],
            )
            .context("Failed to create schema")?;
        Ok(())
    }

    /// Insert a new session
    pub fn insert(&self, session: &Session) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO sessions (name, workspace_path, zellij_tab, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                (
                    &session.name,
                    &session.workspace_path,
                    &session.zellij_tab,
                    session.created_at,
                ),
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    anyhow::anyhow!("Session '{}' already exists", session.name)
                } else {
                    anyhow::anyhow!("Failed to insert session: {e}")
                }
            })?;
        Ok(())
    }

    /// Get a session by name
    pub fn get(&self, name: &str) -> Result<Option<Session>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT name, workspace_path, zellij_tab, created_at
                 FROM sessions WHERE name = ?1",
            )
            .context("Failed to prepare query")?;

        let mut rows = stmt.query([name]).context("Failed to execute query")?;

        match rows.next() {
            Ok(Some(row)) => {
                let session = Session {
                    name: row.get(0).context("Failed to read name")?,
                    workspace_path: row.get(1).context("Failed to read workspace_path")?,
                    zellij_tab: row.get(2).context("Failed to read zellij_tab")?,
                    created_at: row.get(3).context("Failed to read created_at")?,
                };
                Ok(Some(session))
            }
            Ok(None) => Ok(None),
            Err(e) => anyhow::bail!("Failed to read row: {e}"),
        }
    }

    /// List all sessions
    pub fn list(&self) -> Result<Vec<Session>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT name, workspace_path, zellij_tab, created_at
                 FROM sessions ORDER BY created_at",
            )
            .context("Failed to prepare query")?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Session {
                    name: row.get(0)?,
                    workspace_path: row.get(1)?,
                    zellij_tab: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .context("Failed to execute query")?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = row.context("Failed to parse session")?;
            sessions.push(session);
        }
        Ok(sessions)
    }

    /// Delete a session by name
    pub fn delete(&self, name: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM sessions WHERE name = ?1", [name])
            .context("Failed to delete session")?;
        Ok(changes > 0)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new().context("Failed to create temp dir")?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    #[test]
    fn test_insert_and_get() {
        let result = setup_test_db();
        assert!(result.is_ok());
        let (db, _dir) = result.unwrap_or_else(|_| std::process::exit(1));

        let session = Session::new("test", "/path").unwrap_or_else(|_| std::process::exit(1));
        let insert_result = db.insert(&session);
        assert!(insert_result.is_ok());

        let get_result = db.get("test");
        assert!(get_result.is_ok());
        let retrieved = get_result.unwrap_or(None);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.map(|s| s.name).unwrap_or_default(), "test");
    }

    #[test]
    fn test_list_empty() {
        let result = setup_test_db();
        assert!(result.is_ok());
        let (db, _dir) = result.unwrap_or_else(|_| std::process::exit(1));

        let list_result = db.list();
        assert!(list_result.is_ok());
        assert!(list_result.unwrap_or_default().is_empty());
    }

    #[test]
    fn test_delete() {
        let result = setup_test_db();
        assert!(result.is_ok());
        let (db, _dir) = result.unwrap_or_else(|_| std::process::exit(1));

        let session = Session::new("test", "/path").unwrap_or_else(|_| std::process::exit(1));
        let _ = db.insert(&session);

        let delete_result = db.delete("test");
        assert!(delete_result.is_ok());
        assert!(delete_result.unwrap_or(false));

        let get_result = db.get("test");
        assert!(get_result.is_ok());
        assert!(get_result.unwrap_or(Some(session)).is_none());
    }

    #[test]
    fn test_duplicate_insert() {
        let result = setup_test_db();
        assert!(result.is_ok());
        let (db, _dir) = result.unwrap_or_else(|_| std::process::exit(1));

        let session = Session::new("test", "/path").unwrap_or_else(|_| std::process::exit(1));
        let _ = db.insert(&session);

        let duplicate_result = db.insert(&session);
        assert!(duplicate_result.is_err());
    }
}

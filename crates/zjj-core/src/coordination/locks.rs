//! Session lock manager for agent coordination.
//!
//! Provides exclusive locking so that only one agent operates on a session at a time.
//! Locks have a TTL and can be extended via heartbeat.
//!
//! # Session Existence Validation
//!
//! The lock manager validates that a session exists in the sessions table before
//! acquiring a lock. This prevents orphaned locks from being created for
//! non-existent sessions.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::{Error, Result};

/// Default lock TTL in seconds (5 minutes).
const DEFAULT_TTL_SECS: i64 = 300;

/// Information about an active lock.
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// The session that is locked.
    pub session: String,
    /// The agent holding the lock.
    pub agent_id: String,
    /// When the lock was acquired.
    pub acquired_at: DateTime<Utc>,
    /// When the lock expires.
    pub expires_at: DateTime<Utc>,
}

/// Response returned when a lock is successfully acquired.
#[derive(Debug, Clone)]
pub struct LockResponse {
    /// Unique lock identifier.
    pub lock_id: String,
    /// The session that was locked.
    pub session: String,
    /// The agent that acquired the lock.
    pub agent_id: String,
    /// When the lock expires.
    pub expires_at: DateTime<Utc>,
}

/// Audit log entry for lock operations.
#[derive(Debug, Clone)]
pub struct LockAuditEntry {
    /// The session that was operated on.
    pub session: String,
    /// The agent that performed the operation.
    pub agent_id: String,
    /// The operation performed (lock, unlock, `double_unlock_warning`).
    pub operation: String,
    /// When the operation occurred.
    pub timestamp: DateTime<Utc>,
}

/// Current lock state for a session.
#[derive(Debug, Clone)]
pub struct LockState {
    /// The session name.
    pub session: String,
    /// The current lock holder (if any).
    pub holder: Option<String>,
    /// When the lock expires (if locked).
    pub expires_at: Option<DateTime<Utc>>,
}

/// Manages exclusive session locks backed by `SQLite`.
#[derive(Debug, Clone)]
pub struct LockManager {
    db: SqlitePool,
    ttl: Duration,
}

impl LockManager {
    /// Create a new `LockManager` with default TTL.
    #[must_use]
    pub const fn new(db: SqlitePool) -> Self {
        Self {
            db,
            ttl: Duration::seconds(DEFAULT_TTL_SECS),
        }
    }

    /// Get the database pool
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.db
    }

    /// Create a new `LockManager` with a custom TTL.
    #[must_use]
    pub const fn with_ttl(db: SqlitePool, ttl: Duration) -> Self {
        Self { db, ttl }
    }

    /// Initialize the locks table.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS session_locks (
                lock_id TEXT PRIMARY KEY,
                session TEXT NOT NULL UNIQUE,
                agent_id TEXT NOT NULL,
                acquired_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
        )
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Create audit log table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS session_lock_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
        )
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Log a lock operation to the audit trail.
    async fn log_operation(&self, session: &str, agent_id: &str, operation: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO session_lock_audit (session, agent_id, operation, timestamp)
             VALUES (?, ?, ?, ?)",
        )
        .bind(session)
        .bind(agent_id)
        .bind(operation)
        .bind(&now_str)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Acquire an exclusive lock on a session.
    ///
    /// Returns `SessionLocked` error if another agent holds a valid lock.
    /// Returns `SessionNotFound` error if the session doesn't exist in the sessions table.
    pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // CRITICAL: Check session exists BEFORE creating lock
        // This prevents orphaned locks for non-existent sessions
        self.verify_session_exists(session).await?;

        // First, clean up expired locks for this session
        sqlx::query("DELETE FROM session_locks WHERE session = ? AND expires_at < ?")
            .bind(session)
            .bind(&now_str)
            .execute(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Check for existing lock
        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT agent_id, expires_at FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        if let Some((holder, _)) = existing {
            // If same agent, just return the existing lock info
            if holder == agent_id {
                let row: (String, String, String) = sqlx::query_as(
                    "SELECT lock_id, session, expires_at FROM session_locks WHERE session = ? AND agent_id = ?",
                )
                .bind(session)
                .bind(agent_id)
                .fetch_one(&self.db)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

                let expires_at = DateTime::parse_from_rfc3339(&row.2)
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .with_timezone(&Utc);

                return Ok(LockResponse {
                    lock_id: row.0,
                    session: row.1,
                    agent_id: agent_id.to_string(),
                    expires_at,
                });
            }
            return Err(Error::SessionLocked {
                session: session.to_string(),
                holder,
            });
        }

        let expires_at = now + self.ttl;
        let expires_str = expires_at.to_rfc3339();
        let nanos = now
            .timestamp_nanos_opt()
            .ok_or_else(|| Error::ParseError("Failed to get timestamp nanos".into()))?;
        let lock_id = format!("lock-{session}-{nanos}");

        sqlx::query(
            "INSERT INTO session_locks (lock_id, session, agent_id, acquired_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&lock_id)
        .bind(session)
        .bind(agent_id)
        .bind(&now_str)
        .bind(&expires_str)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Log the lock operation
        self.log_operation(session, agent_id, "lock").await?;

        Ok(LockResponse {
            lock_id,
            session: session.to_string(),
            agent_id: agent_id.to_string(),
            expires_at,
        })
    }

    /// Verify that a session exists in the sessions table.
    ///
    /// This is called before acquiring a lock to prevent orphaned locks.
    async fn verify_session_exists(&self, session: &str) -> Result<()> {
        // Try to query the sessions table
        let query_result = sqlx::query("SELECT name FROM sessions WHERE name = ?")
            .bind(session)
            .fetch_optional(&self.db)
            .await;

        match query_result {
            Ok(None) => {
                // Session doesn't exist
                Err(Error::SessionNotFound {
                    session: session.to_string(),
                })
            }
            Ok(Some(_)) => {
                // Session exists
                Ok(())
            }
            Err(e) => {
                // If sessions table doesn't exist (old database), allow the lock
                // This maintains backward compatibility
                let error_msg = e.to_string();
                if error_msg.contains("no such table") || error_msg.contains("does not exist") {
                    Ok(())
                } else {
                    Err(Error::DatabaseError(format!("Failed to query sessions: {e}")))
                }
            }
        }
    }

    /// Release a lock. Only the holder can release it.
    pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();

        // Check who holds the lock
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query("DELETE FROM session_locks WHERE session = ? AND agent_id = ?")
                    .bind(session)
                    .bind(agent_id)
                    .execute(&self.db)
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;

                // Log successful unlock to audit trail
                self.log_operation(session, agent_id, "unlock").await?;
                Ok(())
            }
            Some(_) => Err(Error::NotLockHolder {
                session: session.to_string(),
                agent_id: agent_id.to_string(),
            }),
            None => {
                // No active lock - detect and log double unlock
                self.log_operation(session, agent_id, "double_unlock_warning")
                    .await?;
                Ok(())
            }
        }
    }

    /// Extend a lock's TTL (heartbeat).
    pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let new_expires = now + self.ttl;
        let new_expires_str = new_expires.to_rfc3339();

        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT agent_id FROM session_locks WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        match existing {
            Some((holder,)) if holder == agent_id => {
                sqlx::query(
                    "UPDATE session_locks SET expires_at = ? WHERE session = ? AND agent_id = ?",
                )
                .bind(&new_expires_str)
                .bind(session)
                .bind(agent_id)
                .execute(&self.db)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

                let row: (String,) =
                    sqlx::query_as("SELECT lock_id FROM session_locks WHERE session = ?")
                        .bind(session)
                        .fetch_one(&self.db)
                        .await
                        .map_err(|e| Error::DatabaseError(e.to_string()))?;

                Ok(LockResponse {
                    lock_id: row.0,
                    session: session.to_string(),
                    agent_id: agent_id.to_string(),
                    expires_at: new_expires,
                })
            }
            Some(_) => Err(Error::NotLockHolder {
                session: session.to_string(),
                agent_id: agent_id.to_string(),
            }),
            None => Err(Error::NotFound(format!(
                "No active lock for session '{session}'"
            ))),
        }
    }

    /// Get all active (non-expired) locks.
    pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>> {
        let now_str = Utc::now().to_rfc3339();

        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT session, agent_id, acquired_at, expires_at FROM session_locks WHERE expires_at >= ?",
        )
        .bind(&now_str)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(|(session, agent_id, acquired_str, expires_str)| {
                let acquired_at = DateTime::parse_from_rfc3339(&acquired_str)
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .with_timezone(&Utc);
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(LockInfo {
                    session,
                    agent_id,
                    acquired_at,
                    expires_at,
                })
            })
            .collect()
    }

    /// Get audit log for a session.
    pub async fn get_lock_audit_log(&self, session: &str) -> Result<Vec<LockAuditEntry>> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT session, agent_id, operation, timestamp
             FROM session_lock_audit
             WHERE session = ?
             ORDER BY id ASC",
        )
        .bind(session)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(|(session, agent_id, operation, timestamp_str)| {
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(LockAuditEntry {
                    session,
                    agent_id,
                    operation,
                    timestamp,
                })
            })
            .collect()
    }

    /// Get current lock state for a session.
    pub async fn get_lock_state(&self, session: &str) -> Result<LockState> {
        let now_str = Utc::now().to_rfc3339();

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT agent_id, expires_at FROM session_locks
             WHERE session = ? AND expires_at >= ?",
        )
        .bind(session)
        .bind(&now_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        match existing {
            Some((holder, expires_str)) => {
                let expires_at = DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .with_timezone(&Utc);
                Ok(LockState {
                    session: session.to_string(),
                    holder: Some(holder),
                    expires_at: Some(expires_at),
                })
            }
            None => Ok(LockState {
                session: session.to_string(),
                holder: None,
                expires_at: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn test_pool() -> Result<SqlitePool> {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn setup() -> Result<LockManager> {
        let pool = test_pool().await?;
        let mgr = LockManager::new(pool);
        mgr.init().await?;
        Ok(mgr)
    }

    async fn setup_with_ttl(secs: i64) -> Result<LockManager> {
        let pool = test_pool().await?;
        let mgr = LockManager::with_ttl(pool, Duration::seconds(secs));
        mgr.init().await?;
        Ok(mgr)
    }

    // EARS 1: WHEN lock(session, agent_id) called, acquire exclusive lock within 50ms
    #[tokio::test]
    async fn test_lock_acquire_success() -> Result<()> {
        let mgr = setup().await?;
        let start = std::time::Instant::now();
        let resp = mgr.lock("session-1", "agent-a").await?;
        let elapsed = start.elapsed();

        assert_eq!(resp.session, "session-1");
        assert_eq!(resp.agent_id, "agent-a");
        assert!(
            elapsed.as_millis() < 50,
            "Lock acquisition took {elapsed:?}"
        );
        Ok(())
    }

    // EARS 2: WHEN lock held by another agent, return SESSION_LOCKED error with holder info
    #[tokio::test]
    async fn test_lock_contention_returns_session_locked() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        let result = mgr.lock("session-1", "agent-b").await;

        assert!(result.is_err());
        let err = result
            .err()
            .ok_or_else(|| Error::Unknown("expected error".into()))?;
        assert!(matches!(
            &err,
            Error::SessionLocked { session, holder }
            if session == "session-1" && holder == "agent-a"
        ));
        assert_eq!(err.code(), "SESSION_LOCKED");
        Ok(())
    }

    // EARS 3: WHEN unlock(session, agent_id) called by holder, release lock
    #[tokio::test]
    async fn test_unlock_by_holder_succeeds() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        mgr.unlock("session-1", "agent-a").await?;

        // Lock should be released - another agent can now lock
        let resp = mgr.lock("session-1", "agent-b").await?;
        assert_eq!(resp.agent_id, "agent-b");
        Ok(())
    }

    // EARS 4: WHEN unlock called by non-holder, return NOT_LOCK_HOLDER error
    #[tokio::test]
    async fn test_unlock_by_non_holder_fails() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        let result = mgr.unlock("session-1", "agent-b").await;

        assert!(result.is_err());
        let err = result
            .err()
            .ok_or_else(|| Error::Unknown("expected error".into()))?;
        assert!(matches!(
            &err,
            Error::NotLockHolder { session, agent_id }
            if session == "session-1" && agent_id == "agent-b"
        ));
        assert_eq!(err.code(), "NOT_LOCK_HOLDER");
        Ok(())
    }

    // EARS 5: WHEN lock TTL expires, auto-release lock
    #[tokio::test]
    async fn test_expired_lock_allows_new_acquisition() -> Result<()> {
        let mgr = setup_with_ttl(0).await?;
        let _ = mgr.lock("session-1", "agent-a").await?;

        // Small delay to ensure expiry
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Another agent should be able to lock now
        let resp = mgr.lock("session-1", "agent-b").await?;
        assert_eq!(resp.agent_id, "agent-b");
        Ok(())
    }

    // EARS 6: WHEN get_all_locks() called, return all active locks with expiry times
    #[tokio::test]
    async fn test_get_all_locks_returns_active() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        let _ = mgr.lock("session-2", "agent-b").await?;

        let locks = mgr.get_all_locks().await?;
        assert_eq!(locks.len(), 2);

        let sessions: Vec<&str> = locks.iter().map(|l| l.session.as_str()).collect();
        assert!(sessions.contains(&"session-1"));
        assert!(sessions.contains(&"session-2"));
        Ok(())
    }

    // EARS 6 cont: expired locks should NOT appear
    #[tokio::test]
    async fn test_get_all_locks_excludes_expired() -> Result<()> {
        let mgr = setup_with_ttl(0).await?;
        let _ = mgr.lock("session-1", "agent-a").await?;

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let locks = mgr.get_all_locks().await?;
        assert!(locks.is_empty());
        Ok(())
    }

    // EARS 7: WHEN agent heartbeats, extend lock TTL
    #[tokio::test]
    async fn test_heartbeat_extends_ttl() -> Result<()> {
        let mgr = setup_with_ttl(2).await?;
        let lock_resp = mgr.lock("session-1", "agent-a").await?;
        let original_expires = lock_resp.expires_at;

        // Small delay then heartbeat
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let hb = mgr.heartbeat("session-1", "agent-a").await?;
        assert!(hb.expires_at > original_expires);
        Ok(())
    }

    // Heartbeat by non-holder should fail
    #[tokio::test]
    async fn test_heartbeat_by_non_holder_fails() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        let result = mgr.heartbeat("session-1", "agent-b").await;
        assert!(result.is_err());
        Ok(())
    }

    // Heartbeat on non-existent lock should fail
    #[tokio::test]
    async fn test_heartbeat_no_lock_fails() -> Result<()> {
        let mgr = setup().await?;
        let result = mgr.heartbeat("session-1", "agent-a").await;
        assert!(result.is_err());
        Ok(())
    }

    // Re-locking by same agent should succeed (idempotent)
    #[tokio::test]
    async fn test_relock_same_agent_idempotent() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;
        let r2 = mgr.lock("session-1", "agent-a").await?;
        assert_eq!(r2.session, "session-1");
        Ok(())
    }

    // EARS: Double unlock MUST be logged as warning in audit trail
    #[tokio::test]
    async fn test_double_unlock_logs_warning() -> Result<()> {
        let mgr = setup().await?;
        let _ = mgr.lock("session-1", "agent-a").await?;

        // First unlock succeeds
        mgr.unlock("session-1", "agent-a").await?;

        // Second unlock (double unlock) should be detected
        let audit_log = mgr.get_lock_audit_log("session-1").await?;

        // Should have 2 entries: lock + unlock
        assert_eq!(
            audit_log.len(),
            2,
            "Expected 2 audit entries (lock + unlock)"
        );

        // First entry should be lock
        assert_eq!(audit_log[0].operation, "lock");
        assert_eq!(audit_log[0].agent_id, "agent-a");

        // Second entry should be unlock
        assert_eq!(audit_log[1].operation, "unlock");
        assert_eq!(audit_log[1].agent_id, "agent-a");

        // Now try double unlock
        mgr.unlock("session-1", "agent-a").await?;

        let audit_log2 = mgr.get_lock_audit_log("session-1").await?;
        // Should have 3 entries now: lock + unlock + double_unlock_warning
        assert_eq!(
            audit_log2.len(),
            3,
            "Expected 3 audit entries with double unlock warning"
        );

        // Third entry should be marked as double unlock
        assert_eq!(audit_log2[2].operation, "double_unlock_warning");

        Ok(())
    }

    // EARS: Lock state query MUST show current lock holder
    #[tokio::test]
    async fn test_lock_state_query_shows_holder() -> Result<()> {
        let mgr = setup().await?;

        // Initially no lock
        let state = mgr.get_lock_state("session-1").await?;
        assert!(state.holder.is_none(), "Expected no holder initially");

        // After lock, holder should be agent-a
        let _ = mgr.lock("session-1", "agent-a").await?;
        let state = mgr.get_lock_state("session-1").await?;
        assert_eq!(state.holder.as_deref(), Some("agent-a"));

        // After unlock, no holder
        mgr.unlock("session-1", "agent-a").await?;
        let state = mgr.get_lock_state("session-1").await?;
        assert!(state.holder.is_none(), "Expected no holder after unlock");

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SESSION VALIDATION TESTS (zjj-1w0d: Lock Non-Existent Session)
    // ═══════════════════════════════════════════════════════════════════════

    // Test: Lock non-existent session returns error (when sessions table exists)
    #[tokio::test]
    async fn lock_nonexistent_session_returns_not_found_error() -> Result<()> {
        let pool = test_pool().await?;
        let mgr = LockManager::new(pool.clone());

        // Initialize both lock tables and sessions table
        mgr.init().await?;

        // Create sessions table (normally done by SessionDb)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_path TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Try to lock session that doesn't exist
        let result = mgr.lock("ghost-session", "agent-1").await;

        assert!(result.is_err(), "Should fail for non-existent session");

        match result.unwrap_err() {
            Error::SessionNotFound { session, .. } => {
                assert_eq!(session, "ghost-session");
            }
            other => panic!("Expected SessionNotFound, got {:?}", other),
        }

        // Verify no lock was created
        let locks = mgr.get_all_locks().await?;
        assert!(locks.is_empty(), "No lock should exist for non-existent session");

        Ok(())
    }

    // Test: Lock existing session succeeds (requires creating session in database)
    #[tokio::test]
    async fn lock_existing_session_succeeds() -> Result<()> {
        let pool = test_pool().await?;
        let mgr = LockManager::new(pool.clone());
        mgr.init().await?;

        // Create sessions table (normally done by SessionDb)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_path TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Create session first
        sqlx::query("INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)")
            .bind("real-session")
            .bind("active")
            .bind("working")
            .bind("/workspace")
            .execute(&pool)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Lock should succeed
        let result = mgr.lock("real-session", "agent-1").await;

        assert!(result.is_ok(), "Lock should succeed for existing session");

        // Verify lock exists
        let locks = mgr.get_all_locks().await?;
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].session, "real-session");
        assert_eq!(locks[0].agent_id, "agent-1");

        Ok(())
    }

    // Test: Lock after session is deleted fails
    #[tokio::test]
    async fn lock_deleted_session_fails_with_not_found() -> Result<()> {
        let pool = test_pool().await?;
        let mgr = LockManager::new(pool.clone());
        mgr.init().await?;

        // Create sessions table (normally done by SessionDb)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_path TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Create session
        sqlx::query("INSERT INTO sessions (name, status, state, workspace_path) VALUES (?, ?, ?, ?)")
            .bind("ephemeral-session")
            .bind("active")
            .bind("working")
            .bind("/workspace")
            .execute(&pool)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Delete it
        sqlx::query("DELETE FROM sessions WHERE name = ?")
            .bind("ephemeral-session")
            .execute(&pool)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // Try to lock - should fail
        let result = mgr.lock("ephemeral-session", "agent-1").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::SessionNotFound { .. })));

        Ok(())
    }

    // Regression: The exact reported bug - locking non-existent session no longer creates orphaned lock
    #[tokio::test]
    async fn regression_lock_nonexistent_session_no_longer_creates_orphaned_lock() -> Result<()> {
        let pool = test_pool().await?;
        let mgr = LockManager::new(pool.clone());
        mgr.init().await?;

        // Create sessions table (normally done by SessionDb)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_path TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // No sessions exist

        // Try to lock non-existent session (the bug)
        let result = mgr.lock("ghost-session", "agent-1").await;

        // Should fail
        assert!(result.is_err(), "Lock must fail for non-existent session");

        // Most important: NO orphaned lock should exist
        let locks = mgr.get_all_locks().await?;
        assert!(!locks.iter().any(|l| l.session == "ghost-session"),
                "REGRESSION: Orphaned lock created for non-existent session!");

        Ok(())
    }
}

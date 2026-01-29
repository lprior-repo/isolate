//! Agent registry with heartbeat tracking.
//!
//! Tracks active agents via `SQLite`, using timestamps to detect stale agents.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::Error;

/// An agent registry backed by `SQLite`.
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    db: SqlitePool,
    timeout_secs: u64,
}

/// An active agent record.
#[derive(Debug, Clone)]
pub struct ActiveAgent {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Last heartbeat timestamp.
    pub last_seen: DateTime<Utc>,
    /// When the agent first registered.
    pub registered_at: DateTime<Utc>,
    /// Current session the agent is working on.
    pub current_session: Option<String>,
    /// Current command the agent is executing.
    pub current_command: Option<String>,
    /// Number of actions performed by the agent.
    pub actions_count: u64,
}

impl AgentRegistry {
    /// Create a new registry with the given pool and timeout.
    ///
    /// Creates the agents table if it does not exist.
    pub async fn new(db: SqlitePool, timeout_secs: u64) -> Result<Self, Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agents (
                agent_id TEXT PRIMARY KEY,
                last_seen TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                current_session TEXT,
                current_command TEXT,
                actions_count INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create agents table: {e}")))?;

        Ok(Self { db, timeout_secs })
    }

    /// Register an agent (insert or update).
    pub async fn register(&self, agent_id: &str) -> Result<(), Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO agents (agent_id, last_seen, registered_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?2",
        )
        .bind(agent_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to register agent: {e}")))?;

        Ok(())
    }

    /// Update an agent's heartbeat timestamp.
    pub async fn heartbeat(&self, agent_id: &str) -> Result<(), Error> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
            .bind(&now)
            .bind(agent_id)
            .execute(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to heartbeat agent: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Agent not found: {agent_id}")));
        }
        Ok(())
    }

    /// Get all active agents (`last_seen` within timeout).
    pub async fn get_active(&self) -> Result<Vec<ActiveAgent>, Error> {
        let cutoff = Utc::now() - chrono::Duration::seconds(self.timeout_secs.cast_signed());
        let cutoff_str = cutoff.to_rfc3339();

        let rows: Vec<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, last_seen, registered_at, current_session, current_command, actions_count
         FROM agents WHERE last_seen >= ?1",
        )
        .bind(&cutoff_str)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get active agents: {e}")))?;

        rows.into_iter()
            .map(
                |(
                    agent_id,
                    last_seen,
                    registered_at,
                    current_session,
                    current_command,
                    actions_count,
                )| {
                    let last_seen = DateTime::parse_from_rfc3339(&last_seen)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| {
                            Error::ParseError(format!("Invalid last_seen timestamp: {e}"))
                        })?;
                    let registered_at = DateTime::parse_from_rfc3339(&registered_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| {
                            Error::ParseError(format!("Invalid registered_at timestamp: {e}"))
                        })?;
                    Ok(ActiveAgent {
                        agent_id,
                        last_seen,
                        registered_at,
                        current_session,
                        current_command,
                        actions_count: actions_count.cast_unsigned(),
                    })
                },
            )
            .collect()
    }

    /// Unregister an agent, removing its record.
    pub async fn unregister(&self, agent_id: &str) -> Result<(), Error> {
        let result = sqlx::query("DELETE FROM agents WHERE agent_id = ?1")
            .bind(agent_id)
            .execute(&self.db)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to unregister agent: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Agent not found: {agent_id}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn test_pool() -> Result<SqlitePool, Error> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create test pool: {e}")))
    }

    #[tokio::test]
    async fn test_register_creates_agent() {
        let pool = test_pool().await.unwrap_or_else(|_| {
            // This is test code; if we can't create a pool, the test infrastructure is broken
            std::process::exit(1);
        });
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let result = registry.register("agent-1").await;
        assert!(result.is_ok());

        let active = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_heartbeat_updates_last_seen() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        registry
            .register("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let before = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        let before_ts = before[0].last_seen;

        // Small delay to ensure timestamp differs
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        registry
            .heartbeat("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let after = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert!(after[0].last_seen >= before_ts);
    }

    #[tokio::test]
    async fn test_heartbeat_unknown_agent_returns_error() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let result = registry.heartbeat("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stale_agent_not_in_active_list() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        // Use 0 second timeout so everything is immediately stale
        let registry = AgentRegistry::new(pool, 0)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        registry
            .register("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        // Small delay to ensure agent becomes stale
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let active = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_stale_agent_restored_by_heartbeat() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        // Short timeout
        let registry = AgentRegistry::new(pool, 1)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        registry
            .register("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        // Wait for staleness
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let stale = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert!(stale.is_empty());

        // Re-heartbeat restores
        registry
            .heartbeat("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let active = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_unregister_removes_agent() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        registry
            .register("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        let result = registry.unregister("agent-1").await;
        assert!(result.is_ok());

        let active = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_unregister_unknown_agent_returns_error() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        let result = registry.unregister("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_idempotent() {
        let pool = test_pool().await.unwrap_or_else(|_| std::process::exit(1));
        let registry = AgentRegistry::new(pool, 60)
            .await
            .unwrap_or_else(|_| std::process::exit(1));

        registry
            .register("agent-1")
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        let result = registry.register("agent-1").await;
        assert!(result.is_ok());

        let active = registry
            .get_active()
            .await
            .unwrap_or_else(|_| std::process::exit(1));
        assert_eq!(active.len(), 1);
    }
}

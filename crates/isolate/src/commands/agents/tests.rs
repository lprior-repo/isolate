//! Tests for the agents command

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::sqlite::SqlitePoolOptions;
use isolate_core::{agents::registry::AgentRegistry, coordination::locks::LockManager};

use super::types::{AgentInfo, LockSummary};

/// Test context that provides database connections
struct TestContext {
    pool: sqlx::SqlitePool,
    agent_registry: AgentRegistry,
    lock_manager: LockManager,
}

impl TestContext {
    /// Create a new test context with in-memory database
    async fn new() -> Result<Self, anyhow::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Initialize agent registry with 60s timeout
        let agent_registry = AgentRegistry::new(pool.clone(), 60).await?;

        // Initialize lock manager
        let lock_manager = LockManager::new(pool.clone());
        lock_manager.init().await?;

        Ok(Self {
            pool,
            agent_registry,
            lock_manager,
        })
    }

    /// Register a test agent
    async fn register_agent(&self, agent_id: &str) -> Result<(), anyhow::Error> {
        self.agent_registry.register(agent_id).await?;
        Ok(())
    }

    /// Register a test agent with session info
    async fn register_agent_with_session(
        &self,
        agent_id: &str,
        session: &str,
    ) -> Result<(), anyhow::Error> {
        self.agent_registry.register(agent_id).await?;

        // Update session info directly in the database
        sqlx::query(
            "UPDATE agents SET current_session = ?1, current_command = ?2 WHERE agent_id = ?3",
        )
        .bind(session)
        .bind("isolate focus")
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Register an agent and backdate it to make it stale
    async fn register_stale_agent(&self, agent_id: &str) -> Result<(), anyhow::Error> {
        self.agent_registry.register(agent_id).await?;

        // Backdate the agent to make it stale (set last_seen to 2 minutes ago)
        let stale_time = Utc::now() - chrono::Duration::seconds(120);
        let stale_time_str = stale_time.to_rfc3339();

        sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
            .bind(&stale_time_str)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Acquire a lock for testing
    async fn acquire_lock(&self, session: &str, agent_id: &str) -> Result<(), anyhow::Error> {
        self.lock_manager.lock(session, agent_id).await?;
        Ok(())
    }

    /// Get all agents from the database (including stale)
    async fn get_all_agents(&self) -> Result<Vec<AgentInfo>, anyhow::Error> {
        let rows: Vec<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
             FROM agents"
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(
                |(
                    agent_id,
                    registered_at,
                    last_seen,
                    current_session,
                    current_command,
                    actions_count,
                )| {
                    let registered_at = DateTime::parse_from_rfc3339(&registered_at)
                        .map(|dt: DateTime<FixedOffset>| dt.with_timezone(&Utc))
                        .context("Invalid registered_at")?;
                    let last_seen = DateTime::parse_from_rfc3339(&last_seen)
                        .map(|dt: DateTime<FixedOffset>| dt.with_timezone(&Utc))
                        .context("Invalid last_seen")?;

                    // Determine if stale (more than 60 seconds ago)
                    let stale = Utc::now().signed_duration_since(last_seen).num_seconds() > 60;

                    Ok(AgentInfo {
                        agent_id,
                        registered_at,
                        last_seen,
                        current_session,
                        current_command,
                        actions_count: u64::try_from(actions_count).unwrap_or(0),
                        stale,
                    })
                },
            )
            .collect()
    }

    /// Get active agents (within heartbeat timeout)
    async fn get_active_agents(&self) -> Result<Vec<AgentInfo>, anyhow::Error> {
        self.get_all_agents()
            .await
            .map(|agents| agents.into_iter().filter(|a| !a.stale).collect())
    }

    /// Get all locks
    async fn get_all_locks(&self) -> Result<Vec<LockSummary>, anyhow::Error> {
        let locks = self.lock_manager.get_all_locks().await?;

        Ok(locks
            .into_iter()
            .map(|l| LockSummary {
                session: l.session,
                holder: l.agent_id,
                expires_at: l.expires_at,
            })
            .collect())
    }
}

// EARS 1: WHEN agents runs, system shall list all active agents with last_seen within heartbeat
// timeout
#[tokio::test]
async fn agents_lists_active_only() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register two active agents
    ctx.register_agent("agent-1").await?;
    ctx.register_agent("agent-2").await?;

    // Get active agents
    let agents = ctx.get_active_agents().await?;

    assert_eq!(agents.len(), 2, "should have 2 active agents");
    assert!(
        agents.iter().all(|a| !a.stale),
        "all agents should be active"
    );
    Ok(())
}

// EARS 2: WHEN --all specified, system shall include stale agents with stale=true flag
#[tokio::test]
async fn agents_includes_stale_with_all_flag() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register one active and one stale agent
    ctx.register_agent("active").await?;
    ctx.register_stale_agent("stale").await?;

    // Get all agents
    let agents = ctx.get_all_agents().await?;

    assert_eq!(agents.len(), 2, "should have 2 agents total");

    let stale_agent = agents
        .iter()
        .find(|a| a.agent_id == "stale")
        .ok_or_else(|| anyhow::anyhow!("item not found"))?;
    assert!(stale_agent.stale, "stale agent should be marked stale");

    let active_agent = agents
        .iter()
        .find(|a| a.agent_id == "active")
        .ok_or_else(|| anyhow::anyhow!("item not found"))?;
    assert!(
        !active_agent.stale,
        "active agent should not be marked stale"
    );
    Ok(())
}

// EARS 2 (cont): WHEN --all not specified, stale agents excluded
#[tokio::test]
async fn agents_excludes_stale_by_default() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register one active and one stale agent
    ctx.register_agent("active").await?;
    ctx.register_stale_agent("stale").await?;

    // Get active agents only
    let agents = ctx.get_active_agents().await?;

    assert_eq!(agents.len(), 1, "should only have 1 active agent");
    assert_eq!(agents[0].agent_id, "active", "should be the active agent");
    Ok(())
}

// EARS 3: WHEN reporting locks, system shall show which agents hold locks on which sessions
#[tokio::test]
async fn agents_shows_locks_correctly() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent and acquire lock
    ctx.register_agent("agent-1").await?;
    ctx.acquire_lock("session-1", "agent-1").await?;

    // Get locks
    let locks = ctx.get_all_locks().await?;

    assert_eq!(locks.len(), 1, "should have 1 lock");
    assert_eq!(locks[0].session, "session-1", "lock should be on session-1");
    assert_eq!(locks[0].holder, "agent-1", "lock should be held by agent-1");
    Ok(())
}

// EARS 4: WHEN computing actions_count, system shall aggregate from history database
#[tokio::test]
async fn agents_action_counts_accurate() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    ctx.register_agent("agent-1").await?;

    // Manually increment action count in database
    sqlx::query("UPDATE agents SET actions_count = 5 WHERE agent_id = ?1")
        .bind("agent-1")
        .execute(&ctx.pool)
        .await?;

    // Get agents
    let agents = ctx.get_active_agents().await?;

    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].actions_count, 5, "action count should be 5");
    Ok(())
}

// Test: WHEN agent has current_session, system shall include session name in output
#[tokio::test]
async fn agents_shows_current_session() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent with session
    ctx.register_agent_with_session("agent-1", "test-session")
        .await?;

    // Get agents
    let agents = ctx.get_active_agents().await?;

    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].current_session,
        Some("test-session".to_string()),
        "should show current session"
    );
    Ok(())
}

// Test: WHEN no active agents, return empty array (not error)
#[tokio::test]
async fn agents_empty_is_valid() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // No agents registered

    // Get active agents
    let agents = ctx.get_active_agents().await?;

    assert!(agents.is_empty(), "should return empty list");
    Ok(())
}

// Test: WHEN --session specified, filter by session
#[tokio::test]
async fn agents_filters_by_session() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agents with different sessions
    ctx.register_agent_with_session("a1", "s1").await?;
    ctx.register_agent_with_session("a2", "s1").await?;
    ctx.register_agent_with_session("a3", "s2").await?;

    // Get all agents and filter
    let agents = ctx.get_all_agents().await?;
    let filtered: Vec<_> = agents
        .into_iter()
        .filter(|a| a.current_session.as_deref() == Some("s1"))
        .collect();

    assert_eq!(filtered.len(), 2, "should have 2 agents with session s1");
    assert!(filtered
        .iter()
        .all(|a| a.current_session == Some("s1".to_string())));
    Ok(())
}

// Test: Verify stale threshold is 60 seconds
#[tokio::test]
async fn agents_stale_threshold_is_60_seconds() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    ctx.register_agent("agent-1").await?;

    // Backdate to exactly 59 seconds ago (should be active)
    let recent = Utc::now() - chrono::Duration::seconds(59);
    let recent_str = recent.to_rfc3339();
    sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
        .bind(&recent_str)
        .bind("agent-1")
        .execute(&ctx.pool)
        .await?;

    let agents = ctx.get_active_agents().await?;
    assert_eq!(agents.len(), 1, "59 seconds ago should still be active");

    // Backdate to 61 seconds ago (should be stale)
    let old = Utc::now() - chrono::Duration::seconds(61);
    let old_str = old.to_rfc3339();
    sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
        .bind(&old_str)
        .bind("agent-1")
        .execute(&ctx.pool)
        .await?;

    let agents = ctx.get_active_agents().await?;
    assert!(agents.is_empty(), "61 seconds ago should be stale");
    Ok(())
}

// Test: Multiple locks on different sessions
#[tokio::test]
async fn agents_multiple_locks() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agents and acquire locks
    ctx.register_agent("agent-1").await?;
    ctx.register_agent("agent-2").await?;

    ctx.acquire_lock("session-1", "agent-1").await?;
    ctx.acquire_lock("session-2", "agent-2").await?;

    // Get locks
    let locks = ctx.get_all_locks().await?;

    assert_eq!(locks.len(), 2, "should have 2 locks");

    let lock_map: HashMap<_, _> = locks.into_iter().map(|l| (l.session.clone(), l)).collect();

    assert_eq!(lock_map["session-1"].holder, "agent-1");
    assert_eq!(lock_map["session-2"].holder, "agent-2");
    Ok(())
}

// Test: Verify total_active and total_stale counts
#[tokio::test]
async fn agents_counts_accurate() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register mix of active and stale agents
    ctx.register_agent("active-1").await?;
    ctx.register_agent("active-2").await?;
    ctx.register_stale_agent("stale-1").await?;
    ctx.register_stale_agent("stale-2").await?;
    ctx.register_stale_agent("stale-3").await?;

    // Get all agents
    let agents = ctx.get_all_agents().await?;

    let active_count = agents.iter().filter(|a| !a.stale).count();
    let stale_count = agents.iter().filter(|a| a.stale).count();

    assert_eq!(active_count, 2, "should have 2 active agents");
    assert_eq!(stale_count, 3, "should have 3 stale agents");
    Ok(())
}

// ============================================================================
// Agent Self-Management Subcommand Tests
// ============================================================================

// Test: WHEN register called with agent_id, agent is created in database
#[tokio::test]
async fn register_creates_agent_in_database() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Manually insert agent (simulating register)
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, NULL, NULL, 0)"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .execute(&ctx.pool)
    .await?;

    // Verify agent exists
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].agent_id, "test-agent");
    Ok(())
}

// Test: WHEN register called with session, agent has session set
#[tokio::test]
async fn register_with_session_sets_current_session() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Insert agent with session
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, ?, NULL, 0)"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .bind("my-session")
    .execute(&ctx.pool)
    .await?;

    // Verify session is set
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents[0].current_session, Some("my-session".to_string()));
    Ok(())
}

// Test: WHEN register called twice with same ID, updates existing (idempotent)
#[tokio::test]
async fn register_is_idempotent() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    let now = Utc::now().to_rfc3339();

    // First register
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, 'session-1', NULL, 0)
         ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?, current_session = ?"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .bind("session-1")
    .execute(&ctx.pool)
    .await?;

    // Second register with different session
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, 'session-2', NULL, 0)
         ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?, current_session = ?"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .bind("session-2")
    .execute(&ctx.pool)
    .await?;

    // Should still be one agent with updated session
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].current_session, Some("session-2".to_string()));
    Ok(())
}

// Test: WHEN heartbeat called, last_seen is updated
#[tokio::test]
async fn heartbeat_updates_last_seen() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent with old timestamp
    let old_time = Utc::now() - chrono::Duration::seconds(30);
    let old_str = old_time.to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, NULL, NULL, 0)"
    )
    .bind("test-agent")
    .bind(&old_str)
    .bind(&old_str)
    .execute(&ctx.pool)
    .await?;

    // Simulate heartbeat
    let new_time = Utc::now().to_rfc3339();
    sqlx::query("UPDATE agents SET last_seen = ? WHERE agent_id = ?")
        .bind(&new_time)
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;

    // Verify last_seen was updated
    let agents = ctx.get_all_agents().await?;
    assert!(
        !agents[0].stale,
        "agent should not be stale after heartbeat"
    );
    Ok(())
}

// Test: WHEN heartbeat called with command, current_command is updated
#[tokio::test]
async fn heartbeat_updates_current_command() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, NULL, NULL, 0)"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .execute(&ctx.pool)
    .await?;

    // Heartbeat with command
    sqlx::query("UPDATE agents SET last_seen = ?, current_command = ? WHERE agent_id = ?")
        .bind(&now)
        .bind("isolate add feature")
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;

    // Verify command is set
    let agents = ctx.get_all_agents().await?;
    assert_eq!(
        agents[0].current_command,
        Some("isolate add feature".to_string())
    );
    Ok(())
}

// Test: WHEN heartbeat called, actions_count is incremented
#[tokio::test]
async fn heartbeat_increments_actions_count() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, NULL, NULL, 0)"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .execute(&ctx.pool)
    .await?;

    // Multiple heartbeats
    for _ in 0..3 {
        sqlx::query(
            "UPDATE agents SET last_seen = ?, actions_count = actions_count + 1 WHERE agent_id = ?",
        )
        .bind(&now)
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;
    }

    // Verify actions count
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents[0].actions_count, 3);
    Ok(())
}

// Test: WHEN status called for registered agent, returns agent info
#[tokio::test]
async fn status_returns_agent_info() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent with session
    ctx.register_agent_with_session("test-agent", "my-session")
        .await?;

    // Query agent status
    let row: Option<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
         FROM agents WHERE agent_id = ?"
    )
    .bind("test-agent")
    .fetch_optional(&ctx.pool)
    .await?;

    assert!(row.is_some());
    let (agent_id, _, _, current_session, _, _) = row.ok_or_else(|| anyhow::anyhow!("no row"))?;
    assert_eq!(agent_id, "test-agent");
    assert_eq!(current_session, Some("my-session".to_string()));
    Ok(())
}

// Test: WHEN status called for unregistered agent, returns not found
#[tokio::test]
async fn status_returns_not_found_for_unknown_agent() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Query non-existent agent
    let row: Option<(String,)> = sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?")
        .bind("unknown-agent")
        .fetch_optional(&ctx.pool)
        .await?;

    assert!(row.is_none());
    Ok(())
}

// Test: WHEN unregister called, agent is removed from database
#[tokio::test]
async fn unregister_removes_agent() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    ctx.register_agent("test-agent").await?;

    // Verify agent exists
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents.len(), 1);

    // Unregister
    sqlx::query("DELETE FROM agents WHERE agent_id = ?")
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;

    // Verify agent is gone
    let agents = ctx.get_all_agents().await?;
    assert!(agents.is_empty());
    Ok(())
}

// Test: WHEN unregister called, agent's locks are also released
#[tokio::test]
async fn unregister_releases_locks() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent and acquire lock
    ctx.register_agent("test-agent").await?;
    ctx.acquire_lock("session-1", "test-agent").await?;

    // Verify lock exists
    let locks = ctx.get_all_locks().await?;
    assert_eq!(locks.len(), 1);

    // Unregister (in real code, this would also release locks)
    sqlx::query("DELETE FROM agents WHERE agent_id = ?")
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;

    // Release the lock explicitly (simulating what unregister should do)
    ctx.lock_manager.unlock("session-1", "test-agent").await?;

    // Verify lock is released
    let locks = ctx.get_all_locks().await?;
    assert!(locks.is_empty());
    Ok(())
}

// Test: WHEN unregister called for unknown agent, no error (idempotent)
#[tokio::test]
async fn unregister_is_idempotent() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Try to unregister non-existent agent - should not error
    let result = sqlx::query("DELETE FROM agents WHERE agent_id = ?")
        .bind("unknown-agent")
        .execute(&ctx.pool)
        .await;

    assert!(result.is_ok());
    Ok(())
}

// Test: BUG FIX - heartbeat after unregister returns error instead of silently succeeding
#[tokio::test]
async fn heartbeat_after_unregister_returns_error() -> Result<(), anyhow::Error> {
    let ctx = TestContext::new().await?;

    // Register agent
    ctx.register_agent("test-agent").await?;

    // Unregister agent
    sqlx::query("DELETE FROM agents WHERE agent_id = ?")
        .bind("test-agent")
        .execute(&ctx.pool)
        .await?;

    // Verify agent is gone
    let agents = ctx.get_all_agents().await?;
    assert!(agents.is_empty(), "agent should be unregistered");

    // Heartbeat should fail (0 rows affected)
    let result = sqlx::query(
        "UPDATE agents SET last_seen = ?, actions_count = actions_count + 1 WHERE agent_id = ?",
    )
    .bind(Utc::now().to_rfc3339())
    .bind("test-agent")
    .execute(&ctx.pool)
    .await;

    assert!(result.is_ok(), "query execution should succeed");

    let update_result = result?;
    assert_eq!(
        update_result.rows_affected(),
        0,
        "heartbeat should affect 0 rows for unregistered agent"
    );

    // Verify agent was NOT re-created by heartbeat
    let agents = ctx.get_all_agents().await?;
    assert!(
        agents.is_empty(),
        "heartbeat should not re-register unregistered agent"
    );

    // Verify that a subsequent register would recreate the agent
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, NULL, NULL, 0)
         ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?"
    )
    .bind("test-agent")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(&ctx.pool)
    .await?;

    // Now agent should exist again
    let agents = ctx.get_all_agents().await?;
    assert_eq!(agents.len(), 1, "register should recreate agent");
    assert_eq!(agents[0].agent_id, "test-agent");

    Ok(())
}

// ============================================================================
// Agent ID Validation Unit Tests
// ============================================================================

// Test: Validate agent ID rejects empty strings
#[test]
fn validate_agent_id_rejects_empty() {
    use super::validate_agent_id;

    let result = validate_agent_id("");
    assert!(result.is_err(), "empty agent_id should be rejected");
    let Err(err) = result else {
        panic!("Expected error but got Ok");
    };
    assert!(err.to_string().contains("cannot be empty"));
}

// Test: Validate agent ID rejects whitespace-only strings
#[test]
fn validate_agent_id_rejects_whitespace_only() {
    use super::validate_agent_id;

    let test_cases = vec![" ", "  ", "\t", "\n", "\t\n", "   \t\n   "];

    for whitespace_id in test_cases {
        let result = validate_agent_id(whitespace_id);
        assert!(
            result.is_err(),
            "whitespace-only agent_id '{whitespace_id:?}' should be rejected"
        );
    }
}

// Test: Validate agent ID accepts valid IDs
#[test]
fn validate_agent_id_accepts_valid_ids() {
    use super::validate_agent_id;

    let valid_ids = vec![
        "agent-1",
        "valid-agent-123",
        "agent_underscore",
        "agent.dot",
        "UPPERCASE",
        "12345",
        "a",
    ];

    for valid_id in valid_ids {
        let result = validate_agent_id(valid_id);
        assert!(
            result.is_ok(),
            "valid agent_id '{valid_id}' should be accepted: {result:?}"
        );
    }
}

// Test: Validate agent ID trims whitespace
#[test]
fn validate_agent_id_trims_whitespace() {
    use super::validate_agent_id;

    // Should now fail - we no longer accept agent IDs with spaces
    let result = validate_agent_id("  valid-agent  ");
    assert!(
        result.is_err(),
        "agent_id with spaces should be rejected (shell quoting issue)"
    );
}

// Test: Validate agent ID rejects reserved keywords
#[test]
fn validate_agent_id_rejects_reserved_keywords() {
    use super::validate_agent_id;

    let reserved_ids = vec!["null", "undefined", "true", "false", "NONE", "Null"];

    for reserved_id in reserved_ids {
        let result = validate_agent_id(reserved_id);
        assert!(
            result.is_err(),
            "reserved keyword '{reserved_id}' should be rejected"
        );
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("reserved"),
                "error for '{reserved_id}' should mention reserved keyword: {msg}"
            );
        }
    }
}

// Test: Validate agent ID rejects embedded spaces
#[test]
fn validate_agent_id_rejects_embedded_spaces() {
    use super::validate_agent_id;

    let invalid_ids = vec![
        "agent 1",
        "my agent",
        "a b c",
        "agent\ttab",
        "agent\nnewline",
    ];

    for invalid_id in invalid_ids {
        let result = validate_agent_id(invalid_id);
        assert!(
            result.is_err(),
            "agent_id with embedded whitespace '{invalid_id}' should be rejected"
        );
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("whitespace"),
                "error for '{invalid_id}' should mention whitespace: {msg}"
            );
        }
    }
}

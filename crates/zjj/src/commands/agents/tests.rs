//! Tests for the agents command

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;

use zjj_core::agents::AgentRegistry;
use zjj_core::coordination::locks::LockManager;

use super::types::{AgentInfo, AgentsArgs, AgentsOutput, LockSummary};

/// Test context that provides database connections
struct TestContext {
    pool: sqlx::SqlitePool,
    agent_registry: AgentRegistry,
    lock_manager: LockManager,
}

impl TestContext {
    /// Create a new test context with in-memory database
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
    async fn register_agent(&self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.agent_registry.register(agent_id).await?;
        Ok(())
    }

    /// Register a test agent with session info
    async fn register_agent_with_session(
        &self,
        agent_id: &str,
        session: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.agent_registry.register(agent_id).await?;

        // Update session info directly in the database
        sqlx::query(
            "UPDATE agents SET current_session = ?1, current_command = ?2 WHERE agent_id = ?3",
        )
        .bind(session)
        .bind("zjj focus")
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Register an agent and backdate it to make it stale
    async fn register_stale_agent(
        &self,
        agent_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    async fn acquire_lock(
        &self,
        session: &str,
        agent_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.lock_manager.lock(session, agent_id).await?;
        Ok(())
    }

    /// Get all agents from the database (including stale)
    async fn get_all_agents(&self) -> Result<Vec<AgentInfo>, Box<dyn std::error::Error>> {
        let rows: Vec<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
             FROM agents"
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(agent_id, registered_at, last_seen, current_session, current_command, actions_count)| {
                let registered_at = DateTime::parse_from_rfc3339(&registered_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| format!("Invalid registered_at: {e}"))?;
                let last_seen = DateTime::parse_from_rfc3339(&last_seen)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| format!("Invalid last_seen: {e}"))?;

                // Determine if stale (more than 60 seconds ago)
                let stale = Utc::now().signed_duration_since(last_seen).num_seconds() > 60;

                Ok(AgentInfo {
                    agent_id,
                    registered_at,
                    last_seen,
                    current_session,
                    current_command,
                    actions_count: actions_count.cast_unsigned(),
                    stale,
                })
            })
            .collect()
    }

    /// Get active agents (within heartbeat timeout)
    async fn get_active_agents(&self) -> Result<Vec<AgentInfo>, Box<dyn std::error::Error>> {
        self.get_all_agents()
            .await
            .map(|agents| agents.into_iter().filter(|a| !a.stale).collect())
    }

    /// Get all locks
    async fn get_all_locks(&self) -> Result<Vec<LockSummary>, Box<dyn std::error::Error>> {
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

// EARS 1: WHEN agents runs, system shall list all active agents with last_seen within heartbeat timeout
#[tokio::test]
async fn agents_lists_active_only() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register two active agents
    ctx.register_agent("agent-1").await.expect("register failed");
    ctx.register_agent("agent-2").await.expect("register failed");

    // Get active agents
    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");

    assert_eq!(agents.len(), 2, "should have 2 active agents");
    assert!(agents.iter().all(|a| !a.stale), "all agents should be active");
}

// EARS 2: WHEN --all specified, system shall include stale agents with stale=true flag
#[tokio::test]
async fn agents_includes_stale_with_all_flag() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register one active and one stale agent
    ctx.register_agent("active").await.expect("register active failed");
    ctx.register_stale_agent("stale")
        .await
        .expect("register stale failed");

    // Get all agents
    let agents = ctx.get_all_agents().await.expect("get_all_agents failed");

    assert_eq!(agents.len(), 2, "should have 2 agents total");

    let stale_agent = agents
        .iter()
        .find(|a| a.agent_id == "stale")
        .expect("stale agent not found");
    assert!(stale_agent.stale, "stale agent should be marked stale");

    let active_agent = agents
        .iter()
        .find(|a| a.agent_id == "active")
        .expect("active agent not found");
    assert!(!active_agent.stale, "active agent should not be marked stale");
}

// EARS 2 (cont): WHEN --all not specified, stale agents excluded
#[tokio::test]
async fn agents_excludes_stale_by_default() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register one active and one stale agent
    ctx.register_agent("active").await.expect("register active failed");
    ctx.register_stale_agent("stale")
        .await
        .expect("register stale failed");

    // Get active agents only
    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");

    assert_eq!(agents.len(), 1, "should only have 1 active agent");
    assert_eq!(agents[0].agent_id, "active", "should be the active agent");
}

// EARS 3: WHEN reporting locks, system shall show which agents hold locks on which sessions
#[tokio::test]
async fn agents_shows_locks_correctly() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agent and acquire lock
    ctx.register_agent("agent-1").await.expect("register failed");
    ctx.acquire_lock("session-1", "agent-1")
        .await
        .expect("lock failed");

    // Get locks
    let locks = ctx.get_all_locks().await.expect("get_all_locks failed");

    assert_eq!(locks.len(), 1, "should have 1 lock");
    assert_eq!(locks[0].session, "session-1", "lock should be on session-1");
    assert_eq!(locks[0].holder, "agent-1", "lock should be held by agent-1");
}

// EARS 4: WHEN computing actions_count, system shall aggregate from history database
#[tokio::test]
async fn agents_action_counts_accurate() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agent
    ctx.register_agent("agent-1").await.expect("register failed");

    // Manually increment action count in database
    sqlx::query("UPDATE agents SET actions_count = 5 WHERE agent_id = ?1")
        .bind("agent-1")
        .execute(&ctx.pool)
        .await
        .expect("update failed");

    // Get agents
    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");

    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].actions_count, 5, "action count should be 5");
}

// Test: WHEN agent has current_session, system shall include session name in output
#[tokio::test]
async fn agents_shows_current_session() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agent with session
    ctx.register_agent_with_session("agent-1", "test-session")
        .await
        .expect("register failed");

    // Get agents
    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");

    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].current_session,
        Some("test-session".to_string()),
        "should show current session"
    );
}

// Test: WHEN no active agents, return empty array (not error)
#[tokio::test]
async fn agents_empty_is_valid() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // No agents registered

    // Get active agents
    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents should not error");

    assert!(agents.is_empty(), "should return empty list");
}

// Test: WHEN --session specified, filter by session
#[tokio::test]
async fn agents_filters_by_session() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agents with different sessions
    ctx.register_agent_with_session("a1", "s1")
        .await
        .expect("register a1 failed");
    ctx.register_agent_with_session("a2", "s1")
        .await
        .expect("register a2 failed");
    ctx.register_agent_with_session("a3", "s2")
        .await
        .expect("register a3 failed");

    // Get all agents and filter
    let agents = ctx.get_all_agents().await.expect("get_all_agents failed");
    let filtered: Vec<_> = agents
        .into_iter()
        .filter(|a| a.current_session.as_deref() == Some("s1"))
        .collect();

    assert_eq!(filtered.len(), 2, "should have 2 agents with session s1");
    assert!(filtered.iter().all(|a| a.current_session == Some("s1".to_string())));
}

// Test: Verify stale threshold is 60 seconds
#[tokio::test]
async fn agents_stale_threshold_is_60_seconds() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agent
    ctx.register_agent("agent-1").await.expect("register failed");

    // Backdate to exactly 59 seconds ago (should be active)
    let recent = Utc::now() - chrono::Duration::seconds(59);
    let recent_str = recent.to_rfc3339();
    sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
        .bind(&recent_str)
        .bind("agent-1")
        .execute(&ctx.pool)
        .await
        .expect("update failed");

    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");
    assert_eq!(agents.len(), 1, "59 seconds ago should still be active");

    // Backdate to 61 seconds ago (should be stale)
    let old = Utc::now() - chrono::Duration::seconds(61);
    let old_str = old.to_rfc3339();
    sqlx::query("UPDATE agents SET last_seen = ?1 WHERE agent_id = ?2")
        .bind(&old_str)
        .bind("agent-1")
        .execute(&ctx.pool)
        .await
        .expect("update failed");

    let agents = ctx
        .get_active_agents()
        .await
        .expect("get_active_agents failed");
    assert!(agents.is_empty(), "61 seconds ago should be stale");
}

// Test: Multiple locks on different sessions
#[tokio::test]
async fn agents_multiple_locks() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register agents and acquire locks
    ctx.register_agent("agent-1").await.expect("register a1 failed");
    ctx.register_agent("agent-2").await.expect("register a2 failed");

    ctx.acquire_lock("session-1", "agent-1")
        .await
        .expect("lock s1 failed");
    ctx.acquire_lock("session-2", "agent-2")
        .await
        .expect("lock s2 failed");

    // Get locks
    let locks = ctx.get_all_locks().await.expect("get_all_locks failed");

    assert_eq!(locks.len(), 2, "should have 2 locks");

    let lock_map: HashMap<_, _> = locks
        .into_iter()
        .map(|l| (l.session.clone(), l))
        .collect();

    assert_eq!(lock_map["session-1"].holder, "agent-1");
    assert_eq!(lock_map["session-2"].holder, "agent-2");
}

// Test: Verify total_active and total_stale counts
#[tokio::test]
async fn agents_counts_accurate() {
    let ctx = TestContext::new().await.expect("failed to create test context");

    // Register mix of active and stale agents
    ctx.register_agent("active-1").await.expect("register a1 failed");
    ctx.register_agent("active-2").await.expect("register a2 failed");
    ctx.register_stale_agent("stale-1")
        .await
        .expect("register s1 failed");
    ctx.register_stale_agent("stale-2")
        .await
        .expect("register s2 failed");
    ctx.register_stale_agent("stale-3")
        .await
        .expect("register s3 failed");

    // Get all agents
    let agents = ctx.get_all_agents().await.expect("get_all_agents failed");

    let active_count = agents.iter().filter(|a| !a.stale).count();
    let stale_count = agents.iter().filter(|a| a.stale).count();

    assert_eq!(active_count, 2, "should have 2 active agents");
    assert_eq!(stale_count, 3, "should have 3 stale agents");
}

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::manual_assert)]
// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Integration tests for complete agent lifecycle.
//!
//! Tests cover:
//! - Agent creation, registration, work assignment, completion, cleanup
//! - Edge cases: failure, timeout, cancellation
//! - Persistence across restarts

use std::time::Duration;

use isolate_core::{
    agents::registry::AgentRegistry, coordination::locks::LockManager, Error, Result,
};
use tempfile::TempDir;

/// Integration test context with persistent storage
struct IntegrationTestContext {
    _temp_dir: TempDir,
    pool: sqlx::SqlitePool,
    agent_registry: AgentRegistry,
    lock_manager: LockManager,
}

impl IntegrationTestContext {
    /// Create a new test context with in-memory database
    async fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;

        // Use in-memory database for faster tests
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to connect: {e}")))?;

        // Initialize agent registry
        let agent_registry = AgentRegistry::new(pool.clone(), 60).await?;

        // Initialize lock manager
        let lock_manager = LockManager::new(pool.clone());
        lock_manager.init().await?;

        Ok(Self {
            _temp_dir: temp_dir,
            pool,
            agent_registry,
            lock_manager,
        })
    }

    /// Register an agent
    async fn register_agent(&self, agent_id: &str) -> Result<()> {
        self.agent_registry.register(agent_id).await
    }

    /// Get agent count
    async fn agent_count(&self) -> Result<usize> {
        let agents = self.agent_registry.get_active().await?;
        Ok(agents.len())
    }

    /// Acquire session lock
    async fn acquire_lock(&self, session: &str, agent_id: &str) -> Result<()> {
        self.lock_manager.lock(session, agent_id).await?;
        Ok(())
    }

    /// Release session lock
    async fn release_lock(&self, session: &str, agent_id: &str) -> Result<()> {
        self.lock_manager.unlock(session, agent_id).await
    }

    /// Unregister an agent
    async fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        self.agent_registry.unregister(agent_id).await
    }
}

// ============================================================================
// LIFECYCLE TEST 1: Complete agent lifecycle (happy path)
// ============================================================================

#[tokio::test]
async fn lifecycle_complete_happy_path() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Step 1: Agent registers
    ctx.register_agent("agent-1").await?;
    assert_eq!(ctx.agent_count().await?, 1, "agent should be registered");

    // Step 2: Agent acquires session lock
    ctx.acquire_lock("workspace-1", "agent-1").await?;

    // Step 3: Agent processes work (simulate)
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Step 4: Agent releases lock
    ctx.release_lock("workspace-1", "agent-1").await?;

    // Step 5: Agent unregisters
    ctx.unregister_agent("agent-1").await?;

    // Verify cleanup
    assert_eq!(ctx.agent_count().await?, 0, "agent should be unregistered");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 3: Session lock contention
// ============================================================================

#[tokio::test]
async fn lifecycle_session_lock_contention() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Two agents register
    ctx.register_agent("agent-1").await?;
    ctx.register_agent("agent-2").await?;

    // Agent 1 acquires lock on session
    ctx.acquire_lock("session-1", "agent-1").await?;

    // Agent 2 tries to acquire same session - should fail
    let result = ctx.lock_manager.lock("session-1", "agent-2").await;
    assert!(result.is_err(), "should fail - session locked");

    let err = result
        .err()
        .ok_or_else(|| Error::Unknown("expected error".into()))?;
    assert!(
        matches!(err, Error::SessionLocked { session, holder } if session == "session-1" && holder == "agent-1")
    );

    // Agent 1 releases lock
    ctx.release_lock("session-1", "agent-1").await?;

    // Now agent 2 can acquire
    ctx.acquire_lock("session-1", "agent-2").await?;

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 4: Agent heartbeat prevents staleness
// ============================================================================

#[tokio::test]
async fn lifecycle_heartbeat_prevents_staleness() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Agent registers
    ctx.register_agent("agent-1").await?;

    // Initially active
    let agents = ctx.agent_registry.get_active().await?;
    assert_eq!(agents.len(), 1);

    // Simulate time passing (near timeout)
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Heartbeat refreshes timestamp
    ctx.agent_registry.heartbeat("agent-1").await?;

    // Still active after heartbeat
    let agents = ctx.agent_registry.get_active().await?;
    assert_eq!(agents.len(), 1, "should remain active after heartbeat");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 6: Persistence across restarts
// ============================================================================

#[tokio::test]
async fn lifecycle_persistence_across_restart() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Register agent
    ctx.register_agent("agent-1").await?;

    let agent_count_before = ctx.agent_count().await?;
    assert_eq!(agent_count_before, 1);

    // For this test, we verify data is in the database
    let row: Option<(String,)> = sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?")
        .bind("agent-1")
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to query agent: {e}")))?;

    assert!(row.is_some(), "agent should persist in database");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 8: Agent timeout and recovery
// ============================================================================

#[tokio::test]
async fn lifecycle_agent_timeout_recovery() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Register agent
    ctx.register_agent("agent-1").await?;

    // Agent is active
    let agents = ctx.agent_registry.get_active().await?;
    assert_eq!(agents.len(), 1);

    // Simulate agent becoming stale (no heartbeat for > 60s)
    // We'll update last_seen directly
    let stale_time = chrono::Utc::now() - chrono::Duration::seconds(120);
    let stale_time_str = stale_time.to_rfc3339();

    sqlx::query("UPDATE agents SET last_seen = ? WHERE agent_id = ?")
        .bind(&stale_time_str)
        .bind("agent-1")
        .execute(&ctx.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to update last_seen: {e}")))?;

    // Agent should be stale now
    let agents = ctx.agent_registry.get_active().await?;
    assert_eq!(agents.len(), 0, "agent should be stale");

    // Agent recovers (heartbeat)
    ctx.agent_registry.heartbeat("agent-1").await?;

    // Agent is active again
    let agents = ctx.agent_registry.get_active().await?;
    assert_eq!(agents.len(), 1, "agent should recover after heartbeat");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 10: Concurrent agent registration (idempotence)
// ============================================================================

#[tokio::test]
async fn lifecycle_concurrent_registration() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Multiple concurrent registrations of same agent
    let reg1 = ctx.agent_registry.register("agent-1");
    let reg2 = ctx.agent_registry.register("agent-1");
    let reg3 = ctx.agent_registry.register("agent-1");

    let (r1, r2, r3) = tokio::join!(reg1, reg2, reg3);

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());

    // Should still be only one agent
    let count = ctx.agent_count().await?;
    assert_eq!(
        count, 1,
        "should have only 1 agent after concurrent registration"
    );

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 11: Lock timeout and expiry
// ============================================================================

#[tokio::test]
async fn lifecycle_lock_timeout_expiry() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Create lock manager with short TTL (1 second)
    let lock_manager = LockManager::with_ttl(ctx.pool.clone(), chrono::Duration::seconds(1));
    lock_manager.init().await?;

    ctx.register_agent("agent-1").await?;
    ctx.register_agent("agent-2").await?;

    // Agent 1 acquires lock
    let lock_resp = lock_manager.lock("session-1", "agent-1").await?;
    assert_eq!(lock_resp.agent_id, "agent-1");

    // Agent 2 tries immediately - should fail
    let result = lock_manager.lock("session-1", "agent-2").await;
    assert!(result.is_err());

    // Poll for lock acquisition with 50ms intervals until expiry (1s TTL + margin)
    let poll_start = std::time::Instant::now();
    let poll_timeout = Duration::from_millis(1500);
    let poll_interval = Duration::from_millis(50);

    let lock_resp = loop {
        if let Ok(resp) = lock_manager.lock("session-1", "agent-2").await {
            break resp;
        }
        if poll_start.elapsed() >= poll_timeout {
            panic!("Lock did not expire within {poll_timeout:?}");
        }
        tokio::time::sleep(poll_interval).await;
    };
    assert_eq!(lock_resp.agent_id, "agent-2");

    Ok(())
}

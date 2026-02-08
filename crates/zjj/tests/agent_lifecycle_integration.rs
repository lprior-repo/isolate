//! Integration tests for complete agent lifecycle.
//!
//! Tests cover:
//! - Agent creation, registration, work assignment, completion, cleanup
//! - Edge cases: failure, timeout, cancellation
//! - Persistence across restarts

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

use std::time::Duration;

use tempfile::TempDir;
use zjj_core::{
    agents::registry::AgentRegistry,
    coordination::{locks::LockManager, queue::MergeQueue},
    Error, Result,
};

/// Integration test context with persistent storage
struct IntegrationTestContext {
    _temp_dir: TempDir,
    pool: sqlx::SqlitePool,
    agent_registry: AgentRegistry,
    lock_manager: LockManager,
    merge_queue: MergeQueue,
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

        // Initialize merge queue with in-memory database
        let merge_queue = MergeQueue::open_in_memory().await?;

        Ok(Self {
            _temp_dir: temp_dir,
            pool,
            agent_registry,
            lock_manager,
            merge_queue,
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

    /// Add work to queue
    async fn add_work(&self, workspace: &str, priority: i32) -> Result<()> {
        self.merge_queue
            .add(workspace, Some("bead-1"), priority, None)
            .await?;
        Ok(())
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

    /// Get processing lock holder
    async fn get_processing_lock_holder(&self) -> Result<Option<String>> {
        let lock = self.merge_queue.get_processing_lock().await?;
        Ok(lock.map(|l| l.agent_id))
    }

    /// Get queue stats
    async fn queue_stats(&self) -> Result<zjj_core::coordination::queue::QueueStats> {
        self.merge_queue.stats().await
    }

    /// Simulate timeout by resetting entry status back to pending
    /// This simulates what would happen when a lock expires in real system
    async fn simulate_timeout_recovery(&self, workspace: &str) -> Result<()> {
        sqlx::query("UPDATE merge_queue SET status = 'pending', started_at = NULL, agent_id = NULL WHERE workspace = ?1")
            .bind(workspace)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to reset entry: {e}")))?;
        Ok(())
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

    // Step 2: Work assigned (queue entry exists)
    ctx.add_work("workspace-1", 5).await?;

    // Step 3: Agent claims work
    let entry = ctx.merge_queue.next_with_lock("agent-1").await?;
    assert!(entry.is_some(), "agent should claim work");

    // Step 4: Agent acquires session lock
    ctx.acquire_lock("workspace-1", "agent-1").await?;

    // Step 5: Agent processes work (simulate)
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Step 6: Agent completes work
    ctx.merge_queue.mark_completed("workspace-1").await?;

    // Step 7: Agent releases lock
    ctx.release_lock("workspace-1", "agent-1").await?;

    // Step 8: Agent unregisters
    ctx.unregister_agent("agent-1").await?;

    // Verify cleanup
    assert_eq!(ctx.agent_count().await?, 0, "agent should be unregistered");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 2: Agent failure during processing
// ============================================================================

#[tokio::test]
async fn lifecycle_agent_failure_during_processing() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Agent registers and claims work
    ctx.register_agent("agent-1").await?;
    ctx.add_work("workspace-1", 5).await?;

    let entry = ctx.merge_queue.next_with_lock("agent-1").await?;
    assert!(entry.is_some());

    // Agent acquires lock
    ctx.acquire_lock("workspace-1", "agent-1").await?;

    // Simulate agent crash (database closes, but state persists)
    // In real scenario, agent process dies
    ctx.lock_manager.unlock("workspace-1", "agent-1").await?;

    // After crash, processing lock should timeout (TTL expiry)
    // Simulate timeout by checking lock state
    let lock_holder = ctx.get_processing_lock_holder().await?;
    assert_eq!(lock_holder, Some("agent-1".to_string()));

    // Another agent should be able to claim after timeout
    // (in real system, lock expires after TTL)
    ctx.register_agent("agent-2").await?;

    // Agent 2 cannot claim yet (lock held by dead agent, entry is processing)
    let entry2 = ctx.merge_queue.next_with_lock("agent-2").await?;
    assert!(entry2.is_none(), "should not claim while entry is processing");

    // Simulate timeout by resetting entry status back to pending
    // (in real system, this would happen via timeout/recovery mechanism)
    ctx.simulate_timeout_recovery("workspace-1").await?;

    // Now agent 2 can claim
    let entry3 = ctx.merge_queue.next_with_lock("agent-2").await?;
    assert!(entry3.is_some(), "should claim after timeout and status reset");

    // Complete work
    ctx.merge_queue.mark_completed("workspace-1").await?;

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
// LIFECYCLE TEST 5: Work priority ordering
// ============================================================================

#[tokio::test]
async fn lifecycle_work_priority_ordering() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    ctx.register_agent("agent-1").await?;

    // Add work with different priorities
    ctx.add_work("low-priority", 10).await?;
    ctx.add_work("high-priority", 0).await?;
    ctx.add_work("mid-priority", 5).await?;

    // Next should be high priority
    let entry = ctx.merge_queue.next_with_lock("agent-1").await?;
    assert!(entry.is_some());
    assert_eq!(
        entry.as_ref().map(|e| e.workspace.as_str()),
        Some("high-priority")
    );

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 6: Persistence across restarts
// ============================================================================

#[tokio::test]
async fn lifecycle_persistence_across_restart() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    // Register agent and add work
    ctx.register_agent("agent-1").await?;
    ctx.add_work("workspace-1", 5).await?;

    let agent_count_before = ctx.agent_count().await?;
    assert_eq!(agent_count_before, 1);

    let stats_before = ctx.queue_stats().await?;
    assert_eq!(stats_before.pending, 1);

    // Note: We can't truly restart with the current structure because
    // we can't clone TempDir. In a real persistence test, we'd:
    // 1. Store the temp path
    // 2. Drop the context
    // 3. Create new context with same path
    //
    // For this test, we verify data is in the database
    let row: Option<(String,)> = sqlx::query_as("SELECT agent_id FROM agents WHERE agent_id = ?")
        .bind("agent-1")
        .fetch_optional(&ctx.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to query agent: {e}")))?;

    assert!(row.is_some(), "agent should persist in database");

    let queue_entry = ctx.merge_queue.get_by_workspace("workspace-1").await?;
    assert!(queue_entry.is_some(), "work should persist in queue");

    Ok(())
}

// ============================================================================
// LIFECYCLE TEST 7: Cancellation scenario
// ============================================================================

#[tokio::test]
async fn lifecycle_work_cancellation() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    ctx.register_agent("agent-1").await?;
    ctx.add_work("workspace-1", 5).await?;

    // Agent claims work
    let entry = ctx.merge_queue.next_with_lock("agent-1").await?;
    assert!(entry.is_some());

    // Work is cancelled (removed from queue)
    let removed = ctx.merge_queue.remove("workspace-1").await?;
    assert!(removed, "work should be removed");

    // Verify it's gone
    let entry_after = ctx.merge_queue.get_by_workspace("workspace-1").await?;
    assert!(entry_after.is_none(), "work should not exist after removal");

    // Release lock
    ctx.merge_queue.release_processing_lock("agent-1").await?;

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
// LIFECYCLE TEST 9: Cleanup of completed/failed work
// ============================================================================

#[tokio::test]
async fn lifecycle_cleanup_old_work() -> Result<()> {
    let ctx = IntegrationTestContext::new().await?;

    ctx.register_agent("agent-1").await?;

    // Add and complete work
    ctx.add_work("workspace-1", 5).await?;
    let entry = ctx.merge_queue.next_with_lock("agent-1").await?;
    assert!(entry.is_some());

    ctx.merge_queue.mark_completed("workspace-1").await?;
    ctx.merge_queue.release_processing_lock("agent-1").await?;

    // Verify completed
    let stats = ctx.queue_stats().await?;
    assert_eq!(stats.completed, 1);

    // Cleanup old entries (max_age = 1 second, should clean recent completions)
    let cleaned = ctx.merge_queue.cleanup(Duration::from_secs(1)).await?;
    assert_eq!(cleaned, 1, "should clean 1 completed entry");

    // Verify cleanup
    let stats_after = ctx.queue_stats().await?;
    assert_eq!(stats_after.completed, 0);

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

    // Wait for lock to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Agent 2 should now be able to acquire
    let lock_resp = lock_manager.lock("session-1", "agent-2").await?;
    assert_eq!(lock_resp.agent_id, "agent-2");

    Ok(())
}

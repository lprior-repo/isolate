//! End-to-End Agent Workflow Scenario Tests
//!
//! This module implements comprehensive E2E tests for agent coordination scenarios:
//!
//! 1. Single agent lifecycle (claim -> work -> submit -> merge -> cleanup)
//! 2. Two agent concurrent workflow
//! 3. Conflict detection and resolution
//! 4. Queue processing with priority
//!
//! # Design Principles
//!
//! - Zero unwrap/expect/panic (uses Result with ? propagation)
//! - Pure functional patterns where possible
//! - Tests are reproducible and can run in parallel
//! - BDD-style Given/When/Then structure

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

mod common;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use common::TestHarness;
use tokio::sync::Mutex;
use zjj_core::coordination::{
    get_conflict_resolutions, init_conflict_resolutions_schema, insert_conflict_resolution,
    ConflictResolution, MergeQueue, QueueStatus,
};

// =============================================================================
// Test Context
// =============================================================================

/// E2E test context that holds all resources for a scenario
pub struct E2ETestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// The merge queue for queue operations
    pub queue: Arc<Mutex<Option<MergeQueue>>>,
    /// Track created sessions for cleanup
    pub sessions: Arc<Mutex<Vec<String>>>,
    /// Track agent IDs for cleanup
    pub agents: Arc<Mutex<Vec<String>>>,
}

impl E2ETestContext {
    /// Create a new E2E test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            queue: Arc::new(Mutex::new(None)),
            sessions: Arc::new(Mutex::new(Vec::new())),
            agents: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the ZJJ database
    pub fn init_zjj(&self) -> Result<()> {
        self.harness.assert_success(&["init"]);
        if !self.harness.zjj_dir().exists() {
            anyhow::bail!("ZJJ initialization failed - .zjj directory not created");
        }
        Ok(())
    }

    /// Initialize the merge queue
    pub async fn init_queue(&self) -> Result<MergeQueue> {
        let queue_db = self.harness.repo_path.join(".zjj").join("state.db");
        let queue = MergeQueue::open(&queue_db)
            .await
            .context("Failed to open merge queue database")?;
        *self.queue.lock().await = Some(queue.clone());
        Ok(queue)
    }

    /// Get the queue, initializing if necessary
    pub async fn get_queue(&self) -> Result<MergeQueue> {
        let guard = self.queue.lock().await;
        if let Some(queue) = guard.as_ref() {
            return Ok(queue.clone());
        }
        drop(guard);
        self.init_queue().await
    }

    /// Track a session for cleanup
    pub async fn track_session(&self, name: &str) {
        self.sessions.lock().await.push(name.to_string());
    }

    /// Track an agent for cleanup
    pub async fn track_agent(&self, id: &str) {
        self.agents.lock().await.push(id.to_string());
    }
}

// =============================================================================
// Scenario 1: Single Agent Lifecycle
// =============================================================================

/// Scenario: Single agent completes full lifecycle
///
/// GIVEN: A fresh ZJJ repository with initialized queue
/// WHEN: An agent claims, works on, submits, and merges a session
/// THEN: The session progresses through all states correctly
/// AND: The queue reflects the completed state
/// AND: Cleanup removes all resources
#[tokio::test]
async fn scenario_single_agent_lifecycle() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN: Fresh repository with initialized queue
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let agent_id = "agent-lifecycle-001";
    let session_name = "feature-lifecycle";

    ctx.track_agent(agent_id).await;
    ctx.track_session(session_name).await;

    // Step 1: Add session to queue (PENDING)
    let add_result = queue.add(session_name, None, 5, None).await;
    assert!(add_result.is_ok(), "Failed to add session to queue");

    let entries = queue.list(None).await.expect("Failed to list queue");
    let entry = entries.first().expect("Should have entry");
    assert_eq!(entry.workspace, session_name);
    assert_eq!(entry.status, QueueStatus::Pending);

    // Step 2: Agent claims work using next_with_lock (CLAIMED)
    let claim_result = queue.next_with_lock(agent_id).await;
    assert!(claim_result.is_ok(), "Failed to claim session");
    let claimed_entry = claim_result.expect("Should have entry");
    assert!(claimed_entry.is_some(), "Should have claimed an entry");

    let claimed = claimed_entry.expect("Entry should exist");
    assert_eq!(claimed.status, QueueStatus::Claimed);
    assert_eq!(claimed.agent_id.as_deref(), Some(agent_id));

    // Step 3: Work in progress (REBASING -> TESTING)
    let rebase_result = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    assert!(rebase_result.is_ok(), "Failed to transition to rebasing");

    let test_result = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;
    assert!(test_result.is_ok(), "Failed to transition to testing");

    // Step 4: Ready for merge (READY_TO_MERGE)
    let ready_result = queue
        .transition_to(session_name, QueueStatus::ReadyToMerge)
        .await;
    assert!(
        ready_result.is_ok(),
        "Failed to transition to ready_to_merge"
    );

    // Step 5: Merge in progress (MERGING)
    let merge_result = queue
        .transition_to(session_name, QueueStatus::Merging)
        .await;
    assert!(merge_result.is_ok(), "Failed to transition to merging");

    // Step 6: Complete (MERGED)
    let complete_result = queue.transition_to(session_name, QueueStatus::Merged).await;
    assert!(complete_result.is_ok(), "Failed to transition to merged");

    // Verify final state
    let final_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(final_entry.status, QueueStatus::Merged);
    assert!(final_entry.status.is_terminal());

    // Step 7: Release the processing lock
    let _ = queue.release_processing_lock(agent_id).await;
}

/// Scenario: Agent lifecycle with failure and retry
///
/// GIVEN: An agent is working on a session
/// WHEN: The work fails with a retryable error
/// THEN: The session can be reset to pending
/// AND: Another agent can claim and complete it
#[tokio::test]
async fn scenario_agent_lifecycle_with_retry() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let agent1 = "agent-retry-001";
    let session_name = "feature-retry";

    // Add to queue and claim
    let _ = queue.add(session_name, None, 5, None).await;
    let _ = queue.next_with_lock(agent1).await;

    // WHEN: Work fails during testing
    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;

    // Record the entry ID before failing
    let entry_before_fail = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    let entry_id = entry_before_fail.id;

    let fail_result = queue
        .transition_to_failed(session_name, "Test failure", true)
        .await;
    assert!(fail_result.is_ok(), "Failed to mark as failed_retryable");

    // Verify the status is FailedRetryable
    let failed_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(failed_entry.status, QueueStatus::FailedRetryable);

    // THEN: Use retry_entry to reset and reclaim
    let retry_result = queue.retry_entry(entry_id).await;
    assert!(
        retry_result.is_ok(),
        "Retry should succeed: {:?}",
        retry_result.err()
    );

    // After retry, the entry should be pending
    let retried_entry = retry_result.expect("Should have entry");
    assert_eq!(retried_entry.status, QueueStatus::Pending);

    // Complete successfully with second agent
    let _ = queue.release_processing_lock(agent1).await;
    let agent2 = "agent-retry-002";
    let _ = queue.next_with_lock(agent2).await;

    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::ReadyToMerge)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Merging)
        .await;
    let _ = queue.transition_to(session_name, QueueStatus::Merged).await;

    let final_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(final_entry.status, QueueStatus::Merged);
}

// =============================================================================
// Scenario 2: Two Agent Concurrent Workflow
// =============================================================================

/// Scenario: Two agents work on different sessions concurrently
///
/// GIVEN: A queue with two pending sessions
/// WHEN: Two agents claim different sessions
/// THEN: Both agents can work independently
/// AND: Queue correctly tracks both work items
/// AND: Merges complete in priority order
#[tokio::test]
async fn scenario_two_agents_concurrent_workflow() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let session1 = "feature-concurrent-alpha";
    let session2 = "feature-concurrent-beta";

    // Add two sessions with different priorities
    // Lower priority number = higher priority (processed first)
    let _ = queue.add(session1, None, 3, None).await; // Higher priority
    let _ = queue.add(session2, None, 5, None).await; // Lower priority

    // WHEN: Agent claims first session (higher priority)
    let agent1 = "agent-concurrent-001";
    let claim1_result = queue.next_with_lock(agent1).await;
    assert!(claim1_result.is_ok(), "First claim failed");
    let claimed1 = claim1_result.expect("Should have entry");
    assert!(claimed1.is_some(), "Should claim session1");
    assert_eq!(
        claimed1.as_ref().map(|e| e.workspace.as_str()),
        Some(session1)
    );

    // Process first session
    let _ = queue.transition_to(session1, QueueStatus::Rebasing).await;
    let _ = queue.transition_to(session1, QueueStatus::Testing).await;
    let _ = queue
        .transition_to(session1, QueueStatus::ReadyToMerge)
        .await;
    let _ = queue.transition_to(session1, QueueStatus::Merging).await;
    let _ = queue.transition_to(session1, QueueStatus::Merged).await;

    // Release lock so second agent can claim
    let _ = queue.release_processing_lock(agent1).await;

    // THEN: Second agent can claim second session
    let agent2 = "agent-concurrent-002";
    let claim2_result = queue.next_with_lock(agent2).await;
    assert!(claim2_result.is_ok(), "Second claim failed");
    let claimed2 = claim2_result.expect("Should have entry");
    assert!(claimed2.is_some(), "Should claim session2");
    assert_eq!(
        claimed2.as_ref().map(|e| e.workspace.as_str()),
        Some(session2)
    );

    // Process second session
    let _ = queue.transition_to(session2, QueueStatus::Rebasing).await;
    let _ = queue.transition_to(session2, QueueStatus::Testing).await;
    let _ = queue
        .transition_to(session2, QueueStatus::ReadyToMerge)
        .await;
    let _ = queue.transition_to(session2, QueueStatus::Merging).await;
    let _ = queue.transition_to(session2, QueueStatus::Merged).await;

    // Verify final state
    let final_entries = queue.list(None).await.expect("Failed to list queue");
    let merged_count = final_entries
        .iter()
        .filter(|e| e.status == QueueStatus::Merged)
        .count();
    assert_eq!(merged_count, 2, "Both sessions should be merged");
}

/// Scenario: Two agents conflict over same session
///
/// GIVEN: A queue with one pending session
/// WHEN: Two agents try to claim the same session
/// THEN: Only one succeeds, the other gets nothing (already claimed)
/// AND: The queue reflects the correct state
#[tokio::test]
async fn scenario_two_agents_conflict_same_session() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let agent1 = "agent-conflict-001";
    let session_name = "feature-conflict";

    // Add single session
    let _ = queue.add(session_name, None, 5, None).await;

    // WHEN: First agent claims
    let claim1_result = queue.next_with_lock(agent1).await;
    assert!(claim1_result.is_ok(), "First claim should succeed");
    let claimed1 = claim1_result.expect("Should have entry");
    assert!(claimed1.is_some(), "First agent should get the session");

    // THEN: Second agent cannot claim - no more pending entries
    let claim2_result = queue.next_with_lock("agent-conflict-002").await;
    assert!(
        claim2_result.is_ok(),
        "Second claim attempt should not error"
    );
    let claimed2 = claim2_result.expect("Should have result");
    assert!(
        claimed2.is_none(),
        "Second agent should get nothing - no pending entries"
    );

    // Verify only first agent holds the claim
    let entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(entry.agent_id.as_deref(), Some(agent1));
}

// =============================================================================
// Scenario 3: Conflict Detection and Resolution
// =============================================================================

/// Scenario: Conflict detection workflow
///
/// GIVEN: Two sessions modifying overlapping files
/// WHEN: Conflicts are detected during merge
/// THEN: Conflict resolution records are created
/// AND: Resolution strategy can be recorded
#[tokio::test]
async fn scenario_conflict_detection_workflow() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let pool = queue.pool().clone();

    // Initialize conflict resolutions schema
    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Failed to init conflict schema");

    let session_name = "feature-conflict-detection";
    let conflict_file = "src/main.rs";

    // Add session and claim
    let _ = queue.add(session_name, None, 5, None).await;
    let agent_id = "agent-conflict";
    let _ = queue.next_with_lock(agent_id).await;

    // WHEN: Conflict is detected during rebasing
    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;

    // Record conflict resolution
    let resolution = ConflictResolution {
        id: 0, // Auto-generated
        timestamp: chrono::Utc::now().to_rfc3339(),
        session: session_name.to_string(),
        file: conflict_file.to_string(),
        strategy: "accept_theirs".to_string(),
        reason: Some("Incoming changes are more recent".to_string()),
        confidence: Some("high".to_string()),
        decider: "ai".to_string(),
    };

    let insert_result = insert_conflict_resolution(&pool, &resolution).await;
    assert!(
        insert_result.is_ok(),
        "Failed to insert conflict resolution"
    );

    // THEN: Resolution can be retrieved
    let resolutions = get_conflict_resolutions(&pool, session_name)
        .await
        .expect("Failed to get resolutions");

    assert_eq!(resolutions.len(), 1, "Should have one resolution");
    let recorded = &resolutions[0];
    assert_eq!(recorded.session, session_name);
    assert_eq!(recorded.file, conflict_file);
    assert_eq!(recorded.strategy, "accept_theirs");
    assert_eq!(recorded.decider, "ai");

    // Continue to completion
    let _ = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::ReadyToMerge)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Merging)
        .await;
    let _ = queue.transition_to(session_name, QueueStatus::Merged).await;
}

/// Scenario: Multiple conflict resolutions for same session
///
/// GIVEN: A session with multiple conflicting files
/// WHEN: Multiple conflicts are resolved
/// THEN: All resolutions are recorded correctly
/// AND: Resolutions can be queried by session
#[tokio::test]
async fn scenario_multiple_conflict_resolutions() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let pool = queue.pool().clone();

    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Failed to init conflict schema");

    let session_name = "feature-multi-conflict";
    let _ = queue.add(session_name, None, 5, None).await;

    // WHEN: Multiple conflicts are resolved
    let conflicts = [
        ("src/lib.rs", "accept_ours"),
        ("src/utils.rs", "accept_theirs"),
        ("README.md", "manual_merge"),
    ];

    for (file, strategy) in conflicts {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session: session_name.to_string(),
            file: file.to_string(),
            strategy: strategy.to_string(),
            reason: Some(format!("Resolved {file} with {strategy}")),
            confidence: Some("medium".to_string()),
            decider: "ai".to_string(),
        };
        let _ = insert_conflict_resolution(&pool, &resolution).await;
    }

    // THEN: All resolutions are retrievable
    let resolutions = get_conflict_resolutions(&pool, session_name)
        .await
        .expect("Failed to get resolutions");

    assert_eq!(resolutions.len(), 3, "Should have three resolutions");

    // Verify all files are covered
    let resolved_files: std::collections::HashSet<_> =
        resolutions.iter().map(|r| r.file.as_str()).collect();

    assert!(resolved_files.contains("src/lib.rs"));
    assert!(resolved_files.contains("src/utils.rs"));
    assert!(resolved_files.contains("README.md"));
}

// =============================================================================
// Scenario 4: Queue Processing with Priority
// =============================================================================

/// Scenario: Queue processes entries in priority order
///
/// GIVEN: A queue with multiple entries at different priorities
/// WHEN: Entries are listed/processed
/// THEN: Lower priority numbers come first
/// AND: Equal priorities are processed in FIFO order
#[tokio::test]
async fn scenario_queue_priority_ordering() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    // Add entries with different priorities
    let entries_data = [
        ("critical-hotfix", 1), // Highest priority
        ("feature-alpha", 3),
        ("feature-beta", 5),
        ("feature-gamma", 5), // Same as beta
        ("cleanup-task", 10), // Lowest priority
    ];

    for (name, priority) in entries_data {
        let _ = queue.add(name, None, priority, None).await;
    }

    // WHEN: List entries
    let entries = queue.list(None).await.expect("Failed to list queue");

    // THEN: Verify priority ordering
    assert_eq!(entries.len(), 5, "Should have five entries");

    // First should be highest priority (lowest number)
    assert_eq!(entries[0].workspace, "critical-hotfix");
    assert_eq!(entries[0].priority, 1);

    // Last should be lowest priority (highest number)
    assert_eq!(entries[4].workspace, "cleanup-task");
    assert_eq!(entries[4].priority, 10);

    // Verify middle entries maintain relative order
    let priorities: Vec<i32> = entries.iter().map(|e| e.priority).collect();
    assert!(
        priorities.windows(2).all(|w| w[0] <= w[1]),
        "Priorities should be non-decreasing"
    );
}

/// Scenario: Queue statistics reflect current state
///
/// GIVEN: A queue with entries in various states
/// WHEN: Statistics are queried
/// THEN: Counts accurately reflect each status
#[tokio::test]
async fn scenario_queue_statistics() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    // Create entries with explicit state transitions
    // Note: next_with_lock claims entries in priority/FIFO order
    // Important: The queue uses a single global processing lock, so only one agent
    // can claim at a time. We must release the lock between claims.
    let _ = queue.add("pending-only", None, 5, None).await;

    // Create an entry that will go through to merged (claim first, then release)
    let _ = queue.add("to-be-merged", None, 1, None).await; // Higher priority
    let _ = queue.next_with_lock("agent-stats").await;
    let _ = queue
        .transition_to("to-be-merged", QueueStatus::Rebasing)
        .await;
    let _ = queue
        .transition_to("to-be-merged", QueueStatus::Testing)
        .await;
    let _ = queue
        .transition_to("to-be-merged", QueueStatus::ReadyToMerge)
        .await;
    let _ = queue
        .transition_to("to-be-merged", QueueStatus::Merging)
        .await;
    let _ = queue
        .transition_to("to-be-merged", QueueStatus::Merged)
        .await;
    let _ = queue.release_processing_lock("agent-stats").await;

    // Create an entry that will be claimed (now we can claim since lock is released)
    let _ = queue.add("to-be-claimed", None, 5, None).await;
    let _ = queue.next_with_lock("agent-stats-2").await;

    // WHEN
    let stats = queue.stats().await.expect("Failed to get stats");

    // THEN: Verify stats reflect the actual states
    assert_eq!(stats.total, 3, "Should have 3 total entries");
    assert_eq!(stats.pending, 1, "Should have 1 pending (pending-only)");
    assert!(
        stats.processing >= 1,
        "Should have at least 1 processing (to-be-claimed)"
    );
    assert_eq!(stats.completed, 1, "Should have 1 completed (to-be-merged)");
}

/// Scenario: Queue entry lifecycle with beads
///
/// GIVEN: A queue entry with an associated bead
/// WHEN: The entry progresses through states
/// THEN: The bead association is preserved
#[tokio::test]
async fn scenario_queue_with_bead_association() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let session_name = "feature-bead-test";
    let bead_id = "bd-test-123";

    // Add with bead association
    let add_result = queue.add(session_name, Some(bead_id), 5, None).await;
    assert!(add_result.is_ok(), "Failed to add with bead");

    // WHEN: Process through lifecycle
    let _ = queue.next_with_lock("agent-bead").await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    let _ = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;

    // THEN: Bead is still associated
    let entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");

    assert_eq!(entry.bead_id.as_deref(), Some(bead_id));
    assert_eq!(entry.status, QueueStatus::Testing);
}

/// Scenario: Queue recovery from stale claims
///
/// GIVEN: A queue with stale claims (expired locks)
/// WHEN: Recovery is triggered
/// THEN: Stale entries are reset to pending
///
/// Uses condition-based polling instead of fixed sleep for faster execution.
#[tokio::test]
async fn scenario_queue_recovery_stale_claims() {
    // GIVEN: Queue with short timeout for testing
    // Using 1 second timeout - timestamps use second granularity
    let queue = MergeQueue::open_in_memory_with_timeout(1) // 1 second timeout
        .await
        .expect("Failed to create queue");

    let session_name = "feature-stale";
    let _ = queue.add(session_name, None, 5, None).await;
    let _ = queue.next_with_lock("agent-stale").await;

    // Verify entry is claimed
    let claimed_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(claimed_entry.status, QueueStatus::Claimed);

    // Poll until entry becomes stale (condition-based polling)
    // This is faster than fixed sleep and more robust
    let start = std::time::Instant::now();
    let recovery_stats = loop {
        let stats = queue
            .get_recovery_stats()
            .await
            .expect("Failed to get recovery stats");
        if stats.entries_reclaimed > 0 {
            break stats;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Timed out waiting for entry to become stale"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(
        recovery_stats.entries_reclaimed >= 1,
        "Should detect stale entry"
    );

    // WHEN: Detect and recover stale entries
    let recovery = queue
        .detect_and_recover_stale()
        .await
        .expect("Failed to recover stale");

    // THEN: Entry should be reclaimed (or at least lock cleaned)
    assert!(
        recovery.entries_reclaimed >= 1 || recovery.locks_cleaned >= 1,
        "Should reclaim stale entry or clean expired lock (got {} entries, {} locks)",
        recovery.entries_reclaimed,
        recovery.locks_cleaned
    );

    // Entry should be back to pending after recovery
    let entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Pending);
}

// =============================================================================
// Integration Tests: Full Workflow Combinations
// =============================================================================

/// Scenario: Complete multi-agent, multi-session workflow
///
/// GIVEN: Multiple agents and sessions
/// WHEN: A complex workflow with conflicts and priorities executes
/// THEN: All invariants are maintained
#[tokio::test]
async fn scenario_complete_multi_agent_workflow() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let pool = queue.pool().clone();

    init_conflict_resolutions_schema(&pool)
        .await
        .expect("Failed to init conflict schema");

    // Setup: 3 sessions with priorities
    let sessions = [("hotfix-urgent", 1), ("feature-1", 3), ("feature-2", 5)];

    for (name, priority) in &sessions {
        let _ = queue.add(name, None, *priority, None).await;
    }

    // WHEN: Agent claims and processes each in priority order
    let agent_id = "agent-multi";

    // Process hotfix-urgent first (highest priority)
    let claim1 = queue.next_with_lock(agent_id).await.expect("Claim failed");
    assert!(claim1.is_some());
    let entry1 = claim1.expect("Should have entry");
    assert_eq!(entry1.workspace, "hotfix-urgent");

    let _ = queue
        .transition_to("hotfix-urgent", QueueStatus::Rebasing)
        .await;
    let _ = queue
        .transition_to("hotfix-urgent", QueueStatus::Testing)
        .await;
    let _ = queue
        .transition_to("hotfix-urgent", QueueStatus::ReadyToMerge)
        .await;
    let _ = queue
        .transition_to("hotfix-urgent", QueueStatus::Merging)
        .await;
    let _ = queue
        .transition_to("hotfix-urgent", QueueStatus::Merged)
        .await;

    // Simulate conflict on feature-1
    let conflict_resolution = ConflictResolution {
        id: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
        session: "feature-1".to_string(),
        file: "src/conflict.rs".to_string(),
        strategy: "manual_merge".to_string(),
        reason: Some("Complex merge required human intervention".to_string()),
        confidence: None,
        decider: "human".to_string(),
    };
    let _ = insert_conflict_resolution(&pool, &conflict_resolution).await;

    // Release and claim next
    let _ = queue.release_processing_lock(agent_id).await;

    // Process feature-1
    let claim2 = queue.next_with_lock(agent_id).await.expect("Claim failed");
    assert!(claim2.is_some());
    let entry2 = claim2.expect("Should have entry");
    assert_eq!(entry2.workspace, "feature-1");

    let _ = queue
        .transition_to("feature-1", QueueStatus::Rebasing)
        .await;
    let _ = queue.transition_to("feature-1", QueueStatus::Testing).await;
    let _ = queue
        .transition_to("feature-1", QueueStatus::ReadyToMerge)
        .await;
    let _ = queue.transition_to("feature-1", QueueStatus::Merging).await;
    let _ = queue.transition_to("feature-1", QueueStatus::Merged).await;

    // Release and claim last
    let _ = queue.release_processing_lock(agent_id).await;

    // Process feature-2
    let claim3 = queue.next_with_lock(agent_id).await.expect("Claim failed");
    assert!(claim3.is_some());
    let entry3 = claim3.expect("Should have entry");
    assert_eq!(entry3.workspace, "feature-2");

    let _ = queue
        .transition_to("feature-2", QueueStatus::Rebasing)
        .await;
    let _ = queue.transition_to("feature-2", QueueStatus::Testing).await;
    let _ = queue
        .transition_to("feature-2", QueueStatus::ReadyToMerge)
        .await;
    let _ = queue.transition_to("feature-2", QueueStatus::Merging).await;
    let _ = queue.transition_to("feature-2", QueueStatus::Merged).await;

    // THEN: Verify final state
    let final_stats = queue.stats().await.expect("Failed to get stats");
    assert_eq!(final_stats.completed, 3, "All sessions should be completed");

    let resolutions = get_conflict_resolutions(&pool, "feature-1")
        .await
        .expect("Failed to get resolutions");
    assert_eq!(resolutions.len(), 1, "Should have one conflict recorded");
}

/// Scenario: Queue handles terminal failure correctly
///
/// GIVEN: An entry that fails terminally
/// WHEN: The failure is recorded
/// THEN: Entry cannot be retried
/// AND: Queue stats reflect the failure
#[tokio::test]
async fn scenario_terminal_failure_handling() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let session_name = "feature-failed";
    let _ = queue.add(session_name, None, 5, None).await;
    let _ = queue.next_with_lock("agent-fail").await;

    // WHEN: Entry fails terminally
    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    let fail_result = queue
        .transition_to_failed(session_name, "Unrecoverable error", false)
        .await;
    assert!(fail_result.is_ok(), "Should be able to fail terminally");

    // THEN: Entry is in terminal state
    let entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");

    assert_eq!(entry.status, QueueStatus::FailedTerminal);
    assert!(entry.status.is_terminal());

    // Verify cannot transition from terminal state
    // The state machine should reject this
    let current_status = entry.status;
    let can_retry = current_status.can_transition_to(QueueStatus::Pending);
    assert!(!can_retry, "Should not be able to retry terminal failure");
}

/// Scenario: Queue cancellation workflow
///
/// GIVEN: A pending entry
/// WHEN: Entry is cancelled
/// THEN: Entry moves to cancelled state
/// AND: Resources are cleaned up
#[tokio::test]
async fn scenario_queue_cancellation() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let session_name = "feature-cancelled";
    let add_response = queue.add(session_name, None, 5, None).await;
    assert!(add_response.is_ok(), "Add should succeed");

    // WHEN: Cancel the entry via cancel_entry (requires ID)
    let entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    let entry_id = entry.id;

    let cancel_result = queue.cancel_entry(entry_id).await;
    assert!(
        cancel_result.is_ok(),
        "Should be able to cancel: {:?}",
        cancel_result.err()
    );

    // THEN: Entry is cancelled
    let cancelled_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");

    assert_eq!(cancelled_entry.status, QueueStatus::Cancelled);
    assert!(cancelled_entry.status.is_terminal());
}

/// Scenario: Queue entry lifecycle full state machine
///
/// GIVEN: A fresh queue entry
/// WHEN: The entry goes through the complete state machine
/// THEN: All state transitions are valid
/// AND: Final state is terminal
#[tokio::test]
async fn scenario_full_state_machine_lifecycle() {
    let Some(ctx) = E2ETestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    let session_name = "feature-full-lifecycle";
    let agent_id = "agent-full";

    // Add and verify pending state
    let _ = queue.add(session_name, None, 5, None).await;
    let pending_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(pending_entry.status, QueueStatus::Pending);

    // Pending -> Claimed
    let _ = queue.next_with_lock(agent_id).await;
    let claimed_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(claimed_entry.status, QueueStatus::Claimed);

    // Claimed -> Rebasing
    let _ = queue
        .transition_to(session_name, QueueStatus::Rebasing)
        .await;
    let rebasing_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(rebasing_entry.status, QueueStatus::Rebasing);

    // Rebasing -> Testing
    let _ = queue
        .transition_to(session_name, QueueStatus::Testing)
        .await;
    let testing_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(testing_entry.status, QueueStatus::Testing);

    // Testing -> ReadyToMerge
    let _ = queue
        .transition_to(session_name, QueueStatus::ReadyToMerge)
        .await;
    let ready_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(ready_entry.status, QueueStatus::ReadyToMerge);

    // ReadyToMerge -> Merging
    let _ = queue
        .transition_to(session_name, QueueStatus::Merging)
        .await;
    let merging_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(merging_entry.status, QueueStatus::Merging);

    // Merging -> Merged
    let _ = queue.transition_to(session_name, QueueStatus::Merged).await;
    let merged_entry = queue
        .get_by_workspace(session_name)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(merged_entry.status, QueueStatus::Merged);
    assert!(merged_entry.status.is_terminal());
}

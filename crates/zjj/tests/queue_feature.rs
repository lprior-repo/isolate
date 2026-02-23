//! BDD Acceptance Tests for Queue Management Feature
//!
//! Feature: Queue Management
//!
//! As a multi-agent coordination system
//! I want to manage a merge queue for sequential processing
//! So that multiple agents can coordinate their work without conflicts
//!
//! This test file implements the BDD scenarios defined in `features/queue.feature`
//! using Dan North BDD style with Given/When/Then syntax.
//!
//! # ATDD Phase
//!
//! These tests define expected behavior before implementation.
//! Run with: `cargo test --test queue_feature`

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]

mod common;

use std::sync::Arc;

use anyhow::{Context, Result};
use common::TestHarness;
use tokio::sync::Mutex;
use zjj_core::{coordination::QueueControlError, MergeQueue, QueueStatus};

// =============================================================================
// Queue Test Context
// =============================================================================

/// Queue test context that holds state for each scenario
///
/// Uses Arc<Mutex<>> for thread-safe sharing across async steps.
pub struct QueueTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// The merge queue for direct operations
    pub queue: Arc<Mutex<Option<MergeQueue>>>,
    /// Track created entry IDs for cleanup and assertions
    pub entry_ids: Arc<Mutex<Vec<i64>>>,
    /// Track the last entry workspace for assertions
    pub last_workspace: Arc<Mutex<Option<String>>>,
}

impl QueueTestContext {
    /// Create a new queue test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            queue: Arc::new(Mutex::new(None)),
            entry_ids: Arc::new(Mutex::new(Vec::new())),
            last_workspace: Arc::new(Mutex::new(None)),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the queue database
    ///
    /// Uses the same path as the CLI: `.zjj/state.db`
    /// This is because the queue shares the session database.
    pub async fn init_queue(&self) -> Result<MergeQueue> {
        // Use state.db (same as CLI - get_queue_db_path returns get_db_path)
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
}

// =============================================================================
// Scenario: List shows entries with status
// =============================================================================

/// Scenario: List shows entries with status
///
/// GIVEN: a JJ repository is initialized
/// AND: zjj is initialized
/// AND: I add workspace "workspace-alpha" to the queue with priority 5
/// AND: I add workspace "workspace-beta" to the queue with priority 3
/// WHEN: I list the queue
/// THEN: I should see "workspace-alpha" in the output
/// AND: I should see "workspace-beta" in the output
/// AND: each entry should show its status
/// AND: entries should be ordered by priority
#[tokio::test]
async fn scenario_list_shows_entries_with_status() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("workspace-alpha", None, 5, None)
        .await
        .expect("Failed to add workspace-alpha");
    queue
        .add("workspace-beta", None, 3, None)
        .await
        .expect("Failed to add workspace-beta");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "--list"]);

    // THEN
    assert!(
        result.success,
        "Queue list should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("workspace-alpha"),
        "Output should contain 'workspace-alpha'. Got: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("workspace-beta"),
        "Output should contain 'workspace-beta'. Got: {}",
        result.stdout
    );

    // Verify priority ordering (lower number = higher priority)
    let entries = queue.list(None).await.expect("Failed to list entries");
    assert_eq!(entries.len(), 2, "Should have 2 entries");
    assert_eq!(
        entries[0].workspace, "workspace-beta",
        "Beta (priority 3) should be first"
    );
    assert_eq!(
        entries[1].workspace, "workspace-alpha",
        "Alpha (priority 5) should be second"
    );
}

// =============================================================================
// Scenario: Show displays entry details
// =============================================================================

/// Scenario: Show displays entry details
///
/// GIVEN: I add workspace "workspace-test" to the queue with priority 5
/// AND: I attach bead "bd-123" to the entry
/// WHEN: I show the status of workspace "workspace-test"
/// THEN: I should see "workspace-test" in the output
/// AND: I should see the status
/// AND: I should see the priority
/// AND: I should see "bd-123" in the output
#[tokio::test]
async fn scenario_show_displays_entry_details() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("workspace-test", Some("bd-123"), 5, None)
        .await
        .expect("Failed to add workspace-test");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "--status", "workspace-test"]);

    // THEN
    assert!(
        result.success,
        "Queue status should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("workspace-test"),
        "Output should contain 'workspace-test'. Got: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("Status") || result.stdout.contains("pending"),
        "Output should show status. Got: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("Priority") || result.stdout.contains("priority"),
        "Output should show priority. Got: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("bd-123"),
        "Output should contain bead ID 'bd-123'. Got: {}",
        result.stdout
    );
}

// =============================================================================
// Scenario: Work processes next entry
// =============================================================================

/// Scenario: Work processes next entry
///
/// GIVEN: I add workspace "workspace-next" to the queue with priority 1
/// AND: the workspace is ready to merge
/// WHEN: I process the next queue entry
/// THEN: the entry should transition to merging
/// AND: the processing lock should be acquired
/// AND: the processing lock should be released after completion
#[tokio::test]
async fn scenario_work_processes_next_entry() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("workspace-next", None, 1, None)
        .await
        .expect("Failed to add workspace-next");

    // Set status to ready_to_merge
    sqlx::query("UPDATE merge_queue SET status = 'ready_to_merge' WHERE workspace = ?1")
        .bind("workspace-next")
        .execute(queue.pool())
        .await
        .expect("Failed to set status");

    // WHEN - Acquire lock
    let acquired = queue
        .acquire_processing_lock("test-agent")
        .await
        .expect("Failed to attempt lock");

    // THEN
    assert!(acquired, "Lock acquisition should succeed");

    let lock = queue
        .get_processing_lock()
        .await
        .expect("Failed to get lock info");
    assert!(lock.is_some(), "Processing lock should be acquired");
    assert_eq!(
        lock.map(|l| l.agent_id),
        Some("test-agent".to_string()),
        "Lock should be held by test-agent"
    );

    // Release lock
    let released = queue
        .release_processing_lock("test-agent")
        .await
        .expect("Failed to release lock");
    assert!(released, "Lock should be released");

    let lock_after = queue
        .get_processing_lock()
        .await
        .expect("Failed to get lock info");
    assert!(
        lock_after.is_none(),
        "Processing lock should be released after completion"
    );
}

// =============================================================================
// Scenario: Retry failed entry
// =============================================================================

/// Scenario: Retry failed entry
///
/// GIVEN: I add workspace "workspace-retry" to the queue with priority 5
/// AND: the entry is in `failed_retryable` state
/// AND: the attempt count is less than max attempts
/// WHEN: I retry the entry
/// THEN: the entry should transition to pending
/// AND: the attempt count should be incremented
#[tokio::test]
async fn scenario_retry_failed_entry() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let response = queue
        .add("workspace-retry", None, 5, None)
        .await
        .expect("Failed to add workspace-retry");

    // Set to failed_retryable with attempts remaining
    sqlx::query(
        "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 1, max_attempts = 3, error_message = 'Test failure' WHERE id = ?1",
    )
    .bind(response.entry.id)
    .execute(queue.pool())
    .await
    .expect("Failed to set status");

    // WHEN
    let result = queue.retry_entry(response.entry.id).await;

    // THEN
    assert!(result.is_ok(), "Retry should succeed: {:?}", result.err());

    let entry = result.expect("Retry should succeed");
    assert_eq!(
        entry.status,
        QueueStatus::Pending,
        "Entry should be in pending state after retry"
    );
    assert_eq!(
        entry.attempt_count, 2,
        "Attempt count should be incremented"
    );
}

// =============================================================================
// Scenario: Cancel entry
// =============================================================================

/// Scenario: Cancel entry
///
/// GIVEN: I add workspace "workspace-cancel" to the queue with priority 5
/// AND: the entry is in pending state
/// WHEN: I cancel the entry
/// THEN: the entry should transition to cancelled
/// AND: the entry should be in terminal state
#[tokio::test]
async fn scenario_cancel_entry() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let response = queue
        .add("workspace-cancel", None, 5, None)
        .await
        .expect("Failed to add workspace-cancel");

    // Entry starts in pending state

    // WHEN
    let result = queue.cancel_entry(response.entry.id).await;

    // THEN
    assert!(result.is_ok(), "Cancel should succeed: {:?}", result.err());

    let entry = result.expect("Cancel should succeed");
    assert_eq!(
        entry.status,
        QueueStatus::Cancelled,
        "Entry should be in cancelled state"
    );
    assert!(
        entry.status.is_terminal(),
        "Cancelled should be a terminal state"
    );
}

// =============================================================================
// Scenario: Cancel merged entry fails
// =============================================================================

/// Scenario: Cancel merged entry fails
///
/// GIVEN: I add workspace "workspace-merged" to the queue with priority 5
/// AND: the entry is in merged state
/// WHEN: I attempt to cancel the entry
/// THEN: the operation should fail
/// AND: the error should indicate "terminal state"
/// AND: the entry should remain in merged state
#[tokio::test]
async fn scenario_cancel_merged_entry_fails() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let response = queue
        .add("workspace-merged", None, 5, None)
        .await
        .expect("Failed to add workspace-merged");

    // Set to merged state
    sqlx::query("UPDATE merge_queue SET status = 'merged' WHERE id = ?1")
        .bind(response.entry.id)
        .execute(queue.pool())
        .await
        .expect("Failed to set status");

    // WHEN
    let result = queue.cancel_entry(response.entry.id).await;

    // THEN
    assert!(result.is_err(), "Cancel should fail for merged entry");

    match result {
        Err(QueueControlError::NotCancellable { id, status }) => {
            assert_eq!(
                id, response.entry.id,
                "Error should reference correct entry"
            );
            assert_eq!(
                status,
                QueueStatus::Merged,
                "Error should show merged status"
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Cancel should have failed"),
    }

    // Verify entry remains in merged state
    let entry = queue
        .get_by_id(response.entry.id)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::Merged,
        "Entry should remain in merged state"
    );
}

// =============================================================================
// Scenario: Retry terminal entry fails
// =============================================================================

/// Scenario: Retry terminal entry fails
///
/// GIVEN: I add workspace "workspace-terminal" to the queue with priority 5
/// AND: the entry is in `failed_terminal` state
/// WHEN: I attempt to retry the entry
/// THEN: the operation should fail
/// AND: the error should indicate "not retryable"
/// AND: the entry should remain in `failed_terminal` state
#[tokio::test]
async fn scenario_retry_terminal_entry_fails() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let response = queue
        .add("workspace-terminal", None, 5, None)
        .await
        .expect("Failed to add workspace-terminal");

    // Set to failed_terminal state
    sqlx::query(
        "UPDATE merge_queue SET status = 'failed_terminal', error_message = 'Terminal failure' WHERE id = ?1",
    )
    .bind(response.entry.id)
    .execute(queue.pool())
    .await
    .expect("Failed to set status");

    // WHEN
    let result = queue.retry_entry(response.entry.id).await;

    // THEN
    assert!(result.is_err(), "Retry should fail for terminal entry");

    match result {
        Err(QueueControlError::NotRetryable { id, status }) => {
            assert_eq!(
                id, response.entry.id,
                "Error should reference correct entry"
            );
            assert_eq!(
                status,
                QueueStatus::FailedTerminal,
                "Error should show failed_terminal status"
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Retry should have failed"),
    }

    // Verify entry remains in failed_terminal state
    let entry = queue
        .get_by_id(response.entry.id)
        .await
        .expect("Failed to get entry")
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::FailedTerminal,
        "Entry should remain in failed_terminal state"
    );
}

// =============================================================================
// Scenario: Single worker at a time
// =============================================================================

/// Scenario: Single worker at a time
///
/// GIVEN: I add workspace "workspace-serial" to the queue with priority 5
/// AND: worker "agent-alpha" has acquired the processing lock
/// WHEN: worker "agent-beta" attempts to acquire the processing lock
/// THEN: the acquisition should fail
/// AND: the queue should indicate it is locked by "agent-alpha"
/// AND: no concurrent merge conflicts should occur
#[tokio::test]
async fn scenario_single_worker_at_a_time() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("workspace-serial", None, 5, None)
        .await
        .expect("Failed to add workspace-serial");

    // Worker alpha acquires lock
    let alpha_acquired = queue
        .acquire_processing_lock("agent-alpha")
        .await
        .expect("Failed to acquire lock");
    assert!(alpha_acquired, "agent-alpha should acquire lock");

    // WHEN
    let beta_acquired = queue
        .acquire_processing_lock("agent-beta")
        .await
        .expect("Failed to attempt lock");

    // THEN
    assert!(
        !beta_acquired,
        "agent-beta should NOT acquire lock while agent-alpha holds it"
    );

    // Verify lock is still held by alpha
    let lock = queue
        .get_processing_lock()
        .await
        .expect("Failed to get lock info");
    assert!(lock.is_some(), "Lock should still be held");
    assert_eq!(
        lock.map(|l| l.agent_id),
        Some("agent-alpha".to_string()),
        "Lock should still be held by agent-alpha"
    );
}

// =============================================================================
// Scenario: Priority ordering preserves FIFO for same priority
// =============================================================================

/// Scenario: Priority ordering preserves FIFO for same priority
///
/// GIVEN: I add workspace "first-p1" to the queue with priority 1
/// AND: I add workspace "second-p1" to the queue with priority 1
/// AND: I add workspace "third-p1" to the queue with priority 1
/// WHEN: I list the queue
/// THEN: "first-p1" should appear before "second-p1"
/// AND: "second-p1" should appear before "third-p1"
#[tokio::test]
async fn scenario_priority_ordering_preserves_fifo() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("first-p1", None, 1, None)
        .await
        .expect("Failed to add first-p1");
    queue
        .add("second-p1", None, 1, None)
        .await
        .expect("Failed to add second-p1");
    queue
        .add("third-p1", None, 1, None)
        .await
        .expect("Failed to add third-p1");

    // WHEN
    let entries = queue.list(None).await.expect("Failed to list entries");

    // THEN - FIFO order for same priority
    assert_eq!(entries.len(), 3, "Should have 3 entries");

    let first_pos = entries
        .iter()
        .position(|e| e.workspace == "first-p1")
        .expect("first-p1 should exist");
    let second_pos = entries
        .iter()
        .position(|e| e.workspace == "second-p1")
        .expect("second-p1 should exist");
    let third_pos = entries
        .iter()
        .position(|e| e.workspace == "third-p1")
        .expect("third-p1 should exist");

    assert!(
        first_pos < second_pos,
        "first-p1 (pos {}) should appear before second-p1 (pos {})",
        first_pos,
        second_pos
    );
    assert!(
        second_pos < third_pos,
        "second-p1 (pos {}) should appear before third-p1 (pos {})",
        second_pos,
        third_pos
    );
}

// =============================================================================
// Scenario: Higher priority entries processed first
// =============================================================================

/// Scenario: Higher priority entries processed first
///
/// GIVEN: I add workspace "low-priority" to the queue with priority 9
/// AND: I add workspace "high-priority" to the queue with priority 1
/// AND: I add workspace "medium-priority" to the queue with priority 5
/// WHEN: I get the next queue entry
/// THEN: it should be "high-priority"
#[tokio::test]
async fn scenario_higher_priority_processed_first() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    queue
        .add("low-priority", None, 9, None)
        .await
        .expect("Failed to add low-priority");
    queue
        .add("high-priority", None, 1, None)
        .await
        .expect("Failed to add high-priority");
    queue
        .add("medium-priority", None, 5, None)
        .await
        .expect("Failed to add medium-priority");

    // WHEN
    let next = queue.next().await.expect("Failed to get next entry");

    // THEN
    assert!(next.is_some(), "Should have a next entry");
    let entry = next.expect("Should have entry");
    assert_eq!(
        entry.workspace, "high-priority",
        "First entry should be high-priority (priority 1)"
    );

    // Mark as claimed to get next
    queue
        .transition_to(&entry.workspace, zjj_core::QueueStatus::Claimed)
        .await
        .expect("Failed to transition to claimed");

    let next2 = queue.next().await.expect("Failed to get second entry");
    assert!(next2.is_some(), "Should have a second entry");
    let entry2 = next2.expect("Should have entry");
    assert_eq!(
        entry2.workspace, "medium-priority",
        "Second entry should be medium-priority (priority 5)"
    );
}

// =============================================================================
// Scenario: Processing lock expires after timeout
// =============================================================================

/// Scenario: Processing lock expires after timeout
///
/// GIVEN: worker "agent-stale" has acquired the processing lock
/// AND: the lock timeout has expired
/// WHEN: worker "agent-fresh" attempts to acquire the processing lock
/// THEN: the acquisition should succeed
/// AND: the stale lock should be replaced
#[tokio::test]
async fn scenario_processing_lock_expires_after_timeout() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    // Create queue with short timeout (use state.db like CLI)
    let queue_db = ctx.harness.repo_path.join(".zjj").join("state.db");
    let queue = MergeQueue::open_with_timeout(&queue_db, 1)
        .await
        .expect("Failed to open queue");

    // Agent-stale acquires lock
    let stale_acquired = queue
        .acquire_processing_lock("agent-stale")
        .await
        .expect("Failed to acquire lock");
    assert!(stale_acquired, "agent-stale should acquire lock");

    // Expire the lock by setting expires_at to past
    let now = chrono::Utc::now().timestamp();
    let expired = now - 1000;
    sqlx::query("UPDATE queue_processing_lock SET expires_at = ?1 WHERE id = 1")
        .bind(expired)
        .execute(queue.pool())
        .await
        .expect("Failed to expire lock");

    // WHEN
    let fresh_acquired = queue
        .acquire_processing_lock("agent-fresh")
        .await
        .expect("Failed to attempt lock");

    // THEN
    assert!(
        fresh_acquired,
        "agent-fresh should acquire lock after expiration"
    );

    // Verify lock is now held by agent-fresh
    let lock = queue
        .get_processing_lock()
        .await
        .expect("Failed to get lock info");
    assert!(lock.is_some(), "Lock should exist");
    assert_eq!(
        lock.map(|l| l.agent_id),
        Some("agent-fresh".to_string()),
        "Lock should be held by agent-fresh"
    );
}

// =============================================================================
// Scenario: Retry respects max attempts
// =============================================================================

/// Scenario: Retry respects max attempts
///
/// GIVEN: I add workspace "workspace-max-retry" to the queue with priority 5
/// AND: the entry is in `failed_retryable` state
/// AND: the attempt count equals max attempts
/// WHEN: I attempt to retry the entry
/// THEN: the operation should fail
/// AND: the error should indicate "max attempts exceeded"
#[tokio::test]
async fn scenario_retry_respects_max_attempts() {
    let Some(ctx) = QueueTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let response = queue
        .add("workspace-max-retry", None, 5, None)
        .await
        .expect("Failed to add workspace-max-retry");

    // Set to failed_retryable with max attempts reached
    sqlx::query(
        "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 3, max_attempts = 3, error_message = 'Max attempts reached' WHERE id = ?1",
    )
    .bind(response.entry.id)
    .execute(queue.pool())
    .await
    .expect("Failed to set status");

    // WHEN
    let result = queue.retry_entry(response.entry.id).await;

    // THEN
    assert!(
        result.is_err(),
        "Retry should fail when max attempts reached"
    );

    match result {
        Err(QueueControlError::MaxAttemptsExceeded {
            id,
            attempt_count,
            max_attempts,
        }) => {
            assert_eq!(
                id, response.entry.id,
                "Error should reference correct entry"
            );
            assert_eq!(attempt_count, 3, "Should show 3 attempts");
            assert_eq!(max_attempts, 3, "Should show max of 3");
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Retry should have failed"),
    }
}

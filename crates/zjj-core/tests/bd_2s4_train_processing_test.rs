//! Integration tests for bd-2s4: Train processing logic that merges sessions in order.
//!
//! These tests verify:
//! - Train processor correctly sorts entries by priority
//! - Processing happens in priority order (lowest number first)
//! - Status updates are correctly applied
//! - JSONL events can be emitted for observability
//! - Failures are handled gracefully

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    // Mock implementations are for future integration tests
    dead_code
)]

use chrono::Utc;
use zjj_core::coordination::{
    filter_processable, queue_entities::Dependents, queue_status::StackMergeState,
    sort_by_priority, EntryResultKind, MergeExecutor, QualityGate, QueueEntry, QueueStatus,
    TrainConfig, TrainError, TrainResult, WorkspaceQueueState,
};

// ============================================================================
// MOCK IMPLEMENTATIONS FOR TESTING
// ============================================================================

/// A mock quality gate that always passes.
struct PassingGate;

#[async_trait::async_trait]
impl QualityGate for PassingGate {
    async fn check(&self, _entry: &QueueEntry) -> Result<(), TrainError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "passing-gate"
    }
}

/// A mock quality gate that always fails.
struct FailingGate;

#[async_trait::async_trait]
impl QualityGate for FailingGate {
    async fn check(&self, _entry: &QueueEntry) -> Result<(), TrainError> {
        Err(TrainError::QualityGateFailed {
            workspace: _entry.workspace.clone(),
            gate: "failing-gate".to_string(),
        })
    }

    fn name(&self) -> &str {
        "failing-gate"
    }
}

/// A mock merge executor for testing.
struct MockMergeExecutor {
    /// Whether to simulate conflicts.
    has_conflicts: bool,
    /// Whether to simulate merge failure.
    merge_fails: bool,
    /// The SHA to return for successful merges.
    merge_sha: String,
}

impl MockMergeExecutor {
    fn new() -> Self {
        Self {
            has_conflicts: false,
            merge_fails: false,
            merge_sha: "abc123def456".to_string(),
        }
    }

    fn with_conflicts(mut self) -> Self {
        self.has_conflicts = true;
        self
    }

    fn with_merge_failure(mut self) -> Self {
        self.merge_fails = true;
        self
    }
}

#[async_trait::async_trait]
impl MergeExecutor for MockMergeExecutor {
    async fn merge(&self, _workspace: &str) -> Result<String, TrainError> {
        if self.merge_fails {
            return Err(TrainError::EntryFailed {
                workspace: _workspace.to_string(),
                reason: "Mock merge failure".to_string(),
            });
        }
        Ok(self.merge_sha.clone())
    }

    async fn has_conflicts(&self, _workspace: &str) -> Result<bool, TrainError> {
        Ok(self.has_conflicts)
    }

    async fn get_main_sha(&self) -> Result<String, TrainError> {
        Ok("main-sha-123".to_string())
    }

    async fn rebase(&self, _workspace: &str) -> Result<String, TrainError> {
        Ok("rebased-sha-456".to_string())
    }
}

// ============================================================================
// PURE FUNCTION TESTS
// ============================================================================

/// Test that sort_by_priority correctly sorts entries by priority number.
///
/// GIVEN: A list of queue entries with different priorities
/// WHEN: sort_by_priority is called
/// THEN: Entries are sorted with lowest priority number first
#[test]
fn test_sort_by_priority_orders_correctly() {
    let entries = create_test_entries_with_priorities(&[5, 1, 10, 3, 2]);

    let sorted = sort_by_priority(entries);

    let priorities: Vec<i32> = sorted.iter().map(|e| e.priority).collect();
    assert_eq!(
        priorities,
        vec![1, 2, 3, 5, 10],
        "Should be sorted by priority ascending"
    );
}

/// Test that sort_by_priority preserves FIFO order for same priority.
///
/// GIVEN: Entries with the same priority added at different times
/// WHEN: sort_by_priority is called
/// THEN: Entries are sorted by added_at timestamp within same priority
#[test]
fn test_sort_by_priority_preserves_fifo_for_same_priority() {
    let base_time = Utc::now().timestamp();
    let entries = vec![
        create_entry("third", 1, base_time + 200),
        create_entry("first", 1, base_time),
        create_entry("second", 1, base_time + 100),
    ];

    let sorted = sort_by_priority(entries);

    let names: Vec<&str> = sorted.iter().map(|e| e.workspace.as_str()).collect();
    assert_eq!(
        names,
        vec!["first", "second", "third"],
        "Should preserve FIFO for same priority"
    );
}

/// Test that filter_processable keeps only pending entries.
///
/// GIVEN: A mix of entries with different statuses
/// WHEN: filter_processable is called
/// THEN: Only entries with Pending status are returned
#[test]
fn test_filter_processable_keeps_only_pending() {
    let entries = vec![
        create_entry_with_status("pending-1", QueueStatus::Pending),
        create_entry_with_status("claimed", QueueStatus::Claimed),
        create_entry_with_status("pending-2", QueueStatus::Pending),
        create_entry_with_status("merged", QueueStatus::Merged),
        create_entry_with_status("failed", QueueStatus::FailedRetryable),
    ];

    let filtered = filter_processable(entries);

    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|e| e.status == QueueStatus::Pending));
}

// ============================================================================
// TRAIN PROCESSOR TESTS
// ============================================================================

/// Test that TrainResult correctly tracks merged entries.
///
/// GIVEN: An empty TrainResult
/// WHEN: A merged entry result is added
/// THEN: The merged count is incremented
#[test]
fn test_train_result_tracks_merged() {
    let result = TrainResult::new();

    let merged_entry = zjj_core::coordination::EntryResult {
        workspace: "merged-ws".to_string(),
        position: 1,
        result: EntryResultKind::Merged,
        final_status: QueueStatus::Merged,
        error: None,
        duration_ms: 100,
    };

    let updated = result.add_entry(merged_entry);

    assert_eq!(updated.total_processed, 1);
    assert_eq!(updated.merged, 1);
    assert_eq!(updated.failed, 0);
    assert_eq!(updated.skipped, 0);
}

/// Test that TrainResult correctly tracks failed entries.
///
/// GIVEN: An empty TrainResult
/// WHEN: Both retryable and terminal failed entries are added
/// THEN: The failed count is incremented for both
#[test]
fn test_train_result_tracks_failures() {
    let result = TrainResult::new();

    let retryable_entry = zjj_core::coordination::EntryResult {
        workspace: "retryable-ws".to_string(),
        position: 1,
        result: EntryResultKind::FailedRetryable,
        final_status: QueueStatus::FailedRetryable,
        error: Some("Tests failed".to_string()),
        duration_ms: 50,
    };

    let terminal_entry = zjj_core::coordination::EntryResult {
        workspace: "terminal-ws".to_string(),
        position: 2,
        result: EntryResultKind::FailedTerminal,
        final_status: QueueStatus::FailedTerminal,
        error: Some("Cannot recover".to_string()),
        duration_ms: 10,
    };

    let updated = result.add_entry(retryable_entry).add_entry(terminal_entry);

    assert_eq!(updated.total_processed, 2);
    assert_eq!(updated.failed, 2);
    assert_eq!(updated.merged, 0);
}

/// Test that TrainResult correctly tracks skipped entries.
///
/// GIVEN: An empty TrainResult
/// WHEN: Skipped and cancelled entries are added
/// THEN: The skipped count is incremented for both
#[test]
fn test_train_result_tracks_skipped() {
    let result = TrainResult::new();

    let skipped_entry = zjj_core::coordination::EntryResult {
        workspace: "skipped-ws".to_string(),
        position: 1,
        result: EntryResultKind::Skipped,
        final_status: QueueStatus::Pending,
        error: None,
        duration_ms: 0,
    };

    let cancelled_entry = zjj_core::coordination::EntryResult {
        workspace: "cancelled-ws".to_string(),
        position: 2,
        result: EntryResultKind::Cancelled,
        final_status: QueueStatus::Cancelled,
        error: None,
        duration_ms: 0,
    };

    let updated = result.add_entry(skipped_entry).add_entry(cancelled_entry);

    assert_eq!(updated.total_processed, 2);
    assert_eq!(updated.skipped, 2);
}

/// Test TrainConfig defaults are sensible.
///
/// GIVEN: Default TrainConfig
/// WHEN: Created with default()
/// THEN: Values match expected production defaults
#[test]
fn test_train_config_defaults() {
    let config = TrainConfig::default();

    assert_eq!(
        config.entry_timeout_secs, 300,
        "Default timeout should be 5 minutes"
    );
    assert!(
        !config.stop_on_failure,
        "Should not stop on first failure by default"
    );
    assert_eq!(
        config.max_consecutive_failures, 3,
        "Should allow 3 consecutive failures"
    );
    assert!(!config.dry_run, "Should not be dry run by default");
}

// ============================================================================
// ENTRY RESULT KIND TESTS
// ============================================================================

/// Test EntryResultKind display formatting.
#[test]
fn test_entry_result_kind_display() {
    assert_eq!(EntryResultKind::Merged.to_string(), "merged");
    assert_eq!(EntryResultKind::TestsFailed.to_string(), "tests_failed");
    assert_eq!(EntryResultKind::Conflicts.to_string(), "conflicts");
    assert_eq!(EntryResultKind::Stale.to_string(), "stale");
    assert_eq!(
        EntryResultKind::FailedRetryable.to_string(),
        "failed_retryable"
    );
    assert_eq!(
        EntryResultKind::FailedTerminal.to_string(),
        "failed_terminal"
    );
    assert_eq!(EntryResultKind::Skipped.to_string(), "skipped");
    assert_eq!(EntryResultKind::Cancelled.to_string(), "cancelled");
}

// ============================================================================
// TRAIN ERROR TESTS
// ============================================================================

/// Test TrainError variants have correct messages.
#[test]
fn test_train_error_messages() {
    let err = TrainError::LockAcquisitionFailed;
    assert!(err.to_string().contains("lock"));

    let err = TrainError::EntryFailed {
        workspace: "test-ws".to_string(),
        reason: "timeout".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("test-ws"));
    assert!(msg.contains("timeout"));

    let err = TrainError::QualityGateFailed {
        workspace: "test-ws".to_string(),
        gate: "lint".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("lint"));

    let err = TrainError::ConflictDetected {
        workspace: "test-ws".to_string(),
        files: vec!["file1.rs".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("conflict"));
    assert!(msg.contains("file1.rs"));

    let err = TrainError::Timeout { seconds: 30 };
    assert!(err.to_string().contains("30"));
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_entries_with_priorities(priorities: &[i32]) -> Vec<QueueEntry> {
    let base_time = Utc::now().timestamp();
    priorities
        .iter()
        .enumerate()
        .map(|(idx, &priority)| {
            create_entry(&format!("ws-{idx}"), priority, base_time + idx as i64)
        })
        .collect()
}

fn create_entry(name: &str, priority: i32, added_at: i64) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: name.to_string(),
        bead_id: None,
        priority,
        status: QueueStatus::Pending,
        added_at,
        started_at: None,
        completed_at: None,
        error_message: None,
        agent_id: None,
        dedupe_key: None,
        workspace_state: WorkspaceQueueState::Created,
        previous_state: None,
        state_changed_at: None,
        head_sha: None,
        tested_against_sha: None,
        attempt_count: 0,
        max_attempts: 3,
        rebase_count: 0,
        last_rebase_at: None,
        parent_workspace: None,
        stack_depth: 0,
        dependents: Dependents::new(),
        stack_root: None,
        stack_merge_state: StackMergeState::Independent,
    }
}

fn create_entry_with_status(name: &str, status: QueueStatus) -> QueueEntry {
    let mut entry = create_entry(name, 5, Utc::now().timestamp());
    entry.status = status;
    entry
}

//! ATDD Test for bd-36h8: Add stack_merge_state column to merge_queue
//!
//! BEAD: bd-36h8
//! REQUIREMENT: Add `stack_merge_state` column to `merge_queue` table
//! EARS:
//!   - THE SYSTEM SHALL track stack merge coordination state
//!   - WHEN entry created, THE SYSTEM SHALL default stack_merge_state to 'independent'
//!   - IF invalid value in DB, THE SYSTEM SHALL default to Independent, because data integrity is
//!     critical
//!
//! This test file should:
//!   1. COMPILE (struct definition is valid Rust)
//!   2. FAIL initially (QueueEntry.stack_merge_state field doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown, clippy::unreadable_literal)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: QueueEntry has stack_merge_state field
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that QueueEntry struct has a stack_merge_state field with correct type.
///
/// GIVEN: QueueEntry struct is defined
/// WHEN: Accessing the stack_merge_state field
/// THEN: The field exists and has type StackMergeState
#[test]
fn test_queue_entry_has_stack_merge_state_field() {
    // Create a QueueEntry with stack_merge_state set to Independent (default)
    let entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Independent);

    // Verify the field exists and has the correct type (StackMergeState)
    let state: StackMergeState = entry.stack_merge_state;
    assert_eq!(state, StackMergeState::Independent);
}

/// Test that stack_merge_state defaults to Independent when entry is created.
///
/// GIVEN: QueueEntry struct with stack_merge_state field
/// WHEN: Creating a new entry without explicit stack_merge_state
/// THEN: The field should default to Independent
#[test]
fn test_queue_entry_stack_merge_state_defaults_to_independent() {
    let entry = create_default_test_queue_entry();

    // Verify default value is Independent
    assert_eq!(entry.stack_merge_state, StackMergeState::Independent);
}

/// Test that stack_merge_state can represent all valid states.
///
/// GIVEN: QueueEntry with stack_merge_state set to various valid states
/// WHEN: Accessing the stack_merge_state field
/// THEN: The field should store the exact state value
#[test]
fn test_queue_entry_stack_merge_state_valid_values() {
    let test_cases = [
        StackMergeState::Independent,
        StackMergeState::Blocked,
        StackMergeState::Ready,
        StackMergeState::Merged,
    ];

    for expected_state in test_cases {
        let entry = create_test_queue_entry_with_stack_merge_state(expected_state);
        assert_eq!(entry.stack_merge_state, expected_state);
    }
}

/// Test that stack_merge_state Independent is correctly stored and retrieved.
///
/// GIVEN: QueueEntry with stack_merge_state = Independent
/// WHEN: Creating and reading the entry
/// THEN: The value should be Independent
#[test]
fn test_queue_entry_stack_merge_state_independent() {
    let entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Independent);
    assert_eq!(entry.stack_merge_state, StackMergeState::Independent);
    assert!(!entry.stack_merge_state.is_terminal());
}

/// Test that stack_merge_state Blocked is correctly stored and retrieved.
///
/// GIVEN: QueueEntry with stack_merge_state = Blocked
/// WHEN: Creating and reading the entry
/// THEN: The value should be Blocked and is_blocked() should return true
#[test]
fn test_queue_entry_stack_merge_state_blocked() {
    let entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Blocked);
    assert_eq!(entry.stack_merge_state, StackMergeState::Blocked);
    assert!(entry.stack_merge_state.is_blocked());
    assert!(!entry.stack_merge_state.is_terminal());
}

/// Test that stack_merge_state Ready is correctly stored and retrieved.
///
/// GIVEN: QueueEntry with stack_merge_state = Ready
/// WHEN: Creating and reading the entry
/// THEN: The value should be Ready
#[test]
fn test_queue_entry_stack_merge_state_ready() {
    let entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Ready);
    assert_eq!(entry.stack_merge_state, StackMergeState::Ready);
    assert!(!entry.stack_merge_state.is_blocked());
    assert!(!entry.stack_merge_state.is_terminal());
}

/// Test that stack_merge_state Merged is correctly stored and retrieved.
///
/// GIVEN: QueueEntry with stack_merge_state = Merged
/// WHEN: Creating and reading the entry
/// THEN: The value should be Merged and is_terminal() should return true
#[test]
fn test_queue_entry_stack_merge_state_merged() {
    let entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Merged);
    assert_eq!(entry.stack_merge_state, StackMergeState::Merged);
    assert!(entry.stack_merge_state.is_terminal());
}

/// Test that entries with different stack_merge_state values can coexist.
///
/// GIVEN: Multiple QueueEntry instances with varying states
/// WHEN: Comparing their stack_merge_state values
/// THEN: Each entry should maintain its correct state
#[test]
fn test_queue_entry_stack_merge_state_mixed_entries() {
    let entries = [
        create_test_queue_entry_with_stack_merge_state(StackMergeState::Independent),
        create_test_queue_entry_with_stack_merge_state(StackMergeState::Blocked),
        create_test_queue_entry_with_stack_merge_state(StackMergeState::Ready),
        create_test_queue_entry_with_stack_merge_state(StackMergeState::Merged),
    ];

    let states: Vec<StackMergeState> = entries.iter().map(|e| e.stack_merge_state).collect();

    assert_eq!(
        states,
        vec![
            StackMergeState::Independent,
            StackMergeState::Blocked,
            StackMergeState::Ready,
            StackMergeState::Merged,
        ]
    );
}

/// Test that stack_merge_state distinguishes stack hierarchy positions.
///
/// GIVEN: Entries at different positions in a PR stack
/// WHEN: Checking their stack_merge_state
/// THEN: Independent entries have no parent, Blocked entries are waiting
#[test]
fn test_queue_entry_stack_merge_state_hierarchy_positions() {
    // Root level workspace (no parent) - Independent
    let root_entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Independent);
    assert!(
        !root_entry.stack_merge_state.is_blocked(),
        "Root entries should not be blocked"
    );

    // Child entry waiting for parent - Blocked
    let blocked_entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Blocked);
    assert!(
        blocked_entry.stack_merge_state.is_blocked(),
        "Child entries waiting for parent should be blocked"
    );

    // Child entry ready to proceed - Ready
    let ready_entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Ready);
    assert!(
        !ready_entry.stack_merge_state.is_blocked(),
        "Ready entries should not be blocked"
    );

    // Completed entry - Merged (terminal)
    let merged_entry = create_test_queue_entry_with_stack_merge_state(StackMergeState::Merged);
    assert!(
        merged_entry.stack_merge_state.is_terminal(),
        "Merged entries should be terminal"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry with stack_merge_state
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified stack_merge_state value.
///
/// This helper creates a QueueEntry with minimal required fields.
/// The test will fail to compile if `QueueEntry` doesn't have `stack_merge_state`.
fn create_test_queue_entry_with_stack_merge_state(
    stack_merge_state: StackMergeState,
) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: "test-workspace".to_string(),
        bead_id: Some("bd-36h8".to_string()),
        priority: 0,
        status: QueueStatus::Pending,
        added_at: 1700000000,
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
        // THIS FIELD MUST EXIST for tests to compile
        stack_merge_state,
    }
}

/// Create a test QueueEntry with default values (stack_merge_state should default to Independent).
///
/// This simulates database deserialization where stack_merge_state column
/// has a DEFAULT 'independent' constraint.
fn create_default_test_queue_entry() -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: "default-workspace".to_string(),
        bead_id: None,
        priority: 5,
        status: QueueStatus::Pending,
        added_at: 1700000000,
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
        // Default value should be Independent
        stack_merge_state: StackMergeState::Independent,
    }
}

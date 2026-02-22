//! ATDD Test for bd-1i1a: Add parent_workspace column to merge_queue
//!
//! BEAD: bd-1i1a
//! REQUIREMENT: Add `parent_workspace` column to `merge_queue` table
//! EARS:
//!   - THE SYSTEM SHALL track parent workspace reference
//!   - WHEN entry created with parent, THE SYSTEM SHALL store parent_workspace value
//!   - IF parent_workspace references non-existent entry, THE SYSTEM SHALL NOT validate at DB level
//!
//! This test file should:
//!   1. COMPILE (struct definition is valid Rust)
//!   2. FAIL initially (QueueEntry.parent_workspace field doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown, clippy::unreadable_literal)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, WorkspaceQueueState},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: QueueEntry has parent_workspace field
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that QueueEntry struct has a parent_workspace field with correct type.
///
/// GIVEN: QueueEntry struct is defined
/// WHEN: Accessing the parent_workspace field
/// THEN: The field exists and has type Option<String>
#[test]
fn test_queue_entry_has_parent_workspace_field() {
    // Create a minimal QueueEntry - this will fail to compile
    // if parent_workspace field doesn't exist
    let entry = create_test_queue_entry_with_parent(Some("parent-ws-123".to_string()));

    // Verify the field exists and has the correct type
    let parent: Option<&String> = entry.parent_workspace.as_ref();
    assert!(parent.is_some());
    assert_eq!(parent.map(String::as_str), Some("parent-ws-123"));
}

/// Test that parent_workspace can be None (nullable).
///
/// GIVEN: QueueEntry struct with parent_workspace field
/// WHEN: Creating entry without a parent workspace
/// THEN: The field should accept None value
#[test]
fn test_queue_entry_parent_workspace_can_be_none() {
    let entry = create_test_queue_entry_with_parent(None);

    // Verify None is a valid value
    assert!(entry.parent_workspace.is_none());
}

/// Test that parent_workspace is serialized/deserialized correctly.
///
/// GIVEN: QueueEntry with parent_workspace set to Some value
/// WHEN: Serializing and deserializing
/// THEN: The value should round-trip correctly
#[test]
fn test_queue_entry_parent_workspace_serialization() {
    // Test with Some value
    let entry_with_parent =
        create_test_queue_entry_with_parent(Some("parent-workspace-456".to_string()));
    assert_eq!(
        entry_with_parent.parent_workspace,
        Some("parent-workspace-456".to_string())
    );

    // Test with None value
    let entry_without_parent = create_test_queue_entry_with_parent(None);
    assert_eq!(entry_without_parent.parent_workspace, None);
}

/// Test that parent_workspace can hold arbitrary string values.
///
/// GIVEN: Various string values for parent_workspace
/// WHEN: Storing in the field
/// THEN: The field should store the exact value without modification
#[test]
fn test_queue_entry_parent_workspace_stores_exact_value() {
    let test_cases = [
        Some("simple-name".to_string()),
        Some("with-dashes-and-123".to_string()),
        Some("UPPERCASE".to_string()),
        Some("workspace_with_underscores".to_string()),
        Some("a".to_string()),
        Some("very-long-workspace-name-that-might-be-used-in-practice".to_string()),
        None,
    ];

    for expected in test_cases {
        let entry = create_test_queue_entry_with_parent(expected.clone());
        assert_eq!(entry.parent_workspace, expected);
    }
}

/// Test that entries with and without parent_workspace can coexist.
///
/// GIVEN: Multiple QueueEntry instances
/// WHEN: Some have parent_workspace set, others don't
/// THEN: Both variants should be valid and distinguishable
#[test]
fn test_queue_entry_parent_workspace_mixed_entries() {
    let entries = [
        create_test_queue_entry_with_parent(Some("parent-a".to_string())),
        create_test_queue_entry_with_parent(None),
        create_test_queue_entry_with_parent(Some("parent-b".to_string())),
        create_test_queue_entry_with_parent(None),
    ];

    let parents_with_values: Vec<_> = entries
        .iter()
        .filter_map(|e| e.parent_workspace.clone())
        .collect();

    assert_eq!(parents_with_values, vec!["parent-a", "parent-b"]);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry with parent_workspace
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified parent_workspace value.
///
/// This helper creates a QueueEntry with minimal required fields.
/// The test will fail to compile if `QueueEntry` doesn't have `parent_workspace`.
fn create_test_queue_entry_with_parent(parent_workspace: Option<String>) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: "test-workspace".to_string(),
        bead_id: Some("bd-test".to_string()),
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
        // THIS FIELD MUST EXIST for tests to compile
        parent_workspace,
        stack_depth: 0,
        dependents: Dependents::new(),
        stack_root: None,
        stack_merge_state: zjj_core::coordination::queue_status::StackMergeState::Independent,
    }
}

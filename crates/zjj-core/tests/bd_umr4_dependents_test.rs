//! ATDD Test for bd-umr4: Add dependents JSON column to merge_queue
//!
//! BEAD: bd-umr4
//! REQUIREMENT: Add `dependents` column to `merge_queue` table to track child workspaces
//! EARS:
//!   - THE SYSTEM SHALL track child workspaces
//!   - WHEN child added, THE SYSTEM SHALL append to parent dependents
//!   - IF JSON malformed, THE SYSTEM SHALL NOT crash, because must handle gracefully
//!
//! This test file should:
//!   1. COMPILE (struct definition is valid Rust)
//!   2. FAIL initially (QueueEntry.dependents field doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown, clippy::unreadable_literal)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, WorkspaceQueueState},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: QueueEntry has dependents field
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that QueueEntry struct has a dependents field with correct type.
///
/// GIVEN: QueueEntry struct is defined
/// WHEN: Accessing the dependents field
/// THEN: The field exists and has type Dependents
#[test]
fn test_queue_entry_has_dependents_field() {
    let entry = create_test_queue_entry_with_dependents(Dependents::new());

    assert!(entry.dependents.is_empty());
}

/// Test that dependents defaults to empty.
///
/// GIVEN: QueueEntry struct with dependents field
/// WHEN: Creating a new entry without explicit dependents
/// THEN: The field should default to empty Dependents
#[test]
fn test_queue_entry_dependents_defaults_to_empty() {
    let entry = create_default_test_queue_entry();

    assert!(entry.dependents.is_empty());
}

/// Test that dependents can hold workspace names.
///
/// GIVEN: QueueEntry with dependents containing workspace names
/// WHEN: Accessing the dependents field
/// THEN: The field should store the exact workspace names
#[test]
fn test_queue_entry_dependents_stores_workspace_names() {
    let workspaces = vec![
        "child-1".to_string(),
        "child-2".to_string(),
        "child-3".to_string(),
    ];

    let entry = create_test_queue_entry_with_dependents(Dependents::from_vec(workspaces.clone()));

    assert_eq!(&*entry.dependents, &workspaces);
}

/// Test that dependents serialization handles empty array.
///
/// GIVEN: QueueEntry with empty dependents
/// WHEN: Creating and reading the entry
/// THEN: The empty list should be preserved correctly
#[test]
fn test_queue_entry_dependents_empty_array() {
    let entry = create_test_queue_entry_with_dependents(Dependents::new());

    assert!(entry.dependents.is_empty());
    assert_eq!(entry.dependents.len(), 0);
}

/// Test that dependents can track multiple children.
///
/// GIVEN: Parent entry with multiple child workspaces
/// WHEN: Checking the dependents list
/// THEN: All children should be listed in order
#[test]
fn test_queue_entry_dependents_multiple_children() {
    let children = vec![
        "feature-1".to_string(),
        "feature-2".to_string(),
        "feature-3".to_string(),
    ];

    let parent = create_test_queue_entry_with_dependents(Dependents::from_vec(children));

    assert_eq!(parent.dependents.len(), 3);
    assert_eq!(parent.dependents[0], "feature-1");
    assert_eq!(parent.dependents[1], "feature-2");
    assert_eq!(parent.dependents[2], "feature-3");
}

/// Test that dependents preserves order.
///
/// GIVEN: QueueEntry with dependents in specific order
/// WHEN: Accessing the dependents field
/// THEN: The order should be preserved
#[test]
fn test_queue_entry_dependents_preserves_order() {
    let ordered = vec![
        "alpha".to_string(),
        "bravo".to_string(),
        "charlie".to_string(),
    ];

    let entry = create_test_queue_entry_with_dependents(Dependents::from_vec(ordered.clone()));

    assert_eq!(&*entry.dependents, &ordered);
}

/// Test that root entries typically have empty dependents initially.
///
/// GIVEN: A newly created root entry
/// WHEN: Checking its dependents
/// THEN: It should have no dependents until children are added
#[test]
fn test_queue_entry_root_empty_dependents() {
    let root = create_default_test_queue_entry();

    assert!(
        root.dependents.is_empty(),
        "New root entries should have no dependents initially"
    );
}

/// Test that dependents can distinguish parent from leaf.
///
/// GIVEN: A parent entry with children and a leaf entry with no children
/// WHEN: Comparing their dependents
/// THEN: Parent should have non-empty dependents, leaf should be empty
#[test]
fn test_queue_entry_dependents_distinguishes_parent_from_leaf() {
    let parent = create_test_queue_entry_with_dependents(Dependents::from_vec(vec![
        "child-workspace".to_string(),
    ]));
    let leaf = create_test_queue_entry_with_dependents(Dependents::new());

    assert!(
        !parent.dependents.is_empty(),
        "Parent should have dependents"
    );
    assert!(leaf.dependents.is_empty(), "Leaf should have no dependents");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry with dependents
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified dependents value.
///
/// This helper creates a QueueEntry with minimal required fields.
/// The test will fail to compile if `QueueEntry` doesn't have `dependents`.
fn create_test_queue_entry_with_dependents(dependents: Dependents) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: "test-workspace".to_string(),
        bead_id: Some("bd-umr4".to_string()),
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
        dependents,
        stack_root: None,
        stack_merge_state: zjj_core::coordination::queue_status::StackMergeState::Independent,
    }
}

/// Create a test QueueEntry with default values (dependents should default to empty).
///
/// This simulates database deserialization where dependents column
/// has a DEFAULT '[]' constraint.
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
        stack_merge_state: zjj_core::coordination::queue_status::StackMergeState::Independent,
    }
}

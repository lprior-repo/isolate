//! ATDD Test for bd-35xk: validate_no_cycle function
//!
//! BEAD: bd-35xk
//! REQUIREMENT: Validate that setting a parent won't create a cycle in the stack
//! EARS:
//!   - WHEN parent being set, THE SYSTEM SHALL check for cycles
//!   - IF would create cycle, THE SYSTEM SHALL return error (shall not allow, breaks invariants)
//!   - THE SYSTEM SHALL prevent cycles
//!   - THE SYSTEM SHALL handle self-reference
//!
//! Invariants:
//!   - Function is pure (no side effects)
//!   - Deterministic behavior
//!
//! This test file should:
//!   1. COMPILE (function signature is valid Rust)
//!   2. FAIL initially (validate_no_cycle function doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown, clippy::unreadable_literal)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
    stack_depth::validate_no_cycle,
    stack_error::StackError,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified workspace and parent.
fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: workspace.to_string(),
        bead_id: None,
        priority: 0,
        status: QueueStatus::Pending,
        added_at: 1_700_000_000,
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
        parent_workspace: parent_workspace.map(std::string::ToString::to_string),
        stack_depth: 0,
        dependents: Dependents::new(),
        stack_root: None,
        stack_merge_state: StackMergeState::Independent,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy Path - Valid parent (no cycle)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that setting a valid parent returns Ok.
///
/// GIVEN: A workspace with no parent and an existing parent workspace
/// WHEN: Validating that setting the parent won't create a cycle
/// THEN: Returns Ok(())
#[test]
fn test_validate_no_cycle_happy_path_valid_parent() {
    // Arrange: root exists with no parent, child will be set to parent=root
    let entries = vec![create_entry("root", None)];

    // Act: Validate setting child's parent to root
    let result = validate_no_cycle("child", "root", &entries);

    // Assert: No cycle, should return Ok
    assert!(result.is_ok());
}

/// Test that setting parent to a workspace in a chain returns Ok.
///
/// GIVEN: A chain root -> mid -> leaf
/// WHEN: Validating setting a new workspace's parent to leaf
/// THEN: Returns Ok(()) because no cycle is formed
#[test]
fn test_validate_no_cycle_chain_parent() {
    let entries = vec![
        create_entry("root", None),
        create_entry("mid", Some("root")),
        create_entry("leaf", Some("mid")),
    ];

    let result = validate_no_cycle("new-workspace", "leaf", &entries);

    assert!(result.is_ok());
}

/// Test that an empty entries list allows any parent.
///
/// GIVEN: No existing entries
/// WHEN: Validating setting any parent
/// THEN: Returns Ok(()) (no existing cycle possible)
#[test]
fn test_validate_no_cycle_empty_entries() {
    let entries: Vec<QueueEntry> = vec![];

    let result = validate_no_cycle("new-workspace", "some-parent", &entries);

    assert!(result.is_ok());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Self-Reference (Direct Cycle)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that setting parent to self returns CycleDetected error.
///
/// GIVEN: A workspace attempting to set its parent to itself
/// WHEN: Validating the parent assignment
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_self_reference() {
    let entries = vec![create_entry("workspace", None)];

    let result = validate_no_cycle("workspace", "workspace", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test self-reference even with existing entries.
///
/// GIVEN: Multiple existing workspaces
/// WHEN: A workspace tries to set parent to itself
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_self_reference_with_chain() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
        create_entry("grandchild", Some("child")),
    ];

    // grandchild tries to set parent to grandchild (self)
    let result = validate_no_cycle("grandchild", "grandchild", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Indirect Cycle (Parent is a descendant)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that setting parent to a descendant creates a cycle.
///
/// GIVEN: root -> child relationship exists
/// WHEN: root tries to set parent to child
/// THEN: Returns Err(StackError::CycleDetected) because child -> root -> child
#[test]
fn test_validate_no_cycle_indirect_parent_is_descendant() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    // root tries to set parent to child (which would create cycle: root -> child -> root)
    let result = validate_no_cycle("root", "child", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test indirect cycle through multiple levels.
///
/// GIVEN: root -> mid -> leaf chain
/// WHEN: mid tries to set parent to leaf
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_indirect_multi_level() {
    let entries = vec![
        create_entry("root", None),
        create_entry("mid", Some("root")),
        create_entry("leaf", Some("mid")),
    ];

    // mid tries to set parent to leaf (would create: mid -> leaf -> mid)
    let result = validate_no_cycle("mid", "leaf", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test cycle when target has the workspace as ancestor.
///
/// GIVEN: a -> b -> c chain
/// WHEN: a tries to set parent to c
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_target_has_workspace_as_ancestor() {
    let entries = vec![
        create_entry("a", None),
        create_entry("b", Some("a")),
        create_entry("c", Some("b")),
    ];

    // a tries to set parent to c (cycle: a -> c -> b -> a)
    let result = validate_no_cycle("a", "c", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Deep Cycle Detection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test cycle detection in a deep chain.
///
/// GIVEN: A deep chain a -> b -> c -> d -> e -> f
/// WHEN: a tries to set parent to f
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_deep_chain() {
    let entries = vec![
        create_entry("a", None),
        create_entry("b", Some("a")),
        create_entry("c", Some("b")),
        create_entry("d", Some("c")),
        create_entry("e", Some("d")),
        create_entry("f", Some("e")),
    ];

    // a tries to set parent to f (would create deep cycle)
    let result = validate_no_cycle("a", "f", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that middle of chain cannot set parent to end.
///
/// GIVEN: A chain of 10 workspaces
/// WHEN: Workspace at position 3 tries to set parent to workspace at position 10
/// THEN: Returns Err(StackError::CycleDetected)
#[test]
fn test_validate_no_cycle_middle_to_end() {
    let entries = vec![
        create_entry("w1", None),
        create_entry("w2", Some("w1")),
        create_entry("w3", Some("w2")),
        create_entry("w4", Some("w3")),
        create_entry("w5", Some("w4")),
        create_entry("w6", Some("w5")),
        create_entry("w7", Some("w6")),
        create_entry("w8", Some("w7")),
        create_entry("w9", Some("w8")),
        create_entry("w10", Some("w9")),
    ];

    // w3 tries to set parent to w10
    let result = validate_no_cycle("w3", "w10", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test no false positive in deep chain.
///
/// GIVEN: A deep chain a -> b -> c -> d -> e
/// WHEN: new-workspace tries to set parent to e
/// THEN: Returns Ok(()) because new-workspace is not in the chain
#[test]
fn test_validate_no_cycle_deep_chain_no_false_positive() {
    let entries = vec![
        create_entry("a", None),
        create_entry("b", Some("a")),
        create_entry("c", Some("b")),
        create_entry("d", Some("c")),
        create_entry("e", Some("d")),
    ];

    // new workspace can safely set parent to e
    let result = validate_no_cycle("new-workspace", "e", &entries);

    assert!(result.is_ok());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Function Purity (verifies deterministic behavior)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that the function is deterministic (same input = same output).
///
/// GIVEN: Same entries and parameters
/// WHEN: Calling validate_no_cycle multiple times
/// THEN: Returns the same result each time
#[test]
fn test_validate_no_cycle_is_deterministic() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result1 = validate_no_cycle("root", "child", &entries);
    let result2 = validate_no_cycle("root", "child", &entries);

    assert_eq!(result1, result2);
}

/// Test that the function doesn't modify the input entries.
///
/// GIVEN: A vector of entries
/// WHEN: Calling validate_no_cycle
/// THEN: The entries vector is unchanged
#[test]
fn test_validate_no_cycle_does_not_modify_input() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let entries_clone = entries.clone();

    // Call the function (result is discarded)
    let _ = validate_no_cycle("root", "child", &entries);

    // Entries should be unchanged
    assert_eq!(entries.len(), entries_clone.len());
    assert_eq!(entries[0].workspace, entries_clone[0].workspace);
    assert_eq!(entries[1].workspace, entries_clone[1].workspace);
}

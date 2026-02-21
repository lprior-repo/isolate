//! ATDD Test for bd-1idz: Calculate Stack Depth
//!
//! BEAD: bd-1idz
//! REQUIREMENT: Pure function to calculate stack depth from parent chain
//! EARS:
//!   - THE SYSTEM SHALL provide calculate_stack_depth(workspace, entries) -> Result<u32, StackError>
//!   - WHEN entry has no parent, THE SYSTEM SHALL return depth 0
//!   - WHEN entry has parent chain, THE SYSTEM SHALL count depth from root
//!   - IF cycle detected in parent chain, THE SYSTEM SHALL return StackError::CycleDetected
//!   - THE SYSTEM SHALL be a pure function with no panic paths
//!
//! This test file should:
//!   1. COMPILE (type definitions are valid Rust)
//!   2. FAIL initially (calculate_stack_depth function doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
    stack_error::StackError,
};

// Import the function under test - will fail to compile until implemented
use zjj_core::coordination::stack_depth::calculate_stack_depth;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with no parent returns depth 0
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with no parent (root) returns depth 0.
///
/// GIVEN: A workspace entry with parent_workspace = None
/// WHEN: Calculating stack depth
/// THEN: The function should return Ok(0)
#[test]
fn test_calculate_stack_depth_no_parent_returns_zero() {
    let entries = vec![create_entry("root-workspace", None)];

    let result = calculate_stack_depth("root-workspace", &entries);

    assert_eq!(result, Ok(0));
}

/// Test that multiple root entries all return depth 0.
///
/// GIVEN: Multiple workspaces with no parents
/// WHEN: Calculating stack depth for each
/// THEN: Each should return Ok(0)
#[test]
fn test_calculate_stack_depth_multiple_roots() {
    let entries = vec![
        create_entry("root-a", None),
        create_entry("root-b", None),
        create_entry("root-c", None),
    ];

    assert_eq!(calculate_stack_depth("root-a", &entries), Ok(0));
    assert_eq!(calculate_stack_depth("root-b", &entries), Ok(0));
    assert_eq!(calculate_stack_depth("root-c", &entries), Ok(0));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with one parent (root) returns depth 1
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with a root parent returns depth 1.
///
/// GIVEN: A workspace entry with parent_workspace = Some("root")
///        AND "root" has no parent
/// WHEN: Calculating stack depth for the child
/// THEN: The function should return Ok(1)
#[test]
fn test_calculate_stack_depth_one_parent_returns_one() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result = calculate_stack_depth("child", &entries);

    assert_eq!(result, Ok(1));
}

/// Test that root still returns 0 when children exist.
///
/// GIVEN: Root workspace with children
/// WHEN: Calculating stack depth for root
/// THEN: Root should still return Ok(0)
#[test]
fn test_calculate_stack_depth_root_still_zero_with_children() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child-a", Some("root")),
        create_entry("child-b", Some("root")),
    ];

    let result = calculate_stack_depth("root", &entries);

    assert_eq!(result, Ok(0));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with chain of 3 parents returns depth 3
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with a chain of 3 parents returns depth 3.
///
/// GIVEN: A chain: grandchild -> child -> root (grandchild has depth 3 from root)
///        Actually: leaf -> mid -> child -> root
/// WHEN: Calculating stack depth for the leaf
/// THEN: The function should return Ok(3)
#[test]
fn test_calculate_stack_depth_chain_of_three_returns_three() {
    let entries = vec![
        create_entry("root", None),          // depth 0
        create_entry("child", Some("root")), // depth 1
        create_entry("mid", Some("child")),  // depth 2
        create_entry("leaf", Some("mid")),   // depth 3
    ];

    let result = calculate_stack_depth("leaf", &entries);

    assert_eq!(result, Ok(3));
}

/// Test each entry in the chain has correct depth.
///
/// GIVEN: A chain of 4 workspaces (root -> child -> mid -> leaf)
/// WHEN: Calculating depth for each
/// THEN: Each should have correct incremental depth
#[test]
fn test_calculate_stack_depth_each_in_chain_has_correct_depth() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
        create_entry("mid", Some("child")),
        create_entry("leaf", Some("mid")),
    ];

    assert_eq!(calculate_stack_depth("root", &entries), Ok(0));
    assert_eq!(calculate_stack_depth("child", &entries), Ok(1));
    assert_eq!(calculate_stack_depth("mid", &entries), Ok(2));
    assert_eq!(calculate_stack_depth("leaf", &entries), Ok(3));
}

/// Test a deep chain of 10 entries.
///
/// GIVEN: A chain of 10 workspaces
/// WHEN: Calculating depth for the deepest
/// THEN: Should return Ok(9)
#[test]
fn test_calculate_stack_depth_deep_chain() {
    let mut entries = Vec::with_capacity(10);
    entries.push(create_entry("ws-0", None));

    for i in 1..10 {
        let parent = format!("ws-{}", i - 1);
        entries.push(create_entry(&format!("ws-{i}"), Some(&parent)));
    }

    let result = calculate_stack_depth("ws-9", &entries);

    assert_eq!(result, Ok(9));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Error path - Cycle in chain returns StackError::CycleDetected
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a self-referencing workspace is detected as a cycle.
///
/// GIVEN: A workspace that references itself as parent
/// WHEN: Calculating stack depth
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_calculate_stack_depth_self_reference_returns_cycle_error() {
    let entries = vec![create_entry("self-loop", Some("self-loop"))];

    let result = calculate_stack_depth("self-loop", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that a 2-node cycle is detected.
///
/// GIVEN: A -> B -> A (cycle between two workspaces)
/// WHEN: Calculating stack depth for either
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_calculate_stack_depth_two_node_cycle_returns_cycle_error() {
    let entries = vec![
        create_entry("workspace-a", Some("workspace-b")),
        create_entry("workspace-b", Some("workspace-a")),
    ];

    let result = calculate_stack_depth("workspace-a", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that a 3-node cycle is detected.
///
/// GIVEN: A -> B -> C -> A (cycle among three workspaces)
/// WHEN: Calculating stack depth for any in cycle
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_calculate_stack_depth_three_node_cycle_returns_cycle_error() {
    let entries = vec![
        create_entry("workspace-a", Some("workspace-c")),
        create_entry("workspace-b", Some("workspace-a")),
        create_entry("workspace-c", Some("workspace-b")),
    ];

    let result = calculate_stack_depth("workspace-a", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that cycle error contains the workspace name.
///
/// GIVEN: A cycle exists in the chain
/// WHEN: CycleDetected error is returned
/// THEN: The error should contain the workspace where cycle was detected
#[test]
fn test_calculate_stack_depth_cycle_error_contains_workspace() {
    let entries = vec![create_entry("cyclic-ws", Some("cyclic-ws"))];

    let result = calculate_stack_depth("cyclic-ws", &entries);

    match result {
        Err(StackError::CycleDetected { workspace, .. }) => {
            assert_eq!(workspace, "cyclic-ws");
        }
        _ => panic!("Expected CycleDetected error"),
    }
}

/// Test that cycle error contains the cycle path.
///
/// GIVEN: A cycle exists in the chain
/// WHEN: CycleDetected error is returned
/// THEN: The error should contain the path of workspaces in the cycle
#[test]
fn test_calculate_stack_depth_cycle_error_contains_path() {
    let entries = vec![
        create_entry("ws-a", Some("ws-b")),
        create_entry("ws-b", Some("ws-a")),
    ];

    let result = calculate_stack_depth("ws-a", &entries);

    match result {
        Err(StackError::CycleDetected { cycle_path, .. }) => {
            // Path should contain the cycle
            assert!(!cycle_path.is_empty());
            // The path should include both workspaces in the cycle
            assert!(cycle_path.contains(&"ws-a".to_string()));
            assert!(cycle_path.contains(&"ws-b".to_string()));
        }
        _ => panic!("Expected CycleDetected error"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Edge cases - Parent not in entries list
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a reference to a non-existent parent returns an error.
///
/// GIVEN: A workspace with parent_workspace pointing to non-existent entry
/// WHEN: Calculating stack depth
/// THEN: Should return Err(StackError::ParentNotFound)
#[test]
fn test_calculate_stack_depth_missing_parent_returns_parent_not_found() {
    let entries = vec![create_entry("orphan", Some("missing-parent"))];

    let result = calculate_stack_depth("orphan", &entries);

    assert!(matches!(result, Err(StackError::ParentNotFound { .. })));
}

/// Test that ParentNotFound error contains the missing parent name.
///
/// GIVEN: A workspace referencing a missing parent
/// WHEN: ParentNotFound error is returned
/// THEN: The error should contain the missing parent's name
#[test]
fn test_calculate_stack_depth_parent_not_found_contains_name() {
    let entries = vec![create_entry("child-ws", Some("nonexistent-parent"))];

    let result = calculate_stack_depth("child-ws", &entries);

    match result {
        Err(StackError::ParentNotFound { parent_workspace }) => {
            assert_eq!(parent_workspace, "nonexistent-parent");
        }
        _ => panic!("Expected ParentNotFound error"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Edge cases - Workspace not in entries list
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that calculating depth for non-existent workspace returns error.
///
/// GIVEN: An empty entries list
/// WHEN: Calculating stack depth for any workspace
/// THEN: Should return an error (workspace not found)
#[test]
fn test_calculate_stack_depth_workspace_not_found() {
    let entries: Vec<QueueEntry> = vec![];

    let result = calculate_stack_depth("nonexistent", &entries);

    // The function should handle this case - likely with an error
    assert!(result.is_err());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Pure function - Deterministic behavior
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that the function is pure and deterministic.
///
/// GIVEN: Same inputs provided multiple times
/// WHEN: Calling calculate_stack_depth repeatedly
/// THEN: Should return the same result each time
#[test]
fn test_calculate_stack_depth_is_deterministic() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result1 = calculate_stack_depth("child", &entries);
    let result2 = calculate_stack_depth("child", &entries);
    let result3 = calculate_stack_depth("child", &entries);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Multiple independent stacks
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that independent stacks don't affect each other.
///
/// GIVEN: Two separate stack chains
/// WHEN: Calculating depth for each
/// THEN: Each chain should return its own correct depth
#[test]
fn test_calculate_stack_depth_independent_stacks() {
    let entries = vec![
        // Stack A: root-a -> child-a
        create_entry("root-a", None),
        create_entry("child-a", Some("root-a")),
        // Stack B: root-b -> child-b -> grandchild-b
        create_entry("root-b", None),
        create_entry("child-b", Some("root-b")),
        create_entry("grandchild-b", Some("child-b")),
    ];

    assert_eq!(calculate_stack_depth("root-a", &entries), Ok(0));
    assert_eq!(calculate_stack_depth("child-a", &entries), Ok(1));
    assert_eq!(calculate_stack_depth("root-b", &entries), Ok(0));
    assert_eq!(calculate_stack_depth("child-b", &entries), Ok(1));
    assert_eq!(calculate_stack_depth("grandchild-b", &entries), Ok(2));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified workspace and parent.
///
/// This helper creates a QueueEntry with minimal required fields for
/// stack depth calculation tests.
fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
    QueueEntry {
        id: 1,
        workspace: workspace.to_string(),
        bead_id: None,
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
        parent_workspace: parent_workspace.map(std::string::ToString::to_string),
        stack_depth: 0,
        dependents: Dependents::new(),
        stack_root: None,
        stack_merge_state: StackMergeState::Pending,
    }
}

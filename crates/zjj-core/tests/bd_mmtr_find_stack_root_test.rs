//! ATDD Test for bd-mmtr: Find Stack Root
//!
//! BEAD: bd-mmtr
//! REQUIREMENT: Pure function to find root of any stack
//! EARS:
//!   - THE SYSTEM SHALL provide find_stack_root(workspace, entries) -> Result<String, StackError>
//!   - WHEN entry has no parent, THE SYSTEM SHALL return itself as root
//!   - WHEN entry has parent chain, THE SYSTEM SHALL traverse to root and return root name
//!   - IF cycle detected in parent chain, THE SYSTEM SHALL return StackError::CycleDetected
//!   - THE SYSTEM SHALL be a pure function with no panic paths
//!
//! This test file should:
//!   1. COMPILE (type definitions are valid Rust)
//!   2. FAIL initially (find_stack_root function doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown)]

// Import the function under test - will fail to compile until implemented
use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
    stack_depth::find_stack_root,
    stack_error::StackError,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with no parent returns itself as root
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with no parent (root) returns its own name.
///
/// GIVEN: A workspace entry with parent_workspace = None
/// WHEN: Finding stack root
/// THEN: The function should return Ok(workspace_name)
#[test]
fn test_root_returns_self() {
    let entries = vec![create_entry("root-workspace", None)];

    let result = find_stack_root("root-workspace", &entries);

    assert_eq!(result, Ok("root-workspace".to_string()));
}

/// Test that multiple root entries each return themselves.
///
/// GIVEN: Multiple workspaces with no parents
/// WHEN: Finding stack root for each
/// THEN: Each should return its own name as root
#[test]
fn test_multiple_roots_return_self() {
    let entries = vec![
        create_entry("root-a", None),
        create_entry("root-b", None),
        create_entry("root-c", None),
    ];

    assert_eq!(
        find_stack_root("root-a", &entries),
        Ok("root-a".to_string())
    );
    assert_eq!(
        find_stack_root("root-b", &entries),
        Ok("root-b".to_string())
    );
    assert_eq!(
        find_stack_root("root-c", &entries),
        Ok("root-c".to_string())
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with one parent returns the parent
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with a root parent returns the root name.
///
/// GIVEN: A workspace entry with parent_workspace = Some("root")
///        AND "root" has no parent
/// WHEN: Finding stack root for the child
/// THEN: The function should return Ok("root")
#[test]
fn test_one_level_returns_parent() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result = find_stack_root("child", &entries);

    assert_eq!(result, Ok("root".to_string()));
}

/// Test that root still returns itself when children exist.
///
/// GIVEN: Root workspace with children
/// WHEN: Finding stack root for root
/// THEN: Root should still return itself
#[test]
fn test_root_still_returns_self_with_children() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child-a", Some("root")),
        create_entry("child-b", Some("root")),
    ];

    let result = find_stack_root("root", &entries);

    assert_eq!(result, Ok("root".to_string()));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Entry with chain of parents returns the root
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with a chain of parents returns the root.
///
/// GIVEN: A chain: leaf -> mid -> child -> root
/// WHEN: Finding stack root for the leaf
/// THEN: The function should return Ok("root")
#[test]
fn test_multi_level_returns_root() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
        create_entry("mid", Some("child")),
        create_entry("leaf", Some("mid")),
    ];

    let result = find_stack_root("leaf", &entries);

    assert_eq!(result, Ok("root".to_string()));
}

/// Test each entry in the chain has the same root.
///
/// GIVEN: A chain of 4 workspaces (root -> child -> mid -> leaf)
/// WHEN: Finding root for each
/// THEN: All should return the same root
#[test]
fn test_each_in_chain_has_same_root() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
        create_entry("mid", Some("child")),
        create_entry("leaf", Some("mid")),
    ];

    let expected_root = Ok("root".to_string());
    assert_eq!(find_stack_root("root", &entries), expected_root);
    assert_eq!(find_stack_root("child", &entries), expected_root);
    assert_eq!(find_stack_root("mid", &entries), expected_root);
    assert_eq!(find_stack_root("leaf", &entries), expected_root);
}

/// Test a deep chain of 10 entries returns correct root.
///
/// GIVEN: A chain of 10 workspaces
/// WHEN: Finding root for the deepest
/// THEN: Should return Ok("ws-0") (the root)
#[test]
fn test_deep_chain_returns_root() {
    let mut entries = Vec::with_capacity(10);
    entries.push(create_entry("ws-0", None));

    for i in 1..10 {
        let parent = format!("ws-{}", i - 1);
        entries.push(create_entry(&format!("ws-{i}"), Some(&parent)));
    }

    let result = find_stack_root("ws-9", &entries);

    assert_eq!(result, Ok("ws-0".to_string()));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Error path - Self-reference cycle returns CycleDetected
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a self-referencing workspace is detected as a cycle.
///
/// GIVEN: A workspace that references itself as parent
/// WHEN: Finding stack root
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_self_reference_cycle() {
    let entries = vec![create_entry("self-loop", Some("self-loop"))];

    let result = find_stack_root("self-loop", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that cycle error contains the workspace name.
///
/// GIVEN: A self-referencing workspace
/// WHEN: CycleDetected error is returned
/// THEN: The error should contain the workspace where cycle was detected
#[test]
fn test_self_reference_cycle_error_contains_workspace() {
    let entries = vec![create_entry("cyclic-ws", Some("cyclic-ws"))];

    let result = find_stack_root("cyclic-ws", &entries);

    match result {
        Err(StackError::CycleDetected { workspace, .. }) => {
            assert_eq!(workspace, "cyclic-ws");
        }
        _ => panic!("Expected CycleDetected error"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Error path - Cycle in chain returns CycleDetected
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a 2-node cycle is detected.
///
/// GIVEN: A -> B -> A (cycle between two workspaces)
/// WHEN: Finding stack root for either
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_cycle_in_chain() {
    let entries = vec![
        create_entry("workspace-a", Some("workspace-b")),
        create_entry("workspace-b", Some("workspace-a")),
    ];

    let result = find_stack_root("workspace-a", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that a 3-node cycle is detected.
///
/// GIVEN: A -> C -> B -> A (cycle among three workspaces)
/// WHEN: Finding stack root for any in cycle
/// THEN: Should return Err(StackError::CycleDetected)
#[test]
fn test_three_node_cycle_returns_cycle_error() {
    let entries = vec![
        create_entry("workspace-a", Some("workspace-c")),
        create_entry("workspace-b", Some("workspace-a")),
        create_entry("workspace-c", Some("workspace-b")),
    ];

    let result = find_stack_root("workspace-a", &entries);

    assert!(matches!(result, Err(StackError::CycleDetected { .. })));
}

/// Test that cycle error contains the cycle path.
///
/// GIVEN: A cycle exists in the chain
/// WHEN: CycleDetected error is returned
/// THEN: The error should contain the path of workspaces in the cycle
#[test]
fn test_cycle_in_chain_error_contains_path() {
    let entries = vec![
        create_entry("ws-a", Some("ws-b")),
        create_entry("ws-b", Some("ws-a")),
    ];

    let result = find_stack_root("ws-a", &entries);

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
// TEST: Error path - Parent not in entries list
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a reference to a non-existent parent returns an error.
///
/// GIVEN: A workspace with parent_workspace pointing to non-existent entry
/// WHEN: Finding stack root
/// THEN: Should return Err(StackError::ParentNotFound)
#[test]
fn test_missing_parent() {
    let entries = vec![create_entry("orphan", Some("missing-parent"))];

    let result = find_stack_root("orphan", &entries);

    assert!(matches!(result, Err(StackError::ParentNotFound { .. })));
}

/// Test that ParentNotFound error contains the missing parent name.
///
/// GIVEN: A workspace referencing a missing parent
/// WHEN: ParentNotFound error is returned
/// THEN: The error should contain the missing parent's name
#[test]
fn test_missing_parent_error_contains_name() {
    let entries = vec![create_entry("child-ws", Some("nonexistent-parent"))];

    let result = find_stack_root("child-ws", &entries);

    match result {
        Err(StackError::ParentNotFound { parent_workspace }) => {
            assert_eq!(parent_workspace, "nonexistent-parent");
        }
        _ => panic!("Expected ParentNotFound error"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Error path - Workspace not in entries list
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that finding root for non-existent workspace returns error.
///
/// GIVEN: An empty entries list
/// WHEN: Finding stack root for any workspace
/// THEN: Should return an error (workspace not found)
#[test]
fn test_workspace_not_found() {
    let entries: Vec<QueueEntry> = vec![];

    let result = find_stack_root("nonexistent", &entries);

    // The function should handle this case - likely with an error
    assert!(result.is_err());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Pure function - Deterministic behavior
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that the function is pure and deterministic.
///
/// GIVEN: Same inputs provided multiple times
/// WHEN: Calling find_stack_root repeatedly
/// THEN: Should return the same result each time
#[test]
fn test_is_deterministic() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result1 = find_stack_root("child", &entries);
    let result2 = find_stack_root("child", &entries);
    let result3 = find_stack_root("child", &entries);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Multiple independent stacks
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that independent stacks return their own roots.
///
/// GIVEN: Two separate stack chains
/// WHEN: Finding root for each
/// THEN: Each chain should return its own correct root
#[test]
fn test_independent_stacks() {
    let entries = vec![
        // Stack A: root-a -> child-a
        create_entry("root-a", None),
        create_entry("child-a", Some("root-a")),
        // Stack B: root-b -> child-b -> grandchild-b
        create_entry("root-b", None),
        create_entry("child-b", Some("root-b")),
        create_entry("grandchild-b", Some("child-b")),
    ];

    assert_eq!(
        find_stack_root("root-a", &entries),
        Ok("root-a".to_string())
    );
    assert_eq!(
        find_stack_root("child-a", &entries),
        Ok("root-a".to_string())
    );
    assert_eq!(
        find_stack_root("root-b", &entries),
        Ok("root-b".to_string())
    );
    assert_eq!(
        find_stack_root("child-b", &entries),
        Ok("root-b".to_string())
    );
    assert_eq!(
        find_stack_root("grandchild-b", &entries),
        Ok("root-b".to_string())
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified workspace and parent.
///
/// This helper creates a QueueEntry with minimal required fields for
/// stack root calculation tests.
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

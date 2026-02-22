//! ATDD Test for bd-i6lp: build_dependent_list function
//!
//! BEAD: bd-i6lp
//! REQUIREMENT: List all descendants (children, grandchildren, etc.) of a workspace
//! CONTRACT: `pub fn build_dependent_list(workspace: &str, entries: &[QueueEntry]) -> Vec<String>`
//! EARS:
//!   - THE SYSTEM SHALL list all descendants
//!   - WHEN given workspace, THE SYSTEM SHALL find all children recursively
//!   - IF no dependents, THE SYSTEM SHALL NOT error (empty is valid)
//!   - Order is breadth-first
//!
//! This test file should:
//!   1. COMPILE (function signature is valid Rust)
//!   2. FAIL initially (function doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown, clippy::unreadable_literal)]

use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
    stack_depth::build_dependent_list,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry with parent relationship
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified workspace and optional parent.
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
// TEST: No dependents returns empty vector
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with no children returns an empty vector.
///
/// GIVEN: A workspace with no children
/// WHEN: Calling build_dependent_list
/// THEN: Returns empty Vec<String>
#[test]
fn test_no_dependents_returns_empty() {
    let entries = vec![create_entry("root", None), create_entry("other", None)];

    let result = build_dependent_list("root", &entries);

    assert!(
        result.is_empty(),
        "Workspace with no children should return empty vec"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: One child returns that child
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with one child returns that child.
///
/// GIVEN: A workspace with one direct child
/// WHEN: Calling build_dependent_list
/// THEN: Returns Vec containing the child workspace name
#[test]
fn test_one_child_returns_child() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result = build_dependent_list("root", &entries);

    assert_eq!(result, vec!["child"]);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Multiple direct children
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that a workspace with multiple children returns all of them.
///
/// GIVEN: A workspace with multiple direct children
/// WHEN: Calling build_dependent_list
/// THEN: Returns Vec containing all child workspace names
#[test]
fn test_multiple_children() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child1", Some("root")),
        create_entry("child2", Some("root")),
        create_entry("child3", Some("root")),
    ];

    let result = build_dependent_list("root", &entries);

    // Should contain all children (order may vary, so we check inclusion)
    assert_eq!(result.len(), 3);
    assert!(result.contains(&"child1".to_string()));
    assert!(result.contains(&"child2".to_string()));
    assert!(result.contains(&"child3".to_string()));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Grandchildren are included
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that grandchildren are included in the result.
///
/// GIVEN: A workspace with children and grandchildren
/// WHEN: Calling build_dependent_list
/// THEN: Returns Vec containing both children and grandchildren
#[test]
fn test_grandchildren_included() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
        create_entry("grandchild", Some("child")),
    ];

    let result = build_dependent_list("root", &entries);

    assert_eq!(result.len(), 2);
    assert!(result.contains(&"child".to_string()));
    assert!(result.contains(&"grandchild".to_string()));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Breadth-first ordering
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that descendants are returned in breadth-first order.
///
/// GIVEN: A workspace with a tree of descendants
/// WHEN: Calling build_dependent_list
/// THEN: Children appear before grandchildren (BFS order)
#[test]
fn test_breadth_first_order() {
    // Tree structure:
    //       root
    //      /    \
    //   child1  child2
    //    |
    // grandchild1
    let entries = vec![
        create_entry("root", None),
        create_entry("child1", Some("root")),
        create_entry("child2", Some("root")),
        create_entry("grandchild1", Some("child1")),
    ];

    let result = build_dependent_list("root", &entries);

    // In BFS order, children must come before grandchildren
    let child1_idx = result.iter().position(|s| s == "child1");
    let child2_idx = result.iter().position(|s| s == "child2");
    let grandchild_idx = result.iter().position(|s| s == "grandchild1");

    assert!(child1_idx.is_some(), "child1 should be in result");
    assert!(child2_idx.is_some(), "child2 should be in result");
    assert!(grandchild_idx.is_some(), "grandchild1 should be in result");

    // BFS: both children must appear before grandchild
    assert!(
        child1_idx < grandchild_idx,
        "child1 should appear before grandchild1 in BFS order"
    );
    assert!(
        child2_idx < grandchild_idx,
        "child2 should appear before grandchild1 in BFS order"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Deep chain of descendants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test deeply nested chain of descendants.
///
/// GIVEN: A linear chain of 5 workspaces
/// WHEN: Calling build_dependent_list on root
/// THEN: Returns all 4 descendants in order
#[test]
fn test_deep_chain() {
    let entries = vec![
        create_entry("level0", None),
        create_entry("level1", Some("level0")),
        create_entry("level2", Some("level1")),
        create_entry("level3", Some("level2")),
        create_entry("level4", Some("level3")),
    ];

    let result = build_dependent_list("level0", &entries);

    assert_eq!(result.len(), 4);
    // In a linear chain, BFS order is the same as depth order
    assert_eq!(result[0], "level1");
    assert_eq!(result[1], "level2");
    assert_eq!(result[2], "level3");
    assert_eq!(result[3], "level4");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Missing workspace returns empty (graceful)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that querying a non-existent workspace returns empty vector.
///
/// GIVEN: A set of entries that doesn't include the queried workspace
/// WHEN: Calling build_dependent_list with non-existent workspace
/// THEN: Returns empty Vec (graceful handling, not an error)
#[test]
fn test_missing_workspace_returns_empty() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result = build_dependent_list("nonexistent", &entries);

    assert!(
        result.is_empty(),
        "Non-existent workspace should return empty vec"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Complex tree structure
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test a more complex tree structure with multiple branches.
///
/// GIVEN: A tree with multiple branches and depths
/// WHEN: Calling build_dependent_list on root
/// THEN: All descendants are returned in BFS order
#[test]
fn test_complex_tree_structure() {
    // Tree structure:
    //           root
    //         /   |   \
    //       a     b     c
    //      / \    |     |
    //     d   e   f     g
    //                \
    //                 h
    let entries = vec![
        create_entry("root", None),
        create_entry("a", Some("root")),
        create_entry("b", Some("root")),
        create_entry("c", Some("root")),
        create_entry("d", Some("a")),
        create_entry("e", Some("a")),
        create_entry("f", Some("b")),
        create_entry("g", Some("c")),
        create_entry("h", Some("g")),
    ];

    let result = build_dependent_list("root", &entries);

    // All 8 descendants should be present
    assert_eq!(result.len(), 8);

    // BFS: first level (a, b, c) before second level (d, e, f, g)
    // and second level before third level (h)
    let a_idx = result.iter().position(|s| s == "a");
    let b_idx = result.iter().position(|s| s == "b");
    let c_idx = result.iter().position(|s| s == "c");
    let d_idx = result.iter().position(|s| s == "d");
    let h_idx = result.iter().position(|s| s == "h");

    // Verify all positions are found
    assert!(a_idx.is_some(), "a should be in result");
    assert!(b_idx.is_some(), "b should be in result");
    assert!(c_idx.is_some(), "c should be in result");
    assert!(d_idx.is_some(), "d should be in result");
    assert!(h_idx.is_some(), "h should be in result");

    // First level before second level
    assert!(a_idx < d_idx, "a should come before d");
    assert!(b_idx < d_idx, "b should come before d");
    assert!(c_idx < d_idx, "c should come before d");

    // Second level before third level
    assert!(d_idx < h_idx, "d should come before h");
    assert!(c_idx < h_idx, "c should come before h");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Empty entries slice
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that empty entries slice returns empty vector.
///
/// GIVEN: An empty slice of entries
/// WHEN: Calling build_dependent_list
/// THEN: Returns empty Vec
#[test]
fn test_empty_entries_returns_empty() {
    let entries: Vec<QueueEntry> = vec![];

    let result = build_dependent_list("any-workspace", &entries);

    assert!(result.is_empty(), "Empty entries should return empty vec");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Descendants from middle of tree
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that we can get descendants from a middle node, not just root.
///
/// GIVEN: A tree where we query from a middle node
/// WHEN: Calling build_dependent_list on a non-root workspace
/// THEN: Returns only descendants of that workspace
#[test]
fn test_descendants_from_middle_node() {
    // Tree: root -> a -> b -> c
    let entries = vec![
        create_entry("root", None),
        create_entry("a", Some("root")),
        create_entry("b", Some("a")),
        create_entry("c", Some("b")),
    ];

    let result = build_dependent_list("a", &entries);

    // Should only include b and c, not root
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"b".to_string()));
    assert!(result.contains(&"c".to_string()));
    assert!(!result.contains(&"root".to_string()));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Function is pure (same input, same output)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that the function is pure - same input always produces same output.
///
/// GIVEN: Same inputs called multiple times
/// WHEN: Calling build_dependent_list repeatedly
/// THEN: Returns identical results each time
#[test]
fn test_function_is_pure() {
    let entries = vec![
        create_entry("root", None),
        create_entry("child", Some("root")),
    ];

    let result1 = build_dependent_list("root", &entries);
    let result2 = build_dependent_list("root", &entries);
    let result3 = build_dependent_list("root", &entries);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

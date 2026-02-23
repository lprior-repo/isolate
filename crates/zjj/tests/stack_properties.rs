//! Property-based tests for stack invariants using proptest.
//!
//! These tests verify the core invariants of the stack data structure:
//! - Acyclicity: No cycles in the parent-child relationships
//! - Finite depth: Depth is always bounded
//! - Root reachability: Root is reachable from all descendants
//! - Parent-child consistency: Parents exist and relationships are valid
//!
//! RED PHASE: These tests are designed to FAIL initially.
//! The implementation will be added in a subsequent phase.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::HashSet;

use proptest::prelude::*;
use zjj_core::coordination::{
    queue_entities::{Dependents, QueueEntry},
    queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
    stack_depth::{calculate_stack_depth, find_stack_root},
    stack_error::StackError,
};

// ═══════════════════════════════════════════════════════════════════════════
// TEST DATA GENERATORS
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a valid workspace name (alphanumeric with hyphens).
fn workspace_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,15}"
}

/// Generate an optional parent workspace reference.
fn optional_parent() -> impl Strategy<Value = Option<String>> {
    proptest::option::of(workspace_name())
}

/// Generate a single queue entry with specified workspace and parent.
#[allow(dead_code)]
fn queue_entry_with_parent(
    workspace: String,
    parent: Option<String>,
) -> impl Strategy<Value = QueueEntry> {
    Just(QueueEntry {
        id: 1,
        workspace,
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
        parent_workspace: parent,
        stack_depth: 0,
        dependents: Dependents::new(),
        stack_root: None,
        stack_merge_state: StackMergeState::Independent,
    })
}

/// Generate a valid acyclic stack (tree structure) with specified size.
fn acyclic_stack(size: usize) -> impl Strategy<Value = Vec<QueueEntry>> {
    proptest::collection::vec(workspace_name(), 0..=size).prop_flat_map(move |names| {
        // Create entries where each non-root entry has a valid parent
        let entries: Vec<QueueEntry> = names
            .into_iter()
            .enumerate()
            .map(|(idx, name)| {
                // First entry is root (no parent), others reference a previous entry
                let parent = if idx == 0 {
                    None
                } else {
                    // Reference a random earlier entry (simulated - in real impl would be actual
                    // name)
                    Some(format!("parent-{}", idx.saturating_sub(1)))
                };
                QueueEntry {
                    id: idx as i64 + 1,
                    workspace: name,
                    bead_id: None,
                    priority: 0,
                    status: QueueStatus::Pending,
                    added_at: 1_700_000_000 + idx as i64,
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
                    parent_workspace: parent,
                    stack_depth: 0,
                    dependents: Dependents::new(),
                    stack_root: None,
                    stack_merge_state: StackMergeState::Independent,
                }
            })
            .collect();
        Just(entries)
    })
}

/// Generate a stack with a potential cycle.
fn cyclic_stack() -> impl Strategy<Value = Vec<QueueEntry>> {
    (1..=5usize).prop_flat_map(|size| {
        proptest::collection::vec(workspace_name(), size).prop_map(|names| {
            if names.is_empty() {
                return Vec::new();
            }

            let mut entries: Vec<QueueEntry> = names
                .iter()
                .enumerate()
                .map(|(idx, name)| QueueEntry {
                    id: idx as i64 + 1,
                    workspace: name.clone(),
                    bead_id: None,
                    priority: 0,
                    status: QueueStatus::Pending,
                    added_at: 1_700_000_000 + idx as i64,
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
                })
                .collect();

            // Create a cycle: last -> first
            if entries.len() > 1 {
                let first_name = entries[0].workspace.clone();
                let last_idx = entries.len().saturating_sub(1);
                entries[last_idx].parent_workspace = Some(first_name);
                // Also make first point to last to create the cycle
                entries[0].parent_workspace = Some(entries[last_idx].workspace.clone());
            }

            entries
        })
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: ACYCLICITY
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: Detect if there's a cycle in the entries.
fn has_cycle(entries: &[QueueEntry]) -> bool {
    // Build adjacency: child -> parent
    let workspace_set: HashSet<&str> = entries.iter().map(|e| e.workspace.as_str()).collect();

    for entry in entries {
        let mut visited = HashSet::new();
        let mut current = entry.workspace.as_str();

        loop {
            if visited.contains(current) {
                return true; // Cycle detected
            }
            visited.insert(current);

            let current_entry = entries.iter().find(|e| e.workspace == current);
            match current_entry {
                None => break, // Parent not in set, no cycle through this path
                Some(e) => match &e.parent_workspace {
                    None => break,                          // Reached root
                    Some(p) if p == current => return true, // Self-loop
                    Some(p) => {
                        if !workspace_set.contains(p.as_str()) {
                            break; // Parent outside our set, stop
                        }
                        current = p.as_str();
                    }
                },
            }
        }
    }
    false
}

proptest! {
    /// Property: A valid acyclic stack should never report a cycle.
    ///
    /// Given: A stack with no cycles in parent relationships
    /// When: We calculate depth or find root
    /// Then: Should succeed without CycleDetected error
    #[test]
    fn prop_acyclic_stack_no_cycle_error(
        entries in acyclic_stack(10)
    ) {
        // Skip empty stacks
        if entries.is_empty() {
            return Ok(());
        }

        // For each entry, verify we can calculate depth without cycle error
        for entry in &entries {
            let result = calculate_stack_depth(&entry.workspace, &entries);
            // In a valid acyclic stack, we should either succeed or get ParentNotFound
            // but NEVER CycleDetected
            match result {
                Err(StackError::CycleDetected { .. }) => {
                    // FAIL: This should never happen in an acyclic stack
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        "CycleDetected error in acyclic stack".into()
                    ));
                }
                Ok(_) | Err(StackError::ParentNotFound { .. }) => {}
                Err(StackError::DepthExceeded { .. }) | Err(StackError::InvalidParent { .. }) => {}
            }
        }
    }

    /// Property: A stack with cycles MUST always report a cycle error.
    ///
    /// Given: A stack with a cycle in parent relationships
    /// When: We calculate depth for any entry in the cycle
    /// Then: Must return CycleDetected error
    #[test]
    fn prop_cyclic_stack_reports_cycle(
        entries in cyclic_stack()
    ) {
        // Skip empty or single-entry stacks
        if entries.len() < 2 {
            return Ok(());
        }

        // Verify the generated stack actually has a cycle
        if !has_cycle(&entries) {
            return Ok(()); // Not actually cyclic, skip
        }

        // At least one entry in the cycle should report CycleDetected
        let mut cycle_reported = false;
        for entry in &entries {
            if let Err(StackError::CycleDetected { .. }) =
                calculate_stack_depth(&entry.workspace, &entries)
            {
                cycle_reported = true;
                break;
            }
        }

        if !cycle_reported {
            // FAIL: Cyclic stack did not report cycle
            return Err(proptest::test_runner::TestCaseError::Fail(
                "Cyclic stack did not report CycleDetected error".into()
            ));
        }
    }

    /// Property: No workspace is its own parent (direct cycle).
    ///
    /// Given: Any valid stack
    /// When: We check parent relationships
    /// Then: No workspace should reference itself as parent
    #[test]
    fn prop_no_self_parent(
        workspace in workspace_name(),
        parent in optional_parent()
    ) {
        // A workspace should never be its own parent
        if let Some(ref p) = parent {
            prop_assert_ne!(p, &workspace, "Workspace cannot be its own parent");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: FINITE DEPTH
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Property: Depth is always finite and bounded by entry count.
    ///
    /// Given: A valid acyclic stack with N entries
    /// When: We calculate depth for any entry
    /// Then: Depth must be in range [0, N-1]
    #[test]
    fn prop_depth_is_finite(
        entries in acyclic_stack(20)
    ) {
        let entry_count = entries.len();
        if entry_count == 0 {
            return Ok(());
        }

        for entry in &entries {
            match calculate_stack_depth(&entry.workspace, &entries) {
                Ok(depth) => {
                    // Depth must be less than total entry count
                    // (can't have more ancestors than total entries)
                    let max_allowed = u32::try_from(entry_count.saturating_sub(1)).map_or(u32::MAX, |v| v);
                    if depth > max_allowed {
                        return Err(proptest::test_runner::TestCaseError::Fail(
                            format!("Depth {} exceeds max allowed {}", depth, max_allowed).into()
                        ));
                    }
                }
                Err(StackError::ParentNotFound { .. }) => {
                    // Valid - parent not in our entry set
                }
                Err(StackError::CycleDetected { .. }) => {
                    // Should not happen in acyclic stack
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        "Unexpected cycle in acyclic stack".into()
                    ));
                }
                Err(StackError::DepthExceeded { .. }) | Err(StackError::InvalidParent { .. }) => {}
            }
        }
    }

    /// Property: Root has depth 0.
    ///
    /// Given: A stack where at least one entry has no parent
    /// When: We calculate depth for that entry
    /// Then: Depth must be 0
    #[test]
    fn prop_root_has_depth_zero(
        root_name in workspace_name()
    ) {
        let root_entry = QueueEntry {
            id: 1,
            workspace: root_name.clone(),
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
            parent_workspace: None, // This is a root
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        };

        let entries = vec![root_entry];
        let result = calculate_stack_depth(&root_name, &entries);

        match result {
            Ok(depth) => {
                if depth != 0 {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("Root depth should be 0, got {}", depth).into()
                    ));
                }
            }
            Err(e) => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("Root depth calculation failed: {:?}", e).into()
                ));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: ROOT REACHABILITY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Property: All descendants share the same root.
    ///
    /// Given: A valid acyclic stack
    /// When: We find the root for each entry
    /// Then: All entries in the same tree should have the same root
    #[test]
    fn prop_descendants_share_root(
        entries in acyclic_stack(10)
    ) {
        if entries.len() < 2 {
            return Ok(());
        }

        // Find all roots (entries with no parent in the set)
        let workspace_set: HashSet<&str> = entries.iter().map(|e| e.workspace.as_str()).collect();
        let roots: Vec<&str> = entries
            .iter()
            .filter(|e| {
                match &e.parent_workspace {
                    None => true,
                    Some(p) => !workspace_set.contains(p.as_str()),
                }
            })
            .map(|e| e.workspace.as_str())
            .collect();

        if roots.is_empty() {
            // Could happen if there's a cycle - skip
            return Ok(());
        }

        // For entries that have a path to a root, verify they reach it
        for entry in &entries {
            match find_stack_root(&entry.workspace, &entries) {
                Ok(root) => {
                    // The found root should either be in our roots list
                    // or the entry itself should be a root
                    if !roots.contains(&root.as_str()) {
                        // Check if this is because parent is outside our set
                        let has_external_parent = entry
                            .parent_workspace
                            .as_ref()
                            .is_some_and(|p| !workspace_set.contains(p.as_str()));
                        if !has_external_parent && root != entry.workspace {
                            return Err(proptest::test_runner::TestCaseError::Fail(
                                format!("Root {} not in roots list {:?}", root, roots).into()
                            ));
                        }
                    }
                }
                Err(StackError::CycleDetected { .. }) => {
                    // Should not happen in acyclic stack
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        "Unexpected cycle detected".into()
                    ));
                }
                Err(StackError::ParentNotFound { .. }) => {
                    // Valid - can happen if parent not in set
                }
                Err(StackError::DepthExceeded { .. }) | Err(StackError::InvalidParent { .. }) => {}
            }
        }
    }

    /// Property: Root is reachable from all nodes in the tree.
    ///
    /// Given: A valid tree structure
    /// When: We traverse from any node toward root
    /// Then: We should eventually reach a node with no parent
    #[test]
    fn prop_root_reachable_from_all(
        root in workspace_name(),
        child_count in 1usize..=5
    ) {
        // Create a simple tree: root -> child1 -> child2 -> ...
        let mut entries = Vec::new();

        entries.push(QueueEntry {
            id: 1,
            workspace: root.clone(),
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
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        });

        let mut current_parent = root.clone();
        for i in 1..=child_count {
            let child_name = format!("child-{}", i);
            entries.push(QueueEntry {
                id: (i + 1) as i64,
                workspace: child_name.clone(),
                bead_id: None,
                priority: 0,
                status: QueueStatus::Pending,
                added_at: 1_700_000_000 + i as i64,
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
                parent_workspace: Some(current_parent.clone()),
                stack_depth: 0,
                dependents: Dependents::new(),
                stack_root: None,
                stack_merge_state: StackMergeState::Independent,
            });
            current_parent = child_name;
        }

        // Verify all entries can reach root
        for entry in &entries {
            match find_stack_root(&entry.workspace, &entries) {
                Ok(found_root) => {
                    if found_root != root {
                        return Err(proptest::test_runner::TestCaseError::Fail(
                            format!(
                                "Entry {} reached root {} instead of {}",
                                entry.workspace, found_root, root
                            )
                            .into()
                        ));
                    }
                }
                Err(e) => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("Failed to find root for {}: {:?}", entry.workspace, e).into()
                    ));
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY: PARENT-CHILD CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Property: Child depth = parent depth + 1.
    ///
    /// Given: A valid parent-child relationship
    /// When: We calculate both depths
    /// Then: child.depth should equal parent.depth + 1
    #[test]
    fn prop_child_depth_is_parent_depth_plus_one(
        parent_name in workspace_name(),
        child_name in workspace_name()
    ) {
        // Skip if names are the same
        if parent_name == child_name {
            return Ok(());
        }

        let parent_entry = QueueEntry {
            id: 1,
            workspace: parent_name.clone(),
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
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        };

        let child_entry = QueueEntry {
            id: 2,
            workspace: child_name.clone(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_001,
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
            parent_workspace: Some(parent_name.clone()),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        };

        let entries = vec![parent_entry, child_entry];

        let parent_depth = calculate_stack_depth(&parent_name, &entries);
        let child_depth = calculate_stack_depth(&child_name, &entries);

        match (parent_depth, child_depth) {
            (Ok(p_depth), Ok(c_depth)) => {
                let expected_child_depth = p_depth.saturating_add(1);
                if c_depth != expected_child_depth {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!(
                            "Child depth {} should be {} (parent {} + 1)",
                            c_depth, expected_child_depth, p_depth
                        )
                        .into()
                    ));
                }
            }
            (Ok(_), Err(e)) => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("Child depth calculation failed: {:?}", e).into()
                ));
            }
            (Err(e), _) => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("Parent depth calculation failed: {:?}", e).into()
                ));
            }
        }
    }

    /// Property: Parent must exist in the stack.
    ///
    /// Given: A child referencing a parent
    /// When: The parent is not in the entry list
    /// Then: Must return ParentNotFound error
    #[test]
    fn prop_missing_parent_returns_error(
        child_name in workspace_name(),
        missing_parent in workspace_name()
    ) {
        if child_name == missing_parent {
            return Ok(());
        }

        let child_entry = QueueEntry {
            id: 1,
            workspace: child_name.clone(),
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
            parent_workspace: Some(missing_parent.clone()),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        };

        let entries = vec![child_entry];
        let result = calculate_stack_depth(&child_name, &entries);

        match result {
            Err(StackError::ParentNotFound { parent_workspace }) => {
                if parent_workspace != missing_parent {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!(
                            "ParentNotFound should reference {}, got {}",
                            missing_parent, parent_workspace
                        )
                        .into()
                    ));
                }
            }
            Ok(_) => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    "Should return ParentNotFound for missing parent".into()
                ));
            }
            Err(StackError::CycleDetected { .. })
            | Err(StackError::DepthExceeded { .. })
            | Err(StackError::InvalidParent { .. }) => {}
        }
    }

    /// Property: Dependents list is consistent with actual children.
    ///
    /// Given: A stack with parent-child relationships
    /// When: We check the dependents list of a parent
    /// Then: It should contain exactly its direct children
    #[test]
    fn prop_dependents_list_consistency(
        parent_name in workspace_name(),
        child_names in proptest::collection::vec(workspace_name(), 1..=5)
    ) {
        // Skip if any child name equals parent
        if child_names.iter().any(|c| c == &parent_name) {
            return Ok(());
        }

        let mut entries = Vec::new();

        // Create parent
        let children_set: Vec<String> = child_names.clone();
        entries.push(QueueEntry {
            id: 1,
            workspace: parent_name.clone(),
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
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::from_vec(children_set.clone()),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        });

        // Create children
        for (idx, child_name) in child_names.iter().enumerate() {
            entries.push(QueueEntry {
                id: (idx + 2) as i64,
                workspace: child_name.clone(),
                bead_id: None,
                priority: 0,
                status: QueueStatus::Pending,
                added_at: 1_700_000_000 + idx as i64 + 1,
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
                parent_workspace: Some(parent_name.clone()),
                stack_depth: 0,
                dependents: Dependents::new(),
                stack_root: None,
                stack_merge_state: StackMergeState::Independent,
            });
        }

        // Verify consistency
        let parent_entry = entries
            .iter()
            .find(|e| e.workspace == parent_name);

        if let Some(parent) = parent_entry {
            let actual_children: HashSet<&String> = entries
                .iter()
                .filter(|e| e.parent_workspace.as_ref().is_some_and(|p| p == &parent_name))
                .map(|e| &e.workspace)
                .collect();

            let declared_dependents: HashSet<&String> = parent.dependents.iter().collect();

            if actual_children != declared_dependents {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!(
                        "Dependents mismatch: actual {:?} vs declared {:?}",
                        actual_children, declared_dependents
                    )
                    .into()
                ));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRATION: FULL STACK PROPERTIES
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// Property: Full stack consistency check.
    ///
    /// Given: A complete stack with multiple levels
    /// When: We verify all invariants
    /// Then: All must hold simultaneously
    #[test]
    fn prop_full_stack_consistency(
        root in workspace_name(),
        depth in 1u32..=10
    ) {
        // Build a linear stack: root -> l1 -> l2 -> ... -> l{depth}
        let mut entries = Vec::new();

        entries.push(QueueEntry {
            id: 1,
            workspace: root.clone(),
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
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::from_vec(vec![format!("level-1")]),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        });

        let mut current_parent = root.clone();
        for level in 1..=depth {
            let level_name = format!("level-{}", level);
            let next_level = format!("level-{}", level.saturating_add(1));

            entries.push(QueueEntry {
                id: (level + 1) as i64,
                workspace: level_name.clone(),
                bead_id: None,
                priority: 0,
                status: QueueStatus::Pending,
                added_at: 1_700_000_000 + level as i64,
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
                parent_workspace: Some(current_parent.clone()),
                stack_depth: level as i32,
                dependents: if level < depth {
                    Dependents::from_vec(vec![next_level])
                } else {
                    Dependents::new()
                },
                stack_root: Some(root.clone()),
                stack_merge_state: StackMergeState::Independent,
            });
            current_parent = level_name;
        }

        // Verify all invariants

        // 1. No cycles
        for entry in &entries {
            if let Err(StackError::CycleDetected { .. }) =
                calculate_stack_depth(&entry.workspace, &entries)
            {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    "Cycle detected in valid linear stack".into()
                ));
            }
        }

        // 2. Finite depth (all depths should be <= our max depth)
        for entry in &entries {
            match calculate_stack_depth(&entry.workspace, &entries) {
                Ok(d) => {
                    if d > depth {
                        return Err(proptest::test_runner::TestCaseError::Fail(
                            format!("Depth {} exceeds expected max {}", d, depth).into()
                        ));
                    }
                }
                Err(StackError::ParentNotFound { .. }) => {}
                Err(e) => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("Unexpected error: {:?}", e).into()
                    ));
                }
            }
        }

        // 3. All entries share the same root
        for entry in &entries {
            match find_stack_root(&entry.workspace, &entries) {
                Ok(found_root) => {
                    if found_root != root {
                        return Err(proptest::test_runner::TestCaseError::Fail(
                            format!("Found root {} instead of {}", found_root, root).into()
                        ));
                    }
                }
                Err(StackError::ParentNotFound { .. }) => {}
                Err(e) => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("Failed to find root: {:?}", e).into()
                    ));
                }
            }
        }

        // 4. Parent-child depth consistency
        for entry in &entries {
            if let Some(ref parent_name) = entry.parent_workspace {
                let parent_depth = calculate_stack_depth(parent_name, &entries);
                let child_depth = calculate_stack_depth(&entry.workspace, &entries);

                if let (Ok(p), Ok(c)) = (parent_depth, child_depth) {
                    if c != p.saturating_add(1) {
                        return Err(proptest::test_runner::TestCaseError::Fail(
                            format!(
                                "Depth inconsistency: child {} has depth {}, parent {} has depth {}",
                                entry.workspace, c, parent_name, p
                            )
                            .into()
                        ));
                    }
                }
            }
        }
    }
}

//! Stack Depth Calculation (Pure Domain Logic)
//!
//! Pure functions for calculating stack depth from parent chains.
//! These functions have no side effects and are fully deterministic.

use std::collections::{HashSet, VecDeque};

use super::{queue_entities::QueueEntry, stack_error::StackError};

/// Validate that setting a parent won't create a cycle in the stack.
///
/// This function checks if assigning `parent` as the parent of `workspace`
/// would create a cycle in the dependency chain.
///
/// # Arguments
///
/// * `workspace` - The name of the workspace that would receive a new parent
/// * `parent` - The name of the workspace that would become the parent
/// * `entries` - A slice of queue entries representing the current state
///
/// # Returns
///
/// * `Ok(())` - Setting this parent would NOT create a cycle
/// * `Err(StackError::CycleDetected)` - Setting this parent WOULD create a cycle
///
/// # Errors
///
/// Returns `StackError::CycleDetected` if:
/// - `workspace` equals `parent` (self-reference)
/// - `parent` is a descendant of `workspace` (would create indirect cycle)
///
/// # Example
///
/// ```ignore
/// use zjj_core::coordination::stack_depth::validate_no_cycle;
///
/// let entries = vec![
///     create_entry("root", None),
///     create_entry("child", Some("root")),
/// ];
///
/// // Valid: new-workspace can set parent to child
/// assert!(validate_no_cycle("new-workspace", "child", &entries).is_ok());
///
/// // Invalid: root cannot set parent to child (would create cycle)
/// assert!(validate_no_cycle("root", "child", &entries).is_err());
/// ```
pub fn validate_no_cycle(
    workspace: &str,
    parent: &str,
    entries: &[QueueEntry],
) -> Result<(), StackError> {
    if workspace == parent {
        return Err(StackError::CycleDetected {
            workspace: workspace.to_string(),
            cycle_path: vec![workspace.to_string()],
        });
    }

    if is_ancestor_of(workspace, parent, entries) {
        return Err(StackError::CycleDetected {
            workspace: workspace.to_string(),
            cycle_path: build_cycle_path_from_parent(parent, entries),
        });
    }

    Ok(())
}

/// Check if `workspace` is an ancestor of `potential_descendant`.
///
/// An ancestor is any workspace reachable by following parent relationships
/// upward from `potential_descendant`.
fn is_ancestor_of(workspace: &str, potential_descendant: &str, entries: &[QueueEntry]) -> bool {
    let mut current = potential_descendant;
    let mut visited = HashSet::<&str>::new();

    while !visited.contains(current) {
        visited.insert(current);

        let Some(entry) = entries.iter().find(|e| e.workspace == current) else {
            return false;
        };

        match &entry.parent_workspace {
            None => return false,
            Some(p) if p == workspace => return true,
            Some(p) => current = p.as_str(),
        }
    }

    false
}

/// Build the cycle path starting from a parent workspace.
fn build_cycle_path_from_parent(start: &str, entries: &[QueueEntry]) -> Vec<String> {
    let mut path = vec![start.to_string()];
    let mut current = start;
    let mut visited = HashSet::<&str>::new();

    while !visited.contains(current) {
        visited.insert(current);

        if let Some(entry) = entries.iter().find(|e| e.workspace == current) {
            if let Some(parent) = &entry.parent_workspace {
                path.push(parent.clone());
                current = parent.as_str();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    path
}

/// Find the root workspace of a stack.
///
/// The root is the workspace in the chain that has no parent.
/// If the given workspace has no parent, it is itself the root.
///
/// # Arguments
///
/// * `workspace` - The name of the workspace to find the root for
/// * `entries` - A slice of queue entries to search for parent relationships
///
/// # Returns
///
/// * `Ok(String)` - The name of the root workspace
/// * `Err(StackError::CycleDetected)` - If a cycle is detected in the parent chain
/// * `Err(StackError::ParentNotFound)` - If a parent workspace doesn't exist in entries
///
/// # Errors
///
/// Returns `StackError::CycleDetected` if the workspace references itself
/// or there's a cycle in the parent chain.
///
/// Returns `StackError::ParentNotFound` if a workspace in the chain
/// references a parent that doesn't exist in the provided entries slice.
///
/// # Example
///
/// ```ignore
/// use zjj_core::coordination::stack_depth::find_stack_root;
///
/// let entries = vec![
///     create_entry("root", None),
///     create_entry("child", Some("root")),
/// ];
///
/// assert_eq!(find_stack_root("root", &entries), Ok("root".to_string()));
/// assert_eq!(find_stack_root("child", &entries), Ok("root".to_string()));
/// ```
pub fn find_stack_root(workspace: &str, entries: &[QueueEntry]) -> Result<String, StackError> {
    let mut current = workspace;
    let mut visited = HashSet::<&str>::new();

    loop {
        if visited.contains(current) {
            return Err(StackError::CycleDetected {
                workspace: workspace.to_string(),
                cycle_path: build_cycle_path(current, entries),
            });
        }
        visited.insert(current);

        let entry = entries.iter().find(|e| e.workspace == current);

        match entry {
            None => {
                return Err(StackError::ParentNotFound {
                    parent_workspace: current.to_string(),
                });
            }
            Some(e) => match &e.parent_workspace {
                None => return Ok(current.to_string()),
                Some(parent) => {
                    if parent == current {
                        return Err(StackError::CycleDetected {
                            workspace: workspace.to_string(),
                            cycle_path: build_cycle_path(current, entries),
                        });
                    }
                    current = parent.as_str();
                }
            },
        }
    }
}

/// Calculate the depth of a workspace in a stack hierarchy.
///
/// The depth is the number of ancestors (parent, grandparent, etc.) from
/// the given workspace to the root (a workspace with no parent).
///
/// # Arguments
///
/// * `workspace` - The name of the workspace to calculate depth for
/// * `entries` - A slice of queue entries to search for parent relationships
///
/// # Returns
///
/// * `Ok(u32)` - The depth (0 = root/no parent, 1 = first child, etc.)
/// * `Err(StackError::CycleDetected)` - If a cycle is detected in the parent chain
/// * `Err(StackError::ParentNotFound)` - If a parent workspace doesn't exist in entries
///
/// # Errors
///
/// Returns `StackError::CycleDetected` if the workspace references itself
/// or there's a cycle in the parent chain.
///
/// Returns `StackError::ParentNotFound` if the workspace has a parent that
/// doesn't exist in the provided entries slice.
///
/// # Example
///
/// ```ignore
/// use zjj_core::coordination::stack_depth::calculate_stack_depth;
///
/// let entries = vec![
///     create_entry("root", None),
///     create_entry("child", Some("root")),
/// ];
///
/// assert_eq!(calculate_stack_depth("root", &entries), Ok(0));
/// assert_eq!(calculate_stack_depth("child", &entries), Ok(1));
/// ```
pub fn calculate_stack_depth(workspace: &str, entries: &[QueueEntry]) -> Result<u32, StackError> {
    let mut current = workspace;
    let mut depth = 0u32;
    let mut visited = HashSet::<&str>::new();

    loop {
        if visited.contains(current) {
            return Err(StackError::CycleDetected {
                workspace: workspace.to_string(),
                cycle_path: build_cycle_path(current, entries),
            });
        }
        visited.insert(current);

        let entry = entries.iter().find(|e| e.workspace == current);

        match entry {
            None => {
                if current == workspace {
                    return Err(StackError::ParentNotFound {
                        parent_workspace: workspace.to_string(),
                    });
                }
                return Err(StackError::ParentNotFound {
                    parent_workspace: current.to_string(),
                });
            }
            Some(e) => match &e.parent_workspace {
                None => return Ok(depth),
                Some(parent) => {
                    if parent == current {
                        return Err(StackError::CycleDetected {
                            workspace: workspace.to_string(),
                            cycle_path: build_cycle_path(current, entries),
                        });
                    }
                    depth = depth.saturating_add(1);
                    current = parent.as_str();
                }
            },
        }
    }
}

/// Build the cycle path for error reporting.
fn build_cycle_path(start: &str, entries: &[QueueEntry]) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = start;
    let mut visited = HashSet::<&str>::new();

    while !visited.contains(current) {
        visited.insert(current);
        path.push(current.to_string());

        if let Some(entry) = entries.iter().find(|e| e.workspace == current) {
            if let Some(parent) = &entry.parent_workspace {
                current = parent.as_str();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if visited.contains(current) {
        path.push(current.to_string());
    }

    path
}

/// Build a list of all dependent workspaces (descendants) of a given workspace.
///
/// This function performs a breadth-first search to find all workspaces that
/// have the given workspace as an ancestor (direct or indirect children).
///
/// # Arguments
///
/// * `workspace` - The name of the workspace to find dependents for
/// * `entries` - A slice of queue entries to search for parent relationships
///
/// # Returns
///
/// A `Vec<String>` containing all descendant workspace names in breadth-first order.
/// Returns an empty `Vec` if:
/// - The workspace has no children
/// - The workspace doesn't exist in the entries slice
/// - The entries slice is empty
///
/// # Example
///
/// ```ignore
/// use zjj_core::coordination::stack_depth::build_dependent_list;
///
/// let entries = vec![
///     create_entry("root", None),
///     create_entry("child1", Some("root")),
///     create_entry("child2", Some("root")),
///     create_entry("grandchild", Some("child1")),
/// ];
///
/// let dependents = build_dependent_list("root", &entries);
/// // Returns ["child1", "child2", "grandchild"] in BFS order
/// assert!(dependents.contains(&"child1".to_string()));
/// assert!(dependents.contains(&"child2".to_string()));
/// assert!(dependents.contains(&"grandchild".to_string()));
/// ```
#[must_use]
pub fn build_dependent_list(workspace: &str, entries: &[QueueEntry]) -> Vec<String> {
    let mut result = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back(workspace.to_string());
    visited.insert(workspace.to_string());

    while let Some(current) = queue.pop_front() {
        let children: Vec<_> = entries
            .iter()
            .filter(|e| e.parent_workspace.as_ref().is_some_and(|p| p == &current))
            .map(|e| e.workspace.clone())
            .collect();

        for child in children {
            if !visited.contains(&child) {
                visited.insert(child.clone());
                result.push(child.clone());
                queue.push_back(child);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        use super::super::{
            queue_entities::Dependents,
            queue_status::{QueueStatus, StackMergeState, WorkspaceQueueState},
        };

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

    #[test]
    fn test_no_parent_returns_zero() {
        let entries = vec![create_entry("root", None)];
        assert_eq!(calculate_stack_depth("root", &entries), Ok(0));
    }

    #[test]
    fn test_one_parent_returns_one() {
        let entries = vec![
            create_entry("root", None),
            create_entry("child", Some("root")),
        ];
        assert_eq!(calculate_stack_depth("child", &entries), Ok(1));
    }

    #[test]
    fn test_chain_of_three_returns_three() {
        let entries = vec![
            create_entry("root", None),
            create_entry("child", Some("root")),
            create_entry("mid", Some("child")),
            create_entry("leaf", Some("mid")),
        ];
        assert_eq!(calculate_stack_depth("leaf", &entries), Ok(3));
    }

    #[test]
    fn test_self_reference_returns_cycle_error() {
        let entries = vec![create_entry("loop", Some("loop"))];
        assert!(matches!(
            calculate_stack_depth("loop", &entries),
            Err(StackError::CycleDetected { .. })
        ));
    }

    #[test]
    fn test_missing_parent_returns_error() {
        let entries = vec![create_entry("orphan", Some("missing"))];
        assert!(matches!(
            calculate_stack_depth("orphan", &entries),
            Err(StackError::ParentNotFound { .. })
        ));
    }
}

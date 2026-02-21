//! Stack Depth Calculation (Pure Domain Logic)
//!
//! Pure functions for calculating stack depth from parent chains.
//! These functions have no side effects and are fully deterministic.

use std::collections::HashSet;

use super::{queue_entities::QueueEntry, stack_error::StackError};

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

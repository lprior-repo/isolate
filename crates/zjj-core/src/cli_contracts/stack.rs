//! KIRK Contracts for Stack CLI operations.
//!
//! Stack manages stacked sessions (sessions on top of other sessions).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::cli_contracts::{Contract, ContractError, Invariant, Postcondition, Precondition};

// ═══════════════════════════════════════════════════════════════════════════
// STACK INPUT/OUTPUT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Input for pushing a new session onto the stack.
#[derive(Debug, Clone)]
pub struct PushInput {
    /// Session name
    pub name: String,
    /// Parent session (defaults to current)
    pub parent: Option<String>,
}

/// Input for popping from the stack.
#[derive(Debug, Clone)]
pub struct PopInput {
    /// Session to pop (defaults to current)
    pub session: Option<String>,
    /// Force pop even with uncommitted changes
    pub force: bool,
}

/// Input for listing the stack.
#[derive(Debug, Clone, Default)]
pub struct ListStackInput {
    /// Root session to start from
    pub root: Option<String>,
}

/// Input for syncing the stack.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SyncStackInput {
    /// Session to sync (defaults to current stack)
    pub session: Option<String>,
    /// Rebase strategy
    pub rebase: bool,
}

/// Result of stack operations.
#[derive(Debug, Clone)]
pub struct StackResult {
    /// Session name
    pub name: String,
    /// Parent session (if stacked)
    pub parent: Option<String>,
    /// Depth in stack (0 = root)
    pub depth: u32,
    /// Number of children
    pub children: u32,
}

/// Result of stack listing.
#[derive(Debug, Clone)]
pub struct StackListResult {
    /// Stack entries (ordered from root to leaf)
    pub entries: Vec<StackResult>,
    /// Current session
    pub current: Option<String>,
    /// Maximum depth
    pub max_depth: u32,
}

// ═══════════════════════════════════════════════════════════════════════════
// STACK CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Contracts for Stack CLI operations.
pub struct StackContracts;

impl StackContracts {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PRECONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Precondition: parent session exists.
    pub const PRECOND_PARENT_EXISTS: Precondition =
        Precondition::new("parent_exists", "Parent session must exist");

    /// Precondition: parent session is not completed/failed.
    pub const PRECOND_PARENT_ACTIVE: Precondition = Precondition::new(
        "parent_active",
        "Parent session must be in active or paused state",
    );

    /// Precondition: max stack depth not exceeded.
    pub const PRECOND_MAX_DEPTH: Precondition = Precondition::new(
        "max_depth",
        "Stack depth must not exceed configured maximum",
    );

    /// Precondition: session has no children (for pop).
    pub const PRECOND_NO_CHILDREN: Precondition =
        Precondition::new("no_children", "Session must have no children to pop");

    /// Precondition: session is stacked (has parent).
    pub const PRECOND_IS_STACKED: Precondition =
        Precondition::new("is_stacked", "Session must be stacked (have a parent)");

    /// Precondition: no uncommitted changes (for pop without force).
    pub const PRECOND_NO_UNCOMMITTED: Precondition = Precondition::new(
        "no_uncommitted_stack",
        "Session must have no uncommitted changes (or use --force)",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INVARIANTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Invariant: no cycles in stack.
    pub const INV_NO_CYCLES: Invariant =
        Invariant::documented("no_cycles", "Stack must not contain cycles");

    /// Invariant: parent always exists.
    pub const INV_PARENT_EXISTS: Invariant = Invariant::documented(
        "parent_exists_inv",
        "Parent session always exists for stacked sessions",
    );

    /// Invariant: depth is consistent.
    pub const INV_DEPTH_CONSISTENT: Invariant =
        Invariant::documented("depth_consistent", "Depth = parent's depth + 1");

    /// Invariant: root has no parent.
    pub const INV_ROOT_NO_PARENT: Invariant =
        Invariant::documented("root_no_parent", "Root sessions have no parent");

    /// Invariant: children count is accurate.
    pub const INV_CHILDREN_COUNT: Invariant = Invariant::documented(
        "children_count",
        "Children count matches actual child sessions",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // POSTCONDITIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Postcondition: session was pushed onto stack.
    pub const POST_PUSHED: Postcondition =
        Postcondition::new("pushed", "Session is on stack with correct parent");

    /// Postcondition: session was popped from stack.
    pub const POST_POPPED: Postcondition =
        Postcondition::new("popped", "Session is removed from stack");

    /// Postcondition: parent's children count updated.
    pub const POST_CHILDREN_UPDATED: Postcondition = Postcondition::new(
        "children_updated",
        "Parent's children count reflects the change",
    );

    /// Postcondition: stack order preserved.
    pub const POST_ORDER_PRESERVED: Postcondition = Postcondition::new(
        "order_preserved",
        "Stack order is preserved after operation",
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Validate a stack depth.
    ///
    /// # Errors
    /// Returns `ContractError` if the depth is invalid.
    pub fn validate_depth(depth: u32, max_depth: u32) -> Result<(), ContractError> {
        if depth > max_depth {
            return Err(ContractError::invalid_input(
                "depth",
                format!("cannot exceed maximum depth of {max_depth}"),
            ));
        }
        Ok(())
    }

    /// Check if adding a child would create a cycle.
    ///
    /// # Arguments
    /// * `parent` - The prospective parent session
    /// * `ancestors` - All ancestors of the parent (including parent itself)
    ///
    /// # Errors
    /// Returns `ContractError` if adding would create a cycle.
    pub fn check_no_cycle(child: &str, ancestors: &[String]) -> Result<(), ContractError> {
        if ancestors.iter().any(|a| a == child) {
            return Err(ContractError::ConcurrentModification {
                description: format!("Adding '{child}' as child would create a cycle"),
            });
        }
        Ok(())
    }
}

impl Contract<PushInput, StackResult> for StackContracts {
    fn preconditions(input: &PushInput) -> Result<(), ContractError> {
        if input.name.trim().is_empty() {
            return Err(ContractError::invalid_input("name", "cannot be empty"));
        }

        if let Some(ref parent) = input.parent {
            if parent.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "parent",
                    "cannot be empty if provided",
                ));
            }
        }

        Ok(())
    }

    fn invariants(_input: &PushInput) -> Vec<Invariant> {
        vec![
            Self::INV_NO_CYCLES,
            Self::INV_PARENT_EXISTS,
            Self::INV_DEPTH_CONSISTENT,
        ]
    }

    fn postconditions(input: &PushInput, result: &StackResult) -> Result<(), ContractError> {
        if result.name != input.name {
            return Err(ContractError::PostconditionFailed {
                name: "name_matches",
                description: "Result name must match input",
            });
        }
        // If no parent specified, result should have a parent (the current session)
        // If parent specified, result parent should match
        match (&input.parent, &result.parent) {
            (Some(specified), Some(result_parent)) if specified != result_parent => {
                return Err(ContractError::PostconditionFailed {
                    name: "parent_matches",
                    description: "Result parent must match specified parent",
                });
            }
            _ => {}
        }
        if result.depth == 0 && result.parent.is_some() {
            return Err(ContractError::PostconditionFailed {
                name: "depth_consistency",
                description: "Root sessions (depth 0) should have no parent",
            });
        }
        Ok(())
    }
}

impl Contract<PopInput, ()> for StackContracts {
    fn preconditions(input: &PopInput) -> Result<(), ContractError> {
        // Session is optional (defaults to current)
        if let Some(ref session) = input.session {
            if session.trim().is_empty() {
                return Err(ContractError::invalid_input(
                    "session",
                    "cannot be empty if provided",
                ));
            }
        }
        Ok(())
    }

    fn invariants(_input: &PopInput) -> Vec<Invariant> {
        vec![Self::INV_NO_CYCLES, Self::INV_CHILDREN_COUNT]
    }

    fn postconditions(_input: &PopInput, _result: &()) -> Result<(), ContractError> {
        Ok(())
    }
}

impl Contract<ListStackInput, StackListResult> for StackContracts {
    fn preconditions(_input: &ListStackInput) -> Result<(), ContractError> {
        Ok(())
    }

    fn invariants(_input: &ListStackInput) -> Vec<Invariant> {
        vec![
            Self::INV_NO_CYCLES,
            Self::INV_DEPTH_CONSISTENT,
            Self::INV_CHILDREN_COUNT,
        ]
    }

    fn postconditions(
        _input: &ListStackInput,
        result: &StackListResult,
    ) -> Result<(), ContractError> {
        // Verify depths are consistent (0, 1, 2, ...)
        for (idx, entry) in result.entries.iter().enumerate() {
            let expected_depth =
                u32::try_from(idx).map_err(|_| ContractError::PostconditionFailed {
                    name: "depth_overflow",
                    description: "Too many entries for depth numbering",
                })?;
            if entry.depth != expected_depth {
                return Err(ContractError::PostconditionFailed {
                    name: "depth_ordering",
                    description: "Depths must be consecutive starting from 0",
                });
            }
        }

        // Verify children counts are non-negative
        for entry in &result.entries {
            // Children count is always valid as u32 >= 0
            let _ = entry.children;
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_depth_valid() {
        assert!(StackContracts::validate_depth(0, 10).is_ok());
        assert!(StackContracts::validate_depth(5, 10).is_ok());
        assert!(StackContracts::validate_depth(10, 10).is_ok());
    }

    #[test]
    fn test_validate_depth_invalid() {
        assert!(StackContracts::validate_depth(11, 10).is_err());
    }

    #[test]
    fn test_check_no_cycle_no_cycle() {
        let ancestors = vec!["root".to_string(), "parent".to_string()];
        assert!(StackContracts::check_no_cycle("child", &ancestors).is_ok());
    }

    #[test]
    fn test_check_no_cycle_with_cycle() {
        let ancestors = vec!["root".to_string(), "child".to_string()];
        assert!(StackContracts::check_no_cycle("child", &ancestors).is_err());
    }

    #[test]
    fn test_push_contract_preconditions() {
        let input = PushInput {
            name: "feature".to_string(),
            parent: Some("main".to_string()),
        };
        assert!(StackContracts::preconditions(&input).is_ok());
    }

    #[test]
    fn test_push_contract_preconditions_empty_name() {
        let input = PushInput {
            name: String::new(),
            parent: None,
        };
        assert!(StackContracts::preconditions(&input).is_err());
    }

    #[test]
    fn test_push_contract_postconditions() {
        let input = PushInput {
            name: "feature".to_string(),
            parent: Some("main".to_string()),
        };
        let result = StackResult {
            name: "feature".to_string(),
            parent: Some("main".to_string()),
            depth: 1,
            children: 0,
        };
        assert!(StackContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_push_contract_postconditions_wrong_parent() {
        let input = PushInput {
            name: "feature".to_string(),
            parent: Some("main".to_string()),
        };
        let result = StackResult {
            name: "feature".to_string(),
            parent: Some("other".to_string()),
            depth: 1,
            children: 0,
        };
        assert!(StackContracts::postconditions(&input, &result).is_err());
    }

    #[test]
    fn test_list_stack_contract_postconditions_depths() {
        let input = ListStackInput::default();
        let result = StackListResult {
            entries: vec![
                StackResult {
                    name: "root".to_string(),
                    parent: None,
                    depth: 0,
                    children: 1,
                },
                StackResult {
                    name: "child".to_string(),
                    parent: Some("root".to_string()),
                    depth: 1,
                    children: 0,
                },
            ],
            current: Some("child".to_string()),
            max_depth: 10,
        };
        assert!(StackContracts::postconditions(&input, &result).is_ok());
    }

    #[test]
    fn test_list_stack_contract_postconditions_depths_wrong() {
        let input = ListStackInput::default();
        let result = StackListResult {
            entries: vec![
                StackResult {
                    name: "root".to_string(),
                    parent: None,
                    depth: 0,
                    children: 1,
                },
                StackResult {
                    name: "child".to_string(),
                    parent: Some("root".to_string()),
                    depth: 2, // Wrong! Should be 1
                    children: 0,
                },
            ],
            current: Some("child".to_string()),
            max_depth: 10,
        };
        assert!(StackContracts::postconditions(&input, &result).is_err());
    }
}

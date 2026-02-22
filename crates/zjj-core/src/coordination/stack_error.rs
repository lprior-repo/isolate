//! Stack Error Types (Pure Domain Logic)
//!
//! Error types for stack-related operations including:
//! - Cycle detection in workspace dependency graphs
//! - Parent workspace validation
//! - Stack depth limits

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for stack operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum StackError {
    /// A cycle was detected in the workspace dependency graph.
    #[error("Cycle detected in stack: workspace '{workspace}' is part of cycle {cycle_path:?}")]
    CycleDetected {
        /// The workspace where the cycle was detected.
        workspace: String,
        /// The path of workspaces forming the cycle.
        cycle_path: Vec<String>,
    },

    /// The specified parent workspace could not be found.
    #[error("Parent workspace '{parent_workspace}' not found")]
    ParentNotFound {
        /// The name of the missing parent workspace.
        parent_workspace: String,
    },

    /// The stack depth exceeds the maximum allowed.
    #[error("Stack depth {current_depth} exceeds maximum allowed depth {max_depth}")]
    DepthExceeded {
        /// The current stack depth.
        current_depth: u32,
        /// The maximum allowed stack depth.
        max_depth: u32,
    },

    /// The parent workspace exists but is in an invalid state.
    #[error("Invalid parent workspace '{workspace}': {reason}")]
    InvalidParent {
        /// The workspace with the invalid parent.
        workspace: String,
        /// The reason the parent is invalid.
        reason: String,
    },
}

impl StackError {
    /// Returns the workspace name for the error.
    #[must_use]
    pub fn workspace(&self) -> &str {
        match self {
            Self::CycleDetected { workspace, .. } | Self::InvalidParent { workspace, .. } => {
                workspace
            }
            Self::ParentNotFound {
                parent_workspace, ..
            } => parent_workspace,
            Self::DepthExceeded { .. } => "unknown",
        }
    }

    /// Returns the cycle path for `CycleDetected` errors.
    #[must_use]
    pub fn cycle_path(&self) -> Vec<String> {
        match self {
            Self::CycleDetected { cycle_path, .. } => cycle_path.clone(),
            Self::ParentNotFound { .. }
            | Self::DepthExceeded { .. }
            | Self::InvalidParent { .. } => Vec::new(),
        }
    }

    /// Returns the parent workspace name for `ParentNotFound` errors.
    #[must_use]
    pub fn parent_workspace(&self) -> &str {
        match self {
            Self::ParentNotFound {
                parent_workspace, ..
            } => parent_workspace,
            Self::CycleDetected { .. }
            | Self::DepthExceeded { .. }
            | Self::InvalidParent { .. } => "",
        }
    }

    /// Returns the current depth for `DepthExceeded` errors.
    #[must_use]
    pub const fn current_depth(&self) -> u32 {
        match self {
            Self::DepthExceeded { current_depth, .. } => *current_depth,
            Self::CycleDetected { .. }
            | Self::ParentNotFound { .. }
            | Self::InvalidParent { .. } => 0,
        }
    }

    /// Returns the max depth for `DepthExceeded` errors.
    #[must_use]
    pub const fn max_depth(&self) -> u32 {
        match self {
            Self::DepthExceeded { max_depth, .. } => *max_depth,
            Self::CycleDetected { .. }
            | Self::ParentNotFound { .. }
            | Self::InvalidParent { .. } => 0,
        }
    }

    /// Returns the reason for `InvalidParent` errors.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::InvalidParent { reason, .. } => reason,
            Self::CycleDetected { .. }
            | Self::ParentNotFound { .. }
            | Self::DepthExceeded { .. } => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_cycle_detected_creation() {
        let error = StackError::CycleDetected {
            workspace: "feature-auth".to_string(),
            cycle_path: vec![
                "feature-auth".to_string(),
                "feature-db".to_string(),
                "feature-auth".to_string(),
            ],
        };

        assert_eq!(error.workspace(), "feature-auth");
        assert_eq!(error.cycle_path().len(), 3);
    }

    #[test]
    fn test_parent_not_found_creation() {
        let error = StackError::ParentNotFound {
            parent_workspace: "feature-base".to_string(),
        };

        assert_eq!(error.parent_workspace(), "feature-base");
    }

    #[test]
    fn test_depth_exceeded_creation() {
        let error = StackError::DepthExceeded {
            current_depth: 15,
            max_depth: 10,
        };

        assert_eq!(error.current_depth(), 15);
        assert_eq!(error.max_depth(), 10);
    }

    #[test]
    fn test_invalid_parent_creation() {
        let error = StackError::InvalidParent {
            workspace: "feature-auth".to_string(),
            reason: "parent workspace is in 'conflict' state".to_string(),
        };

        assert_eq!(error.workspace(), "feature-auth");
        assert_eq!(error.reason(), "parent workspace is in 'conflict' state");
    }

    #[test]
    fn test_implements_std_error() {
        let error = StackError::CycleDetected {
            workspace: "test".to_string(),
            cycle_path: vec!["test".to_string()],
        };

        let _: &dyn Error = &error;
    }

    #[test]
    fn test_display_cycle_detected() {
        let error = StackError::CycleDetected {
            workspace: "feature-auth".to_string(),
            cycle_path: vec![
                "feature-auth".to_string(),
                "feature-db".to_string(),
                "feature-auth".to_string(),
            ],
        };

        let message = format!("{error}");
        assert!(message.to_lowercase().contains("cycle"));
        assert!(message.contains("feature-auth"));
    }

    #[test]
    fn test_display_parent_not_found() {
        let error = StackError::ParentNotFound {
            parent_workspace: "feature-base".to_string(),
        };

        let message = format!("{error}");
        assert!(message.to_lowercase().contains("parent"));
        assert!(message.contains("feature-base"));
    }

    #[test]
    fn test_display_depth_exceeded() {
        let error = StackError::DepthExceeded {
            current_depth: 15,
            max_depth: 10,
        };

        let message = format!("{error}");
        assert!(message.contains("15"));
        assert!(message.contains("10"));
    }

    #[test]
    fn test_display_invalid_parent() {
        let error = StackError::InvalidParent {
            workspace: "feature-auth".to_string(),
            reason: "conflict state".to_string(),
        };

        let message = format!("{error}");
        assert!(message.to_lowercase().contains("parent"));
        assert!(message.contains("feature-auth"));
    }

    #[test]
    fn test_clone() {
        let error = StackError::DepthExceeded {
            current_depth: 5,
            max_depth: 3,
        };

        let cloned = error.clone();
        assert_eq!(format!("{error}"), format!("{cloned}"));
    }

    #[test]
    fn test_partial_eq() {
        let error1 = StackError::DepthExceeded {
            current_depth: 5,
            max_depth: 3,
        };

        let error2 = StackError::DepthExceeded {
            current_depth: 5,
            max_depth: 3,
        };

        assert_eq!(error1, error2);

        let error3 = StackError::DepthExceeded {
            current_depth: 6,
            max_depth: 3,
        };

        assert_ne!(error1, error3);
    }
}

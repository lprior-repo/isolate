//! JJ workspace lifecycle management
//!
//! This module provides safe, functional APIs for managing JJ workspaces.
//! All operations return `Result` and never panic.

use std::io::ErrorKind;

use crate::error::system::SystemError;

pub mod check;
pub mod parse;
pub mod types;
pub mod version;
pub mod workspace;

// Re-export public API
pub use check::{check_in_jj_repo, check_jj_installed, has_uncommitted_changes};
pub use types::{DiffSummary, Status, WorkspaceInfo};
pub use version::{check_jj_version_compatible, get_jj_version, JjVersion};
pub use workspace::{
    workspace_create, workspace_create_at_revision, workspace_diff, workspace_forget,
    workspace_git_push, workspace_list, workspace_rebase_onto_main, workspace_squash,
    workspace_status,
};

/// Helper to create a JJ command error with appropriate context
fn jj_command_error(operation: &str, error: &std::io::Error) -> crate::Error {
    let is_not_found = error.kind() == ErrorKind::NotFound;
    crate::Error::System(SystemError::JjCommandError {
        operation: operation.to_string(),
        source: error.to_string(),
        is_not_found,
    })
}

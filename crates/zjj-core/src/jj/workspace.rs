//! JJ workspace lifecycle operations

use std::path::Path;
use std::process::Command;

use crate::{Error, Result};

use super::parse::{parse_diff_stat, parse_status, parse_workspace_list};
use super::types::{DiffSummary, Status};

/// Create a new JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Unable to create workspace directory
/// - JJ command fails
pub fn workspace_create(name: &str, path: &Path) -> Result<()> {
    // Validate inputs
    if name.is_empty() {
        return Err(Error::invalid_config(
            "workspace name cannot be empty".into(),
        ));
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io_error(format!("Failed to create workspace directory: {e}")))?;
    }

    // Execute: jj workspace add --name <name> <path>
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .output()
        .map_err(|e| super::jj_command_error("create workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "create workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// Forget (remove) a JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace doesn't exist
/// - JJ command fails
pub fn workspace_forget(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_config(
            "workspace name cannot be empty".into(),
        ));
    }

    // Execute: jj workspace forget <name>
    let output = Command::new("jj")
        .args(["workspace", "forget", name])
        .output()
        .map_err(|e| super::jj_command_error("forget workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "forget workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// List all JJ workspaces
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_list() -> Result<Vec<super::types::WorkspaceInfo>> {
    // Execute: jj workspace list
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .output()
        .map_err(|e| super::jj_command_error("list workspaces", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "list workspaces".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_workspace_list(&stdout)
}

/// Get status of a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_status(path: &Path) -> Result<Status> {
    // Execute: jj status (in the workspace directory)
    let output = Command::new("jj")
        .args(["status"])
        .current_dir(path)
        .output()
        .map_err(|e| super::jj_command_error("get workspace status", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get workspace status".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_status(&stdout))
}

/// Get diff summary for a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_diff(path: &Path) -> Result<DiffSummary> {
    // Execute: jj diff --stat (in the workspace directory)
    let output = Command::new("jj")
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .map_err(|e| super::jj_command_error("get workspace diff", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get workspace diff".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_stat(&stdout))
}

/// Squash commits in a workspace into a single commit
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - No changes to squash
/// - JJ command fails
pub fn workspace_squash(workspace_path: &Path) -> Result<()> {
    // Execute: jj squash (in the workspace directory)
    let output = Command::new("jj")
        .args(["squash"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("squash commits", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "squash commits".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// Rebase workspace onto main branch
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Main branch doesn't exist
/// - Rebase conflicts occur
/// - JJ command fails
pub fn workspace_rebase_onto_main(workspace_path: &Path, main_branch: &str) -> Result<()> {
    if main_branch.is_empty() {
        return Err(Error::invalid_config(
            "main branch name cannot be empty".into(),
        ));
    }

    // Execute: jj rebase -d <main_branch> (in the workspace directory)
    let output = Command::new("jj")
        .args(["rebase", "-d", main_branch])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("rebase onto main", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("rebase onto {main_branch}"),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// Push changes from workspace to git remote
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - No git remote configured
/// - Push fails (e.g., network error, authentication)
/// - JJ command fails
pub fn workspace_git_push(workspace_path: &Path) -> Result<()> {
    // Execute: jj git push (in the workspace directory)
    let output = Command::new("jj")
        .args(["git", "push"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("git push", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "git push".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

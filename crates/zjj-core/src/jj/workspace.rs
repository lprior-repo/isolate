//! JJ workspace lifecycle operations

use std::path::Path;
use std::process::Command;

use crate::{error::system::SystemError, Error, Result};

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
        return Err(Error::invalid_config("workspace name cannot be empty"));
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
        }));
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
        return Err(Error::invalid_config("workspace name cannot be empty"));
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
        }));
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
        }));
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
        }));
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
        }));
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
        }));
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
        return Err(Error::invalid_config("main branch name cannot be empty"));
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
        }));
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
        }));
    }

    Ok(())
}

/// Create a new JJ workspace at a specific revision
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Revision does not exist
/// - JJ command fails
pub fn workspace_create_at_revision(name: &str, path: &Path, revision: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::invalid_config("workspace name cannot be empty"));
    }
    if revision.is_empty() {
        return Err(Error::invalid_config("revision cannot be empty"));
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io_error(format!("Failed to create workspace directory: {e}")))?;
    }

    // Execute: jj workspace add --name <name> -r <revision> <path>
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", name, "-r", revision])
        .arg(path)
        .output()
        .map_err(|e| super::jj_command_error("create workspace at revision", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("create workspace at revision {revision}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Describe (annotate) the current revision with a message
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - JJ command fails
pub fn workspace_describe(workspace_path: &Path, message: &str) -> Result<()> {
    if message.is_empty() {
        return Err(Error::invalid_config("description message cannot be empty"));
    }

    // Execute: jj describe -m <message>
    let output = Command::new("jj")
        .args(["describe", "-m", message])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("describe revision", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "describe revision".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Fetch from git remote
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - No git remote configured
/// - Network error
/// - JJ command fails
pub fn workspace_git_fetch(workspace_path: &Path) -> Result<()> {
    // Execute: jj git fetch
    let output = Command::new("jj")
        .args(["git", "fetch"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("git fetch", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "git fetch".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Create a new bookmark at current revision
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Bookmark name is invalid
/// - JJ command fails
pub fn workspace_bookmark_create(workspace_path: &Path, bookmark_name: &str) -> Result<()> {
    if bookmark_name.is_empty() {
        return Err(Error::invalid_config("bookmark name cannot be empty"));
    }

    // Execute: jj bookmark create <name>
    let output = Command::new("jj")
        .args(["bookmark", "create", bookmark_name])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("create bookmark", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("create bookmark {bookmark_name}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Set (move) a bookmark to a specific revision
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Bookmark or revision doesn't exist
/// - JJ command fails
pub fn workspace_bookmark_set(
    workspace_path: &Path,
    bookmark_name: &str,
    revision: &str,
) -> Result<()> {
    if bookmark_name.is_empty() {
        return Err(Error::invalid_config("bookmark name cannot be empty"));
    }
    if revision.is_empty() {
        return Err(Error::invalid_config("revision cannot be empty"));
    }

    // Execute: jj bookmark set <name> -r <revision>
    let output = Command::new("jj")
        .args(["bookmark", "set", bookmark_name, "-r", revision])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("set bookmark", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("set bookmark {bookmark_name} to {revision}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Push a specific bookmark to git remote
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Bookmark doesn't exist
/// - Push fails
/// - JJ command fails
pub fn workspace_git_push_bookmark(workspace_path: &Path, bookmark_name: &str) -> Result<()> {
    if bookmark_name.is_empty() {
        return Err(Error::invalid_config("bookmark name cannot be empty"));
    }

    // Execute: jj git push --bookmark <name>
    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark_name])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("git push bookmark", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("git push bookmark {bookmark_name}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Create a new revision (child of current working copy)
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - JJ command fails
pub fn workspace_new(workspace_path: &Path) -> Result<()> {
    // Execute: jj new
    let output = Command::new("jj")
        .args(["new"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("create new revision", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "create new revision".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Create a new revision at a specific parent
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Parent revision doesn't exist
/// - JJ command fails
pub fn workspace_new_at(workspace_path: &Path, parent_revision: &str) -> Result<()> {
    if parent_revision.is_empty() {
        return Err(Error::invalid_config("parent revision cannot be empty"));
    }

    // Execute: jj new <parent>
    let output = Command::new("jj")
        .args(["new", parent_revision])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("create new revision at parent", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("create new revision at {parent_revision}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Log output entry for a revision
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Change ID (short form)
    pub change_id: String,
    /// Commit ID (short form)
    pub commit_id: String,
    /// Author name
    pub author: String,
    /// Commit description (first line)
    pub description: String,
    /// Whether this is the working copy
    pub is_working_copy: bool,
}

/// Get log of revisions in workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - JJ command fails
pub fn workspace_log(workspace_path: &Path, revset: Option<&str>) -> Result<String> {
    // Execute: jj log [-r <revset>]
    let mut args = vec!["log"];
    if let Some(r) = revset {
        args.push("-r");
        args.push(r);
    }

    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("get log", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get log".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get log across all workspaces
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - JJ command fails
pub fn workspace_log_all(workspace_path: &Path) -> Result<String> {
    // Execute: jj log --all-workspaces (show work from all workspaces)
    let output = Command::new("jj")
        .args(["log"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("get all workspaces log", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get all workspaces log".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Restore files from another revision
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Source revision doesn't exist
/// - JJ command fails
pub fn workspace_restore(workspace_path: &Path, from_revision: &str) -> Result<()> {
    if from_revision.is_empty() {
        return Err(Error::invalid_config("source revision cannot be empty"));
    }

    // Execute: jj restore --from <revision>
    let output = Command::new("jj")
        .args(["restore", "--from", from_revision])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("restore from revision", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("restore from {from_revision}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Restore specific files from another revision
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Source revision or files don't exist
/// - JJ command fails
pub fn workspace_restore_files(
    workspace_path: &Path,
    from_revision: &str,
    files: &[&str],
) -> Result<()> {
    if from_revision.is_empty() {
        return Err(Error::invalid_config("source revision cannot be empty"));
    }
    if files.is_empty() {
        return Err(Error::invalid_config("file list cannot be empty"));
    }

    // Execute: jj restore --from <revision> <files...>
    let mut args = vec!["restore", "--from", from_revision];
    args.extend(files);

    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("restore files from revision", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("restore files from {from_revision}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Undo the last JJ operation (safety net)
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - No operations to undo
/// - JJ command fails
pub fn workspace_undo(workspace_path: &Path) -> Result<()> {
    // Execute: jj undo
    let output = Command::new("jj")
        .args(["undo"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("undo operation", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "undo operation".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

/// Get operation log (for recovery)
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - JJ command fails
pub fn workspace_op_log(workspace_path: &Path) -> Result<String> {
    // Execute: jj op log
    let output = Command::new("jj")
        .args(["op", "log"])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("get operation log", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "get operation log".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Restore repository to a previous operation state
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Operation ID doesn't exist
/// - JJ command fails
pub fn workspace_op_restore(workspace_path: &Path, operation_id: &str) -> Result<()> {
    if operation_id.is_empty() {
        return Err(Error::invalid_config("operation ID cannot be empty"));
    }

    // Execute: jj op restore <op-id>
    let output = Command::new("jj")
        .args(["op", "restore", operation_id])
        .current_dir(workspace_path)
        .output()
        .map_err(|e| super::jj_command_error("restore operation", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: format!("restore operation {operation_id}"),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    Ok(())
}

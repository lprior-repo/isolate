//! JJ operation graph synchronization for workspace creation
//!
//! This module solves the problem where multiple concurrent workspace
//! creations can cause operation graph corruption. The issue occurs when:
//!
//! 1. Workspace A is created based on operation X
//! 2. Workspace B is created based on operation Y (sibling of X)
//! 3. Each workspace has its own working copy operation ID
//! 4. JJ detects a mismatch and refuses to load the repo
//!
//! The solution is to ensure all workspace creations are serialized
//! and based on the same repository operation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use tokio::sync::Mutex;

use crate::{Error, Result};

/// Global workspace creation lock to prevent concurrent JJ workspace operations
///
/// This ensures that workspace creations are serialized, preventing
/// operation graph divergence when multiple workspaces are created
/// in quick succession.
static WORKSPACE_CREATION_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

/// Information about the current repository operation
#[derive(Debug, Clone)]
pub struct RepoOperationInfo {
    /// Operation ID (hash)
    pub operation_id: String,
    /// Repository root path
    pub repo_root: PathBuf,
}

/// Get the current repository operation ID
///
/// This queries JJ for the current operation ID to establish a baseline
/// for workspace creation. All workspaces should be created based on
/// this same operation to prevent graph divergence.
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub async fn get_current_operation(root: &Path) -> Result<RepoOperationInfo> {
    let output = tokio::process::Command::new("jj")
        .args(["log", "--no-graph", "--limit", "1", "-T", "change_id"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "get current operation".to_string(),
            source: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get current operation".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let operation_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if operation_id.is_empty() {
        return Err(Error::JjCommandError {
            operation: "get current operation".to_string(),
            source: "Empty operation ID returned".to_string(),
            is_not_found: false,
        });
    }

    // Get repo root
    let root_output = tokio::process::Command::new("jj")
        .args(["root"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "get repo root".to_string(),
            source: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !root_output.status.success() {
        let stderr = String::from_utf8_lossy(&root_output.stderr);
        return Err(Error::JjCommandError {
            operation: "get repo root".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let repo_root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_string();

    Ok(RepoOperationInfo {
        operation_id,
        repo_root: PathBuf::from(repo_root),
    })
}

/// Create a JJ workspace with operation graph synchronization
///
/// This function ensures workspace creation is serialized and based on
/// a consistent repository operation, preventing graph corruption.
///
/// # Workflow
///
/// 1. Acquire global workspace creation lock
/// 2. Get current repository operation ID
/// 3. Create the workspace using `jj workspace add`
/// 4. Verify workspace is based on the correct operation
/// 5. Release lock
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Unable to create workspace directory
/// - JJ command fails
/// - Operation verification fails
///
/// # Example
///
/// ```no_run
/// use zjj_core::jj_operation_sync::create_workspace_synced;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let workspace_path = std::path::PathBuf::from("/tmp/workspace");
/// create_workspace_synced("my-workspace", &workspace_path).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_workspace_synced(name: &str, path: &Path) -> Result<()> {
    // Validate inputs
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    // Validate path has parent BEFORE acquiring lock
    let repo_root = path.parent().ok_or_else(|| {
        Error::InvalidConfig("workspace path must have a parent directory (repo root)".into())
    })?;

    // Acquire global lock to serialize workspace creation
    let _lock = WORKSPACE_CREATION_LOCK.lock().await;

    // Step 1: Get current repository operation ID
    let operation_info = get_current_operation(repo_root).await?;

    // Step 2: Create parent directory if needed
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::IoError(format!("Failed to create workspace directory: {e}")))?;
    }

    // Step 3: Execute jj workspace add --name <name> <path>
    let output = tokio::process::Command::new("jj")
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .current_dir(repo_root)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    // Step 4: Verify workspace was created and is consistent
    verify_workspace_consistency(name, path, &operation_info).await?;

    Ok(())
}

/// Verify workspace consistency after creation
///
/// Ensures the new workspace is based on the expected repository operation
/// and doesn't have a divergent operation graph.
///
/// # Errors
///
/// Returns error if:
/// - Workspace doesn't exist
/// - Operation IDs don't match
/// - Working copy is out of sync
async fn verify_workspace_consistency(
    name: &str,
    path: &Path,
    expected_operation: &RepoOperationInfo,
) -> Result<()> {
    // Get the operation ID in the new workspace
    let output = tokio::process::Command::new("jj")
        .args(["log", "--no-graph", "--limit", "1", "-T", "change_id"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| Error::JjCommandError {
            operation: "verify workspace operation".to_string(),
            source: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check for specific error patterns indicating operation graph issues
        let error_str = stderr.to_string();

        if error_str.contains("sibling of the working copy's operation")
            || error_str.contains("working copy")
            || error_str.contains("operation")
        {
            return Err(Error::JjWorkspaceConflict {
                conflict_type: crate::error::JjConflictType::Stale,
                workspace_name: name.to_string(),
                source: format!(
                    "Operation graph mismatch: {error_str}"
                ),
                recovery_hint: format!(
                    "The workspace '{name}' was created but has an inconsistent operation graph.\n\n\
                     Recovery: Run 'jj workspace forget {name}' and retry creation.\n\n\
                     This error indicates concurrent workspace creation or repo state change."
                ),
            });
        }

        return Err(Error::JjCommandError {
            operation: "verify workspace operation".to_string(),
            source: error_str,
            is_not_found: false,
        });
    }

    // Verify operation matches
    let workspace_operation = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Operations should match - if not, we have graph divergence
    if workspace_operation != expected_operation.operation_id {
        return Err(Error::JjWorkspaceConflict {
            conflict_type: crate::error::JjConflictType::Stale,
            workspace_name: name.to_string(),
            source: format!(
                "Operation ID mismatch: expected '{}', got '{}'",
                expected_operation.operation_id, workspace_operation
            ),
            recovery_hint: format!(
                "The workspace '{name}' was created on a different operation than expected.\n\n\
                 Expected: {}\n\
                 Got: {}\n\n\
                 Recovery: Run 'jj workspace forget {name}' and retry creation.",
                expected_operation.operation_id, workspace_operation
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation_lock_exists() {
        // Verify the lock exists and can be referenced
        // This is a compile-time check that the mutex is accessible
        let _ = &WORKSPACE_CREATION_LOCK;
    }

    #[tokio::test]
    async fn test_empty_workspace_name_returns_error() {
        let temp_dir = std::env::temp_dir().join("test-empty-name");
        let result = create_workspace_synced("", &temp_dir).await;
        assert!(result.is_err());

        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("workspace name cannot be empty"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[tokio::test]
    async fn test_workspace_without_parent_returns_error() {
        // Test that workspace path without parent directory returns error
        // Use "/" which has no parent
        let workspace_path = PathBuf::from("/");
        let result = create_workspace_synced("test", &workspace_path).await;

        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("parent directory"));
            }
            Err(other) => {
                panic!("Expected InvalidConfig error, got: {:?}", other);
            }
            Ok(_) => {
                panic!("Expected InvalidConfig error, but got Ok");
            }
        }
    }
}

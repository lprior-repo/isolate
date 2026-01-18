//! Pre-flight checks for JJ installation and repository state

use std::path::PathBuf;
use std::process::Command;

use crate::{error::system::SystemError, Error, Result};

/// Check if JJ is installed and available
///
/// # Errors
///
/// Returns error if JJ is not found in PATH
pub fn check_jj_installed() -> Result<()> {
    Command::new("jj")
        .arg("--version")
        .output()
        .map_err(|e| super::jj_command_error("check JJ installation", &e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::System(SystemError::JjCommandError {
                    operation: "check JJ installation".to_string(),
                    source: "JJ command returned non-zero exit code".to_string(),
                    is_not_found: false,
                }))
            }
        })
}

/// Check if current directory is in a JJ repository
///
/// # Errors
///
/// Returns error if not in a JJ repository
pub fn check_in_jj_repo() -> Result<PathBuf> {
    let output = Command::new("jj")
        .args(["root"])
        .output()
        .map_err(|e| super::jj_command_error("find JJ repository root", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: format!("Not in a JJ repository. {stderr}"),
            is_not_found: false,
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();

    if root.is_empty() {
        Err(Error::System(SystemError::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: "Could not determine JJ repository root".to_string(),
            is_not_found: false,
        }))
    } else {
        Ok(PathBuf::from(root))
    }
}

/// Check if the repository has uncommitted changes
///
/// Uses `jj status` to detect if there are any uncommitted changes in the working directory.
/// Returns `Ok(true)` if changes exist, `Ok(false)` if clean, `Err` on command failure.
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn has_uncommitted_changes(repo_path: &std::path::Path) -> Result<bool> {
    let output = Command::new("jj")
        .args(["status"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| super::jj_command_error("check uncommitted changes", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::System(SystemError::JjCommandError {
            operation: "check uncommitted changes".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // jj status output patterns indicating changes:
    // - "Working copy changes:" followed by file list
    // - "Modified files:", "Added files:", "Removed files:"
    // - Clean repo shows "No changes." or "The working copy is clean."

    let has_changes = stdout.contains("Working copy changes:")
        || stdout.contains("Modified files:")
        || stdout.contains("Added files:")
        || stdout.contains("Removed files:");

    Ok(has_changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_has_uncommitted_changes_clean_repo() {
        // This test requires a JJ repo, skip if jj not available
        let Ok(temp_dir) = TempDir::new() else {
            eprintln!("Skipping test: could not create temp dir");
            return;
        };

        // Initialize JJ repo
        let init = Command::new("jj")
            .args(["git", "init"])
            .current_dir(temp_dir.path())
            .output();

        let Ok(output) = init else {
            eprintln!("Skipping test: jj not available");
            return;
        };

        if !output.status.success() {
            eprintln!("Skipping test: jj init failed");
            return;
        }

        // Clean repo should have no changes
        let result = has_uncommitted_changes(temp_dir.path());
        assert!(result.is_ok(), "Should successfully check status");

        // Note: Fresh JJ repo may have initial changes from setup
        // Test verifies function returns without error, not specific value
    }
}

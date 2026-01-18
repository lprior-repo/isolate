//! Validator for checking JJ workspace availability
//!
//! This module validates that a workspace name is not already in use
//! in the JJ repository by querying the jj workspace list command.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::commands::add::error_messages;

/// Validate that JJ workspace is available (no name collision)
///
/// This check ensures no workspace with the same name already exists
/// in the JJ repository, preventing conflicts during workspace creation.
///
/// # Arguments
/// * `repo_root` - Path to the repository root
/// * `name` - Workspace name to check
///
/// # Errors
/// Returns error if:
/// - Failed to execute 'jj workspace list'
/// - The command execution fails
/// - Workspace with the given name already exists
pub fn validate_workspace_available(repo_root: &Path, name: &str) -> Result<()> {
    // Get list of existing workspaces
    let output = std::process::Command::new("jj")
        .arg("workspace")
        .arg("list")
        .current_dir(repo_root)
        .output()
        .context(error_messages::JJ_WORKSPACE_LIST_FAILED)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(error_messages::jj_workspace_list_error_details(&stderr));
    }

    let workspaces = String::from_utf8_lossy(&output.stdout);

    // Check if any line contains the workspace name
    // JJ workspace list output format: "workspace_name: revision"
    let exists = workspaces.lines().any(|line| {
        line.split(':')
            .next()
            .is_some_and(|ws_name| ws_name.trim() == name)
    });

    if exists {
        bail!(error_messages::workspace_already_exists(name));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_message_workspace_exists() {
        let name = "test-workspace";
        let msg = error_messages::workspace_already_exists(name);
        assert!(msg.contains(name));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn test_workspace_name_parsing_single_workspace() {
        // Test the parsing logic with a sample workspace list output
        let sample_output = "default: rev123abc";
        let name = "default";

        let exists = sample_output.lines().any(|line| {
            line.split(':')
                .next()
                .is_some_and(|ws_name| ws_name.trim() == name)
        });

        assert!(exists, "Should find 'default' workspace");
    }

    #[test]
    fn test_workspace_name_parsing_multiple_workspaces() {
        let sample_output = "default: rev123abc\nfeature: rev456def\nhotfix: rev789ghi";

        // Should find each workspace
        for name in &["default", "feature", "hotfix"] {
            let exists = sample_output.lines().any(|line| {
                line.split(':')
                    .next()
                    .is_some_and(|ws_name| ws_name.trim() == *name)
            });
            assert!(exists, "Should find '{name}' workspace");
        }

        // Should not find non-existent workspace
        let exists = sample_output.lines().any(|line| {
            line.split(':')
                .next()
                .is_some_and(|ws_name| ws_name.trim() == "nonexistent")
        });
        assert!(!exists, "Should not find 'nonexistent' workspace");
    }
}

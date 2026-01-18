//! Dependency checking and version detection
//!
//! This module provides functionality to check for required and optional
//! dependencies, detect their versions, and build dependency information.

use im::HashMap;
use zjj_core::introspection::DependencyInfo;

use crate::cli::{is_command_available, run_command};

/// Get version of a command by running `command --version`
///
/// Returns the first line of version output if successful.
fn get_command_version(command: &str) -> Option<String> {
    run_command(command, &["--version"])
        .ok()
        .and_then(|output| output.lines().next().map(|line| line.trim().to_string()))
}

/// Build dependency info for a single command
///
/// # Arguments
/// * `command` - Command name to check
/// * `required` - Whether this dependency is required for zjj to function
fn build_dependency_info(command: &str, required: bool) -> DependencyInfo {
    let installed = is_command_available(command);
    DependencyInfo {
        required,
        installed,
        version: if installed {
            get_command_version(command)
        } else {
            None
        },
        command: command.to_string(),
    }
}

/// Check all dependencies and return their status
///
/// Returns a `HashMap` containing information about all dependencies:
/// - jj (required): Jujutsu version control
/// - zellij (required): Terminal multiplexer
/// - claude (optional): Claude CLI for AI assistance
/// - beads (optional): Issue tracking integration
pub fn check_dependencies() -> HashMap<String, DependencyInfo> {
    let dependencies = [
        ("jj", true),
        ("zellij", true),
        ("claude", false),
        ("beads", false),
    ];

    dependencies
        .iter()
        .fold(HashMap::new(), |acc, (cmd, required)| {
            acc.update(cmd.to_string(), build_dependency_info(cmd, *required))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_dependencies_returns_all_deps() {
        let deps = check_dependencies();
        assert!(deps.contains_key("jj"));
        assert!(deps.contains_key("zellij"));
        assert!(deps.contains_key("claude"));
        assert!(deps.contains_key("beads"));
    }

    #[test]
    fn test_required_deps_are_marked_correctly() {
        let deps = check_dependencies();

        if let Some(jj) = deps.get("jj") {
            assert!(jj.required, "jj should be marked as required");
        }

        if let Some(zellij) = deps.get("zellij") {
            assert!(zellij.required, "zellij should be marked as required");
        }

        if let Some(claude) = deps.get("claude") {
            assert!(!claude.required, "claude should be marked as optional");
        }

        if let Some(beads) = deps.get("beads") {
            assert!(!beads.required, "beads should be marked as optional");
        }
    }
}

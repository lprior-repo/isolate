//! Configuration types for Zellij layout generation
//!
//! This module provides types for configuring Zellij layouts.
//! All operations are pure functions with no side effects.

use std::path::PathBuf;

/// Supported layout templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTemplate {
    /// Single Claude pane
    Minimal,
    /// Claude (70%) + beads/status sidebar (30%)
    Standard,
    /// Standard + floating pane + jj log
    Full,
    /// Two Claude instances side-by-side
    Split,
    /// Diff view + beads + Claude
    Review,
}

/// Configuration for layout generation
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Session name for variable substitution
    pub session_name: String,
    /// Workspace path for cwd settings
    pub workspace_path: PathBuf,
    /// Command to run in main pane (default: "claude")
    pub claude_command: String,
    /// Command to run in beads pane (default: "bv")
    pub beads_command: String,
    /// Tab name prefix (default: "zjj")
    pub tab_prefix: String,
}

impl LayoutConfig {
    /// Create a new layout configuration
    #[must_use]
    pub fn new(session_name: String, workspace_path: PathBuf) -> Self {
        Self {
            session_name,
            workspace_path,
            claude_command: "claude".to_string(),
            beads_command: "bv".to_string(),
            tab_prefix: "zjj".to_string(),
        }
    }

    /// Set the Claude command
    #[must_use]
    pub fn with_claude_command(mut self, command: String) -> Self {
        self.claude_command = command;
        self
    }

    /// Set the beads command
    #[must_use]
    pub fn with_beads_command(mut self, command: String) -> Self {
        self.beads_command = command;
        self
    }

    /// Set the tab prefix
    #[must_use]
    pub fn with_tab_prefix(mut self, prefix: String) -> Self {
        self.tab_prefix = prefix;
        self
    }

    /// Get the full tab name
    #[must_use]
    pub fn tab_name(&self) -> String {
        format!("{}:{}", self.tab_prefix, self.session_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LayoutConfig {
        LayoutConfig::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        )
    }

    #[test]
    fn test_variable_substitution_in_config() {
        let config = LayoutConfig::new("my-feature".to_string(), PathBuf::from("/workspace"));

        assert_eq!(config.session_name, "my-feature");
        assert_eq!(config.tab_name(), "zjj:my-feature");
    }

    #[test]
    fn test_custom_commands_in_config() {
        let config = test_config()
            .with_claude_command("custom-claude".to_string())
            .with_beads_command("custom-bv".to_string())
            .with_tab_prefix("custom".to_string());

        assert_eq!(config.claude_command, "custom-claude");
        assert_eq!(config.beads_command, "custom-bv");
        assert_eq!(config.tab_name(), "custom:test-session");
    }
}

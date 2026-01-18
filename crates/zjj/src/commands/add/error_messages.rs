//! Centralized error messages for add command validation
//!
//! This module consolidates all error message strings used in validation,
//! making them easy to maintain and reuse across validators.

/// Error message for when a session already exists
pub fn session_already_exists(name: &str) -> String {
    format!("Session '{name}' already exists")
}

/// Error message for when a JJ workspace already exists
pub fn workspace_already_exists(name: &str) -> String {
    format!("JJ workspace '{name}' already exists")
}

/// Error message for when ZELLIJ environment variable is not set
pub const ZELLIJ_NOT_SET: &str = "Not running inside Zellij. Use --no-open flag to create session without opening a tab, or start a Zellij session first.";

/// Error message for jj not found
pub const JJ_NOT_FOUND: &str = "jj command not found in PATH. Please install Jujutsu (jj)";

/// Error message for zellij not found
pub const ZELLIJ_NOT_FOUND: &str = "zellij command not found in PATH. Please install Zellij";

/// Error message for failed to query session database
pub const SESSION_DB_QUERY_FAILED: &str = "Failed to query session database";

/// Error message for failed to execute jj workspace list
pub const JJ_WORKSPACE_LIST_FAILED: &str = "Failed to execute 'jj workspace list'";

/// Error message template for jj workspace list failure with details
pub fn jj_workspace_list_error_details(stderr: &str) -> String {
    format!("Failed to list JJ workspaces: {stderr}")
}

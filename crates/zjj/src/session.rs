//! Session data structures and utilities

use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// A ZJJ session representing a JJ workspace + Zellij tab pair
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    /// Unique session name
    pub name: String,
    /// Path to the JJ workspace directory
    pub workspace_path: String,
    /// Zellij tab name (format: `jjz:NAME`)
    pub zellij_tab: String,
    /// Unix timestamp when session was created
    pub created_at: u64,
}

impl Session {
    /// Create a new session with the given name and workspace path
    pub fn new(name: &str, workspace_path: &str) -> Result<Self> {
        validate_session_name(name)?;

        let created_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .context("System time error")?
            .as_secs();

        Ok(Self {
            name: name.to_string(),
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("jjz:{name}"),
            created_at,
        })
    }
}

/// Validate a session name
fn validate_session_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Session name cannot be empty");
    }

    if name.len() > 64 {
        bail!("Session name cannot exceed 64 characters");
    }

    // Only allow alphanumeric, dash, and underscore
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Session name can only contain alphanumeric characters, dashes, and underscores");
    }

    // Cannot start with dash or underscore
    if name.starts_with('-') || name.starts_with('_') {
        bail!("Session name cannot start with a dash or underscore");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new_valid() {
        let session = Session::new("my-session", "/path/to/workspace");
        assert!(session.is_ok());
        let s = session.unwrap_or_default();
        assert_eq!(s.name, "my-session");
        assert_eq!(s.zellij_tab, "jjz:my-session");
    }

    #[test]
    fn test_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_invalid_chars() {
        let result = validate_session_name("my session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_starts_with_dash() {
        let result = validate_session_name("-session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_valid_with_underscore() {
        let result = validate_session_name("my_session");
        assert!(result.is_ok());
    }
}

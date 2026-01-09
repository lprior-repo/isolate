//! Session data structures and utilities

use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use zjj_core::{Error, Result};

/// Session status representing the lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created
    Creating,
    /// Session is active and ready for use
    Active,
    /// Session is temporarily paused
    Paused,
    /// Session work is completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for SessionStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "creating" => Ok(Self::Creating),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::ValidationError(format!("Invalid status: {s}"))),
        }
    }
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Creating
    }
}

/// A ZJJ session representing a JJ workspace + Zellij tab pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Auto-generated database ID (None for new sessions not yet persisted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Unique session name
    pub name: String,
    /// Current status of the session
    pub status: SessionStatus,
    /// Path to the JJ workspace directory
    pub workspace_path: String,
    /// Zellij tab name (format: `jjz:NAME`)
    pub zellij_tab: String,
    /// Git branch associated with this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Unix timestamp when session was created
    pub created_at: u64,
    /// Unix timestamp when session was last updated
    pub updated_at: u64,
    /// Unix timestamp of last sync operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<u64>,
    /// Extensible metadata as JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            status: SessionStatus::default(),
            workspace_path: String::new(),
            zellij_tab: String::new(),
            branch: None,
            created_at: 0,
            updated_at: 0,
            last_synced: None,
            metadata: None,
        }
    }
}

impl Session {
    /// Create a new session with the given name and workspace path
    pub fn new(name: &str, workspace_path: &str) -> Result<Self> {
        validate_session_name(name)?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(format!("System time error: {e}")))?
            .as_secs();

        Ok(Self {
            id: None,
            name: name.to_string(),
            status: SessionStatus::Creating,
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("jjz:{name}"),
            branch: None,
            created_at: now,
            updated_at: now,
            last_synced: None,
            metadata: None,
        })
    }
}

/// Fields that can be updated on an existing session
#[derive(Debug, Clone, Default)]
pub struct SessionUpdate {
    /// Update the session status
    pub status: Option<SessionStatus>,
    /// Update the branch
    pub branch: Option<String>,
    /// Update the last synced timestamp
    pub last_synced: Option<u64>,
    /// Update the metadata
    pub metadata: Option<serde_json::Value>,
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

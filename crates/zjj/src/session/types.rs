//! Session data types

#[cfg(test)]
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::status::SessionStatus;

/// A ZJJ session representing a JJ workspace + Zellij tab pair
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// Zellij tab name (format: `zjj:NAME`)
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

impl Session {
    /// Create a new session with the given name and workspace path
    ///
    /// NOTE: This is primarily for testing. Production code should use
    /// `SessionDb::create` which handles persistence.
    #[cfg(test)]
    pub fn new(name: &str, workspace_path: &str) -> zjj_core::Result<Self> {
        use zjj_core::Error;

        super::validation::validate_session_name(name)?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(format!("System time error: {e}")))?
            .as_secs();

        Ok(Self {
            id: None,
            name: name.to_string(),
            status: SessionStatus::Creating,
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("zjj:{name}"),
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

//! Type definitions for list command data module

use serde::Serialize;
use zjj_core::json::{SchemaEnvelope, SchemaType};

/// JSON response wrapper for list command
///
/// Provides metadata alongside the sessions array for AI parsing and tooling.
#[derive(Debug, Clone, Serialize)]
pub struct SessionListResponse {
    /// Number of sessions in the response
    pub count: usize,
    /// Applied filter description (null if no filter)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// The sessions matching the query
    pub sessions: Vec<SessionListItem>,
}

impl SessionListResponse {
    /// Create a new response from sessions and optional filter
    #[must_use]
    pub fn new(sessions: Vec<SessionListItem>, filter: Option<String>) -> Self {
        Self {
            count: sessions.len(),
            filter,
            sessions,
        }
    }

    /// Wrap this response with schema metadata for JSON output
    #[must_use]
    pub fn with_schema(self) -> SchemaEnvelope<Self> {
        SchemaEnvelope::new(SchemaType::List, self)
    }
}

/// Enhanced session information for list output
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    pub name: String,
    pub status: String,
    pub branch: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub changes: String,
    pub beads: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead: Option<SessionBeadInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<SessionAgentInfo>,
}

/// Bead metadata for a session
#[derive(Debug, Clone, Serialize)]
pub struct SessionBeadInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_type: Option<String>,
}

/// Agent metadata for a session
#[derive(Debug, Clone, Serialize)]
pub struct SessionAgentInfo {
    pub agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_seconds: Option<u64>,
}

/// Filter criteria for list command
#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    /// Filter by specific bead ID
    pub bead_id: Option<String>,
    /// Filter by specific agent ID
    pub agent_id: Option<String>,
    /// Show only sessions with beads
    pub with_beads: bool,
    /// Show only sessions with agents
    pub with_agents: bool,
}

impl ListFilter {
    /// Returns a description of the applied filters, or None if no filters are set
    #[must_use]
    pub fn description(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(ref id) = self.bead_id {
            parts.push(format!("bead_id={id}"));
        }
        if let Some(ref id) = self.agent_id {
            parts.push(format!("agent_id={id}"));
        }
        if self.with_beads {
            parts.push("with_beads".to_string());
        }
        if self.with_agents {
            parts.push("with_agents".to_string());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

/// Beads issue counts
#[derive(Debug, Clone, Default)]
pub struct BeadCounts {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
}

impl std::fmt::Display for BeadCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.open, self.in_progress, self.blocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_counts_display() {
        let counts = BeadCounts {
            open: 5,
            in_progress: 3,
            blocked: 2,
        };
        assert_eq!(counts.to_string(), "5/3/2");
    }

    #[test]
    fn test_bead_counts_default() {
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[test]
    fn test_session_list_item_serialization() -> anyhow::Result<()> {
        let item = SessionListItem {
            name: "test".to_string(),
            status: "active".to_string(),
            branch: "feature".to_string(),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            bead: None,
            agent: None,
        };

        let json = serde_json::to_string(&item)?;
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"changes\":\"5\""));
        Ok(())
    }

    #[test]
    fn test_session_list_item_with_none_branch() {
        let item = SessionListItem {
            name: "test".to_string(),
            status: "active".to_string(),
            branch: "-".to_string(),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            changes: "-".to_string(),
            beads: "0/0/0".to_string(),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            bead: None,
            agent: None,
        };

        assert_eq!(item.branch, "-");
        assert_eq!(item.changes, "-");
    }
}

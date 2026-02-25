//! Agent coordination types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    /// Create a new agent ID
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An agent in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,
    /// When registered
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat
    pub last_seen: DateTime<Utc>,
    /// Current session
    pub current_session: Option<String>,
    /// Current command
    pub current_command: Option<String>,
    /// Actions count
    pub actions_count: u64,
}

impl Agent {
    /// Create a new agent
    #[must_use]
    pub fn new(id: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id,
            registered_at: now,
            last_seen: now,
            current_session: None,
            current_command: None,
            actions_count: 0,
        }
    }

    /// Check if agent is active (heartbeat within last 60 seconds)
    #[must_use]
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        (now - self.last_seen).num_seconds() < 60
    }

    /// Get the status of this agent
    #[must_use]
    pub fn status(&self) -> AgentStatus {
        if self.is_active() {
            AgentStatus::Active
        } else {
            AgentStatus::Stale
        }
    }
}

/// Agent status summary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is active
    Active,
    /// Agent is stale (no recent heartbeat)
    Stale,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Stale => write!(f, "stale"),
        }
    }
}

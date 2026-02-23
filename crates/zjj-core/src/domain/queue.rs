//! Queue domain types
//!
//! Provides types for queue operations and state.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::identifiers::{AgentId, WorkspaceName};
use chrono::{DateTime, Utc};

/// Queue entry claim state - replaces `Option` fields for `claimed_by`/`claimed_at`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimState {
    /// Entry is not claimed
    Unclaimed,
    /// Entry is claimed by an agent
    Claimed {
        agent: AgentId,
        claimed_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    /// Previous claim has expired
    Expired {
        previous_agent: AgentId,
        expired_at: DateTime<Utc>,
    },
}

impl ClaimState {
    #[must_use]
    pub const fn is_claimed(&self) -> bool {
        matches!(self, Self::Claimed { .. })
    }

    #[must_use]
    pub const fn is_unclaimed(&self) -> bool {
        matches!(self, Self::Unclaimed)
    }

    #[must_use]
    pub const fn holder(&self) -> Option<&AgentId> {
        match self {
            Self::Unclaimed | Self::Expired { .. } => None,
            Self::Claimed { agent, .. } => Some(agent),
        }
    }

    /// Check if a transition from self to target is valid
    #[must_use]
    #[allow(clippy::match_same_arms)] // More readable as explicit patterns
    pub const fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            // Unclaimed can be claimed
            (Self::Unclaimed, Self::Claimed { .. }) => true,

            // Claimed can expire or be explicitly released to Unclaimed
            (Self::Claimed { .. }, Self::Expired { .. } | Self::Unclaimed) => true,

            // Expired can go back to Unclaimed (reclaimed)
            (Self::Expired { .. }, Self::Unclaimed) => true,

            // No self-loops or other transitions
            _ => false,
        }
    }

    /// Get all valid target state types from this state
    #[must_use]
    pub fn valid_transition_types(&self) -> Vec<&'static str> {
        match self {
            Self::Unclaimed => vec!["Claimed"],
            Self::Claimed { .. } => vec!["Expired", "Unclaimed"],
            Self::Expired { .. } => vec!["Unclaimed"],
        }
    }
}

impl std::fmt::Display for ClaimState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unclaimed => write!(f, "unclaimed"),
            Self::Claimed { agent, .. } => write!(f, "claimed by {agent}"),
            Self::Expired { previous_agent, .. } => {
                write!(f, "expired (was {previous_agent})")
            }
        }
    }
}

/// Queue command - replaces boolean flags in `QueueOptions`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueCommand {
    /// List queue entries
    List,
    /// Process next queue entry
    Process,
    /// Get next queue entry without processing
    Next,
    /// Show queue statistics
    Stats,
    /// Show status for a specific workspace
    ShowStatus { workspace: WorkspaceName },
    /// Add a workspace to the queue
    Add {
        workspace: WorkspaceName,
        bead: Option<String>,
        priority: i32,
        agent: Option<AgentId>,
    },
    /// Remove a workspace from the queue
    Remove { workspace: WorkspaceName },
    /// Retry a failed queue entry
    Retry { entry_id: i64 },
    /// Cancel a queue entry
    Cancel { entry_id: i64 },
    /// Reclaim stale entries
    ReclaimStale { threshold_secs: i64 },
    /// Show status by entry ID
    ShowById { entry_id: i64 },
}

impl QueueCommand {
    /// Get a short name for the command
    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::List => "list",
            Self::Process => "process",
            Self::Next => "next",
            Self::Stats => "stats",
            Self::ShowStatus { .. } => "status",
            Self::Add { .. } => "add",
            Self::Remove { .. } => "remove",
            Self::Retry { .. } => "retry",
            Self::Cancel { .. } => "cancel",
            Self::ReclaimStale { .. } => "reclaim-stale",
            Self::ShowById { .. } => "status-id",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_state() {
        let unclaimed = ClaimState::Unclaimed;
        assert!(unclaimed.is_unclaimed());
        assert!(!unclaimed.is_claimed());
        assert!(unclaimed.holder().is_none());

        let agent = AgentId::parse("agent-1").unwrap();
        let now = Utc::now();
        let expires = now + chrono::Duration::seconds(300);

        let claimed = ClaimState::Claimed {
            agent: agent.clone(),
            claimed_at: now,
            expires_at: expires,
        };
        assert!(!claimed.is_unclaimed());
        assert!(claimed.is_claimed());
        assert_eq!(claimed.holder(), Some(&agent));
    }

    #[test]
    fn test_queue_command_names() {
        assert_eq!(QueueCommand::List.name(), "list");
        assert_eq!(QueueCommand::Process.name(), "process");
        assert_eq!(QueueCommand::Next.name(), "next");
        assert_eq!(QueueCommand::Stats.name(), "stats");
    }
}

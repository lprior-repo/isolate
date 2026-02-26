//! Types for the agents command

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Command-line arguments for the agents command
#[derive(Debug, Clone)]
pub struct AgentsArgs {
    /// Include stale agents (not seen within heartbeat timeout)
    pub all: bool,

    /// Filter by session
    pub session: Option<String>,
}

/// Arguments for agent register subcommand
#[derive(Debug, Clone)]
pub struct RegisterArgs {
    /// Agent ID to register (auto-generated if not provided)
    pub agent_id: Option<String>,
    /// Session to associate with this agent
    pub session: Option<String>,
}

/// Arguments for agent heartbeat subcommand
#[derive(Debug, Clone)]
pub struct HeartbeatArgs {
    /// Command being executed (optional)
    pub command: Option<String>,
}

/// Arguments for agent status subcommand
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StatusArgs {}

/// Arguments for agent unregister subcommand
#[derive(Debug, Clone)]
pub struct UnregisterArgs {
    /// Agent ID to unregister (uses `Isolate_AGENT_ID` if not provided)
    pub agent_id: Option<String>,
}

/// Output for register command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterOutput {
    /// The registered agent ID
    pub agent_id: String,
    /// Session if specified
    pub session: Option<String>,
    /// Message describing what happened
    pub message: String,
}

/// Output for heartbeat command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatOutput {
    /// Agent ID that sent heartbeat
    pub agent_id: String,
    /// Timestamp of heartbeat
    pub timestamp: String,
    /// Message
    pub message: String,
}

/// Output for agent status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusOutput {
    /// Whether agent is registered
    pub registered: bool,
    /// Agent info if registered
    pub agent: Option<AgentInfo>,
    /// Message
    pub message: String,
}

/// Output for unregister command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterOutput {
    /// Agent ID that was unregistered
    pub agent_id: String,
    /// Message
    pub message: String,
}

/// Agent information for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier
    pub agent_id: String,
    /// When the agent first registered
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat timestamp
    pub last_seen: DateTime<Utc>,
    /// Current session the agent is working on
    pub current_session: Option<String>,
    /// Current command the agent is executing
    pub current_command: Option<String>,
    /// Number of actions performed by the agent
    pub actions_count: u64,
    /// Whether the agent is stale (outside heartbeat timeout)
    pub stale: bool,
}

/// Summary of a session lock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockSummary {
    /// The session that is locked
    pub session: String,
    /// The agent holding the lock
    pub holder: String,
    /// When the lock expires
    pub expires_at: DateTime<Utc>,
}

/// Output format for the agents command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsOutput {
    /// List of agents (active and possibly stale)
    pub agents: Vec<AgentInfo>,
    /// Active session locks
    pub locks: Vec<LockSummary>,
    /// Number of active agents
    pub total_active: usize,
    /// Number of stale agents (only if --all is used)
    pub total_stale: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agents_args_default() {
        let args = AgentsArgs {
            all: false,
            session: None,
        };
        assert!(!args.all);
        assert!(args.session.is_none());
    }

    #[test]
    fn test_agents_args_with_all() {
        let args = AgentsArgs {
            all: true,
            session: None,
        };
        assert!(args.all);
    }

    #[test]
    fn test_agents_args_with_session() {
        let args = AgentsArgs {
            all: false,
            session: Some("test-session".to_string()),
        };
        assert!(!args.all);
        assert_eq!(args.session, Some("test-session".to_string()));
    }

    #[test]
    fn test_agent_info_serialization() -> Result<(), anyhow::Error> {
        let info = AgentInfo {
            agent_id: "agent-1".to_string(),
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            current_session: Some("session-1".to_string()),
            current_command: Some("isolate list".to_string()),
            actions_count: 5,
            stale: false,
        };

        let json = serde_json::to_string(&info)?;
        let parsed: AgentInfo = serde_json::from_str(&json)?;

        assert_eq!(parsed.agent_id, info.agent_id);
        assert_eq!(parsed.current_session, info.current_session);
        assert_eq!(parsed.actions_count, info.actions_count);
        assert_eq!(parsed.stale, info.stale);
        Ok(())
    }

    #[test]
    fn test_lock_summary_serialization() -> Result<(), anyhow::Error> {
        let lock = LockSummary {
            session: "session-1".to_string(),
            holder: "agent-1".to_string(),
            expires_at: Utc::now(),
        };

        let json = serde_json::to_string(&lock)?;
        let parsed: LockSummary = serde_json::from_str(&json)?;

        assert_eq!(parsed.session, lock.session);
        assert_eq!(parsed.holder, lock.holder);
        Ok(())
    }

    #[test]
    fn test_agents_output_empty() {
        let output = AgentsOutput {
            agents: vec![],
            locks: vec![],
            total_active: 0,
            total_stale: 0,
        };

        assert!(output.agents.is_empty());
        assert!(output.locks.is_empty());
        assert_eq!(output.total_active, 0);
        assert_eq!(output.total_stale, 0);
    }

    #[test]
    fn test_agents_output_with_data() {
        let output = AgentsOutput {
            agents: vec![AgentInfo {
                agent_id: "agent-1".to_string(),
                registered_at: Utc::now(),
                last_seen: Utc::now(),
                current_session: None,
                current_command: None,
                actions_count: 0,
                stale: false,
            }],
            locks: vec![],
            total_active: 1,
            total_stale: 0,
        };

        assert_eq!(output.agents.len(), 1);
        assert_eq!(output.total_active, 1);
        assert_eq!(output.total_stale, 0);
    }

    #[test]
    fn test_agents_output_serialization() -> Result<(), anyhow::Error> {
        let output = AgentsOutput {
            agents: vec![],
            locks: vec![],
            total_active: 0,
            total_stale: 0,
        };

        let json = serde_json::to_string(&output)?;
        let parsed: AgentsOutput = serde_json::from_str(&json)?;

        assert_eq!(parsed.total_active, 0);
        assert_eq!(parsed.total_stale, 0);
        Ok(())
    }
}

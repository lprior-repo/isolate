//! Types for the broadcast command

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use serde::{Deserialize, Serialize};

/// Command-line arguments for the broadcast command
#[derive(Debug, Clone)]
pub struct BroadcastArgs {
    /// Message to broadcast
    pub message: String,

    /// Agent ID of the sender
    pub agent_id: String,
}

/// Output for broadcast command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResponse {
    /// Whether the broadcast was successful
    pub success: bool,

    /// The message that was broadcast
    pub message: String,

    /// List of agent IDs the message was sent to
    pub sent_to: Vec<String>,

    /// Timestamp of the broadcast (RFC3339)
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_broadcast_args() {
        let args = BroadcastArgs {
            message: "Hello, agents!".to_string(),
            agent_id: "agent-1".to_string(),
        };

        assert_eq!(args.message, "Hello, agents!");
        assert_eq!(args.agent_id, "agent-1");
    }

    #[test]
    fn test_broadcast_response_serialization() -> Result<(), anyhow::Error> {
        let response = BroadcastResponse {
            success: true,
            message: "Hello, agents!".to_string(),
            sent_to: vec!["agent-2".to_string(), "agent-3".to_string()],
            timestamp: Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&response)?;
        let parsed: BroadcastResponse = serde_json::from_str(&json)?;

        assert_eq!(parsed.success, response.success);
        assert_eq!(parsed.message, response.message);
        assert_eq!(parsed.sent_to, response.sent_to);
        Ok(())
    }

    #[test]
    fn test_broadcast_response_empty_sent_to() {
        let response = BroadcastResponse {
            success: true,
            message: "No other agents".to_string(),
            sent_to: vec![],
            timestamp: Utc::now().to_rfc3339(),
        };

        assert!(response.success);
        assert!(response.sent_to.is_empty());
    }
}

//! List agents working in sessions
//!
//! This module implements the `zjj agent list` command, which shows
//! AI agents currently working in sessions along with their metadata.

use super::formatting;
use super::output;
use super::queries;

use anyhow::Result;

/// Run the agent list command
///
/// Lists all agents working in sessions, or agent for a specific session.
///
/// # Arguments
///
/// * `session` - Optional session name to filter by
/// * `json` - Whether to output as JSON
pub async fn run(session: Option<&str>, json: bool) -> Result<()> {
    // Query sessions
    let sessions = queries::get_sessions(session).await?;

    // Extract agent info from metadata
    let agents = formatting::extract_agents(sessions);

    // Output results
    output::output_agents(agents, json, session)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::formatting;
    use super::queries;

    #[test]
    fn test_extract_agents_empty() {
        // Test extracting agents from empty session list
        let agents = formatting::extract_agents(vec![]);
        assert_eq!(agents.len(), 0);
    }

    #[test]
    fn test_extract_agent_with_metadata() {
        // Test extracting agent from session with metadata
        let session = queries::SessionWithAgent {
            name: "session1".to_string(),
            metadata: Some(serde_json::json!({
                "agent_id": "claude-code-1234",
                "task_id": "zjj-1fei",
                "spawned_at": 1_234_567_890_u64,
                "pid": 5678_u32,
            })),
        };

        let agents = formatting::extract_agents(vec![session]);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "claude-code-1234");
        assert_eq!(agents[0].task_id, Some("zjj-1fei".to_string()));
        assert_eq!(agents[0].spawned_at, Some(1_234_567_890));
        assert_eq!(agents[0].pid, Some(5678));
    }

    #[test]
    fn test_extract_agent_without_metadata() {
        // Test that sessions without agent metadata are filtered out
        let session = queries::SessionWithAgent {
            name: "session1".to_string(),
            metadata: None,
        };

        let agents = formatting::extract_agents(vec![session]);
        assert_eq!(agents.len(), 0);
    }

    #[test]
    fn test_extract_agent_nested_metadata() {
        // Test extracting agent from nested "agent" object
        let session = queries::SessionWithAgent {
            name: "session1".to_string(),
            metadata: Some(serde_json::json!({
                "agent": {
                    "agent_id": "nested-agent",
                    "task_id": "zjj-nested",
                }
            })),
        };

        let agents = formatting::extract_agents(vec![session]);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "nested-agent");
        assert_eq!(agents[0].task_id, Some("zjj-nested".to_string()));
    }
}

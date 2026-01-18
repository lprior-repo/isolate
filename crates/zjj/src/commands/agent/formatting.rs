//! Result formatting - convert session metadata to `AgentInfo` structures

use serde_json::Value;

use crate::json_output::AgentInfo;

use super::queries::SessionWithAgent;

/// Extract agent info from sessions with metadata
///
/// Converts session metadata into `AgentInfo` structures, handling both
/// direct top-level fields and nested "agent" object structures.
///
/// # Arguments
///
/// * `sessions` - Sessions with metadata to extract from
///
/// # Returns
///
/// Vector of `AgentInfo` for sessions that have agent metadata
pub fn extract_agents(sessions: Vec<SessionWithAgent>) -> Vec<AgentInfo> {
    sessions
        .into_iter()
        .filter_map(|session| extract_single_agent(&session.name, session.metadata))
        .collect()
}

/// Extract agent info from a single session
fn extract_single_agent(session_name: &str, metadata: Option<Value>) -> Option<AgentInfo> {
    let metadata = metadata?;

    // Agent metadata can be stored in different ways, check both:
    // 1. Direct top-level fields: agent_id, task_id, etc.
    // 2. Nested under "agent" key: { "agent": { "agent_id": "...", ... } }
    let agent_data = if metadata.get("agent_id").is_some() {
        // Direct top-level fields
        metadata
    } else if let Some(agent_obj) = metadata.get("agent") {
        // Nested under "agent" key
        agent_obj.clone()
    } else {
        // No agent metadata
        return None;
    };

    // Parse agent fields (all optional, best-effort)
    let agent_id = agent_data
        .get("agent_id")
        .and_then(Value::as_str)
        .map(String::from)?;

    let task_id = agent_data
        .get("task_id")
        .and_then(Value::as_str)
        .map(String::from);

    let spawned_at = agent_data.get("spawned_at").and_then(Value::as_u64);

    let pid = agent_data
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok());

    let exit_code = agent_data
        .get("exit_code")
        .and_then(Value::as_i64)
        .and_then(|v| i32::try_from(v).ok());

    let artifacts_path = agent_data
        .get("artifacts_path")
        .and_then(Value::as_str)
        .map(String::from);

    Some(AgentInfo {
        session_name: session_name.to_string(),
        agent_id,
        task_id,
        spawned_at,
        pid,
        exit_code,
        artifacts_path,
    })
}

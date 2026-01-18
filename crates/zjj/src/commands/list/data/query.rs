//! Query and filtering logic for list command
//!
//! Provides functional filtering operations on sessions based on criteria
//! like bead ID, agent ID, and metadata presence.

use serde_json::Value;

use crate::session::Session;

use super::types::ListFilter;

/// Check if a session matches the bead_id filter
///
/// Uses functional approach with `and_then` for safe Option chaining.
fn matches_bead_id_filter(metadata: &Value, bead_id: &str) -> bool {
    metadata
        .get("bead_id")
        .and_then(Value::as_str)
        .map(|id| id == bead_id)
        .unwrap_or(false)
}

/// Check if a session matches the agent_id filter
///
/// Checks both direct top-level fields and nested "agent" object.
/// Uses functional approach with `or_else` for fallback checking.
fn matches_agent_id_filter(metadata: &Value, agent_id: &str) -> bool {
    metadata
        .get("agent_id")
        .and_then(Value::as_str)
        .or_else(|| {
            metadata
                .get("agent")
                .and_then(|a| a.get("agent_id"))
                .and_then(Value::as_str)
        })
        .map(|id| id == agent_id)
        .unwrap_or(false)
}

/// Check if a session has bead metadata
fn has_bead_metadata(metadata: &Value) -> bool {
    metadata.get("bead_id").is_some()
}

/// Check if a session has agent metadata
///
/// Checks both direct and nested locations for agent_id field.
fn has_agent_metadata(metadata: &Value) -> bool {
    metadata.get("agent_id").is_some()
        || metadata
            .get("agent")
            .and_then(|a| a.get("agent_id"))
            .is_some()
}

/// Check if metadata passes all presence filters
///
/// Returns true if metadata matches with_beads and with_agents requirements.
fn passes_presence_filters(metadata: &Value, filter: &ListFilter) -> bool {
    if filter.with_beads && !has_bead_metadata(metadata) {
        return false;
    }

    if filter.with_agents && !has_agent_metadata(metadata) {
        return false;
    }

    true
}

/// Check if metadata passes all exact-match filters
///
/// Returns true if metadata matches bead_id and agent_id requirements.
fn passes_exact_filters(metadata: &Value, filter: &ListFilter) -> bool {
    // Check bead_id filter
    if let Some(ref bead_id) = filter.bead_id {
        if !matches_bead_id_filter(metadata, bead_id) {
            return false;
        }
    }

    // Check agent_id filter
    if let Some(ref agent_id) = filter.agent_id {
        if !matches_agent_id_filter(metadata, agent_id) {
            return false;
        }
    }

    true
}

/// Check if a session should be included based on filters
///
/// Returns true if session metadata exists and passes all filters.
/// Sessions without metadata are only included if no filters are active.
fn should_include_session(session: &Session, filter: &ListFilter) -> bool {
    match &session.metadata {
        Some(metadata) => {
            // Session has metadata: apply exact and presence filters
            passes_exact_filters(metadata, filter) && passes_presence_filters(metadata, filter)
        }
        None => {
            // No metadata: only include if no filters are active
            !filter.with_beads
                && !filter.with_agents
                && filter.bead_id.is_none()
                && filter.agent_id.is_none()
        }
    }
}

/// Apply filters to a list of sessions
///
/// Uses functional patterns with filter/map for clean composition.
/// Filters sessions based on:
/// - `bead_id`: Exact match on bead_id in metadata
/// - `agent_id`: Exact match on agent_id (checks both direct and nested)
/// - `with_beads`: Presence of bead_id in metadata
/// - `with_agents`: Presence of agent_id in metadata
///
/// # Returns
/// Filtered vector of sessions matching all criteria. Sessions without
/// metadata are only included if no filters are active.
#[must_use]
pub fn apply_filters(sessions: Vec<Session>, filter: &ListFilter) -> Vec<Session> {
    sessions
        .into_iter()
        .filter(|session| should_include_session(session, filter))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStatus;

    #[test]
    fn test_matches_bead_id_filter() {
        let metadata = serde_json::json!({"bead_id": "zjj-1234"});
        assert!(matches_bead_id_filter(&metadata, "zjj-1234"));
        assert!(!matches_bead_id_filter(&metadata, "zjj-5678"));
    }

    #[test]
    fn test_matches_agent_id_filter_direct() {
        let metadata = serde_json::json!({"agent_id": "claude-code-1234"});
        assert!(matches_agent_id_filter(&metadata, "claude-code-1234"));
        assert!(!matches_agent_id_filter(&metadata, "other-agent"));
    }

    #[test]
    fn test_matches_agent_id_filter_nested() {
        let metadata = serde_json::json!({"agent": {"agent_id": "claude-code-5678"}});
        assert!(matches_agent_id_filter(&metadata, "claude-code-5678"));
        assert!(!matches_agent_id_filter(&metadata, "other-agent"));
    }

    #[test]
    fn test_has_bead_metadata() {
        let with_bead = serde_json::json!({"bead_id": "zjj-1234"});
        let without_bead = serde_json::json!({"other": "field"});

        assert!(has_bead_metadata(&with_bead));
        assert!(!has_bead_metadata(&without_bead));
    }

    #[test]
    fn test_has_agent_metadata_direct() {
        let with_agent = serde_json::json!({"agent_id": "claude-code-1234"});
        assert!(has_agent_metadata(&with_agent));
    }

    #[test]
    fn test_has_agent_metadata_nested() {
        let with_agent = serde_json::json!({"agent": {"agent_id": "claude-code-1234"}});
        assert!(has_agent_metadata(&with_agent));
    }

    #[test]
    fn test_apply_filters_bead_id() {
        let session1 = Session {
            name: "session1".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/session1".to_string(),
            zellij_tab: "zjj:session1".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: Some(serde_json::json!({"bead_id": "zjj-1234"})),
            ..Default::default()
        };

        let session2 = Session {
            name: "session2".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/session2".to_string(),
            zellij_tab: "zjj:session2".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: Some(serde_json::json!({"bead_id": "zjj-5678"})),
            ..Default::default()
        };

        let sessions = vec![session1, session2];
        let filter = ListFilter {
            bead_id: Some("zjj-1234".to_string()),
            ..Default::default()
        };

        let filtered = apply_filters(sessions, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "session1");
    }

    #[test]
    fn test_apply_filters_with_beads() {
        let session1 = Session {
            name: "session1".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/session1".to_string(),
            zellij_tab: "zjj:session1".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: Some(serde_json::json!({"bead_id": "zjj-1234"})),
            ..Default::default()
        };

        let session2 = Session {
            name: "session2".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/session2".to_string(),
            zellij_tab: "zjj:session2".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: None,
            ..Default::default()
        };

        let sessions = vec![session1, session2];
        let filter = ListFilter {
            with_beads: true,
            ..Default::default()
        };

        let filtered = apply_filters(sessions, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "session1");
    }
}

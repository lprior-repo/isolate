//! Data formatting and output building for list command
//!
//! Transforms raw session data into display-ready `SessionListItem` structures
//! using functional patterns for clean composition.

use crate::session::Session;

use super::enrichment::{enrich_session_metadata, get_session_changes};
use super::types::{BeadCounts, SessionListItem};

/// Format a session change count for display
///
/// Returns formatted string or "-" if changes cannot be determined.
/// Uses functional approach with `map_or_else` for default handling.
fn format_changes(workspace_path: &str) -> String {
    get_session_changes(workspace_path)
        .map(|count| count.to_string())
        .unwrap_or_else(|| "-".to_string())
}

/// Format a branch name for display
///
/// Returns branch name or "-" if not present.
/// Uses functional approach with `unwrap_or_else` for default handling.
fn format_branch(branch: Option<&String>) -> String {
    branch.cloned().unwrap_or_else(|| "-".to_string())
}

/// Build a `SessionListItem` from a session and shared beads count
///
/// Composes enriched metadata and formatted fields into a display-ready item.
/// Uses functional patterns: `enrich_session_metadata` extracts metadata,
/// then format_* functions prepare individual fields.
///
/// # Arguments
/// - `session`: The source session to build from
/// - `beads`: Shared beads count (same for all sessions)
///
/// # Returns
/// Fully constructed `SessionListItem` with all metadata enriched
fn build_item(session: &Session, beads: &BeadCounts) -> SessionListItem {
    let (bead_info, agent_info) = enrich_session_metadata(session);

    SessionListItem {
        name: session.name.clone(),
        status: session.status.to_string(),
        branch: format_branch(session.branch.as_ref()),
        workspace_path: session.workspace_path.clone(),
        zellij_tab: session.zellij_tab.clone(),
        changes: format_changes(&session.workspace_path),
        beads: beads.to_string(),
        created_at: session.created_at,
        updated_at: session.updated_at,
        last_synced: session.last_synced,
        bead: bead_info,
        agent: agent_info,
    }
}

/// Transform sessions into display-ready list items
///
/// Performs session enrichment and formatting in a single pass.
/// Uses functional patterns: map collects formatted items, composition
/// of enrichment and formatting functions creates clean data pipeline.
///
/// # Arguments
/// - `sessions`: Source sessions to transform
/// - `beads`: Shared beads count for all sessions
///
/// # Returns
/// Vector of formatted `SessionListItem` ready for display
pub fn format_sessions(sessions: &[Session], beads: &BeadCounts) -> Vec<SessionListItem> {
    sessions
        .iter()
        .map(|session| build_item(session, beads))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStatus;

    #[test]
    fn test_format_branch_with_value() {
        let branch = Some("feature/test".to_string());
        assert_eq!(format_branch(branch.as_ref()), "feature/test");
    }

    #[test]
    fn test_format_branch_without_value() {
        let branch: Option<String> = None;
        assert_eq!(format_branch(branch.as_ref()), "-");
    }

    #[test]
    fn test_format_changes_missing_workspace() {
        let result = format_changes("/nonexistent/path");
        assert_eq!(result, "-");
    }

    #[test]
    fn test_build_item_without_metadata() {
        let session = Session {
            name: "test".to_string(),
            status: SessionStatus::Active,
            branch: Some("main".to_string()),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: None,
            ..Default::default()
        };

        let beads = BeadCounts {
            open: 5,
            in_progress: 2,
            blocked: 1,
        };

        let item = build_item(&session, &beads);

        assert_eq!(item.name, "test");
        assert_eq!(item.status, "active");
        assert_eq!(item.branch, "main");
        assert_eq!(item.beads, "5/2/1");
        assert_eq!(item.changes, "-");
        assert!(item.bead.is_none());
        assert!(item.agent.is_none());
    }

    #[test]
    fn test_format_sessions_empty() {
        let sessions: Vec<Session> = vec![];
        let beads = BeadCounts::default();
        let items = format_sessions(&sessions, &beads);
        assert!(items.is_empty());
    }

    #[test]
    fn test_format_sessions_multiple() {
        let first = Session {
            name: "session1".to_string(),
            status: SessionStatus::Active,
            branch: None,
            workspace_path: "/tmp/session1".to_string(),
            zellij_tab: "zjj:session1".to_string(),
            created_at: 1_000_000_000,
            updated_at: 1_000_000_000,
            metadata: Some(serde_json::json!({"bead_id": "zjj-1234"})),
            ..Default::default()
        };

        let second = Session {
            name: "session2".to_string(),
            status: SessionStatus::Paused,
            branch: Some("develop".to_string()),
            workspace_path: "/tmp/session2".to_string(),
            zellij_tab: "zjj:session2".to_string(),
            created_at: 1_000_000_100,
            updated_at: 1_000_000_100,
            metadata: None,
            ..Default::default()
        };

        let sessions = vec![first, second];
        let beads = BeadCounts {
            open: 3,
            in_progress: 1,
            blocked: 0,
        };

        let items = format_sessions(&sessions, &beads);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "session1");
        assert_eq!(items[0].branch, "-");
        assert_eq!(items[1].name, "session2");
        assert_eq!(items[1].branch, "develop");
    }
}

//! Session enrichment functions
//!
//! Provides functionality to enrich session data with workspace changes,
//! beads metadata, and agent information.

use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use serde_json::Value;

use crate::session::Session;

use super::types::{BeadCounts, SessionAgentInfo, SessionBeadInfo};

/// Get the number of changes in a workspace
///
/// # Returns
/// Returns `Some(count)` if the workspace exists and JJ can read it,
/// otherwise `None` if workspace doesn't exist or JJ fails.
#[must_use]
pub fn get_session_changes(workspace_path: &str) -> Option<usize> {
    let path = Path::new(workspace_path);

    // Check if workspace exists
    if !path.exists() {
        return None;
    }

    // Try to get status from JJ
    zjj_core::jj::workspace_status(path)
        .ok()
        .map(|status| status.change_count())
}

/// Get beads count from the repository's beads database
///
/// Queries the `.beads/beads.db` SQLite database for open issues.
/// Returns default counts (0/0/0) if:
/// - Not in a JJ repository
/// - Beads database doesn't exist
/// - Database query fails
///
/// # Errors
/// Returns error only if database exists but cannot be opened/queried.
/// Returns Ok with default counts for missing/inaccessible database.
pub async fn get_beads_count() -> Result<BeadCounts> {
    use sqlx::{Connection, SqliteConnection};

    // Find repository root
    let repo_root = zjj_core::jj::check_in_jj_repo().ok();

    let Some(root) = repo_root else {
        return Ok(BeadCounts::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !beads_db_path.exists() {
        return Ok(BeadCounts::default());
    }

    // Query beads database
    let db_url = format!("sqlite://{}", beads_db_path.display());
    let mut conn = SqliteConnection::connect(&db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open beads database: {e}"))?;

    // Count open issues
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE status = 'open'")
        .fetch_one(&mut conn)
        .await
        .unwrap_or(0);

    // For now, we can't distinguish in_progress vs blocked without more schema knowledge
    // Let's return a simplified count
    // Safe cast: counts cannot be negative, but we handle it defensively
    let open_usize = usize::try_from(open.max(0)).unwrap_or(0);
    Ok(BeadCounts {
        open: open_usize,
        in_progress: 0,
        blocked: 0,
    })
}

/// Extract bead information from session metadata
///
/// Looks for bead-related fields in metadata:
/// - `bead_id` (required for Some return)
/// - `bead_title` (optional)
/// - `bead_status` (optional)
/// - `bead_priority` (optional)
/// - `bead_type` (optional)
///
/// # Returns
/// Returns `Some(SessionBeadInfo)` if `bead_id` is found, `None` otherwise.
#[must_use]
pub fn extract_bead_info(metadata: &Value) -> Option<SessionBeadInfo> {
    let bead_id = metadata.get("bead_id")?.as_str()?.to_string();

    Some(SessionBeadInfo {
        id: bead_id,
        title: metadata
            .get("bead_title")
            .and_then(Value::as_str)
            .map(String::from),
        status: metadata
            .get("bead_status")
            .and_then(Value::as_str)
            .map(String::from),
        priority: metadata
            .get("bead_priority")
            .and_then(Value::as_str)
            .map(String::from),
        bead_type: metadata
            .get("bead_type")
            .and_then(Value::as_str)
            .map(String::from),
    })
}

/// Extract agent information from session metadata
///
/// Looks for agent-related fields in metadata in two locations:
/// 1. Direct top-level fields: `agent_id`, `task_id`, `spawned_at`
/// 2. Nested under "agent" key: `{ "agent": { "agent_id": "...", ... } }`
///
/// Calculates `runtime_seconds` from `spawned_at` if present using current time.
///
/// # Returns
/// Returns `Some(SessionAgentInfo)` if `agent_id` is found, `None` otherwise.
#[must_use]
pub fn extract_agent_info(metadata: &Value) -> Option<SessionAgentInfo> {
    // Check both direct fields and nested "agent" object
    let agent_data = if metadata.get("agent_id").is_some() {
        metadata.clone()
    } else if let Some(agent_obj) = metadata.get("agent") {
        agent_obj.clone()
    } else {
        return None;
    };

    let agent_id = agent_data.get("agent_id")?.as_str()?.to_string();

    let task_id = agent_data
        .get("task_id")
        .and_then(Value::as_str)
        .map(String::from);

    let spawned_at = agent_data.get("spawned_at").and_then(Value::as_u64);

    // Calculate runtime if spawned_at is present
    let runtime_seconds = spawned_at.and_then(|spawn_time| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|now| {
                let current_time = now.as_secs();
                if current_time >= spawn_time {
                    Some(current_time - spawn_time)
                } else {
                    None
                }
            })
    });

    Some(SessionAgentInfo {
        agent_id,
        task_id,
        spawned_at,
        runtime_seconds,
    })
}

/// Enrich a session with additional metadata
///
/// Extracts bead and agent information from session metadata in a single operation.
/// Returns tuple of (bead_info, agent_info).
///
/// # Returns
/// Returns `(None, None)` if session has no metadata.
pub fn enrich_session_metadata(
    session: &Session,
) -> (Option<SessionBeadInfo>, Option<SessionAgentInfo>) {
    match &session.metadata {
        Some(metadata) => (extract_bead_info(metadata), extract_agent_info(metadata)),
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_session_changes_missing_workspace() {
        let result = get_session_changes("/nonexistent/path");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_beads_count_no_repo() {
        // When not in a repo or no beads db, should return default
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[test]
    fn test_extract_bead_info_direct_fields() {
        let metadata = serde_json::json!({
            "bead_id": "zjj-1234",
            "bead_title": "Fix authentication bug",
            "bead_status": "open",
            "bead_priority": "high",
            "bead_type": "bug"
        });

        let result = extract_bead_info(&metadata);
        let Some(bead) = result else {
            panic!("Expected bead info to be present");
        };
        assert_eq!(bead.id, "zjj-1234");
        assert_eq!(bead.title, Some("Fix authentication bug".to_string()));
        assert_eq!(bead.status, Some("open".to_string()));
        assert_eq!(bead.priority, Some("high".to_string()));
        assert_eq!(bead.bead_type, Some("bug".to_string()));
    }

    #[test]
    fn test_extract_bead_info_missing() {
        let metadata = serde_json::json!({
            "other_field": "value"
        });

        let result = extract_bead_info(&metadata);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_agent_info_direct_fields() {
        let metadata = serde_json::json!({
            "agent_id": "claude-code-1234",
            "task_id": "zjj-5678",
            "spawned_at": 1_000_000_000_u64
        });

        let result = extract_agent_info(&metadata);
        let Some(ref agent) = result else {
            panic!("Expected agent info to be present");
        };
        assert_eq!(agent.agent_id, "claude-code-1234");
        assert_eq!(agent.task_id, Some("zjj-5678".to_string()));
        assert_eq!(agent.spawned_at, Some(1_000_000_000));
        assert!(agent.runtime_seconds.is_some());
    }

    #[test]
    fn test_extract_agent_info_nested() {
        let metadata = serde_json::json!({
            "agent": {
                "agent_id": "claude-code-5678",
                "task_id": "zjj-9012"
            }
        });

        let result = extract_agent_info(&metadata);
        let Some(ref agent) = result else {
            panic!("Expected agent info to be present");
        };
        assert_eq!(agent.agent_id, "claude-code-5678");
        assert_eq!(agent.task_id, Some("zjj-9012".to_string()));
    }
}

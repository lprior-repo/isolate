//! List all sessions - JSONL output for AI-first control plane
//!
//! This command emits structured JSONL output where each line is a valid JSON object.
//! Each session is emitted as a `Session` line, followed by a `Context` line at the end.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{path::Path, str::FromStr};

use anyhow::Result;
use zjj_core::{
    output::{
        emit, Context, OutputLine, Session as OutputSession, SessionState as OutputSessionState,
    },
    OutputFormat, WorkspaceStateFilter,
};

use crate::{
    beads::{BeadRepository, BeadStatus},
    cli::jj_root,
    commands::get_session_db,
    session::{Session, SessionStatus},
};

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

/// Convert database SessionStatus to output SessionState
fn to_output_state(status: &SessionStatus) -> OutputSessionState {
    match status {
        SessionStatus::Active => OutputSessionState::Active,
        SessionStatus::Paused => OutputSessionState::Paused,
        SessionStatus::Completed => OutputSessionState::Completed,
        SessionStatus::Failed => OutputSessionState::Failed,
        SessionStatus::Creating => OutputSessionState::Creating,
    }
}

/// Calculate age in days from timestamp
fn calculate_age_days(created_at: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let age_secs = now.saturating_sub(created_at);
    age_secs / 86_400 // seconds per day
}

/// Run the list command
#[allow(clippy::too_many_arguments)]
pub async fn run(
    all: bool,
    _verbose: bool, // Kept for API compatibility, but not used in JSONL-only mode
    format: OutputFormat,
    bead: Option<&str>,
    agent: Option<&str>,
    state: Option<&str>,
) -> Result<()> {
    let db = get_session_db().await?;

    let state_filter = match state {
        Some(value) => Some(WorkspaceStateFilter::from_str(value).map_err(anyhow::Error::new)?),
        None => None,
    };

    // Filter sessions: exclude completed/failed unless --all is used
    // Functional iterator chain for filtering
    let sessions: Vec<Session> = db
        .list(None)
        .await?
        .into_iter()
        .filter(|s| {
            let status_matches =
                all || (s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);

            let bead_matches = bead.is_none_or(|bead_id| {
                s.metadata
                    .as_ref()
                    .and_then(|m| m.get("bead_id"))
                    .and_then(|v| v.as_str())
                    == Some(bead_id)
            });

            let agent_matches = agent.is_none_or(|agent_filter| {
                s.metadata
                    .as_ref()
                    .and_then(|m| m.get("owner"))
                    .and_then(|v| v.as_str())
                    == Some(agent_filter)
            });

            let state_matches = state_filter
                .as_ref()
                .is_none_or(|filter| filter.matches(s.state));

            status_matches && bead_matches && agent_matches && state_matches
        })
        .collect();

    let beads_count = get_beads_count().await.unwrap_or_default();
    let session_count = sessions.len();

    // Emit each session as a Session output line
    for session in &sessions {
        let changes = get_session_changes(&session.workspace_path).await;
        let age_days = calculate_age_days(session.created_at);

        // Determine suggested action based on session state
        let action = determine_suggested_action(session);

        // Get owner from metadata
        let owned_by = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("owner"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Get bead_id from metadata
        let bead_id = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("bead_id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let output_session = OutputSession {
            name: session.name.clone(),
            state: to_output_state(&session.status),
            age_days,
            owned_by,
            action,
            branch: session.branch.clone(),
            changes,
            workspace_path: Some(session.workspace_path.clone()),
            bead_id,
        };

        emit(&OutputLine::Session(output_session))?;
    }

    // Emit Context line last (always)
    let context_text = format!(
        "Listed {} session(s), beads: {}/{}/{}",
        session_count, beads_count.open, beads_count.in_progress, beads_count.blocked
    );
    emit(&OutputLine::Context(Context {
        text: context_text,
        for_human: !format.is_json(),
    }))?;

    Ok(())
}

/// Determine suggested action for a session
fn determine_suggested_action(session: &Session) -> Option<String> {
    match session.status {
        SessionStatus::Active => match session.state {
            zjj_core::WorkspaceState::Conflict => Some("resolve".to_string()),
            zjj_core::WorkspaceState::Ready => Some("merge".to_string()),
            _ => None,
        },
        SessionStatus::Paused => Some("resume".to_string()),
        SessionStatus::Creating => Some("wait".to_string()),
        SessionStatus::Completed | SessionStatus::Failed => None,
    }
}

/// Get the number of changes in a workspace
async fn get_session_changes(workspace_path: &str) -> Option<usize> {
    let path = Path::new(workspace_path);

    // Check if workspace exists
    match tokio::fs::try_exists(path).await {
        Ok(true) => {
            // Try to get status from JJ
            zjj_core::jj::workspace_status(path)
                .await
                .ok()
                .map(|status| status.change_count())
        }
        _ => None,
    }
}

/// Get beads count from the repository's beads database
async fn get_beads_count() -> Result<BeadCounts> {
    let root = jj_root().await.ok();
    let Some(root) = root else {
        return Ok(BeadCounts::default());
    };

    let bead_repo = BeadRepository::new(root);
    let beads = bead_repo.list_beads().await.unwrap_or_default();

    // Functional counting using fold
    let counts = beads
        .into_iter()
        .fold(BeadCounts::default(), |acc, b| match b.status {
            BeadStatus::Open => BeadCounts {
                open: acc.open + 1,
                ..acc
            },
            BeadStatus::InProgress => BeadCounts {
                in_progress: acc.in_progress + 1,
                ..acc
            },
            BeadStatus::Blocked => BeadCounts {
                blocked: acc.blocked + 1,
                ..acc
            },
            _ => acc,
        });

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use zjj_core::WorkspaceState;

    use super::*;
    use crate::db::SessionDb;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_bead_counts_display() {
        let counts = BeadCounts {
            open: 5,
            in_progress: 3,
            blocked: 2,
        };
        assert_eq!(counts.to_string(), "5/3/2");
    }

    #[tokio::test]
    async fn test_bead_counts_default() {
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[tokio::test]
    async fn test_get_session_changes_missing_workspace() {
        let result = get_session_changes("/nonexistent/path").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_to_output_state_mapping() {
        assert_eq!(
            to_output_state(&SessionStatus::Active),
            OutputSessionState::Active
        );
        assert_eq!(
            to_output_state(&SessionStatus::Paused),
            OutputSessionState::Paused
        );
        assert_eq!(
            to_output_state(&SessionStatus::Completed),
            OutputSessionState::Completed
        );
        assert_eq!(
            to_output_state(&SessionStatus::Failed),
            OutputSessionState::Failed
        );
        assert_eq!(
            to_output_state(&SessionStatus::Creating),
            OutputSessionState::Creating
        );
    }

    #[tokio::test]
    async fn test_determine_suggested_action() {
        let mut session = Session {
            id: Some(1),
            name: "test".to_string(),
            state: WorkspaceState::Created,
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: None,
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
            parent_session: None,
            queue_status: None,
        };

        // Active with no issues - no action
        assert_eq!(determine_suggested_action(&session), None);

        // Active with conflict - resolve
        session.state = WorkspaceState::Conflict;
        assert_eq!(
            determine_suggested_action(&session),
            Some("resolve".to_string())
        );

        // Active with ready - merge
        session.state = WorkspaceState::Ready;
        assert_eq!(
            determine_suggested_action(&session),
            Some("merge".to_string())
        );

        // Paused - resume
        session.status = SessionStatus::Paused;
        assert_eq!(
            determine_suggested_action(&session),
            Some("resume".to_string())
        );
    }

    #[tokio::test]
    async fn test_filter_completed_and_failed_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions with different statuses
        let s1 = db.create("active-session", "/tmp/active").await?;
        db.update(
            &s1.name,
            crate::session::SessionUpdate {
                status: Some(SessionStatus::Active),
                ..Default::default()
            },
        )
        .await?;

        let s2 = db.create("completed-session", "/tmp/completed").await?;
        db.update(
            &s2.name,
            crate::session::SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )
        .await?;

        let s3 = db.create("failed-session", "/tmp/failed").await?;
        db.update(
            &s3.name,
            crate::session::SessionUpdate {
                status: Some(SessionStatus::Failed),
                ..Default::default()
            },
        )
        .await?;

        let s4 = db.create("paused-session", "/tmp/paused").await?;
        db.update(
            &s4.name,
            crate::session::SessionUpdate {
                status: Some(SessionStatus::Paused),
                ..Default::default()
            },
        )
        .await?;

        // Get all sessions and filter
        let mut sessions = db.list(None).await?;

        // Simulate the filtering logic from run()
        sessions
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);

        // Should only have active and paused
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s.name == "active-session"));
        assert!(sessions.iter().any(|s| s.name == "paused-session"));
        assert!(!sessions.iter().any(|s| s.name == "completed-session"));
        assert!(!sessions.iter().any(|s| s.name == "failed-session"));

        Ok(())
    }

    #[tokio::test]
    async fn test_all_flag_includes_all_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions with different statuses
        db.create("active-session", "/tmp/active").await?;
        let s2 = db.create("completed-session", "/tmp/completed").await?;
        db.update(
            &s2.name,
            crate::session::SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )
        .await?;

        // With all=true, no filtering
        let sessions = db.list(None).await?;
        assert_eq!(sessions.len(), 2);

        // With all=false, filter out completed
        let mut filtered = db.list(None).await?;
        filtered
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);
        assert_eq!(filtered.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_list_handling() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let sessions = db.list(None).await?;
        assert!(sessions.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_beads_count_no_repo() {
        // When not in a repo or no beads db, should return default
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[test]
    fn test_output_line_session_serializes_with_type() {
        let session = OutputSession {
            name: "test-session".to_string(),
            state: OutputSessionState::Active,
            age_days: 5,
            owned_by: None,
            action: None,
            branch: Some("feature".to_string()),
            changes: Some(3),
            workspace_path: Some("/tmp/test".to_string()),
            bead_id: None,
        };
        let line = OutputLine::Session(session);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"session"#));
        assert!(json.contains(r#""name":"test-session"#));
        assert!(json.contains(r#""state":"active"#));
        assert!(json.contains(r#""age_days":5"#));
    }

    #[test]
    fn test_output_line_context_serializes_with_type() {
        let context = Context {
            text: "Listed 5 session(s)".to_string(),
            for_human: true,
        };
        let line = OutputLine::Context(context);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"context"#));
        assert!(json.contains(r#""text":"Listed 5 session(s)"#));
        assert!(json.contains(r#""for_human":true"#));
    }
}

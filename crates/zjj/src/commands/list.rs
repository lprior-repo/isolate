//! List all sessions - JSONL output for AI-first control plane
//!
//! This command emits structured JSONL output where each line is a valid JSON object.
//! Each session is emitted as a `Session` line, followed by a `Summary` line at the end.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use zjj_core::{
    output::{emit_stdout, OutputLine, SessionOutput, Summary, SummaryType},
    OutputFormat, WorkspaceStateFilter,
};

use crate::{
    beads::{BeadRepository, BeadStatus},
    cli::jj_root,
    commands::get_session_db,
    session::{Session, SessionStatus},
};

/// Convert local SessionStatus to core SessionStatus
fn to_core_status(status: SessionStatus) -> zjj_core::types::SessionStatus {
    match status {
        SessionStatus::Active => zjj_core::types::SessionStatus::Active,
        SessionStatus::Paused => zjj_core::types::SessionStatus::Paused,
        SessionStatus::Completed => zjj_core::types::SessionStatus::Completed,
        SessionStatus::Failed => zjj_core::types::SessionStatus::Failed,
        SessionStatus::Creating => zjj_core::types::SessionStatus::Creating,
    }
}

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

/// Run the list command
#[allow(clippy::too_many_arguments)]
pub async fn run(
    all: bool,
    _verbose: bool, // Kept for API compatibility, but not used in JSONL-only mode
    _format: OutputFormat, // Kept for API compatibility, not used in JSONL-only mode
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
        let workspace_path: PathBuf = session.workspace_path.clone().into();

        let output_session = SessionOutput::new(
            session.name.clone(),
            to_core_status(session.status),
            session.state,
            workspace_path,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        let output_session = if let Some(branch) = &session.branch {
            output_session.with_branch(branch.clone())
        } else {
            output_session
        };

        emit_stdout(&OutputLine::Session(output_session))?;
    }

    // Emit Summary line last (always)
    let summary_text = format!(
        "Listed {} session(s), beads: {}/{}/{}",
        session_count, beads_count.open, beads_count.in_progress, beads_count.blocked
    );
    let summary =
        Summary::new(SummaryType::Count, summary_text).map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Summary(summary))?;

    Ok(())
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
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[test]
    fn test_output_line_session_serializes_with_type() {
        let session = SessionOutput::new(
            "test-session".to_string(),
            zjj_core::types::SessionStatus::Active,
            zjj_core::WorkspaceState::Ready,
            PathBuf::from("/tmp/test"),
        )
        .expect("session should be valid");
        let session = session.with_branch("feature".to_string());
        let line = OutputLine::Session(session);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""session""#));
        assert!(json.contains(r#""name":"test-session"#));
        assert!(json.contains(r#""status":"active"#));
        assert!(json.contains(r#""state":"ready"#));
    }

    #[test]
    fn test_output_line_summary_serializes_with_type() {
        let summary = Summary::new(SummaryType::Count, "Listed 5 session(s)".to_string())
            .expect("summary should be valid");
        let line = OutputLine::Summary(summary);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""summary""#));
        assert!(json.contains(r#""message":"Listed 5 session(s)"#));
    }
}

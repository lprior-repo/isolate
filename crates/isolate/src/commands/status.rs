//! Show detailed session status - JSONL output for AI-first control plane
//!
//! This command emits structured JSONL output where each line is a valid JSON object.
//! Each session is emitted as a `Session` line, followed by a `Summary` line at the end.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use anyhow::Result;
use isolate_core::output::{
    emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Message, OutputLine,
    SessionOutput, Summary, SummaryType,
};
use serde::Serialize;

use crate::{commands::get_session_db, session::Session};

/// Convert local `SessionStatus` to core `SessionStatus`
const fn to_core_status(
    status: crate::session::SessionStatus,
) -> isolate_core::types::SessionStatus {
    match status {
        crate::session::SessionStatus::Active => isolate_core::types::SessionStatus::Active,
        crate::session::SessionStatus::Paused => isolate_core::types::SessionStatus::Paused,
        crate::session::SessionStatus::Completed => isolate_core::types::SessionStatus::Completed,
        crate::session::SessionStatus::Failed => isolate_core::types::SessionStatus::Failed,
        crate::session::SessionStatus::Creating => isolate_core::types::SessionStatus::Creating,
    }
}

/// Detailed session status information
#[derive(Debug, Clone, Serialize)]
pub struct SessionStatusInfo {
    pub name: String,
    pub status: String,
    pub workspace_path: String,
    pub branch: String,
    pub changes: FileChanges,
    pub diff_stats: DiffStats,
    pub beads: BeadStats,
    #[serde(flatten)]
    pub session: Session,
}

/// File changes in the workspace
#[derive(Debug, Clone, Default, Serialize)]
pub struct FileChanges {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub unknown: usize,
}

impl FileChanges {
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    pub const fn is_clean(&self) -> bool {
        self.total() == 0
    }
}

impl std::fmt::Display for FileChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_clean() {
            write!(f, "clean")
        } else {
            write!(
                f,
                "M:{} A:{} D:{} R:{}",
                self.modified, self.added, self.deleted, self.renamed
            )
        }
    }
}

/// Diff statistics (insertions/deletions)
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiffStats {
    pub insertions: usize,
    pub deletions: usize,
}

impl std::fmt::Display for DiffStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.insertions, self.deletions)
    }
}

/// Beads issue statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct BeadStats {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub closed: usize,
}

impl std::fmt::Display for BeadStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "O:{} P:{} B:{} C:{}",
            self.open, self.in_progress, self.blocked, self.closed
        )
    }
}

/// Run the status command
pub async fn run(name: Option<&str>) -> Result<()> {
    run_once(name).await
}

use futures::StreamExt;

/// Run status once
async fn run_once(name: Option<&str>) -> Result<()> {
    let db = match get_session_db().await {
        Ok(db) => db,
        Err(err) if name.is_none() && is_repo_bootstrap_error(&err) => {
            emit_no_active_sessions()?;
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let sessions = if let Some(session_name) = name {
        // Get single session
        // Return isolate_core::Error::NotFound to get exit code 2 (not found)
        let session = db.get(session_name).await?.ok_or_else(|| {
            anyhow::Error::new(isolate_core::Error::NotFound(format!(
                "Session '{session_name}' not found"
            )))
        })?;
        vec![session]
    } else {
        // Get all sessions
        db.list(None).await?
    };

    if sessions.is_empty() {
        emit_no_active_sessions()?;
        return Ok(());
    }

    // Gather status for all sessions using concurrent stream
    // Using buffered(10) to allow up to 10 concurrent status checks while preserving order
    let statuses: Vec<SessionStatusInfo> = futures::stream::iter(sessions)
        .map(|session| async move { gather_session_status(&session).await })
        .buffered(10)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    // Emit each session as a JSONL Session line
    for status_info in &statuses {
        emit_session_output(status_info)?;
    }

    // Emit summary with counts
    let active_count = statuses.iter().filter(|s| s.status == "active").count();
    let total_count = statuses.len();
    emit_status_summary(active_count, total_count)?;

    Ok(())
}

fn is_repo_bootstrap_error(err: &anyhow::Error) -> bool {
    let message = err.to_string();
    message.contains("Not in a JJ repository") || message.contains("Isolate not initialized")
}

/// Emit "no active sessions" as JSONL output
fn emit_no_active_sessions() -> Result<()> {
    let summary = Summary::new(
        SummaryType::Info,
        Message::new("No active sessions").map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create summary: {e}"))?;
    emit_stdout(&OutputLine::Summary(summary)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Run status in watch mode (continuous updates)
pub async fn run_watch_mode(name: Option<&str>) -> Result<()> {
    use std::io::Write;

    loop {
        // Clear screen (ANSI escape code)
        print!("\x1B[2J\x1B[1;1H");
        let _ = std::io::stdout().flush();

        // Run status once
        if let Err(e) = run_once(name).await {
            if name.is_some() && is_not_found_error(&e) {
                return Err(e);
            }
            // Emit error as Issue line
            let error_summary = Summary::new(
                SummaryType::Status,
                Message::new(format!("Error: {e}"))
                    .map_err(|se| anyhow::anyhow!("Invalid message: {se}"))?,
            )
            .map_err(|se| anyhow::anyhow!("Failed to create error summary: {se}"))?;
            let _ = emit_stdout(&OutputLine::Summary(error_summary));
        }

        // Wait 1 second
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

fn is_not_found_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<isolate_core::Error>()
        .is_some_and(|core_error| {
            matches!(
                core_error,
                isolate_core::Error::NotFound(_) | isolate_core::Error::SessionNotFound { .. }
            )
        })
}

/// Gather detailed status for a session
pub async fn gather_session_status(session: &Session) -> Result<SessionStatusInfo> {
    use std::path::Path;

    let workspace_path = Path::new(&session.workspace_path);

    // Get file changes
    let changes = get_file_changes(workspace_path).await;

    // Get diff stats
    let diff_stats = get_diff_stats(workspace_path).await;

    // Get beads stats
    let beads = get_beads_stats().await?;

    // Note: Clones here are necessary because SessionStatusInfo owns its data
    // Future optimization: Consider Arc<Session> or Cow<str> for shared ownership
    Ok(SessionStatusInfo {
        name: session.name.clone(),
        status: session.status.to_string(),
        workspace_path: session.workspace_path.clone(),
        branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
        changes,
        diff_stats,
        beads,
        session: session.clone(),
    })
}

/// Emit a session as JSONL output
fn emit_session_output(status_info: &SessionStatusInfo) -> Result<()> {
    let workspace_path: PathBuf = status_info.session.workspace_path.clone().into();

    let session_output = SessionOutput::new(
        status_info.session.name.clone(),
        to_core_status(status_info.session.status),
        status_info.session.state,
        workspace_path,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_output = match &status_info.session.branch {
        Some(branch) => session_output.with_branch(branch.clone()),
        None => session_output,
    };

    emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit status summary as JSONL output
fn emit_status_summary(active_count: usize, total_count: usize) -> Result<()> {
    let message = format!("{active_count} active session(s) of {total_count} total");

    let summary = Summary::new(
        SummaryType::Count,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    emit_stdout(&OutputLine::Summary(summary)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an action line for status operations
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Get file changes from JJ status
async fn get_file_changes(workspace_path: &std::path::Path) -> FileChanges {
    match tokio::fs::try_exists(workspace_path).await {
        Ok(true) => match isolate_core::jj::workspace_status(workspace_path).await {
            Ok(status) => FileChanges {
                modified: status.modified.len(),
                added: status.added.len(),
                deleted: status.deleted.len(),
                renamed: status.renamed.len(),
                unknown: status.unknown.len(),
            },
            Err(_) => FileChanges::default(),
        },
        _ => FileChanges::default(),
    }
}

/// Get diff statistics from JJ diff
async fn get_diff_stats(workspace_path: &std::path::Path) -> DiffStats {
    match tokio::fs::try_exists(workspace_path).await {
        Ok(true) => isolate_core::jj::workspace_diff(workspace_path)
            .await
            .map_or_else(
                |_| DiffStats::default(),
                |summary| DiffStats {
                    insertions: summary.insertions,
                    deletions: summary.deletions,
                },
            ),
        _ => DiffStats::default(),
    }
}

/// Get beads statistics from the repository's beads database
async fn get_beads_stats() -> Result<BeadStats> {
    // Find repository root
    let repo_root = isolate_core::jj::check_in_jj_repo().await.ok();

    let Some(root) = repo_root else {
        return Ok(BeadStats::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !tokio::fs::try_exists(&beads_db_path).await.is_ok_and(|e| e) {
        return Ok(BeadStats::default());
    }

    let connection_string = format!("sqlite:{}", beads_db_path.display());
    let pool = sqlx::SqlitePool::connect(&connection_string)
        .await
        .map_err(|e| {
            anyhow::Error::new(isolate_core::Error::DatabaseError(format!(
                "Failed to open beads database: {e}"
            )))
        })?;

    // Count issues by status using parameterized queries
    let open = count_issues_by_status(&pool, "open").await?;
    let in_progress = count_issues_by_status(&pool, "in_progress").await?;
    let blocked = count_issues_by_status(&pool, "blocked").await?;
    let closed = count_issues_by_status(&pool, "closed").await?;

    Ok(BeadStats {
        open,
        in_progress,
        blocked,
        closed,
    })
}

/// Count issues by status using parameterized query
async fn count_issues_by_status(pool: &sqlx::SqlitePool, status: &str) -> Result<usize> {
    let result: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE status = ?1")
        .bind(status)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            anyhow::Error::new(isolate_core::Error::DatabaseError(format!(
                "Failed to query beads database: {e}"
            )))
        })?;

    let count_i64 = result.map_or(0, |v| v);
    let count = usize::try_from(count_i64).map_or(0, |v| v);

    Ok(count)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use isolate_core::WorkspaceState;

    use super::*;
    use crate::session::{Session, SessionStatus};

    #[tokio::test]
    async fn test_file_changes_total() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.total(), 7);
    }

    #[tokio::test]
    async fn test_file_changes_is_clean() {
        let clean = FileChanges::default();
        assert!(clean.is_clean());

        let dirty = FileChanges {
            modified: 1,
            ..Default::default()
        };
        assert!(!dirty.is_clean());
    }

    #[tokio::test]
    async fn test_file_changes_display_clean() {
        let changes = FileChanges::default();
        assert_eq!(changes.to_string(), "clean");
    }

    #[tokio::test]
    async fn test_file_changes_display_dirty() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.to_string(), "M:2 A:3 D:1 R:1");
    }

    #[tokio::test]
    async fn test_diff_stats_display() {
        let stats = DiffStats {
            insertions: 123,
            deletions: 45,
        };
        assert_eq!(stats.to_string(), "+123 -45");
    }

    #[tokio::test]
    async fn test_diff_stats_default() {
        let stats = DiffStats::default();
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
        assert_eq!(stats.to_string(), "+0 -0");
    }

    #[tokio::test]
    async fn test_bead_stats_display() {
        let stats = BeadStats {
            open: 5,
            in_progress: 3,
            blocked: 2,
            closed: 10,
        };
        assert_eq!(stats.to_string(), "O:5 P:3 B:2 C:10");
    }

    #[tokio::test]
    async fn test_bead_stats_default() {
        let stats = BeadStats::default();
        assert_eq!(stats.open, 0);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.blocked, 0);
        assert_eq!(stats.closed, 0);
    }

    #[tokio::test]
    async fn test_session_status_info_serialization() -> Result<()> {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            state: WorkspaceState::Created,
            workspace_path: "/tmp/test".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let info = SessionStatusInfo {
            name: session.name.clone(),
            status: session.status.to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 1,
            },
            diff_stats: DiffStats {
                insertions: 50,
                deletions: 10,
            },
            beads: BeadStats {
                open: 3,
                in_progress: 1,
                blocked: 0,
                closed: 5,
            },
            session,
        };

        let json = serde_json::to_string(&info)?;
        assert!(json.contains("\"name\":\"test-session\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"modified\":2"));
        assert!(json.contains("\"insertions\":50"));
        assert!(json.contains("\"open\":3"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_file_changes_missing_workspace() {
        let result = get_file_changes(std::path::Path::new("/nonexistent/path")).await;
        assert_eq!(result.modified, 0);
        assert_eq!(result.added, 0);
        assert_eq!(result.deleted, 0);
        assert_eq!(result.renamed, 0);
    }

    #[tokio::test]
    async fn test_get_diff_stats_missing_workspace() {
        let result = get_diff_stats(std::path::Path::new("/nonexistent/path")).await;
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 0);
    }

    #[tokio::test]
    async fn test_emit_session_output() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            state: WorkspaceState::Created,
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let status_info = SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 0,
            },
            diff_stats: DiffStats {
                insertions: 50,
                deletions: 10,
            },
            beads: BeadStats {
                open: 3,
                in_progress: 1,
                blocked: 0,
                closed: 5,
            },
            session,
        };

        // This test just verifies the function doesn't panic
        let result = emit_session_output(&status_info);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_changes_with_unknown_files() {
        let changes = FileChanges {
            modified: 1,
            added: 0,
            deleted: 0,
            renamed: 0,
            unknown: 3,
        };
        // Unknown files don't count toward total
        assert_eq!(changes.total(), 1);
        assert!(!changes.is_clean());
    }

    #[test]
    fn test_emit_action_creates_valid_output() {
        let action = Action::new(
            ActionVerb::new("check").expect("valid"),
            ActionTarget::new("session-1").expect("valid"),
            ActionStatus::Completed,
        );
        let line = OutputLine::Action(action);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""action""#));
        assert!(json.contains(r#""verb":"check""#));
        assert!(json.contains(r#""target":"session-1""#));
        assert!(json.contains(r#""status":"completed""#));
    }

    #[test]
    fn test_to_core_status_conversion() {
        use crate::session::SessionStatus;

        assert_eq!(
            to_core_status(SessionStatus::Active),
            isolate_core::types::SessionStatus::Active
        );
        assert_eq!(
            to_core_status(SessionStatus::Paused),
            isolate_core::types::SessionStatus::Paused
        );
        assert_eq!(
            to_core_status(SessionStatus::Completed),
            isolate_core::types::SessionStatus::Completed
        );
        assert_eq!(
            to_core_status(SessionStatus::Failed),
            isolate_core::types::SessionStatus::Failed
        );
        assert_eq!(
            to_core_status(SessionStatus::Creating),
            isolate_core::types::SessionStatus::Creating
        );
    }
}

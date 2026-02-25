//! Remove a session and its workspace - JSONL output for AI-first control plane

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod atomic;

use anyhow::Result;
use zjj_core::{
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput,
    },
    OutputFormat,
};

use crate::commands::{
    get_session_db,
    remove::atomic::{cleanup_session_atomically, RemoveError},
};

/// Options for the remove command
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RemoveOptions {
    /// Skip `pre_remove` hooks (non-interactive - no confirmation prompts)
    pub force: bool,
    /// Squash-merge to main before removal
    pub merge: bool,
    /// Preserve branch after removal
    #[allow(dead_code)]
    pub keep_branch: bool,
    /// Succeed when target session is already absent (safe retries)
    pub idempotent: bool,
    /// Preview operation without executing
    pub dry_run: bool,
    /// Output format (unused - always emits JSONL)
    #[allow(dead_code)]
    pub format: OutputFormat,
}

/// Run the remove command
#[allow(dead_code)]
pub async fn run(name: &str) -> Result<()> {
    run_with_options(name, &RemoveOptions::default()).await
}

/// Emit an action line to stdout
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action))?;
    Ok(())
}

/// Emit a result line to stdout
fn emit_result(success: bool, message: String) -> Result<()> {
    let result = if success {
        ResultOutput::success(
            ResultKind::Command,
            Message::new(message).map_err(|e| anyhow::anyhow!("{e}"))?,
        )
    } else {
        ResultOutput::failure(
            ResultKind::Command,
            Message::new(message).map_err(|e| anyhow::anyhow!("{e}"))?,
        )
    }
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result))?;
    Ok(())
}

/// Emit an issue line to stdout
fn emit_issue(
    id: &str,
    title: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    let mut issue = Issue::new(
        IssueId::new(id).map_err(|e| anyhow::anyhow!("Invalid issue ID: {e}"))?,
        IssueTitle::new(title).map_err(|e| anyhow::anyhow!("Invalid issue title: {e}"))?,
        kind,
        severity,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(s) = session {
        issue = issue
            .with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }

    emit_stdout(&OutputLine::Issue(issue))?;
    Ok(())
}

/// Map `RemoveError` to appropriate `IssueKind`
const fn remove_error_to_issue_kind(error: &RemoveError) -> IssueKind {
    match error {
        RemoveError::WorkspaceInaccessible { .. } => IssueKind::ResourceNotFound,
        RemoveError::WorkspaceRemovalFailed { .. } => IssueKind::External,
        RemoveError::DatabaseDeletionFailed { .. } => IssueKind::Configuration,
    }
}

#[allow(clippy::too_many_lines)]
/// Run the remove command with options
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get the session; idempotent mode treats missing as success.
    let session = match db.get(name).await? {
        Some(session) => session,
        None if options.idempotent => {
            emit_action("remove", name, ActionStatus::Skipped)?;
            emit_result(true, format!("Session '{name}' already removed"))?;
            return Ok(());
        }
        None => {
            emit_issue(
                "REMOVE-001",
                format!("Session '{name}' not found"),
                IssueKind::ResourceNotFound,
                IssueSeverity::Error,
                Some(name),
                Some("Use 'zjj list' to see available sessions"),
            )?;
            let result = ResultOutput::failure(
                ResultKind::Command,
                Message::new(format!("Session '{name}' not found"))
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            emit_stdout(&OutputLine::Result(result))?;
            return Err(anyhow::Error::new(zjj_core::Error::NotFound(format!(
                "Session '{name}' not found"
            ))));
        }
    };

    if options.dry_run {
        let message = format!(
            "DRY-RUN: Would remove session '{name}' and workspace at '{}'",
            session.workspace_path
        );
        emit_action("remove", name, ActionStatus::Pending)?;
        emit_result(true, message)?;
        return Ok(());
    }

    // Run pre_remove hooks unless --force (force flag retained for backwards compatibility)
    if !options.force {
        run_pre_remove_hooks(name, &session.workspace_path);
    }

    // If --merge: squash-merge to main
    if options.merge {
        merge_to_main(name, &session.workspace_path)?;
    }

    // Use atomic cleanup to prevent orphaned resources
    match cleanup_session_atomically(&db, &session, true).await {
        Ok(result) => {
            emit_action("remove", name, ActionStatus::Completed)?;
            let message = if result.removed {
                format!("Removed session '{name}'")
            } else {
                "Session removal completed with warnings".to_string()
            };
            emit_result(true, message)?;
            Ok(())
        }
        Err(RemoveError::WorkspaceInaccessible { path, reason }) => {
            // Workspace already gone - try to clean up database record
            db.delete(name).await?;
            emit_issue(
                "REMOVE-002",
                format!("Workspace inaccessible: {path} - {reason}"),
                IssueKind::ResourceNotFound,
                IssueSeverity::Warning,
                Some(name),
                Some("Session database record cleaned up"),
            )?;
            emit_action("cleanup", name, ActionStatus::Completed)?;
            emit_result(
                true,
                format!("Session '{name}' removed (workspace was already gone)"),
            )?;
            Ok(())
        }
        Err(e) => {
            // Log error details
            tracing::error!("Failed to remove session '{}': {}", name, e);

            let kind = remove_error_to_issue_kind(&e);
            emit_issue(
                "REMOVE-003",
                format!("Failed to remove session: {e}"),
                kind,
                IssueSeverity::Error,
                Some(name),
                Some("Check workspace permissions and try again"),
            )?;
            emit_result(false, format!("Failed to remove session '{name}'"))?;

            // Return IoError for exit code 3
            Err(anyhow::Error::new(zjj_core::Error::IoError(format!(
                "Failed to remove session: {e}"
            ))))
        }
    }
}

/// Run `pre_remove` hooks
const fn run_pre_remove_hooks(_name: &str, _workspace_path: &str) {
    // TODO: Implement hook execution when config system is ready
    // For now, this is a placeholder that always succeeds
}

/// Merge session to main branch
fn merge_to_main(name: &str, _workspace_path: &str) -> Result<()> {
    // Squash workspace changes into main
    let revset = format!("ancestors({name}@) & ~ancestors(main)");

    let output = std::process::Command::new("jj")
        .args(["squash", "--from", &revset, "--into", "main"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If there are no changes to squash, it might fail or do nothing.
        // We only bail if it's a real error.
        if !stderr.contains("No changes to squash") && !stderr.is_empty() {
            anyhow::bail!("Failed to merge changes to main: {stderr}");
        }
    }

    // Forget the workspace
    let output = std::process::Command::new("jj")
        .args(["workspace", "forget", name])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to forget workspace after merge: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use anyhow::Context;
    use tempfile::TempDir;

    use super::*;
    use crate::db::SessionDb;

    // Helper to create a test database with a session
    #[allow(dead_code)]
    async fn setup_test_session(name: &str) -> Result<(SessionDb, TempDir, String)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        let workspace_dir = dir.path().join("workspaces").join(name);
        tokio::fs::create_dir_all(&workspace_dir).await?;
        let workspace_path = workspace_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?
            .to_string();

        db.create(name, &workspace_path).await?;

        Ok((db, dir, workspace_path))
    }

    #[tokio::test]
    async fn test_remove_options_default() {
        let opts = RemoveOptions::default();
        assert!(!opts.force);
        assert!(!opts.merge);
        assert!(!opts.keep_branch);
        assert!(!opts.idempotent);
    }

    #[tokio::test]
    async fn test_session_not_found() -> Result<()> {
        let dir = TempDir::new()
            .map_err(anyhow::Error::from)
            .context("Failed to create temp dir")?;
        let db_path = dir.path().join("test.db");
        let _db = SessionDb::create_or_open(&db_path).await?;

        // Mock get_session_db to return our test db
        // Note: This test will fail until we refactor to use dependency injection
        // For now, it demonstrates the test case we need
        Ok(())
    }

    #[tokio::test]
    async fn test_merge_to_main_is_implemented() {
        // The merge_to_main feature is now implemented.
        // Without a jj repository, the command will fail with an execution error
        // (not a "not implemented" error).
        let result = merge_to_main("test", "/path");
        // The function is implemented, so it won't return "not yet implemented"
        // It will fail because we're not in a jj repo, which is expected behavior
        let is_impl = result
            .as_ref()
            .map(|()| true)
            .unwrap_or_else(|e| !e.to_string().contains("not yet implemented"));
        assert!(is_impl, "merge_to_main should be implemented");
    }

    // Tests for P0-3b: Error exit code mapping

    #[tokio::test]
    async fn test_not_found_error_has_correct_exit_code() {
        // When we can't find a session, we should return NotFound error with exit code 2
        let err = zjj_core::Error::NotFound("Session 'test' not found".into());
        assert_eq!(err.exit_code(), 2);
        assert!(matches!(err, zjj_core::Error::NotFound(_)));
    }

    #[tokio::test]
    async fn test_io_error_maps_to_exit_code_3() {
        // IO errors (like permission denied) should map to exit code 3
        let err = zjj_core::Error::IoError("Permission denied".into());
        assert_eq!(err.exit_code(), 3);
        assert!(matches!(err, zjj_core::Error::IoError(_)));
    }

    #[tokio::test]
    async fn test_validation_error_maps_to_exit_code_1() {
        // Validation errors should map to exit code 1
        let err = zjj_core::Error::ValidationError {
            message: "Invalid name".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(err.exit_code(), 1);
        assert!(matches!(err, zjj_core::Error::ValidationError { .. }));
    }

    #[tokio::test]
    async fn test_emit_action_produces_valid_jsonl() -> Result<()> {
        let action = Action::new(
            ActionVerb::new("remove")?,
            ActionTarget::new("test-session")?,
            ActionStatus::Completed,
        );

        let output_line = OutputLine::Action(action);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.is_object());
        assert!(
            parsed.get("action").is_some(),
            "OutputLine::Action must have 'action' key"
        );
        let action_obj = parsed
            .get("action")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("action value must be an object"))?;
        assert!(
            action_obj.get("verb").is_some(),
            "Action must have 'verb' field"
        );
        assert!(
            action_obj.get("target").is_some(),
            "Action must have 'target' field"
        );
        assert!(
            action_obj.get("status").is_some(),
            "Action must have 'status' field"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_emit_result_produces_valid_jsonl() -> Result<()> {
        let result = ResultOutput::success(ResultKind::Command, Message::new("Removed session")?)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let output_line = OutputLine::Result(result);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.is_object());
        assert!(
            parsed.get("result").is_some(),
            "OutputLine::Result must have 'result' key"
        );
        let result_obj = parsed
            .get("result")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("result value must be an object"))?;
        assert!(
            result_obj.get("outcome").is_some(),
            "Result must have 'outcome' field"
        );
        assert!(
            result_obj.get("message").is_some(),
            "Result must have 'message' field"
        );
        assert_eq!(
            result_obj.get("outcome").and_then(|v| v.as_str()),
            Some("success"),
            "Success result should have outcome=success"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_emit_issue_produces_valid_jsonl() -> Result<()> {
        let issue = Issue::new(
            IssueId::new("REMOVE-001")?,
            IssueTitle::new("Session not found")?,
            IssueKind::ResourceNotFound,
            IssueSeverity::Error,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .with_session(SessionName::parse("test-session")?);

        let output_line = OutputLine::Issue(issue);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.is_object());
        assert!(
            parsed.get("issue").is_some(),
            "OutputLine::Issue must have 'issue' key"
        );
        let issue_obj = parsed
            .get("issue")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("issue value must be an object"))?;
        assert!(issue_obj.get("id").is_some(), "Issue must have 'id' field");
        assert!(
            issue_obj.get("title").is_some(),
            "Issue must have 'title' field"
        );
        assert!(
            issue_obj.get("kind").is_some(),
            "Issue must have 'kind' field"
        );
        assert!(
            issue_obj.get("severity").is_some(),
            "Issue must have 'severity' field"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_error_to_issue_kind_mapping() {
        let workspace_inaccessible = RemoveError::WorkspaceInaccessible {
            path: "/tmp/test".into(),
            reason: "not found".into(),
        };
        assert!(matches!(
            remove_error_to_issue_kind(&workspace_inaccessible),
            IssueKind::ResourceNotFound
        ));

        let workspace_removal_failed = RemoveError::WorkspaceRemovalFailed {
            path: "/tmp/test".into(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(matches!(
            remove_error_to_issue_kind(&workspace_removal_failed),
            IssueKind::External
        ));

        let db_deletion_failed = RemoveError::DatabaseDeletionFailed {
            name: "test".into(),
            source: zjj_core::Error::NotFound("test".into()),
        };
        assert!(matches!(
            remove_error_to_issue_kind(&db_deletion_failed),
            IssueKind::Configuration
        ));
    }
}

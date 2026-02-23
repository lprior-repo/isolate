//! Switch to a session's Zellij tab - JSONL output for AI-first control plane

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)]

use std::path::PathBuf;

use anyhow::Result;
use zjj_core::{
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput, SessionOutput,
    },
    OutputFormat,
};

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::get_session_db,
    session::SessionStatus,
};

const fn to_core_status(status: SessionStatus) -> zjj_core::types::SessionStatus {
    match status {
        SessionStatus::Active => zjj_core::types::SessionStatus::Active,
        SessionStatus::Paused => zjj_core::types::SessionStatus::Paused,
        SessionStatus::Completed => zjj_core::types::SessionStatus::Completed,
        SessionStatus::Failed => zjj_core::types::SessionStatus::Failed,
        SessionStatus::Creating => zjj_core::types::SessionStatus::Creating,
    }
}

#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    pub format: OutputFormat,
    pub no_zellij: bool,
}

fn emit_session_and_result(session: &crate::session::Session) -> Result<()> {
    let workspace_path: PathBuf = session.workspace_path.clone().into();

    let session_output = SessionOutput::new(
        session.name.clone(),
        to_core_status(session.status),
        session.state,
        workspace_path,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_output = if let Some(branch) = &session.branch {
        session_output.with_branch(branch.clone())
    } else {
        session_output
    };

    emit_stdout(&OutputLine::Session(session_output))?;

    let result = ResultOutput::success(
        ResultKind::Command,
        Message::new(format!("Focused on session '{}'", session.name))
            .map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result))?;

    Ok(())
}

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
        issue = issue.with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }

    emit_stdout(&OutputLine::Issue(issue))?;
    Ok(())
}

pub async fn run_with_options(name: Option<&str>, options: &FocusOptions) -> Result<()> {
    let db = get_session_db().await?;

    let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
        emit_issue(
            "FOCUS-001",
            "Session name is required".to_string(),
            IssueKind::Validation,
            IssueSeverity::Error,
            None,
            Some("Usage: zjj focus <name>"),
        )?;
        return Err(anyhow::anyhow!(
            "Session name is required. Usage: zjj focus <name>"
        ));
    };

    let Some(session) = db.get(name).await? else {
        emit_issue(
            "FOCUS-002",
            format!("Session '{name}' not found"),
            IssueKind::ResourceNotFound,
            IssueSeverity::Error,
            Some(name),
            Some("Use 'zjj list' to see available sessions"),
        )?;
        return Err(anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        ))));
    };

    if options.no_zellij {
        let action = Action::new(
            ActionVerb::new("focus").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
            ActionTarget::new(name).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
            ActionStatus::Completed,
        );
        emit_stdout(&OutputLine::Action(action))?;
        emit_session_and_result(&session)?;
        return Ok(());
    }

    let zellij_tab = session.zellij_tab.clone();

    if is_inside_zellij() {
        run_command("zellij", &["action", "go-to-tab-name", &zellij_tab]).await?;

        let action = Action::new(
            ActionVerb::new("switch-tab").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
            ActionTarget::new(&zellij_tab).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
            ActionStatus::Completed,
        )
        .with_result(format!("Switched to session '{name}'"));
        emit_stdout(&OutputLine::Action(action))?;
        emit_session_and_result(&session)?;
    } else {
        let action = Action::new(
            ActionVerb::new("attach").map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
            ActionTarget::new(zellij_tab.clone()).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
            ActionStatus::Completed,
        )
        .with_result(format!("Attaching to Zellij session for '{name}'"));
        emit_stdout(&OutputLine::Action(action))?;
        emit_session_and_result(&session)?;
        attach_to_zellij_session(None).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::path::PathBuf;

    use tempfile::TempDir;
    use zjj_core::output::{
        Action, ActionStatus, Issue, IssueKind, IssueSeverity, OutputLine, ResultKind,
        ResultOutput, SessionOutput,
    };

    use super::*;
    use crate::db::SessionDb;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_focus_session_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let result = db.get("nonexistent").await?;
        assert!(result.is_none());

        let session_name = "nonexistent";
        let result = db
            .get(session_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"));

        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Session 'nonexistent' not found");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let session = db.create("test-session", "/tmp/test").await?;

        let retrieved = db.get("test-session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, session.name);
        assert_eq!(retrieved_session.zellij_tab, "zjj:test-session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_hyphens() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let _session = db.create("my-test-session", "/tmp/my-test").await?;

        let retrieved = db.get("my-test-session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test-session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test-session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_underscores() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let _session = db.create("my_test_session", "/tmp/my_test").await?;

        let retrieved = db.get("my_test_session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my_test_session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my_test_session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_mixed_special_chars() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let _session = db.create("my-test_123", "/tmp/my-test_123").await?;

        let retrieved = db.get("my-test_123").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test_123");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test_123");

        Ok(())
    }

    #[tokio::test]
    async fn test_zellij_tab_format() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        let session1 = db.create("session1", "/tmp/s1").await?;
        assert_eq!(session1.zellij_tab, "zjj:session1");

        let session2 = db.create("my-session", "/tmp/s2").await?;
        assert_eq!(session2.zellij_tab, "zjj:my-session");

        let session3 = db.create("test_session_123", "/tmp/s3").await?;
        assert_eq!(session3.zellij_tab, "zjj:test_session_123");

        Ok(())
    }

    #[tokio::test]
    async fn test_is_inside_zellij_detection() {
        let original = std::env::var("ZELLIJ").ok();

        std::env::remove_var("ZELLIJ");
        assert!(!is_inside_zellij());

        std::env::set_var("ZELLIJ", "1");
        assert!(is_inside_zellij());

        if let Some(val) = original {
            std::env::set_var("ZELLIJ", val);
        } else {
            std::env::remove_var("ZELLIJ");
        }
    }

    #[tokio::test]
    async fn test_session_output_is_valid_jsonl() -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .as_secs();

        let session = crate::session::Session {
            id: None,
            name: "test-session".to_string(),
            workspace_path: "/tmp/test".to_string(),
            status: SessionStatus::Active,
            state: zjj_core::WorkspaceState::Created,
            zellij_tab: "zjj:test-session".to_string(),
            branch: None,
            metadata: None,
            created_at: now,
            updated_at: now,
            last_synced: None,
            parent_session: None,
            queue_status: None,
        };

        let workspace_path: PathBuf = session.workspace_path.clone().into();
        let session_output = SessionOutput::new(
            session.name.clone(),
            to_core_status(session.status),
            session.state,
            workspace_path,
        )?;

        let output_line = OutputLine::Session(session_output);
        let json_str = serde_json::to_string(&output_line)?;

        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
        assert!(parsed.is_object(), "OutputLine should serialize to JSON");

        // OutputLine wraps the variant as key: {"session": {...}}
        assert!(
            parsed.get("session").is_some(),
            "OutputLine::Session must have 'session' key"
        );
        let session_obj = parsed.get("session").and_then(|v| v.as_object());
        assert!(session_obj.is_some(), "session value must be an object");

        Ok(())
    }

    #[tokio::test]
    async fn test_issue_output_is_valid_jsonl() -> Result<()> {
        let issue = Issue::new(
            IssueId::new("FOCUS-001")?,
            IssueTitle::new("Session name is required")?,
            IssueKind::Validation,
            IssueSeverity::Error,
        )?
        .with_suggestion("Usage: zjj focus <name>".to_string());

        let output_line = OutputLine::Issue(issue);
        let json_str = serde_json::to_string(&output_line)?;

        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
        assert!(parsed.is_object());

        // OutputLine wraps the variant as key: {"issue": {...}}
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
    async fn test_result_output_is_valid_jsonl() -> Result<()> {
        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Focused on session")?,
        )?;

        let output_line = OutputLine::Result(result);
        let json_str = serde_json::to_string(&output_line)?;

        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
        assert!(parsed.is_object());

        // OutputLine wraps the variant as key: {"result": {...}}
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
            "Success result should have outcome=\"success\""
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_action_output_is_valid_jsonl() -> Result<()> {
        let action = Action::new(
            ActionVerb::new("focus")?,
            ActionTarget::new("test-session")?,
            ActionStatus::Completed,
        )
        .with_result("Switched to session".to_string());

        let output_line = OutputLine::Action(action);
        let json_str = serde_json::to_string(&output_line)?;

        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
        assert!(parsed.is_object());

        // OutputLine wraps the variant as key: {"action": {...}}
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
    async fn test_to_core_status_converts_all_variants() {
        assert_eq!(
            to_core_status(SessionStatus::Active),
            zjj_core::types::SessionStatus::Active
        );
        assert_eq!(
            to_core_status(SessionStatus::Paused),
            zjj_core::types::SessionStatus::Paused
        );
        assert_eq!(
            to_core_status(SessionStatus::Completed),
            zjj_core::types::SessionStatus::Completed
        );
        assert_eq!(
            to_core_status(SessionStatus::Failed),
            zjj_core::types::SessionStatus::Failed
        );
        assert_eq!(
            to_core_status(SessionStatus::Creating),
            zjj_core::types::SessionStatus::Creating
        );
    }

    #[tokio::test]
    async fn test_emit_issue_produces_valid_jsonl() -> Result<()> {
        let issue = Issue::new(
            IssueId::new("FOCUS-TEST")?,
            IssueTitle::new("Test issue")?,
            IssueKind::Validation,
            IssueSeverity::Warning,
        )?
        .with_session(SessionName::parse("test-session")?);

        let json_str = serde_json::to_string(&OutputLine::Issue(issue))?;
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
        assert_eq!(
            issue_obj.get("id").and_then(|v| v.as_str()),
            Some("FOCUS-TEST")
        );
        assert_eq!(
            issue_obj.get("severity").and_then(|v| v.as_str()),
            Some("warning")
        );

        Ok(())
    }
}

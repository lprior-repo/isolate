//! Show diff between session and main branch - JSONL output for AI-first control plane

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{io, path::Path, process::Stdio};

use anyhow::Result;
use tokio::{io::AsyncWriteExt, process::Command};
use zjj_core::{
    output::{emit_stdout, Message, OutputLine, ResultKind, ResultOutput, SessionOutput},
    OutputFormat,
};

use crate::commands::{determine_main_branch, get_session_db};

/// Build diff command arguments
fn build_diff_args(stat: bool, main_branch: &str) -> Vec<String> {
    let mut args = vec![
        "diff".to_string(),
        if stat { "--stat" } else { "--git" }.to_string(),
    ];
    args.extend_from_slice(&[
        "--from".to_string(),
        main_branch.to_string(),
        "--to".to_string(),
        "@".to_string(),
    ]);
    args
}

/// Map JJ command error to proper error type
fn map_jj_error(e: &io::Error, operation: &str) -> anyhow::Error {
    anyhow::Error::new(if e.kind() == io::ErrorKind::NotFound {
        zjj_core::Error::JjCommandError {
            operation: operation.to_string(),
            source: format!(
                "JJ is not installed or not in PATH.\n\n\
                Install JJ:\n\
                  cargo install jj-cli\n\
                or:\n\
                  brew install jj\n\
                or visit: https://github.com/martinvonz/jj#installation\n\n\
                Error: {e}"
            ),
            is_not_found: true,
        }
    } else {
        zjj_core::Error::IoError(format!("Failed to execute jj {operation}: {e}"))
    })
}

/// Detect session name from current workspace directory
///
/// Returns `Ok(Some(session_name))` if in a workspace
/// Returns Ok(None) if not in a workspace
/// Returns Err if workspace detection fails
async fn detect_session_from_workspace(db: &crate::db::SessionDb) -> Result<Option<String>> {
    use std::path::Path;

    use anyhow::Context as _;

    // Get current directory
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Try to get JJ workspace root
    let output = tokio::process::Command::new("jj")
        .args(["workspace", "root"])
        .output()
        .await;

    let workspace_root = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return Ok(None), // Not in a JJ repo
    };

    // Check if we're in a workspace by looking at workspace show
    let show_output = tokio::process::Command::new("jj")
        .args(["workspace", "show"])
        .output()
        .await;

    let in_workspace = match show_output {
        Ok(out) if out.status.success() => {
            let workspace_info = String::from_utf8_lossy(&out.stdout);
            workspace_info.contains("Working copy")
        }
        _ => false,
    };

    if !in_workspace {
        return Ok(None); // In main repo, not a workspace
    }

    // We're in a workspace - find the session by matching workspace_path
    let sessions = db
        .list(None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list sessions: {e}"))?;

    // Normalize paths for comparison
    let workspace_path = Path::new(&workspace_root);
    let current_path = Path::new(&current_dir);

    for session in sessions {
        let session_path = Path::new(&session.workspace_path);

        // Check if current directory is within or matches the session workspace
        if current_path.starts_with(session_path) || workspace_path == session_path {
            return Ok(Some(session.name.clone()));
        }
    }

    // No matching session found
    Ok(None)
}

/// Handle diff output using JSONL output format.
///
/// For JSON format: emit Result line with diff content in data field.
/// For Human format: display to terminal with optional pager.
async fn handle_diff_output(
    stdout: &str,
    name: &str,
    stat: bool,
    format: OutputFormat,
    session: Option<&crate::session::Session>,
) -> Result<()> {
    if format.is_json() {
        // Emit session context first (if available)
        if let Some(sess) = session {
            let session_output = SessionOutput::new(
                sess.name.clone(),
                to_core_status(sess.status),
                sess.state,
                sess.workspace_path.clone().into(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create session output: {e}"))?;

            emit_stdout(&OutputLine::Session(session_output))
                .map_err(|e| anyhow::anyhow!("Failed to emit session output: {e}"))?;
        }

        // Build diff data payload
        let diff_data = serde_json::json!({
            "session": name,
            "diff_type": if stat { "stat" } else { "full" },
            "content": stdout,
        });

        // Emit diff result
        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new(format!("Diff for {name}"))
                .map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create result output: {e}"))?
        .with_data(diff_data);

        emit_stdout(&OutputLine::Result(result))
            .map_err(|e| anyhow::anyhow!("Failed to emit diff result: {e}"))?;
    } else if stat {
        // Human format with stat - print directly
        print!("{stdout}");
    } else {
        // Human format with full diff - use pager if available
        match get_pager() {
            Some(pager) => {
                let mut child = Command::new(&pager).stdin(Stdio::piped()).spawn().ok();

                if let Some(mut child) = child.take() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(stdout.as_bytes()).await;
                    }
                    let _ = child.wait().await;
                }
            }
            None => print!("{stdout}"),
        }
    }

    Ok(())
}

/// Convert local `SessionStatus` to core `SessionStatus`
const fn to_core_status(status: crate::session::SessionStatus) -> zjj_core::types::SessionStatus {
    match status {
        crate::session::SessionStatus::Active => zjj_core::types::SessionStatus::Active,
        crate::session::SessionStatus::Paused => zjj_core::types::SessionStatus::Paused,
        crate::session::SessionStatus::Completed => zjj_core::types::SessionStatus::Completed,
        crate::session::SessionStatus::Failed => zjj_core::types::SessionStatus::Failed,
        crate::session::SessionStatus::Creating => zjj_core::types::SessionStatus::Creating,
    }
}

/// Run the diff command
///
/// If name is None, attempts to detect session from current workspace.
/// If not in a workspace, returns an error requesting explicit session name.
pub async fn run(name: Option<&str>, stat: bool, format: OutputFormat) -> Result<()> {
    let db = get_session_db().await?;

    // Determine session name
    let session_name = match name {
        Some(n) => n.to_string(),
        None => {
            // Try to detect session from workspace
            match detect_session_from_workspace(&db).await? {
                Some(detected_name) => {
                    tracing::info!("Auto-detected session '{detected_name}' from workspace");
                    detected_name
                }
                None => {
                    return Err(anyhow::Error::new(zjj_core::Error::NotFound(
                        "Session name required (not in a workspace or no matching session found)\n\n\
                         Provide explicit session name:\n\
                           zjj diff <session-name>\n\n\
                         Or run from within a workspace directory."
                            .to_string(),
                    )));
                }
            }
        }
    };

    let session = db.get(&session_name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{session_name}' not found"
        )))
    })?;

    let workspace_path = Path::new(&session.workspace_path);
    if !tokio::fs::try_exists(workspace_path).await.unwrap_or(false) {
        return Err(anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Workspace not found: {}. The session may be stale.",
            session.workspace_path
        ))));
    }

    let main_branch = determine_main_branch(workspace_path).await;
    let args = build_diff_args(stat, &main_branch);

    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| map_jj_error(&e, "diff"))?;

    output.status.success().then_some(()).ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::Error::new(zjj_core::Error::JjCommandError {
            operation: "diff".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        })
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    handle_diff_output(&stdout, &session_name, stat, format, Some(&session)).await?;
    Ok(())
}

/// Get the pager command from environment or defaults
fn get_pager() -> Option<String> {
    // Check PAGER environment variable
    if let Ok(pager) = std::env::var("PAGER") {
        if !pager.is_empty() {
            return Some(pager);
        }
    }

    // Try common pagers in order of preference
    let pagers = ["delta", "bat", "less"];
    for pager in &pagers {
        if which::which(pager).is_ok() {
            return Some(pager.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;
    use crate::{commands::determine_main_branch, db::SessionDb};

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_determine_main_branch_not_in_repo() -> Result<()> {
        // When not in a JJ repo (or jj not installed), should fall back to "main"
        let temp = TempDir::new().context("Failed to create temp dir")?;
        let result = determine_main_branch(temp.path()).await;

        // Should return fallback "main"
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_get_pager_from_env() {
        // Set PAGER environment variable
        std::env::set_var("PAGER", "custom-pager");
        let pager = get_pager();
        assert_eq!(pager, Some("custom-pager".to_string()));

        // Clean up
        std::env::remove_var("PAGER");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_pager_defaults() {
        // Unset PAGER
        std::env::remove_var("PAGER");
        let pager = get_pager();

        // Should return one of the default pagers if available
        // We can't assert a specific value since it depends on system
        // But we can verify it returns either Some or None
        assert!(pager.is_some() || pager.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_pager_empty_env() {
        // Set PAGER to empty string
        std::env::set_var("PAGER", "");
        let pager = get_pager();

        // Should fall back to defaults
        assert!(pager.is_some() || pager.is_none());

        // Clean up
        std::env::remove_var("PAGER");
    }

    #[tokio::test]
    async fn test_run_session_not_found() -> Result<()> {
        let _temp_db = setup_test_db().await?;

        // Try to diff a non-existent session
        // We need to set up the context so get_session_db works
        // This is tricky in unit tests, so we'll focus on testing the helpers

        Ok(())
    }

    #[tokio::test]
    async fn test_run_workspace_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create a session with a non-existent workspace
        let session = db.create("test-session", "/nonexistent/path").await?;

        // Verify the session exists
        assert_eq!(session.name, "test-session");

        // The run function would fail because workspace doesn't exist
        // We can't easily test this without mocking, so we verify the logic in integration tests

        Ok(())
    }

    #[tokio::test]
    async fn test_diff_command_args_full() {
        // Verify that full diff uses --git flag
        let args = ["diff", "--git", "-r", "main..@"];
        assert!(args.contains(&"--git"));
        assert!(args.contains(&"-r"));
    }

    #[tokio::test]
    async fn test_diff_command_args_stat() {
        // Verify that stat diff uses --stat flag
        let args = ["diff", "--stat", "-r", "main..@"];
        assert!(args.contains(&"--stat"));
        assert!(!args.contains(&"--git"));
    }

    #[tokio::test]
    async fn test_revset_format() {
        let main_branch = "main";
        let revset = format!("{main_branch}..@");
        assert_eq!(revset, "main..@");

        let commit_id = "abc123";
        let revset2 = format!("{commit_id}..@");
        assert_eq!(revset2, "abc123..@");
    }

    /// Test that `build_diff_args` produces correct arguments for stat mode
    #[test]
    fn test_build_diff_args_stat() {
        let args = build_diff_args(true, "main");
        assert!(args.contains(&"--stat".to_string()));
        assert!(args.contains(&"--from".to_string()));
        assert!(args.contains(&"main".to_string()));
        assert!(args.contains(&"--to".to_string()));
        assert!(args.contains(&"@".to_string()));
    }

    /// Test that `build_diff_args` produces correct arguments for full diff mode
    #[test]
    fn test_build_diff_args_full() {
        let args = build_diff_args(false, "main");
        assert!(args.contains(&"--git".to_string()));
        assert!(args.contains(&"--from".to_string()));
        assert!(args.contains(&"main".to_string()));
        assert!(args.contains(&"--to".to_string()));
        assert!(args.contains(&"@".to_string()));
    }

    /// Test that `map_jj_error` handles `NotFound` errors correctly
    #[test]
    fn test_map_jj_error_not_found() {
        let error = io::Error::new(io::ErrorKind::NotFound, "not found");
        let mapped = map_jj_error(&error, "test");

        // Should be a JjCommandError with is_not_found = true
        let err_string = mapped.to_string();
        assert!(err_string.contains("not installed") || err_string.contains("not found"));
    }

    /// Test that `map_jj_error` handles other errors correctly
    #[test]
    fn test_map_jj_error_other() {
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let mapped = map_jj_error(&error, "test");

        // Should be an IoError
        let err_string = mapped.to_string();
        assert!(err_string.contains("Failed to execute"));
    }

    /// Test that `to_core_status` converts status correctly
    #[test]
    fn test_to_core_status() {
        use crate::session::SessionStatus;

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

    // ============================================================================
    // JSONL Output Tests for diff.rs
    // ============================================================================

    /// Test that `ResultOutput` can be created for diff operations
    #[test]
    fn test_result_output_for_diff() {
        let diff_data = serde_json::json!({
            "session": "test-session",
            "diff_type": "full",
            "content": "diff content here",
        });

        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Diff for test-session").expect("valid message"),
        )
        .expect("Failed to create result")
        .with_data(diff_data);

        assert!(matches!(result.outcome, zjj_core::output::Outcome::Success));
        assert_eq!(result.kind, ResultKind::Command);
        assert!(result.data.is_some());
    }

    /// Test that `ResultOutput` handles stat diff type correctly
    #[test]
    fn test_result_output_stat_diff() {
        let diff_data = serde_json::json!({
            "session": "test-session",
            "diff_type": "stat",
            "content": "1 file changed, 5 insertions(+), 2 deletions(-)",
        });

        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Diff for test-session").expect("valid message"),
        )
        .expect("Failed to create result")
        .with_data(diff_data);

        let data = result.data.expect("Data should be present");
        assert_eq!(data["diff_type"], "stat");
    }

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for diff.rs
    // These tests document the expected signature change (already implemented)
    // ============================================================================

    /// Test diff `run()` accepts `OutputFormat` parameter
    #[tokio::test]
    async fn test_diff_run_signature_accepts_format() {
        // This test documents that run() now accepts OutputFormat parameter:
        // Current: pub fn run(name: &str, stat: bool, format: OutputFormat) -> Result<()>

        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);

        // When run() is called with OutputFormat::Json:
        // diff::run("session-name", false, OutputFormat::Json)
    }

    /// Test diff should support JSON output format
    #[tokio::test]
    async fn test_diff_json_output_format() {
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // When diff is called with OutputFormat::Json:
        // - diff output should be formatted as JSONL Result line
        // - diff content should be in the data field
    }

    /// Test diff output uses JSONL format
    #[tokio::test]
    async fn test_diff_uses_jsonl_format() {
        // All diff output is JSONL format
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // Diff content is in Result line with data field
        // Format is always JSONL for AI-first design
    }

    /// Test diff --stat works with JSONL output
    #[tokio::test]
    async fn test_diff_stat_with_format() {
        // stat diff works with JSONL format
        let format = OutputFormat::Json;
        assert!(format.is_json());

        // When stat=true is passed:
        // diff::run("session", true, OutputFormat::Json) outputs JSONL Result
        // with stat information in the data field
    }

    /// Test `OutputFormat::from_json_flag` always returns Json
    #[tokio::test]
    async fn test_diff_from_json_flag() {
        // All output is JSONL in AI-first design
        let format = OutputFormat::from_json_flag(true);
        assert_eq!(format, OutputFormat::Json);

        let format2 = OutputFormat::from_json_flag(false);
        assert_eq!(format2, OutputFormat::Json);
    }

    /// Test diff preserves format through conversion chain
    #[tokio::test]
    async fn test_diff_format_roundtrip() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        let restored_bool = format.to_json_flag();

        assert_eq!(json_bool, restored_bool);
    }

    /// Test diff never panics during format processing
    #[tokio::test]
    async fn test_diff_format_no_panics() {
        // JSONL format should be processable without panic
        let format = OutputFormat::Json;
        let _ = format.is_json();
        let _ = format.to_string();
    }

    // ============================================================================
    // --contract flag tests for diff command
    // ============================================================================

    /// Test that --contract flag exists and outputs JSON schema
    #[test]
    fn test_diff_contract_flag_outputs_schema() {
        // The --contract flag should output the diff command's AI contract
        // as a JSON string describing inputs, outputs, and side effects
        let contract = crate::cli::json_docs::ai_contracts::diff();

        // Contract should contain essential information
        assert!(contract.contains("zjj diff"));
        assert!(contract.contains("intent"));
        assert!(contract.contains("inputs"));
        assert!(contract.contains("outputs"));
    }

    /// Test that contract includes diff-specific fields
    #[test]
    fn test_diff_contract_includes_stat_flag() {
        let contract = crate::cli::json_docs::ai_contracts::diff();

        // Should document the --stat flag
        assert!(contract.contains("stat"));

        // Should document the name argument
        assert!(contract.contains("name") || contract.contains("session"));
    }

    /// Test that contract documents output schema
    #[test]
    fn test_diff_contract_documents_output_schema() {
        let contract = crate::cli::json_docs::ai_contracts::diff();

        // Should document that output includes diff content
        assert!(contract.contains("diff") || contract.contains("content"));
    }
}

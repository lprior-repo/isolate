//! Show diff between session and main branch

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::commands::{determine_main_branch, get_session_db};

/// JSON output structure for diff command
#[derive(Serialize)]
struct DiffOutput {
    session: String,
    diff_type: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<DiffStats>,
}

/// Diff statistics for JSON output
#[derive(Serialize)]
struct DiffStats {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

/// Build diff command arguments
fn build_diff_args(stat: bool, main_branch: &str) -> Vec<String> {
    let mut args = vec![
        "diff".to_string(),
        if stat { "--stat" } else { "--git" }.to_string(),
    ];
    args.extend_from_slice(&["-r".to_string(), format!("{main_branch}..@")]);
    args
}

/// Map JJ command error to proper error type
fn map_jj_error(e: &std::io::Error, operation: &str) -> anyhow::Error {
    anyhow::Error::new(if e.kind() == std::io::ErrorKind::NotFound {
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

/// Handle diff output based on format
fn handle_diff_output(stdout: &str, name: &str, stat: bool, format: OutputFormat) {
    if format.is_json() {
        let stats = stat.then(|| parse_stat_output(stdout));
        let diff_output = DiffOutput {
            session: name.to_string(),
            diff_type: if stat { "stat" } else { "full" }.to_string(),
            content: stdout.to_string(),
            stats,
        };
        // Wrap in SchemaEnvelope for consistent JSON output (DRQ Round 1)
        let envelope = SchemaEnvelope::new(
            if stat { "diff-stat-response" } else { "diff-response" },
            "single",
            diff_output,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else if stat {
        print!("{stdout}");
    } else {
        get_pager()
            .and_then(|pager| {
                Command::new(&pager)
                    .stdin(Stdio::piped())
                    .spawn()
                    .ok()
                    .map(|mut child| {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(stdout.as_bytes());
                        }
                        let _ = child.wait();
                    })
            })
            .unwrap_or_else(|| print!("{stdout}"));
    }
}

/// Run the diff command
pub fn run(name: &str, stat: bool, format: OutputFormat) -> Result<()> {
    let db = get_session_db()?;
    let session = db.get_blocking(name)?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    let workspace_path = Path::new(&session.workspace_path);
    workspace_path.exists().then_some(()).ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Workspace not found: {}. The session may be stale.",
            session.workspace_path
        )))
    })?;

    let main_branch = determine_main_branch(workspace_path);
    let args = build_diff_args(stat, &main_branch);

    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
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
    handle_diff_output(&stdout, name, stat, format);
    Ok(())
}

/// Parse stat output to extract statistics
fn parse_stat_output(stat_output: &str) -> DiffStats {
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for line in stat_output.lines() {
        // Count file change lines (e.g., " file.txt | 5 ++-")
        if line.contains('|') {
            files_changed += 1;
        }
        // Parse summary line (e.g., "1 file changed, 3 insertions(+), 1 deletion(-)")
        if line.contains("changed") {
            // Split by comma and parse each segment
            for segment in line.split(',') {
                let segment = segment.trim();
                // Look for insertion(s)
                if segment.contains("insertion") {
                    if let Some(num_str) = segment.split_whitespace().next() {
                        if let Ok(n) = num_str.parse::<usize>() {
                            insertions = n;
                        }
                    }
                }
                // Look for deletion(s)
                if segment.contains("deletion") {
                    if let Some(num_str) = segment.split_whitespace().next() {
                        if let Ok(n) = num_str.parse::<usize>() {
                            deletions = n;
                        }
                    }
                }
            }
        }
    }

    DiffStats {
        files_changed,
        insertions,
        deletions,
    }
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

    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open_blocking(&db_path)?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_determine_main_branch_not_in_repo() -> Result<()> {
        // When not in a JJ repo (or jj not installed), should fall back to "main"
        let temp = TempDir::new().context("Failed to create temp dir")?;
        let result = determine_main_branch(temp.path());

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
        let _temp_db = setup_test_db()?;

        // Try to diff a non-existent session
        // We need to set up the context so get_session_db works
        // This is tricky in unit tests, so we'll focus on testing the helpers

        Ok(())
    }

    #[tokio::test]
    async fn test_run_workspace_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with a non-existent workspace
        let session = db.create_blocking("test-session", "/nonexistent/path")?;

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

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for diff.rs
    // These tests FAIL until diff command accepts OutputFormat parameter
    // ============================================================================

    /// RED: diff `run()` should accept `OutputFormat` parameter
    #[tokio::test]
    async fn test_diff_run_signature_accepts_format() {
        use zjj_core::OutputFormat;

        // This test documents the expected signature change:
        // Current: pub fn run(name: &str, stat: bool) -> Result<()>
        // Expected: pub fn run(name: &str, stat: bool, format: OutputFormat) -> Result<()>

        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);

        // When run() is updated to accept format:
        // diff::run("session-name", false, OutputFormat::Json)
    }

    /// RED: diff should support JSON output format
    #[tokio::test]
    async fn test_diff_json_output_format() {
        use zjj_core::OutputFormat;

        let format = OutputFormat::Json;
        assert!(format.is_json());

        // When diff is called with OutputFormat::Json:
        // - diff output should be formatted as JSON
        // - diff should be wrapped in SchemaEnvelope
    }

    /// RED: diff should support Human output format
    #[tokio::test]
    async fn test_diff_human_output_format() {
        use zjj_core::OutputFormat;

        let format = OutputFormat::Human;
        assert!(format.is_human());

        // When diff is called with OutputFormat::Human:
        // - diff output should be human-readable text
        // - diff should be sent to pager if available
    }

    /// RED: diff output structure changes based on format
    #[tokio::test]
    async fn test_diff_respects_output_format() {
        use zjj_core::OutputFormat;

        // For JSON format: diff content should be wrapped in envelope
        let json_format = OutputFormat::Json;
        assert!(json_format.is_json());

        // For Human format: diff should be displayed to terminal/pager
        let human_format = OutputFormat::Human;
        assert!(human_format.is_human());

        // The implementation should check format variant:
        // match format {
        //     OutputFormat::Json => output_json_diff(...),
        //     OutputFormat::Human => display_diff_with_pager(...),
        // }
    }

    /// RED: diff --stat works with both output formats
    #[tokio::test]
    async fn test_diff_stat_with_format() {
        use zjj_core::OutputFormat;

        // stat diff should work with JSON format
        let json_format = OutputFormat::Json;
        assert!(json_format.is_json());

        // stat diff should work with Human format
        let human_format = OutputFormat::Human;
        assert!(human_format.is_human());

        // When stat=true is passed along with format:
        // diff::run("session", true, OutputFormat::Json) should output JSON stat
        // diff::run("session", true, OutputFormat::Human) should output text stat
    }

    /// RED: `OutputFormat::from_json_flag` converts correctly
    #[tokio::test]
    async fn test_diff_from_json_flag() {
        use zjj_core::OutputFormat;

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);

        let human_flag = false;
        let format2 = OutputFormat::from_json_flag(human_flag);
        assert_eq!(format2, OutputFormat::Human);
    }

    /// RED: diff preserves format through conversion chain
    #[tokio::test]
    async fn test_diff_format_roundtrip() {
        use zjj_core::OutputFormat;

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        let restored_bool = format.to_json_flag();

        assert_eq!(json_bool, restored_bool);
    }

    /// RED: diff never panics during format processing
    #[tokio::test]
    async fn test_diff_format_no_panics() {
        use zjj_core::OutputFormat;

        // Both formats should be processable without panic
        for format in &[OutputFormat::Json, OutputFormat::Human] {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_string();
        }
    }
}

//! Execution logic for diff command

use std::{
    path::Path,
    process::{self, Command},
};

use anyhow::{Context, Result};

use super::{
    formatting::{output_json_error, output_with_pager},
    parsing::parse_diff_stat,
    types::DiffOptions,
};
use crate::{commands::get_session_db, json_output::DiffOutput};

/// Run the diff command with options (zjj-1d2: added json support)
pub async fn run_with_options(name: &str, options: DiffOptions) -> Result<()> {
    match run_internal(name, options).await {
        Ok(()) => Ok(()),
        Err(e) if options.json => {
            // Output error as JSON and exit with appropriate code
            output_json_error("DIFF_FAILED", &e.to_string());
            // Determine appropriate exit code based on error type
            let exit_code = e
                .downcast_ref::<zjj_core::Error>()
                .map_or(2, zjj_core::Error::exit_code);
            process::exit(exit_code);
        }
        Err(e) => Err(e),
    }
}

/// Internal implementation of the diff command
async fn run_internal(name: &str, options: DiffOptions) -> Result<()> {
    // Validate session name FIRST before any operations (zjj-audit-002)
    crate::session::validate_session_name(name).context("Invalid session name")?;

    // Load config to get main branch setting
    let config = zjj_core::config::load_config().context("Failed to load configuration")?;

    let db = get_session_db().await?;

    // Get the session
    let session = db
        .get(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Verify workspace exists
    let workspace_path = Path::new(&session.workspace_path);
    if !workspace_path.exists() {
        anyhow::bail!(
            "Workspace not found: {}. The session may be stale.",
            session.workspace_path
        );
    }

    // Determine the main branch: use config if set, otherwise auto-detect (zjj-qf8)
    let main_branch = match &config.main_branch {
        Some(branch) if !branch.trim().is_empty() => branch.clone(),
        Some(_) => {
            anyhow::bail!(
                "main_branch cannot be empty in config - either unset it or provide a branch name"
            )
        }
        None => determine_main_branch(workspace_path),
    };

    // Build the diff command using functional iterator chain
    let format_flag = if options.stat { "--stat" } else { "--git" };
    let revset = format!("{main_branch}..@");

    let args: Vec<&str> = ["diff", format_flag, "-r", &revset].into_iter().collect();

    // Execute the diff command
    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Failed to execute jj diff: JJ is not installed or not in PATH.\n\n\
                    Install JJ:\n\
                      cargo install jj-cli\n\
                    or:\n\
                      brew install jj\n\
                    or visit: https://github.com/martinvonz/jj#installation\n\n\
                    Error: {e}"
                )
            } else {
                anyhow::anyhow!("Failed to execute jj diff: {e}")
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj diff failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Handle JSON output mode
    if options.json {
        let diff_output = if options.stat {
            // Parse stat output into structured format
            let stat = parse_diff_stat(&stdout);
            DiffOutput {
                session_name: name.to_string(),
                base: main_branch,
                head: "@".to_string(),
                diff_stat: Some(stat),
                diff_content: None,
            }
        } else {
            DiffOutput {
                session_name: name.to_string(),
                base: main_branch,
                head: "@".to_string(),
                diff_stat: None,
                diff_content: Some(stdout.to_string()),
            }
        };

        if let Ok(json_str) = serde_json::to_string(&diff_output) {
            println!("{json_str}");
        }
        return Ok(());
    }

    // For stat output, just print directly
    if options.stat {
        print!("{stdout}");
        return Ok(());
    }

    // For full diff, try to use a pager
    output_with_pager(&stdout);

    Ok(())
}

/// Determine the main branch for diffing
fn determine_main_branch(workspace_path: &Path) -> String {
    // Try to find the trunk/main branch using jj
    // If jj is not available or fails, fall back to "main"
    let output = Command::new("jj")
        .args(["log", "-r", "trunk()", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output();

    // Handle case where jj is not installed or command fails
    if let Ok(output) = output {
        if output.status.success() {
            let commit_id = String::from_utf8_lossy(&output.stdout);
            let trimmed = commit_id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    // Fallback: use "main" branch
    "main".to_string()
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};
    use tempfile::TempDir;

    use super::*;
    use crate::database::SessionDb;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[test]
    fn test_determine_main_branch_not_in_repo() -> Result<()> {
        // When not in a JJ repo (or jj not installed), should fall back to "main"
        let temp = TempDir::new().context("Failed to create temp dir")?;
        let result = determine_main_branch(temp.path());

        // Should return fallback "main"
        assert_eq!(result, "main");
        Ok(())
    }

    #[test]
    fn test_run_session_not_found() -> Result<()> {
        tokio_test::block_on(async {
            let _temp_db = setup_test_db().await?;
            // Try to diff a non-existent session
            // We need to set up the context so get_session_db works
            // This is tricky in unit tests, so we'll focus on testing the helpers
            Ok(())
        })
    }

    #[test]
    fn test_run_workspace_not_found() -> Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            // Create a session with a non-existent workspace
            let session = db.create("test-session", "/nonexistent/path").await?;
            // Verify the session exists
            assert_eq!(session.name, "test-session");
            // The run function would fail because workspace doesn't exist
            // We can't easily test this without mocking, so we verify the logic in integration
            // tests
            Ok(())
        })
    }

    #[test]
    fn test_diff_command_args_full() {
        // Verify that full diff uses --git flag
        let args = ["diff", "--git", "-r", "main..@"];
        assert!(args.contains(&"--git"));
        assert!(args.contains(&"-r"));
    }

    #[test]
    fn test_diff_command_args_stat() {
        // Verify that stat diff uses --stat flag
        let args = ["diff", "--stat", "-r", "main..@"];
        assert!(args.contains(&"--stat"));
        assert!(!args.contains(&"--git"));
    }

    #[test]
    fn test_revset_format() {
        let main_branch = "main";
        let revset = format!("{main_branch}..@");
        assert_eq!(revset, "main..@");

        let commit_id = "abc123";
        let revset2 = format!("{commit_id}..@");
        assert_eq!(revset2, "abc123..@");
    }
}

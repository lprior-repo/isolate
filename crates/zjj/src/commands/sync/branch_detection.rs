//! Main branch detection logic for sync operations

use anyhow::Result;

use crate::cli::run_command;

/// Detect the main branch using JJ revsets
///
/// Tries in order:
/// 1. `trunk()` revset (common default)
/// 2. main@origin
/// 3. master@origin
///
/// Returns the first one that exists, or an error if none are found.
pub fn detect_main_branch(workspace_path: &str) -> Result<String> {
    let candidates = ["trunk()", "main@origin", "master@origin"];

    candidates
        .iter()
        .find_map(|candidate| try_revset(workspace_path, candidate).ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not detect main branch. Tried: {}. \
                Please set 'main_branch' in .jjz/config.toml",
                candidates.join(", ")
            )
        })
}

/// Try to evaluate a revset and return it if valid
fn try_revset(workspace_path: &str, revset: &str) -> Result<String> {
    let output = run_command(
        "jj",
        &[
            "--repository",
            workspace_path,
            "log",
            "-r",
            revset,
            "--no-graph",
            "-T",
            "commit_id",
        ],
    )?;

    // If successful and produces output, this branch exists
    if output.trim().is_empty() {
        anyhow::bail!("Revset {revset} produced no output")
    }
    Ok(revset.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_main_branch_error_message() {
        // This will fail since we don't have a real JJ repo in tests
        let result = detect_main_branch("/nonexistent/path");
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("Could not detect main branch"));
            assert!(msg.contains("trunk()"));
            assert!(msg.contains("main@origin"));
            assert!(msg.contains("master@origin"));
        }
    }
}

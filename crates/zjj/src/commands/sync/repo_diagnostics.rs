//! Repository diagnostics for sync error handling
//!
//! Provides functions to diagnose repository state when sync fails,
//! enabling informative error messages with actionable guidance.

use anyhow::Result;

use crate::cli::run_command;

/// Repository state diagnostic information
#[derive(Debug, Clone)]
pub struct RepoState {
    pub commit_count: usize,
    pub has_commits: bool,
}

/// Count commits in a JJ repository
///
/// Returns the number of commits in the repository by counting non-empty revisions.
/// Uses `jj log --no-graph -r 'all()' -T commit_id` to enumerate all commits.
pub fn count_commits(workspace_path: &str) -> Result<usize> {
    let output = run_command(
        "jj",
        &[
            "--repository",
            workspace_path,
            "log",
            "--no-graph",
            "-r",
            "all()",
            "-T",
            "commit_id",
        ],
    )?;

    // Count non-empty lines (each line is a commit ID)
    let count = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    Ok(count)
}

/// Check if a bookmark exists in the repository
///
/// Returns true if the bookmark exists, false otherwise.
pub fn bookmark_exists(workspace_path: &str, bookmark_name: &str) -> Result<bool> {
    let result = run_command(
        "jj",
        &[
            "--repository",
            workspace_path,
            "log",
            "-r",
            bookmark_name,
            "--no-graph",
            "-T",
            "commit_id",
        ],
    );

    // If command succeeds and produces output, bookmark exists
    result.map_or_else(|_| Ok(false), |output| Ok(!output.trim().is_empty()))
}

/// Get diagnostic information about repository state
///
/// Combines multiple checks to provide comprehensive state information
/// for error reporting and user guidance.
pub fn get_repo_state(workspace_path: &str) -> Result<RepoState> {
    let commit_count = count_commits(workspace_path)?;

    Ok(RepoState {
        commit_count,
        has_commits: commit_count > 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_commits_nonexistent_path() {
        // Should fail gracefully on nonexistent path
        let result = count_commits("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_bookmark_exists_nonexistent_path() {
        // Should return false or error on nonexistent path
        let result = bookmark_exists("/nonexistent/path", "main");
        assert!(result.is_err() || !result.unwrap_or(true));
    }

    #[test]
    fn test_get_repo_state_nonexistent_path() {
        // Should fail gracefully on nonexistent path
        let result = get_repo_state("/nonexistent/path");
        assert!(result.is_err());
    }
}

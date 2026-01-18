//! JJ rebase operations and output parsing

use anyhow::{Context, Result};

use super::repo_diagnostics;
use crate::cli::run_command;

/// Result of a rebase operation
#[derive(Debug, Default, Clone)]
pub struct RebaseStats {
    pub rebased_commits: usize,
    pub conflicts: usize,
}

/// Execute a rebase operation in a JJ workspace
pub fn execute_rebase(workspace_path: &str, target_branch: &str) -> Result<RebaseStats> {
    let output = run_command(
        "jj",
        &[
            "--repository",
            workspace_path,
            "rebase",
            "-d",
            target_branch,
        ],
    )
    .or_else(|err| {
        // Check if the error is about the revision/bookmark not existing
        let err_msg = err.to_string();

        if err_msg.contains("doesn't exist") || err_msg.contains("Revision") {
            // Diagnose the repository state to provide helpful error
            diagnose_sync_failure(workspace_path, target_branch, err)
        } else {
            // Return original error for other failure types
            Err(err)
        }
    })
    .context(format!("Failed to sync workspace with {target_branch}"))?;

    Ok(parse_rebase_output(&output))
}

/// Diagnose sync failure and provide actionable error
fn diagnose_sync_failure(
    workspace_path: &str,
    target_branch: &str,
    original_error: anyhow::Error,
) -> Result<String> {
    // Try to get repository state
    match repo_diagnostics::get_repo_state(workspace_path) {
        Ok(state) => {
            if !state.has_commits {
                // No commits in repository
                return Err(anyhow::Error::from(zjj_core::Error::no_commits_yet(
                    workspace_path,
                )));
            }

            // Check if the target bookmark exists
            match repo_diagnostics::bookmark_exists(workspace_path, target_branch) {
                Ok(exists) if !exists => {
                    // Bookmark doesn't exist
                    Err(anyhow::Error::from(zjj_core::Error::main_bookmark_missing(
                        workspace_path,
                        target_branch,
                        state.commit_count,
                    )))
                }
                _ => {
                    // Bookmark exists or check failed, return original error
                    Err(original_error)
                }
            }
        }
        Err(_) => {
            // Could not diagnose, return original error
            Err(original_error)
        }
    }
}

/// Parse jj rebase output to extract commit and conflict counts
///
/// Handles multiple JJ version output formats:
/// - v0.8.0: "Rebased 3 commits"
/// - v0.9.0+: "Rebased 3 descendant commits"
/// - Future versions: May have different wording
pub fn parse_rebase_output(output: &str) -> RebaseStats {
    // Functional approach: fold over lines to accumulate stats
    let (stats, found_rebase_line) = output.lines().fold(
        (RebaseStats::default(), false),
        |(mut stats, found_rebase_line), line| {
            // Check for rebase line
            let new_found_rebase_line = if line.starts_with("Rebased ") {
                stats.rebased_commits = extract_rebase_count(line);
                true
            } else {
                found_rebase_line
            };

            // Check for conflicts (case-insensitive)
            if line.to_lowercase().contains("conflict") {
                stats.conflicts = stats.conflicts.saturating_add(1);
            }

            (stats, new_found_rebase_line)
        },
    );

    // Warn if output is non-empty but we couldn't parse it
    warn_if_unparseable(output, found_rebase_line, stats.conflicts);

    stats
}

/// Extract the commit count from a "Rebased N ..." line
fn extract_rebase_count(line: &str) -> usize {
    line.strip_prefix("Rebased ")
        .and_then(|rest| {
            // Split on whitespace and find first word that's a number
            rest.split_whitespace()
                .find_map(|word| word.parse::<usize>().ok())
        })
        .unwrap_or(0)
}

/// Warn if output appears unparseable
fn warn_if_unparseable(output: &str, found_rebase_line: bool, conflicts: usize) {
    if !output.trim().is_empty() && !found_rebase_line && conflicts == 0 {
        let preview = output
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(200)
            .collect::<String>();

        eprintln!(
            "Warning: Could not parse JJ rebase output. Stats may be inaccurate.\n\
             Output preview: {preview}\n\
             This may indicate a JJ version change. Please report to zjj developers."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rebase_output_with_commits() {
        let output = "Rebased 3 commits\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 3);
        assert_eq!(stats.conflicts, 0);
    }

    #[test]
    fn test_parse_rebase_output_with_conflicts() {
        let output = "Rebased 2 commits\nNew conflicts appeared in these commits:\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 2);
        assert_eq!(stats.conflicts, 1);
    }

    #[test]
    fn test_parse_rebase_output_no_commits() {
        let output = "Already up to date\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 0);
        assert_eq!(stats.conflicts, 0);
    }

    #[test]
    fn test_parse_rebase_output_empty() {
        let output = "";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 0);
        assert_eq!(stats.conflicts, 0);
    }

    #[test]
    fn test_parse_rebase_output_jj_v09_format() {
        let output = "Rebased 5 descendant commits\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 5);
        assert_eq!(stats.conflicts, 0);
    }

    #[test]
    fn test_parse_rebase_output_multiple_number_words() {
        let output = "Rebased 7 commits onto 2 ancestors\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 7);
        assert_eq!(stats.conflicts, 0);
    }

    #[test]
    fn test_parse_rebase_output_case_insensitive_conflicts() {
        let output = "Rebased 1 commits\nCONFLICT in file.txt\n";
        let stats = parse_rebase_output(output);
        assert_eq!(stats.rebased_commits, 1);
        assert_eq!(stats.conflicts, 1);
    }

    #[test]
    fn test_extract_rebase_count_simple() {
        assert_eq!(extract_rebase_count("Rebased 5 commits"), 5);
    }

    #[test]
    fn test_extract_rebase_count_descendant() {
        assert_eq!(extract_rebase_count("Rebased 10 descendant commits"), 10);
    }

    #[test]
    fn test_extract_rebase_count_complex() {
        assert_eq!(
            extract_rebase_count("Rebased 7 commits onto 2 ancestors"),
            7
        );
    }

    #[test]
    fn test_extract_rebase_count_no_number() {
        assert_eq!(extract_rebase_count("Rebased nothing"), 0);
    }
}

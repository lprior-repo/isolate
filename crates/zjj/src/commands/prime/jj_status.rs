//! JJ repository status gathering
//!
//! This module handles querying JJ for repository information,
//! including the repo root, current bookmark, and change status.

use crate::cli::run_command;

use super::output_types::JjStatus;

/// Gather JJ repository status
///
/// Returns a summary of the current JJ repository state, or indicates
/// if not in a JJ repository.
pub fn gather_jj_status() -> JjStatus {
    let in_repo = run_command("jj", &["root"]).is_ok();

    if !in_repo {
        return JjStatus {
            in_repo: false,
            repo_root: None,
            current_bookmark: None,
            has_changes: false,
            change_summary: None,
        };
    }

    let repo_root = run_command("jj", &["root"])
        .ok()
        .map(|s| s.trim().to_string());

    let current_bookmark = run_command("jj", &["log", "-r", "@", "--no-graph", "-T", "bookmarks"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Check for changes
    let status_output = run_command("jj", &["status"]).ok();
    let has_changes = status_output
        .as_ref()
        .map(|s| !s.contains("No changes"))
        .unwrap_or(false);

    let change_summary = if has_changes {
        status_output.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
    } else {
        None
    };

    JjStatus {
        in_repo: true,
        repo_root,
        current_bookmark,
        has_changes,
        change_summary,
    }
}

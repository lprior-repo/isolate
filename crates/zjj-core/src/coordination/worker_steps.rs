//! Worker step implementations for queue processing.
//!
//! This module contains pure functional implementations of each step in the
//! worker pipeline: rebase, test, and merge. Each step is designed to be
//! composable and testable in isolation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::Path;

use thiserror::Error;
use tokio::process::Command;

use crate::coordination::queue::{MergeQueue, QueueStatus};

/// Error type for rebase step operations.
#[derive(Debug, Clone, Error)]
pub enum RebaseError {
    /// The rebase operation resulted in conflicts.
    #[error("rebase conflict: {0}")]
    Conflict(String),

    /// Git fetch failed (network or remote issue).
    #[error("git fetch failed: {0}")]
    FetchFailed(String),

    /// The rebase command failed for an unexpected reason.
    #[error("rebase command failed: {0}")]
    CommandFailed(String),

    /// Failed to get current HEAD SHA.
    #[error("failed to get HEAD SHA: {0}")]
    HeadShaError(String),

    /// Failed to get main branch SHA.
    #[error("failed to get main SHA: {0}")]
    MainShaError(String),

    /// Queue operation failed.
    #[error("queue operation failed: {0}")]
    QueueError(String),

    /// Entry is not in the expected state for rebase.
    #[error("invalid entry state for rebase: expected {expected}, got {actual}")]
    InvalidState {
        expected: &'static str,
        actual: String,
    },
}

/// Result of a successful rebase operation.
#[derive(Debug, Clone)]
pub struct RebaseSuccess {
    /// The new HEAD SHA after rebase.
    pub head_sha: String,
    /// The main branch SHA that was rebased onto.
    pub tested_against_sha: String,
}

/// Perform the rebase step on a workspace.
///
/// This function:
/// 1. Transitions the queue entry to 'rebasing' status
/// 2. Fetches the latest main branch
/// 3. Rebases the workspace onto main
/// 4. On success: persists `head_sha` and `tested_against_sha`, transitions to 'testing'
/// 5. On conflict: transitions to `failed_retryable`
///
/// # Arguments
/// * `queue` - The merge queue to update
/// * `workspace` - The workspace name
/// * `workspace_path` - The filesystem path to the workspace
/// * `main_branch` - The name of the main branch (default: "main")
///
/// # Returns
/// - `Ok(RebaseSuccess)` on successful rebase
/// - `Err(RebaseError::Conflict)` if rebase has conflicts
/// - `Err(RebaseError::FetchFailed)` if network fetch fails
/// - Other `Err(RebaseError)` variants for other failures
///
/// # Errors
///
/// Returns `RebaseError::Conflict` if the rebase operation resulted in conflicts.
/// Returns `RebaseError::FetchFailed` if the git fetch failed due to network or remote issues.
/// Returns `RebaseError::CommandFailed` if the rebase command failed for an unexpected reason.
/// Returns `RebaseError::HeadShaError` if the current HEAD SHA could not be retrieved.
/// Returns `RebaseError::MainShaError` if the main branch SHA could not be retrieved.
/// Returns `RebaseError::QueueError` if a queue operation failed.
/// Returns `RebaseError::InvalidState` if the entry is not in the expected state for rebase.
pub async fn rebase_step(
    queue: &MergeQueue,
    workspace: &str,
    workspace_path: &Path,
    main_branch: &str,
) -> std::result::Result<RebaseSuccess, RebaseError> {
    // Step 1: Validate entry is in claimed state and transition to rebasing
    transition_to_rebasing(queue, workspace).await?;

    // Step 2: Fetch latest main
    fetch_main(workspace_path, main_branch).await?;

    // Step 3: Get the main SHA (for tested_against_sha)
    let tested_against_sha = get_main_sha(workspace_path, main_branch).await?;

    // Step 4: Perform the rebase
    let rebase_result = perform_rebase(workspace_path, main_branch).await;

    match rebase_result {
        Ok(()) => {
            // Step 5: Get new HEAD SHA
            let head_sha = get_head_sha(workspace_path).await?;

            // Step 6: Update queue with rebase metadata and transition to testing
            queue
                .update_rebase_metadata(workspace, &head_sha, &tested_against_sha)
                .await
                .map_err(|e| RebaseError::QueueError(e.to_string()))?;

            Ok(RebaseSuccess {
                head_sha,
                tested_against_sha,
            })
        }
        Err(RebaseError::Conflict(msg)) => {
            // Conflict: transition to failed_retryable
            let _ = queue
                .transition_to_failed(workspace, &msg, true)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to mark entry as failed_retryable: {e}");
                    e
                });
            Err(RebaseError::Conflict(msg))
        }
        Err(e) => {
            // Other error: transition to failed_retryable
            let _ = queue
                .transition_to_failed(workspace, &e.to_string(), true)
                .await;
            Err(e)
        }
    }
}

/// Transition entry to rebasing status.
async fn transition_to_rebasing(
    queue: &MergeQueue,
    workspace: &str,
) -> std::result::Result<(), RebaseError> {
    // Get current entry to validate state
    let entry = queue
        .get_by_workspace(workspace)
        .await
        .map_err(|e| RebaseError::QueueError(e.to_string()))?
        .ok_or_else(|| RebaseError::QueueError(format!("Workspace '{workspace}' not found")))?;

    // Validate entry is in claimed state
    if entry.status != QueueStatus::Claimed {
        return Err(RebaseError::InvalidState {
            expected: "claimed",
            actual: entry.status.as_str().to_string(),
        });
    }

    // Transition to rebasing
    queue
        .transition_to(workspace, QueueStatus::Rebasing)
        .await
        .map_err(|e| RebaseError::QueueError(e.to_string()))?;

    Ok(())
}

/// Fetch the latest main branch from remote.
async fn fetch_main(
    workspace_path: &Path,
    main_branch: &str,
) -> std::result::Result<(), RebaseError> {
    let output = Command::new("jj")
        .args(["git", "fetch", "--branch", main_branch])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| RebaseError::FetchFailed(format!("Failed to execute jj git fetch: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RebaseError::FetchFailed(stderr.to_string()));
    }

    Ok(())
}

/// Get the current HEAD SHA of the workspace.
async fn get_head_sha(workspace_path: &Path) -> std::result::Result<String, RebaseError> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "-T", "commit_id", "--no-graph"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| RebaseError::HeadShaError(format!("Failed to execute jj log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RebaseError::HeadShaError(stderr.to_string()));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(RebaseError::HeadShaError(
            "Empty commit ID returned".to_string(),
        ));
    }

    Ok(sha)
}

/// Get the SHA of the main branch.
async fn get_main_sha(
    workspace_path: &Path,
    main_branch: &str,
) -> std::result::Result<String, RebaseError> {
    let remote_branch = format!("remote-tracking/origin/{main_branch}");
    let output = Command::new("jj")
        .args(["log", "-r", &remote_branch, "-T", "commit_id", "--no-graph"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| RebaseError::MainShaError(format!("Failed to execute jj log for main: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RebaseError::MainShaError(stderr.to_string()));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(RebaseError::MainShaError(
            "Empty commit ID for main branch".to_string(),
        ));
    }

    Ok(sha)
}

/// Perform the rebase operation onto main.
async fn perform_rebase(
    workspace_path: &Path,
    main_branch: &str,
) -> std::result::Result<(), RebaseError> {
    let output = Command::new("jj")
        .args([
            "rebase",
            "-d",
            &format!("remote-tracking/origin/{main_branch}"),
        ])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| RebaseError::CommandFailed(format!("Failed to execute jj rebase: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check for conflict indicators
        if is_conflict_error(&stderr) {
            return Err(RebaseError::Conflict(stderr.to_string()));
        }

        return Err(RebaseError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}

/// Check if the error output indicates a rebase conflict.
fn is_conflict_error(stderr: &str) -> bool {
    let stderr_lower = stderr.to_lowercase();
    stderr_lower.contains("conflict")
        || stderr_lower.contains("could not resolve")
        || stderr_lower.contains("merge conflict")
        || stderr_lower.contains("3-way merge failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_conflict_error_detects_conflicts() {
        assert!(is_conflict_error("Error: conflict in file.rs"));
        assert!(is_conflict_error("Could not resolve revs"));
        assert!(is_conflict_error("Merge conflict detected"));
        assert!(is_conflict_error("3-way merge failed"));
    }

    #[test]
    fn test_is_conflict_error_ignores_other_errors() {
        assert!(!is_conflict_error("Error: network timeout"));
        assert!(!is_conflict_error("Error: permission denied"));
        assert!(!is_conflict_error("something went wrong"));
    }

    #[test]
    fn test_rebase_error_display() {
        let err = RebaseError::Conflict("file conflict".to_string());
        assert_eq!(err.to_string(), "rebase conflict: file conflict");

        let err = RebaseError::FetchFailed("network error".to_string());
        assert_eq!(err.to_string(), "git fetch failed: network error");

        let err = RebaseError::InvalidState {
            expected: "claimed",
            actual: "pending".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid entry state for rebase: expected claimed, got pending"
        );
    }
}

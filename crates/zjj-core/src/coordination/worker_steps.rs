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

use crate::{
    coordination::{
        queue::{MergeQueue, QueueEntry, QueueStatus},
        queue_status::QueueEventType,
    },
    worker_error::{classify_with_attempts, should_retry, ErrorClass},
};

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

/// Metadata about a rebase operation for persistence.
#[derive(Debug, Clone)]
pub struct RebaseMetadata {
    /// The new HEAD SHA after rebase.
    pub head_sha: String,
    /// The main branch SHA that was rebased onto.
    pub tested_against_sha: String,
    /// Total number of rebase attempts for this entry (including this one).
    pub rebase_count: i32,
    /// Timestamp of this rebase (Unix epoch seconds).
    pub rebase_timestamp: i64,
}

/// Result of a successful rebase operation.
#[derive(Debug, Clone)]
pub struct RebaseSuccess {
    /// The new HEAD SHA after rebase.
    pub head_sha: String,
    /// The main branch SHA that was rebased onto.
    pub tested_against_sha: String,
    /// Total number of rebase attempts for this entry (including this one).
    pub rebase_count: i32,
    /// Timestamp of this rebase (Unix epoch seconds).
    pub rebase_timestamp: i64,
}

impl RebaseSuccess {
    /// Convert to metadata for persistence.
    #[must_use]
    pub fn to_metadata(&self) -> RebaseMetadata {
        RebaseMetadata {
            head_sha: self.head_sha.clone(),
            tested_against_sha: self.tested_against_sha.clone(),
            rebase_count: self.rebase_count,
            rebase_timestamp: self.rebase_timestamp,
        }
    }
}

/// Perform the rebase step on a workspace.
///
/// This function:
/// 1. Transitions the queue entry to 'rebasing' status
/// 2. Fetches the latest main branch
/// 3. Rebases the workspace onto main
/// 4. On success: persists rebase metadata (`head_sha`, `tested_against_sha`, `rebase_count`,
///    timestamp) and emits an audit event, then transitions to 'testing'
/// 5. On conflict: increments `rebase_count`, emits failure event, transitions to
///    `failed_retryable`
///
/// # Arguments
/// * `queue` - The merge queue to update
/// * `workspace` - The workspace name
/// * `workspace_path` - The filesystem path to the workspace
/// * `main_branch` - The name of the main branch (default: "main")
///
/// # Returns
/// - `Ok(RebaseSuccess)` on successful rebase, including metadata about the rebase
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
    let entry = transition_to_rebasing(queue, workspace).await?;

    // Calculate rebase_count: increment from current entry's count
    let rebase_count = entry.rebase_count + 1;
    let rebase_timestamp = chrono::Utc::now().timestamp();

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
                .update_rebase_metadata_with_count(
                    workspace,
                    &head_sha,
                    &tested_against_sha,
                    rebase_count,
                    rebase_timestamp,
                )
                .await
                .map_err(|e| RebaseError::QueueError(e.to_string()))?;

            // Step 7: Record rebase success event in audit trail
            record_rebase_event(
                queue,
                entry.id,
                &head_sha,
                &tested_against_sha,
                rebase_count,
                true,
            )
            .await;

            Ok(RebaseSuccess {
                head_sha,
                tested_against_sha,
                rebase_count,
                rebase_timestamp,
            })
        }
        Err(RebaseError::Conflict(msg)) => {
            // Conflict: record failure event and transition to failed_retryable
            record_rebase_event(
                queue,
                entry.id,
                "",
                &tested_against_sha,
                rebase_count,
                false,
            )
            .await;

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
            // Other error: record failure event and transition to failed_retryable
            let error_msg = e.to_string();
            record_rebase_event(
                queue,
                entry.id,
                "",
                &tested_against_sha,
                rebase_count,
                false,
            )
            .await;

            let _ = queue
                .transition_to_failed(workspace, &error_msg, true)
                .await;
            Err(e)
        }
    }
}

/// Transition entry to rebasing status.
///
/// Returns the queue entry on success so callers can access entry metadata.
async fn transition_to_rebasing(
    queue: &MergeQueue,
    workspace: &str,
) -> std::result::Result<QueueEntry, RebaseError> {
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

    Ok(entry)
}

/// Get the jj binary path from environment or use default.
#[allow(clippy::unnecessary_result_map_or_else)]
fn jj_bin_path() -> String {
    std::env::var("ZJJ_JJ_PATH").map_or_else(|_| "jj".to_string(), |path| path)
}

/// Fetch the latest main branch from remote.
async fn fetch_main(
    workspace_path: &Path,
    main_branch: &str,
) -> std::result::Result<(), RebaseError> {
    let jj_bin = jj_bin_path();

    let output = Command::new(&jj_bin)
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
    let jj_bin = jj_bin_path();

    let output = Command::new(&jj_bin)
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
    let jj_bin = jj_bin_path();
    let remote_branch = format!("remote-tracking/origin/{main_branch}");

    let output = Command::new(&jj_bin)
        .args(["log", "-r", &remote_branch, "-T", "commit_id", "--no-graph"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| {
            RebaseError::MainShaError(format!("Failed to execute jj log for main: {e}"))
        })?;

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
    let jj_bin = jj_bin_path();

    let output = Command::new(&jj_bin)
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

/// Record a rebase event in the audit trail.
///
/// This is a best-effort operation that logs failures but does not propagate errors.
/// Events are for audit trail purposes and are not critical path.
#[allow(clippy::too_many_arguments)]
async fn record_rebase_event(
    queue: &MergeQueue,
    queue_id: i64,
    head_sha: &str,
    tested_against_sha: &str,
    rebase_count: i32,
    success: bool,
) {
    let event_type = if success {
        QueueEventType::Transitioned
    } else {
        QueueEventType::Failed
    };

    let details = if success {
        format!(
            r#"{{"step": "rebase", "head_sha": "{head_sha}", "tested_against_sha": "{tested_against_sha}", "rebase_count": {rebase_count}}}"#
        )
    } else {
        format!(
            r#"{{"step": "rebase", "tested_against_sha": "{tested_against_sha}", "rebase_count": {rebase_count}, "success": false}}"#
        )
    };

    match queue
        .append_typed_event(queue_id, event_type, Some(&details))
        .await
    {
        Ok(()) => {}
        Err(e) => {
            tracing::warn!("Failed to record rebase event for queue entry {queue_id}: {e}");
        }
    }
}

/// Classify an error message and determine if it should be retried.
///
/// This function combines error classification with attempt count checking
/// to determine if an error should result in a retryable or terminal failure.
///
/// # Arguments
/// * `error_msg` - The error message to classify
/// * `entry` - The queue entry with attempt count information
///
/// # Returns
/// `true` if the error is retryable and attempts remain, `false` otherwise.
#[must_use]
pub fn classify_step_error(error_msg: &str, entry: &QueueEntry) -> bool {
    should_retry(error_msg, entry.attempt_count, entry.max_attempts)
}

/// Determine the target failure status based on error classification.
///
/// WHEN retryable failure occurs -> move to `failed_retryable` and increment attempts.
/// WHEN max attempts exceeded -> move to `failed_terminal`.
///
/// # Arguments
/// * `error_msg` - The error message to classify
/// * `entry` - The queue entry with attempt count information
///
/// # Returns
/// `QueueStatus::FailedRetryable` if the error can be retried, `QueueStatus::FailedTerminal`
/// otherwise.
#[must_use]
pub fn determine_failure_status(error_msg: &str, entry: &QueueEntry) -> QueueStatus {
    let error_class = classify_with_attempts(error_msg, entry.attempt_count, entry.max_attempts);
    match error_class {
        ErrorClass::Retryable => QueueStatus::FailedRetryable,
        ErrorClass::Terminal => QueueStatus::FailedTerminal,
    }
}

/// Error type for moon gate step operations.
#[derive(Debug, Clone, Error)]
pub enum MoonGateError {
    /// The moon command execution failed.
    #[error("moon gate failed: {0}")]
    GateFailed(String),

    /// Failed to execute the moon command.
    #[error("moon command execution error: {0}")]
    ExecutionError(String),

    /// Queue operation failed.
    #[error("queue operation failed: {0}")]
    QueueError(String),

    /// Entry is not in the expected state for testing.
    #[error("invalid entry state for moon gate: expected {expected}, got {actual}")]
    InvalidState {
        expected: &'static str,
        actual: String,
    },
}

/// Result of a successful moon gate operation.
#[derive(Debug, Clone)]
pub struct MoonGateSuccess {
    /// The exit code (0 for success).
    pub exit_code: i32,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
    /// The gate that was run (e.g., ":check", ":test").
    pub gate: String,
}

/// Configuration for moon gate execution.
#[derive(Debug, Clone)]
pub struct MoonGateConfig {
    /// The moon gate to run (e.g., ":check", ":test", ":quick").
    pub gate: String,
}

impl Default for MoonGateConfig {
    fn default() -> Self {
        Self {
            gate: ":check".to_string(),
        }
    }
}

impl MoonGateConfig {
    /// Create a new configuration with the specified gate.
    #[must_use]
    pub fn new(gate: &str) -> Self {
        Self {
            gate: gate.to_string(),
        }
    }

    /// Create a configuration for the :check gate.
    #[must_use]
    pub fn check() -> Self {
        Self::new(":check")
    }

    /// Create a configuration for the :test gate.
    #[must_use]
    pub fn test() -> Self {
        Self::new(":test")
    }

    /// Create a configuration for the :quick gate.
    #[must_use]
    pub fn quick() -> Self {
        Self::new(":quick")
    }
}

/// Perform the moon gate step on a workspace.
///
/// This function:
/// 1. Validates the entry is in 'testing' status
/// 2. Executes `moon run :<gate>` in the workspace
/// 3. Captures exit code, stdout, and stderr
/// 4. On success (exit code 0): transitions to 'ready'
/// 5. On failure: transitions to `failed_retryable`
///
/// # Arguments
/// * `queue` - The merge queue to update
/// * `workspace` - The workspace name
/// * `workspace_path` - The filesystem path to the workspace
/// * `config` - Configuration for which gate to run
///
/// # Returns
/// - `Ok(MoonGateSuccess)` on successful gate pass
/// - `Err(MoonGateError::GateFailed)` if the gate fails
/// - `Err(MoonGateError::ExecutionError)` if command execution fails
/// - Other `Err(MoonGateError)` variants for other failures
///
/// # Errors
///
/// Returns `MoonGateError::GateFailed` if the moon gate command returned a non-zero exit code.
/// Returns `MoonGateError::ExecutionError` if the moon command could not be executed.
/// Returns `MoonGateError::QueueError` if a queue operation failed.
/// Returns `MoonGateError::InvalidState` if the entry is not in the expected state for testing.
pub async fn moon_gate_step(
    queue: &MergeQueue,
    workspace: &str,
    workspace_path: &Path,
    config: &MoonGateConfig,
) -> std::result::Result<MoonGateSuccess, MoonGateError> {
    // Step 1: Validate entry is in testing state
    validate_testing_state(queue, workspace).await?;

    // Step 2: Execute moon gate
    let gate_result = execute_moon_gate(workspace_path, &config.gate).await;

    match gate_result {
        Ok(success) => {
            // Step 3: Transition to ready on success
            queue
                .transition_to(workspace, QueueStatus::ReadyToMerge)
                .await
                .map_err(|e| MoonGateError::QueueError(e.to_string()))?;

            Ok(success)
        }
        Err(MoonGateError::GateFailed(ref msg)) => {
            // Failure: transition to failed_retryable
            let _ = queue
                .transition_to_failed(workspace, msg, true)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to mark entry as failed_retryable: {e}");
                    e
                });
            Err(MoonGateError::GateFailed(msg.clone()))
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

/// Validate entry is in testing state.
async fn validate_testing_state(
    queue: &MergeQueue,
    workspace: &str,
) -> std::result::Result<(), MoonGateError> {
    let entry = queue
        .get_by_workspace(workspace)
        .await
        .map_err(|e| MoonGateError::QueueError(e.to_string()))?
        .ok_or_else(|| MoonGateError::QueueError(format!("Workspace '{workspace}' not found")))?;

    if entry.status != QueueStatus::Testing {
        return Err(MoonGateError::InvalidState {
            expected: "testing",
            actual: entry.status.as_str().to_string(),
        });
    }

    Ok(())
}

/// Get the moon binary path from environment or use default.
#[allow(clippy::unnecessary_result_map_or_else)]
fn moon_bin_path() -> String {
    std::env::var("ZJJ_MOON_PATH").map_or_else(|_| "moon".to_string(), |path| path)
}

/// Execute the moon gate command and capture output.
async fn execute_moon_gate(
    workspace_path: &Path,
    gate: &str,
) -> std::result::Result<MoonGateSuccess, MoonGateError> {
    let moon_bin = moon_bin_path();

    let output = Command::new(&moon_bin)
        .args(["run", gate])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| {
            MoonGateError::ExecutionError(format!("Failed to execute moon run {gate}: {e}"))
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    // Use map_or to handle None case for exit code
    let exit_code = output.status.code().map_or(-1, |code| code);

    if !output.status.success() {
        let error_msg = if stderr.is_empty() {
            format!("moon run {gate} exited with code {exit_code}")
        } else {
            stderr
        };
        return Err(MoonGateError::GateFailed(error_msg));
    }

    Ok(MoonGateSuccess {
        exit_code,
        stdout,
        stderr,
        gate: gate.to_string(),
    })
}

/// Handle a step failure with proper error classification.
///
/// This function classifies the error, determines the appropriate target status,
/// and transitions the entry accordingly.
///
/// # Arguments
/// * `queue` - The merge queue
/// * `workspace` - The workspace that failed
/// * `error_msg` - The error message
/// * `entry` - The queue entry
///
/// # Returns
/// The target status that the entry was transitioned to.
///
/// # Errors
/// Returns an error if the queue transition fails.
pub async fn handle_step_failure(
    queue: &MergeQueue,
    workspace: &str,
    error_msg: &str,
    entry: &QueueEntry,
) -> crate::Result<QueueStatus> {
    let target_status = determine_failure_status(error_msg, entry);
    let is_retryable = target_status == QueueStatus::FailedRetryable;

    queue
        .transition_to_failed(workspace, error_msg, is_retryable)
        .await?;

    Ok(target_status)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

    #[cfg(unix)]
    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    #[cfg(unix)]
    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    #[cfg(unix)]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[cfg(unix)]
    fn write_fake_jj_script(dir: &std::path::Path) -> std::io::Result<PathBuf> {
        let script = r#"#!/bin/sh
if [ "$1" = "git" ] && [ "$2" = "fetch" ]; then
  exit 0
fi

if [ "$1" = "log" ] && [ "$2" = "-r" ]; then
  if [ "$3" = "@" ]; then
    echo "HEAD_SHA_TEST"
    exit 0
  fi
  case "$3" in
    remote-tracking/origin/*)
      echo "MAIN_SHA_TEST"
      exit 0
      ;;
  esac
fi

if [ "$1" = "rebase" ]; then
  if [ "${ZJJ_TEST_REBASE_CONFLICT}" = "1" ]; then
    echo "conflict: test" 1>&2
    exit 1
  fi
  exit 0
fi

echo "unexpected jj invocation" 1>&2
exit 1
"#;

        let path = dir.join("jj");
        std::fs::write(&path, script)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(path)
    }

    #[cfg(unix)]
    fn prepend_path(dir: &std::path::Path) -> EnvGuard {
        let previous = std::env::var("PATH").ok();
        let new_path = previous.as_ref().map_or_else(
            || dir.display().to_string(),
            |existing| format!("{}:{}", dir.display(), existing),
        );
        EnvGuard::set("PATH", &new_path)
    }

    #[cfg(unix)]
    fn set_moon_path(moon_path: &std::path::Path) -> EnvGuard {
        EnvGuard::set("ZJJ_MOON_PATH", &moon_path.display().to_string())
    }

    #[cfg(unix)]
    fn set_jj_path(jj_path: &std::path::Path) -> EnvGuard {
        EnvGuard::set("ZJJ_JJ_PATH", &jj_path.display().to_string())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_rebase_step_persists_metadata_on_success(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let jj_path = write_fake_jj_script(temp_dir.path())?;
        let _jj_guard = set_jj_path(&jj_path);
        // Ensure conflict mode is NOT set (test isolation)
        // Save the previous value and clear it for this test
        let previous_conflict = std::env::var("ZJJ_TEST_REBASE_CONFLICT").ok();
        std::env::remove_var("ZJJ_TEST_REBASE_CONFLICT");

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-rebase-step", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-rebase-step").await?;
        assert!(claimed.is_some(), "Entry should be claimed");

        let result = rebase_step(&queue, "ws-rebase-step", workspace_dir.path(), "main")
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        assert_eq!(result.head_sha, "HEAD_SHA_TEST");
        assert_eq!(result.tested_against_sha, "MAIN_SHA_TEST");
        // Verify new metadata fields
        assert!(
            result.rebase_count >= 1,
            "rebase_count should be at least 1"
        );
        assert!(
            result.rebase_timestamp > 0,
            "rebase_timestamp should be positive"
        );

        let updated = queue
            .get_by_workspace("ws-rebase-step")
            .await?
            .ok_or("entry missing")?;
        assert_eq!(updated.status, QueueStatus::Testing);
        assert_eq!(updated.head_sha, Some("HEAD_SHA_TEST".to_string()));
        assert_eq!(
            updated.tested_against_sha,
            Some("MAIN_SHA_TEST".to_string())
        );
        // Verify rebase_count and last_rebase_at are persisted
        assert!(
            updated.rebase_count >= 1,
            "persisted rebase_count should be at least 1"
        );
        assert!(
            updated.last_rebase_at.is_some(),
            "last_rebase_at should be set"
        );

        // Restore previous env var state
        match previous_conflict {
            Some(val) => std::env::set_var("ZJJ_TEST_REBASE_CONFLICT", &val),
            None => std::env::remove_var("ZJJ_TEST_REBASE_CONFLICT"),
        }

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_rebase_step_conflict_marks_failed_retryable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let jj_path = write_fake_jj_script(temp_dir.path())?;
        let _jj_guard = set_jj_path(&jj_path);
        let _conflict_guard = EnvGuard::set("ZJJ_TEST_REBASE_CONFLICT", "1");

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-rebase-conflict", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-rebase-conflict").await?;
        assert!(claimed.is_some(), "Entry should be claimed");

        let result = rebase_step(&queue, "ws-rebase-conflict", workspace_dir.path(), "main").await;

        assert!(matches!(result, Err(RebaseError::Conflict(_))));

        let updated = queue
            .get_by_workspace("ws-rebase-conflict")
            .await?
            .ok_or("entry missing")?;
        assert_eq!(updated.status, QueueStatus::FailedRetryable);
        assert!(updated.head_sha.is_none(), "head_sha should be unset");
        assert!(
            updated.tested_against_sha.is_none(),
            "tested_against_sha should be unset"
        );

        Ok(())
    }

    #[test]
    fn test_moon_gate_config_defaults_to_check() {
        let config = MoonGateConfig::default();
        assert_eq!(config.gate, ":check");
    }

    #[test]
    fn test_moon_gate_config_convenience_constructors() {
        assert_eq!(MoonGateConfig::check().gate, ":check");
        assert_eq!(MoonGateConfig::test().gate, ":test");
        assert_eq!(MoonGateConfig::quick().gate, ":quick");
    }

    #[test]
    fn test_moon_gate_error_display() {
        let err = MoonGateError::GateFailed("lint error".to_string());
        assert_eq!(err.to_string(), "moon gate failed: lint error");

        let err = MoonGateError::ExecutionError("command not found".to_string());
        assert_eq!(
            err.to_string(),
            "moon command execution error: command not found"
        );

        let err = MoonGateError::InvalidState {
            expected: "testing",
            actual: "pending".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid entry state for moon gate: expected testing, got pending"
        );
    }

    #[cfg(unix)]
    fn write_fake_moon_script(dir: &std::path::Path) -> std::io::Result<PathBuf> {
        let script = r#"#!/bin/sh
if [ "${ZJJ_TEST_MOON_FAIL}" = "1" ]; then
  echo "error: lint failed" 1>&2
  exit 1
fi

if [ "${ZJJ_TEST_MOON_STDERR}" = "1" ]; then
  echo "warning: deprecated" 1>&2
fi

echo "moon run :check completed"
exit 0
"#;

        let path = dir.join("moon");
        std::fs::write(&path, script)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(path)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_moon_gate_step_success() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let moon_path = write_fake_moon_script(temp_dir.path())?;
        let _moon_guard = set_moon_path(&moon_path);

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-moon-test", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-moon-test").await?;
        assert!(claimed.is_some(), "Entry should be claimed");

        // Manually transition through state machine: claimed -> rebasing -> testing
        queue
            .transition_to("ws-moon-test", QueueStatus::Rebasing)
            .await?;
        queue
            .transition_to("ws-moon-test", QueueStatus::Testing)
            .await?;

        let config = MoonGateConfig::check();
        let result = moon_gate_step(&queue, "ws-moon-test", workspace_dir.path(), &config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("moon run :check completed"));
        assert_eq!(result.gate, ":check");

        let updated = queue
            .get_by_workspace("ws-moon-test")
            .await?
            .ok_or("entry missing")?;
        assert_eq!(updated.status, QueueStatus::ReadyToMerge);

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_moon_gate_step_failure_marks_retryable() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let moon_path = write_fake_moon_script(temp_dir.path())?;
        let _moon_guard = set_moon_path(&moon_path);
        let _fail_guard = EnvGuard::set("ZJJ_TEST_MOON_FAIL", "1");

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-moon-fail", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-moon-fail").await?;
        assert!(claimed.is_some(), "Entry should be claimed");

        // Manually transition through state machine: claimed -> rebasing -> testing
        queue
            .transition_to("ws-moon-fail", QueueStatus::Rebasing)
            .await?;
        queue
            .transition_to("ws-moon-fail", QueueStatus::Testing)
            .await?;

        let config = MoonGateConfig::check();
        let result = moon_gate_step(&queue, "ws-moon-fail", workspace_dir.path(), &config).await;

        assert!(matches!(result, Err(MoonGateError::GateFailed(_))));

        let updated = queue
            .get_by_workspace("ws-moon-fail")
            .await?
            .ok_or("entry missing")?;
        assert_eq!(updated.status, QueueStatus::FailedRetryable);

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_moon_gate_step_invalid_state() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let moon_path = write_fake_moon_script(temp_dir.path())?;
        let _moon_guard = set_moon_path(&moon_path);

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-moon-invalid", None, 5, None).await?;

        // Entry is in 'pending' state, not 'testing'
        let config = MoonGateConfig::check();
        let result = moon_gate_step(&queue, "ws-moon-invalid", workspace_dir.path(), &config).await;

        assert!(matches!(result, Err(MoonGateError::InvalidState { .. })));

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_moon_gate_step_captures_stderr() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let workspace_dir = tempfile::tempdir()?;
        let moon_path = write_fake_moon_script(temp_dir.path())?;
        let _moon_guard = set_moon_path(&moon_path);
        let _stderr_guard = EnvGuard::set("ZJJ_TEST_MOON_STDERR", "1");

        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-moon-stderr", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-moon-stderr").await?;
        assert!(claimed.is_some(), "Entry should be claimed");

        // Manually transition through state machine: claimed -> rebasing -> testing
        queue
            .transition_to("ws-moon-stderr", QueueStatus::Rebasing)
            .await?;
        queue
            .transition_to("ws-moon-stderr", QueueStatus::Testing)
            .await?;

        let config = MoonGateConfig::check();
        let result = moon_gate_step(&queue, "ws-moon-stderr", workspace_dir.path(), &config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

        assert_eq!(result.exit_code, 0);
        assert!(result.stderr.contains("warning: deprecated"));

        Ok(())
    }
}

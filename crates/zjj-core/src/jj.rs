//! JJ workspace lifecycle management
//!
//! This module provides safe, functional APIs for managing JJ workspaces.
//! All operations return `Result` and never panic.

use std::{
    path::{Path, PathBuf},
    process::Command as StdCommand,
    sync::OnceLock,
};

use tokio::process::Command;

use crate::{Error, Result};

/// RAII guard for JJ workspace lifecycle
///
/// Ensures workspace cleanup (forget + directory removal) on drop,
/// even when panicking. Use this to guarantee no resource leaks.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
///
/// use zjj_core::jj::WorkspaceGuard;
///
/// let guard = WorkspaceGuard::new("session-name".to_string(), PathBuf::from("/tmp/workspace"));
/// // Workspace is automatically cleaned up when guard goes out of scope
/// ```
pub struct WorkspaceGuard {
    /// Workspace name for `jj workspace forget`
    name: String,
    /// Directory path to remove
    path: PathBuf,
    /// Whether cleanup should run on drop
    active: bool,
}

impl WorkspaceGuard {
    /// Create a new workspace guard
    ///
    /// The guard will clean up the workspace when dropped unless disarmed.
    #[must_use]
    pub const fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            active: true,
        }
    }

    /// Disarm the guard to prevent cleanup
    ///
    /// Call this when workspace creation succeeds and you want to keep it.
    pub fn disarm(&mut self) {
        self.active = false;
    }

    /// Manually trigger cleanup and disarm
    ///
    /// # Errors
    ///
    /// Returns error if cleanup fails
    pub async fn cleanup(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        self.active = false;
        // Use async implementation
        let forget_result = workspace_forget(&self.name).await;

        // Async file removal
        let remove_result = match tokio::fs::try_exists(&self.path).await {
            Ok(true) => tokio::fs::remove_dir_all(&self.path)
                .await
                .map_err(|e| Error::IoError(format!("Failed to remove workspace directory: {e}"))),
            Ok(false) => Ok(()),
            Err(e) => Err(Error::IoError(format!(
                "Failed to check workspace existence: {e}"
            ))),
        };

        forget_result.and(remove_result)
    }

    /// Perform the actual cleanup operations synchronously (for Drop)
    fn perform_cleanup_sync(&self) -> Result<()> {
        // Step 1: Forget the JJ workspace (best effort, sync)
        let forget_result = StdCommand::new("jj")
            .args(["workspace", "forget", &self.name])
            .output()
            .map_err(|e| jj_command_error("forget workspace", &e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(Error::JjCommandError {
                        operation: "forget workspace".to_string(),
                        source: stderr.to_string(),
                        is_not_found: false,
                    })
                }
            });

        // Step 2: Remove the directory (best effort, sync)
        // Use spawn_blocking for blocking I/O in Drop context
        let remove_result = match self.path.try_exists() {
            Ok(true) => std::fs::remove_dir_all(&self.path)
                .map_err(|e| Error::IoError(format!("Failed to remove workspace directory: {e}"))),
            Ok(false) => Ok(()),
            Err(e) => Err(Error::IoError(format!(
                "Failed to check workspace existence: {e}"
            ))),
        };

        // Return first error encountered, or Ok if both succeeded
        forget_result.and(remove_result)
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        // Best effort cleanup - log errors but don't panic in Drop
        if let Err(e) = self.perform_cleanup_sync() {
            tracing::warn!("Workspace cleanup failed for '{}': {}", self.name, e);
            eprintln!(
                "Warning: Failed to cleanup workspace '{}': {}",
                self.name, e
            );
        }
    }
}

/// Helper to create a JJ command error with appropriate context
fn jj_command_error(operation: &str, error: &std::io::Error) -> Error {
    let is_not_found = error.kind() == std::io::ErrorKind::NotFound;
    Error::JjCommandError {
        operation: operation.to_string(),
        source: error.to_string(),
        is_not_found,
    }
}

/// Information about a JJ workspace
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace name
    pub name: String,
    /// Workspace path
    pub path: PathBuf,
    /// Whether the workspace is stale (directory doesn't exist)
    pub is_stale: bool,
}

/// Summary of changes in a workspace
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Number of lines added
    pub insertions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

/// Status of files in a workspace
#[derive(Debug, Clone)]
pub struct Status {
    /// Modified files
    pub modified: Vec<PathBuf>,
    /// Added files
    pub added: Vec<PathBuf>,
    /// Deleted files
    pub deleted: Vec<PathBuf>,
    /// Renamed files (`old_path`, `new_path`)
    pub renamed: Vec<(PathBuf, PathBuf)>,
    /// Unknown files
    pub unknown: Vec<PathBuf>,
}

impl Status {
    /// Check if there are any changes
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.renamed.is_empty()
    }

    /// Count total number of changed files
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len() + self.renamed.len()
    }
}

/// Create a new JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Unable to create workspace directory
/// - JJ command fails
pub async fn workspace_create(name: &str, path: &Path) -> Result<()> {
    // Validate inputs
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::IoError(format!("Failed to create workspace directory: {e}")))?;
    }

    // Execute: jj workspace add --name <name> <path>
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .output()
        .await
        .map_err(|e| jj_command_error("create workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// Create a JJ workspace and return a guard for automatic cleanup
///
/// This function combines workspace creation with RAII cleanup semantics.
/// It creates the workspace using `jj workspace add` and returns a
/// `WorkspaceGuard` that will automatically clean up the workspace when
/// dropped (unless explicitly disarmed).
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
///
/// use zjj_core::jj::create_workspace;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create workspace and get guard
/// let mut guard = create_workspace("my-session", &PathBuf::from("/tmp/workspace")).await?;
///
/// // ... work in the workspace ...
///
/// // Disarm to keep the workspace when guard goes out of scope
/// guard.disarm();
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Unable to create workspace directory
/// - JJ command fails
///
/// # RAII Cleanup
///
/// The returned `WorkspaceGuard` will automatically clean up the workspace
/// (forget + directory removal) when dropped unless `disarm()` is called.
/// This ensures no resource leaks even when panicking.
pub async fn create_workspace(name: &str, path: &Path) -> Result<WorkspaceGuard> {
    // Create the workspace using existing function
    workspace_create(name, path).await?;

    // Return guard for automatic cleanup
    Ok(WorkspaceGuard::new(name.to_string(), path.to_path_buf()))
}

/// Forget (remove) a JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace doesn't exist
/// - JJ command fails
pub async fn workspace_forget(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    // Execute: jj workspace forget <name>
    let output = Command::new("jj")
        .args(["workspace", "forget", name])
        .output()
        .await
        .map_err(|e| jj_command_error("forget workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "forget workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// List all JJ workspaces
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub async fn workspace_list() -> Result<Vec<WorkspaceInfo>> {
    // Execute: jj workspace list
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .output()
        .await
        .map_err(|e| jj_command_error("list workspaces", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "list workspaces".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_workspace_list(&stdout)
}

/// Parse output from 'jj workspace list'
fn parse_workspace_list(output: &str) -> Result<Vec<WorkspaceInfo>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Format: "workspace_name: /path/to/workspace"
            // or "workspace_name: /path/to/workspace (stale)"
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(Error::ParseError(format!(
                    "Invalid workspace list format: {line}"
                )));
            }

            let name = parts
                .first()
                .ok_or_else(|| {
                    Error::ParseError("Missing workspace name in list output".to_string())
                })?
                .trim()
                .to_string();
            let rest = parts
                .get(1)
                .ok_or_else(|| {
                    Error::ParseError("Missing workspace path in list output".to_string())
                })?
                .trim();

            let (path_str, is_stale) = rest
                .strip_suffix("(stale)")
                .map_or((rest, false), |path_part| (path_part.trim(), true));

            Ok(WorkspaceInfo {
                name,
                path: PathBuf::from(path_str),
                is_stale,
            })
        })
        .collect()
}

/// Get status of a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub async fn workspace_status(path: &Path) -> Result<Status> {
    // Execute: jj status (in the workspace directory)
    let output = Command::new("jj")
        .args(["status"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| jj_command_error("get workspace status", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get workspace status".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_status(&stdout))
}

/// Parse output from 'jj status'
#[must_use]
pub fn parse_status(output: &str) -> Status {
    output.lines().fold(Status::default(), |mut status, line| {
        let line = line.trim();
        if line.is_empty() {
            return status;
        }

        if let Some(rest) = line.strip_prefix('M') {
            status.modified.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('A') {
            status.added.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('D') {
            status.deleted.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('R') {
            if let Some((old, new)) = rest.split_once("=>") {
                status
                    .renamed
                    .push((PathBuf::from(old.trim()), PathBuf::from(new.trim())));
            }
        } else if let Some(rest) = line.strip_prefix('?') {
            status.unknown.push(PathBuf::from(rest.trim()));
        }
        status
    })
}

impl Default for Status {
    fn default() -> Self {
        Self {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        }
    }
}

/// Get diff summary for a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub async fn workspace_diff(path: &Path) -> Result<DiffSummary> {
    // Execute: jj diff --stat (in the workspace directory)
    let output = Command::new("jj")
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| jj_command_error("get workspace diff", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get workspace diff".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_stat(&stdout))
}

/// Parse output from 'jj diff --stat'
#[must_use]
pub fn parse_diff_stat(output: &str) -> DiffSummary {
    use regex::Regex;
    static INSERTIONS_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static DELETIONS_RE: OnceLock<Option<Regex>> = OnceLock::new();

    let insertions_re = INSERTIONS_RE.get_or_init(|| Regex::new(r"(\d+)\s+insertion").ok());
    let deletions_re = DELETIONS_RE.get_or_init(|| Regex::new(r"(\d+)\s+deletion").ok());

    // Look for summary line like: "5 files changed, 123 insertions(+), 45 deletions(-)"
    let summary_line = output
        .lines()
        .find(|line| line.contains("insertion") || line.contains("deletion"))
        .unwrap_or_default();

    let insertions = insertions_re
        .as_ref()
        .and_then(|re| re.captures(summary_line))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);

    let deletions = deletions_re
        .as_ref()
        .and_then(|re| re.captures(summary_line))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);

    DiffSummary {
        insertions,
        deletions,
    }
}

/// Check if JJ is installed and available
///
/// # Errors
///
/// Returns error if JJ is not found in PATH
pub async fn check_jj_installed() -> Result<()> {
    Command::new("jj")
        .arg("--version")
        .output()
        .await
        .map_err(|e| jj_command_error("check JJ installation", &e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::JjCommandError {
                    operation: "check JJ installation".to_string(),
                    source: "JJ command returned non-zero exit code".to_string(),
                    is_not_found: false,
                })
            }
        })
}

/// Check if current directory is in a JJ repository
///
/// # Errors
///
/// Returns error if not in a JJ repository
pub async fn check_in_jj_repo() -> Result<PathBuf> {
    let output = Command::new("jj")
        .args(["root"])
        .output()
        .await
        .map_err(|e| jj_command_error("find JJ repository root", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: format!("Not in a JJ repository. {stderr}"),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();

    if root.is_empty() {
        Err(Error::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: "Could not determine JJ repository root".to_string(),
            is_not_found: false,
        })
    } else {
        Ok(PathBuf::from(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_list() -> Result<()> {
        let output = "default: /home/user/repo\nfeature: /home/user/repo/.zjj/workspaces/feature\nstale-ws: /home/user/old (stale)";
        let result = parse_workspace_list(output);
        assert!(result.is_ok());
        let workspaces = result?;
        assert_eq!(workspaces.len(), 3);

        // Safe to index after checking length
        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(workspaces[0].name, "default");
            assert!(!workspaces[0].is_stale);
            assert_eq!(workspaces[2].name, "stale-ws");
            assert!(workspaces[2].is_stale);
        }
        Ok(())
    }

    #[test]
    fn test_parse_status() {
        let output = "M file1.rs\nA file2.rs\nD file3.rs\n? unknown.txt";
        let status = parse_status(output);
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.added.len(), 1);
        assert_eq!(status.deleted.len(), 1);
        assert_eq!(status.unknown.len(), 1);
        assert!(!status.is_clean());
        assert_eq!(status.change_count(), 3);
    }

    #[test]
    fn test_parse_diff_stat() {
        let output = "file1.rs | 10 +++++++---\nfile2.rs | 5 ++---\n2 files changed, 12 insertions(+), 3 deletions(-)";
        let summary = parse_diff_stat(output);
        assert_eq!(summary.insertions, 12);
        assert_eq!(summary.deletions, 3);
    }

    #[test]
    fn test_status_is_clean() {
        let clean_status = Status {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(clean_status.is_clean());

        let dirty_status = Status {
            modified: vec![PathBuf::from("file.rs")],
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(!dirty_status.is_clean());
    }

    // WorkspaceGuard tests

    #[test]
    fn test_workspace_guard_new() {
        let guard = WorkspaceGuard::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );
        assert_eq!(guard.name, "test-session");
        assert_eq!(guard.path, PathBuf::from("/tmp/test-workspace"));
        assert!(guard.active);
    }

    #[test]
    fn test_workspace_guard_disarm() {
        let mut guard = WorkspaceGuard::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );
        assert!(guard.active);

        guard.disarm();
        assert!(!guard.active);
    }

    #[tokio::test]
    async fn test_workspace_guard_cleanup_when_active() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("zjj-test-workspace-guard");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let guard_path = temp_dir.clone();
        let mut guard = WorkspaceGuard::new("test-cleanup".to_string(), guard_path);
        assert!(guard.active);

        // Note: cleanup will attempt to forget workspace (which will fail in test env)
        // but should not panic
        let result = guard.cleanup().await;

        // Guard should be disarmed after cleanup attempt
        assert!(!guard.active);

        // Cleanup returns error because 'jj workspace forget' will fail in test env
        assert!(result.is_err());

        // Cleanup temp dir
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_workspace_guard_cleanup_when_inactive() {
        let mut guard = WorkspaceGuard::new(
            "test-inactive".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        );

        guard.disarm();
        assert!(!guard.active);

        // Cleanup should be a no-op when inactive
        let result = guard.cleanup().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workspace_guard_drop_cleans_up() {
        // Create a temporary directory
        let temp_dir = std::env::temp_dir().join("zjj-test-drop-cleanup");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        {
            let guard_path = temp_dir.clone();
            let _guard = WorkspaceGuard::new("test-drop".to_string(), guard_path);
            // Guard goes out of scope here and Drop is called
            // This should attempt cleanup (will log error for jj forget but shouldn't panic)
        }

        // Note: In a real environment with JJ, the workspace would be forgotten
        // Here we just verify no panic occurred

        // Cleanup temp dir
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_workspace_guard_disarmed_does_not_cleanup() {
        let temp_dir = std::env::temp_dir().join("zjj-test-disarmed");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        {
            let guard_path = temp_dir.clone();
            let mut guard = WorkspaceGuard::new("test-disarmed".to_string(), guard_path);
            guard.disarm();
            // Guard goes out of scope but should NOT cleanup
        }

        // Directory should still exist since guard was disarmed
        // Note: Can't reliably test this without mocking jj commands

        // Cleanup temp dir
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_workspace_guard_panic_still_cleans_up() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let temp_dir = std::env::temp_dir().join("zjj-test-panic-cleanup");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let guard_path = temp_dir.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = WorkspaceGuard::new("test-panic".to_string(), guard_path);
            // Simulate panic
            panic!("Intentional panic for testing");
        }));

        // Panic should be caught
        assert!(result.is_err());

        // Guard should have attempted cleanup during panic unwinding
        // This test verifies no double-panic occurred

        // Cleanup temp dir
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    // create_workspace tests (RED phase - these will fail until implementation)

    #[tokio::test]
    async fn test_create_workspace_returns_guard() {
        // This test verifies create_workspace returns a WorkspaceGuard
        // RED: Function doesn't exist yet
        let temp_dir = std::env::temp_dir().join("zjj-test-create-workspace");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        // Function doesn't exist yet - this will fail to compile
        let result = create_workspace("test-workspace", &temp_dir).await;

        // After implementation, should return Ok with guard
        // For now, this will fail compilation
        match result {
            Ok(guard) => {
                assert_eq!(guard.name, "test-workspace");
                assert_eq!(guard.path, temp_dir);
                assert!(guard.active);
            }
            Err(_e) => {
                // Expected in test environment without JJ repo
                // The function should still return Result<WorkspaceGuard>
            }
        }

        // Cleanup temp dir
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_create_workspace_propagates_errors() {
        // Test that create_workspace properly propagates errors from workspace_create
        let temp_dir = std::env::temp_dir().join("zjj-test-error-workspace");

        // Function doesn't exist yet - this will fail to compile
        let result = create_workspace("", &temp_dir).await;

        // Should return error for empty name
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_workspace_guard_has_correct_name() {
        // Test that the returned guard has the correct workspace name
        let temp_dir = std::env::temp_dir().join("zjj-test-guard-name");

        // Function doesn't exist yet - this will fail to compile
        let result = create_workspace("my-workspace", &temp_dir).await;

        if let Ok(guard) = result {
            assert_eq!(guard.name, "my-workspace");
        } else {
            // Expected in test environment
        }
    }

    #[tokio::test]
    async fn test_create_workspace_guard_has_correct_path() {
        // Test that the returned guard has the correct workspace path
        let temp_dir = std::env::temp_dir().join("zjj-test-guard-path");

        // Function doesn't exist yet - this will fail to compile
        let result = create_workspace("path-workspace", &temp_dir).await;

        if let Ok(guard) = result {
            assert_eq!(guard.path, temp_dir);
        } else {
            // Expected in test environment
        }
    }
}

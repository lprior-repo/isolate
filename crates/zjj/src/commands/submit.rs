//! Submit command - submit changes for review/merge
//!
//! This command prepares and submits the current workspace changes
//! for review or direct merge into the main branch.
//!
//! ## Implementation (bd-1kj, bd-3am, bd-1sh)
//!
//! 1. Detects dirty workspace state before submission
//! 2. Fails with exit code 3 if dirty and --auto-commit not set
//! 3. Commits automatically if --auto-commit is set
//! 4. Pushes bookmarks to remote
//! 5. Extracts stable commit identities (`head_sha`, `change_id`)
//! 6. Computes `logical_change_id` for deduplication
//! 7. Returns structured JSON with schema envelope (bd-3am)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use zjj_core::{config, jj, json::schemas, OutputFormat};

use crate::commands::{check_in_jj_repo, workspace_utils};

/// Submit-specific errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SubmitError {
    #[error("no bookmark found in current workspace")]
    NoBookmark,

    #[error("bookmark push failed: {0}")]
    PushFailed(String),

    #[error("remote is unreachable: {0}")]
    RemoteUnreachable(String),

    #[error("workspace has uncommitted changes")]
    DirtyWorkspace,

    #[error("failed to extract identity: {0}")]
    IdentityExtractionFailed(String),

    #[error("auto-commit failed: {0}")]
    AutoCommitFailed(String),

    #[error("failed to check workspace status: {0}")]
    StatusCheckFailed(String),
}

/// Options for the submit command
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SubmitOptions {
    /// Session name to submit (optional)
    pub name: Option<String>,
    /// Output format (JSON or human-readable)
    pub format: OutputFormat,
    /// Show what would happen without making changes
    pub dry_run: bool,
    /// Automatically commit changes if needed
    pub auto_commit: bool,
    /// Custom commit message
    pub message: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT SCHEMAS (bd-3am: zjj://submit-response/v1)
// ═══════════════════════════════════════════════════════════════════════════

/// Submit response envelope (bd-3am contract)
///
/// Always includes `schema` and `ok` fields.
/// On success: `data` contains success details.
/// On error: `error` contains error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    /// Schema URI: "<zjj://submit-response/v1>"
    pub schema: String,
    /// Success flag
    pub ok: bool,
    /// Success data (present when ok=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<SubmitSuccessData>,
    /// Error details (present when ok=false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SubmitErrorData>,
}

/// Success data for submit response (bd-3am contract)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSuccessData {
    /// Workspace name
    pub workspace: String,
    /// Bookmark name
    pub bookmark: String,
    /// JJ change ID (stable across rebases)
    pub change_id: String,
    /// Current HEAD SHA
    pub head_sha: String,
    /// Deduplication key (`workspace:change_id`)
    pub dedupe_key: String,
    /// Whether this was a dry run
    pub dry_run: bool,
}

/// Error data for submit response (bd-3am contract)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitErrorData {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

impl SubmitResponse {
    /// Create a success response
    fn success(data: SubmitSuccessData) -> Self {
        Self {
            schema: schemas::uri(schemas::SUBMIT_RESPONSE),
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema: schemas::uri(schemas::SUBMIT_RESPONSE),
            ok: false,
            data: None,
            error: Some(SubmitErrorData {
                code: code.into(),
                message: message.into(),
            }),
        }
    }

    /// Convert to JSON string
    fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("failed to serialize submit response")
    }
}

/// Extracted identity from the workspace
#[derive(Debug, Clone)]
struct WorkspaceIdentity {
    change_id: String,
    head_sha: String,
    bookmark_name: String,
    workspace_name: String,
}

/// Run the submit command with the given options
///
/// # Errors
///
/// Returns an error if:
/// - The workspace is in an invalid state
/// - Preconditions for submit are not met
/// - The underlying JJ operations fail
///
/// # Exit Codes
///
/// - 0: Success
/// - 1: General error
/// - 2: Invalid arguments
/// - 3: Precondition failed (no bookmark, dirty state)
/// - 5: Remote/network errors
pub async fn run_with_options(options: &SubmitOptions) -> Result<i32> {
    // Check prerequisites
    let root = check_in_jj_repo().await?;

    // Determine workspace context
    let workspace_info = if let Some(ref name) = options.name {
        let db = crate::commands::get_session_db().await?;
        let session = db.get(name).await?.ok_or_else(|| {
            anyhow::anyhow!("Session '{name}' not found")
        })?;
        WorkspaceInfo {
            path: PathBuf::from(session.workspace_path),
            name: session.name,
        }
    } else {
        resolve_workspace_context(&root).await?
    };

    // Extract initial identity information
    let identity = match extract_workspace_identity(&workspace_info.path).await {
        Ok(id) => id,
        Err(e) => {
            return output_error(
                options.format.is_json(),
                "PRECONDITION_FAILED",
                e.to_string(),
                3,
            );
        }
    };

    // For dry run, output what would happen and exit
    if options.dry_run {
        let is_dirty = is_workspace_dirty(&workspace_info.path)
            .await
            .unwrap_or(false);
        let dedupe_key = compute_dedupe_key(&identity.change_id, &identity.workspace_name);

        // If it's dirty and we won't auto-commit, the real command would fail.
        // Dry-run should reflect this precondition failure (bd-34k fix).
        if is_dirty && !options.auto_commit {
            return output_error(
                options.format.is_json(),
                "DIRTY_WORKSPACE",
                "Working copy has uncommitted changes.\nUse --auto-commit to commit automatically, or run 'jj commit' first. (Dry run validation failure)".to_string(),
                3,
            );
        }

        if !options.format.is_json() && is_dirty {
            println!("Note: Workspace has uncommitted changes.");
            println!("      These changes WOULD be committed automatically.");
            println!();
        }

        return output_dry_run(options.format.is_json(), &identity, &dedupe_key);
    }

    // Check dirty workspace state (bd-1sh) - only for real submission
    match check_and_handle_dirty_state(&workspace_info.path, options).await {
        Ok(()) => {}
        Err(SubmitError::DirtyWorkspace) => {
            return output_error(
                options.format.is_json(),
                "DIRTY_WORKSPACE",
                "Working copy has uncommitted changes.\nUse --auto-commit to commit automatically, or run 'jj commit' first.".to_string(),
                3,
            );
        }
        Err(e) => {
            return output_error(
                options.format.is_json(),
                "PRECONDITION_FAILED",
                e.to_string(),
                3,
            );
        }
    }

    // Re-extract identity after potential auto-commit to get the new HEAD SHA
    let identity = match extract_workspace_identity(&workspace_info.path).await {
        Ok(id) => id,
        Err(e) => {
            return output_error(
                options.format.is_json(),
                "PRECONDITION_FAILED",
                e.to_string(),
                3,
            );
        }
    };

    // Compute final dedupe_key
    let dedupe_key = compute_dedupe_key(&identity.change_id, &identity.workspace_name);

    // Push bookmark to remote
    if let Err(e) = push_bookmark(&identity.bookmark_name, &workspace_info.path).await {
        let error_msg = e.to_string();
        // Check if it's a remote/network error
        let is_remote_error = error_msg.contains("remote")
            || error_msg.contains("network")
            || error_msg.contains("connection")
            || error_msg.contains("unreachable")
            || error_msg.contains("timeout");

        let (code, exit_code) = if is_remote_error {
            ("REMOTE_ERROR", 5)
        } else {
            ("PRECONDITION_FAILED", 3)
        };

        return output_error(options.format.is_json(), code, error_msg, exit_code);
    }

    output_success(options.format.is_json(), &identity, &dedupe_key)
}

/// Workspace context information
#[allow(dead_code)]
struct WorkspaceInfo {
    path: PathBuf,
    name: String,
}

/// Resolve the current workspace context
async fn resolve_workspace_context(root: &Path) -> Result<WorkspaceInfo> {
    let current_dir = std::env::current_dir().context("failed to get current directory")?;

    if current_dir.join(".jj/repo").is_file() {
        let name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Workspace directory name is not valid UTF-8"))?
            .to_string();
        return Ok(WorkspaceInfo {
            path: current_dir,
            name,
        });
    }

    // Check if we're in a non-default workspace
    // In JJ, non-default workspaces have `.jj/repo` as a FILE (pointer to main repo)
    if root.join(".jj/repo").is_file() {
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Workspace directory name is not valid UTF-8"))?
            .to_string();
        return Ok(WorkspaceInfo {
            path: root.to_path_buf(),
            name,
        });
    }

    let workspace_dir = config::load_config()
        .await
        .map_or_else(|_| ".zjj/workspaces".to_string(), |cfg| cfg.workspace_dir);
    let workspace_roots = workspace_utils::candidate_workspace_roots(root, &workspace_dir);

    for workspace_root in workspace_roots {
        if current_dir.starts_with(&workspace_root) {
            let workspace_name = current_dir
                .strip_prefix(&workspace_root)
                .context("failed to strip workspace prefix")?
                .components()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Unable to determine workspace name"))?
                .as_os_str()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Workspace name contains invalid UTF-8"))?
                .to_string();

            let workspace_path = workspace_root.join(&workspace_name);
            return Ok(WorkspaceInfo {
                path: workspace_path,
                name: workspace_name,
            });
        }
    }

    // We're in the main workspace
    Ok(WorkspaceInfo {
        path: root.to_path_buf(),
        name: "main".to_string(),
    })
}

/// Extract identity information from the workspace
///
/// # Returns
///
/// Returns the workspace identity containing:
/// - `change_id`: The jj change ID (stable across rebases)
/// - `head_sha`: The current commit hash
/// - `bookmark_name`: The current bookmark name
/// - `workspace_name`: The workspace name
async fn extract_workspace_identity(
    workspace_path: &PathBuf,
) -> Result<WorkspaceIdentity, SubmitError> {
    // Get change_id (stable across rebases)
    let change_id = get_change_id(workspace_path).await?;

    // Get head_sha (current commit hash)
    let head_sha = get_head_sha(workspace_path).await?;

    // Get bookmark name
    let bookmark_name = get_current_bookmark(workspace_path).await?;

    // Get workspace name from path
    let workspace_name = workspace_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SubmitError::IdentityExtractionFailed("invalid workspace path".to_string()))?
        .to_string();

    Ok(WorkspaceIdentity {
        change_id,
        head_sha,
        bookmark_name,
        workspace_name,
    })
}

/// Get the `change_id` from jj log (stable across rebases)
async fn get_change_id(workspace_path: &PathBuf) -> Result<String, SubmitError> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "change_id"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| SubmitError::IdentityExtractionFailed(format!("failed to run jj log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SubmitError::IdentityExtractionFailed(format!(
            "jj log failed: {stderr}"
        )));
    }

    let change_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if change_id.is_empty() {
        return Err(SubmitError::IdentityExtractionFailed(
            "empty change_id returned".to_string(),
        ));
    }

    Ok(change_id)
}

/// Get the current commit hash (`head_sha`)
async fn get_head_sha(workspace_path: &PathBuf) -> Result<String, SubmitError> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| SubmitError::IdentityExtractionFailed(format!("failed to run jj log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SubmitError::IdentityExtractionFailed(format!(
            "jj log failed: {stderr}"
        )));
    }

    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if head_sha.is_empty() {
        return Err(SubmitError::IdentityExtractionFailed(
            "empty commit_id returned".to_string(),
        ));
    }

    Ok(head_sha)
}

/// Get the current bookmark name
async fn get_current_bookmark(workspace_path: &PathBuf) -> Result<String, SubmitError> {
    // Use jj log to get bookmarks pointing to the current revision
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "bookmarks"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| {
            SubmitError::IdentityExtractionFailed(format!("failed to get current bookmarks: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SubmitError::IdentityExtractionFailed(format!(
            "jj log failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bookmarks: Vec<&str> = stdout.split_whitespace().collect();

    // Prefer a bookmark that matches the workspace name if possible,
    // otherwise just take the first one.
    let workspace_name = workspace_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let bookmark_name = bookmarks
        .iter()
        .find(|&&b| b == workspace_name)
        .or_else(|| bookmarks.first())
        .ok_or(SubmitError::NoBookmark)?;

    Ok((*bookmark_name).to_string())
}

/// Compute `dedupe_key` for deduplication
///
/// The `dedupe_key` is derived from the jj `change_id` (which is stable across rebases)
/// combined with the workspace name to ensure uniqueness per workspace.
fn compute_dedupe_key(change_id: &str, workspace_name: &str) -> String {
    // Use change_id as the primary identifier since it's stable across rebases
    // Include workspace name to ensure uniqueness when same change is in multiple workspaces
    format!("{workspace_name}:{change_id}")
}

/// Push bookmark to remote
///
/// # Errors
///
/// Returns an error if:
/// - The jj git push command fails
/// - Remote is unreachable
async fn push_bookmark(bookmark_name: &str, workspace_path: &PathBuf) -> Result<(), SubmitError> {
    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark_name])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| SubmitError::PushFailed(format!("failed to execute jj git push: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = stderr.to_string();

        // Check for remote/network errors
        if error_msg.contains("could not resolve host")
            || error_msg.contains("network")
            || error_msg.contains("connection refused")
            || error_msg.contains("timed out")
            || error_msg.contains("unreachable")
        {
            return Err(SubmitError::RemoteUnreachable(error_msg));
        }

        return Err(SubmitError::PushFailed(error_msg));
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// DIRTY WORKSPACE DETECTION AND AUTO-COMMIT (bd-1sh)
// ═══════════════════════════════════════════════════════════════════════════

/// Check if workspace has uncommitted changes
///
/// # Errors
///
/// Returns an error if:
/// - JJ status command fails
/// - Unable to parse status output
async fn is_workspace_dirty(workspace_path: &Path) -> Result<bool, SubmitError> {
    let status = jj::workspace_status(workspace_path)
        .await
        .map_err(|e| SubmitError::StatusCheckFailed(e.to_string()))?;

    Ok(!status.is_clean())
}

/// Check dirty state and handle based on --auto-commit flag (bd-1sh)
///
/// # Behavior
///
/// - Clean workspace: Returns Ok(())
/// - Dirty without --auto-commit: Returns `Err(SubmitError::DirtyWorkspace)`
/// - Dirty with --auto-commit: Commits changes and returns Ok(())
///
/// # Errors
///
/// Returns an error if:
/// - Status check fails
/// - Auto-commit is enabled but commit fails
async fn check_and_handle_dirty_state(
    workspace_path: &PathBuf,
    options: &SubmitOptions,
) -> Result<(), SubmitError> {
    let is_dirty = is_workspace_dirty(workspace_path).await?;

    if !is_dirty {
        // Clean workspace - proceed with submission
        return Ok(());
    }

    // Workspace is dirty
    if options.auto_commit {
        // Auto-commit enabled - commit changes and proceed
        auto_commit_changes(workspace_path, options.message.as_deref()).await
    } else {
        // No auto-commit - fail with explicit error
        Err(SubmitError::DirtyWorkspace)
    }
}

/// Commit changes automatically (bd-1sh)
///
/// Uses provided message or generates a default message.
///
/// # Errors
///
/// Returns an error if:
/// - JJ commit command fails
async fn auto_commit_changes(
    workspace_path: &PathBuf,
    message: Option<&str>,
) -> Result<(), SubmitError> {
    let commit_message = message.map_or_else(
        || "wip: auto-commit before submit".to_string(),
        std::string::ToString::to_string,
    );

    let output = Command::new("jj")
        .args(["commit", "-m", &commit_message])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| SubmitError::AutoCommitFailed(format!("failed to execute jj commit: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SubmitError::AutoCommitFailed(format!(
            "jj commit failed: {stderr}"
        )));
    }

    Ok(())
}

/// Output for dry run (bd-3am contract)
fn output_dry_run(is_json: bool, identity: &WorkspaceIdentity, dedupe_key: &str) -> Result<i32> {
    if is_json {
        let data = SubmitSuccessData {
            workspace: identity.workspace_name.clone(),
            bookmark: identity.bookmark_name.clone(),
            change_id: identity.change_id.clone(),
            head_sha: identity.head_sha.clone(),
            dedupe_key: dedupe_key.to_string(),
            dry_run: true,
        };

        let response = SubmitResponse::success(data);
        println!("{}", response.to_json()?);
    } else {
        println!("Would submit changes (dry run)");
        println!();
        println!("Identity:");
        println!("  Workspace: {}", identity.workspace_name);
        println!("  Bookmark: {}", identity.bookmark_name);
        println!("  Change ID: {}", identity.change_id);
        println!("  HEAD SHA: {}", identity.head_sha);
        println!("  Dedupe Key: {dedupe_key}");
        println!();
        println!("Actions that would be performed:");
        println!("  1. Push bookmark '{}' to remote", identity.bookmark_name);
    }

    Ok(0)
}

/// Output for successful submission (bd-3am contract)
fn output_success(is_json: bool, identity: &WorkspaceIdentity, dedupe_key: &str) -> Result<i32> {
    if is_json {
        let data = SubmitSuccessData {
            workspace: identity.workspace_name.clone(),
            bookmark: identity.bookmark_name.clone(),
            change_id: identity.change_id.clone(),
            head_sha: identity.head_sha.clone(),
            dedupe_key: dedupe_key.to_string(),
            dry_run: false,
        };

        let response = SubmitResponse::success(data);
        println!("{}", response.to_json()?);
    } else {
        println!("Submitted successfully!");
        println!();
        println!("Identity:");
        println!("  Workspace: {}", identity.workspace_name);
        println!("  Bookmark: {}", identity.bookmark_name);
        println!("  Change ID: {}", identity.change_id);
        println!("  HEAD SHA: {}", identity.head_sha);
        println!("  Dedupe Key: {dedupe_key}");
    }

    Ok(0)
}

/// Output for error cases (bd-3am contract)
fn output_error(is_json: bool, code: &str, message: String, exit_code: i32) -> Result<i32> {
    if is_json {
        let response = SubmitResponse::error(code, message);
        println!("{}", response.to_json()?);
    } else {
        eprintln!("Error: {message}");
    }

    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_options_defaults() {
        let options = SubmitOptions {
            format: OutputFormat::Json,
            dry_run: false,
            auto_commit: false,
            message: None,
        };

        assert!(options.format.is_json());
        assert!(!options.dry_run);
        assert!(!options.auto_commit);
        assert!(options.message.is_none());
    }

    #[test]
    fn test_compute_dedupe_key() {
        // Test that dedupe_key is stable and combines workspace with change_id
        let change_id = "abc123xyz";
        let workspace = "feature-branch";

        let dedupe_key = compute_dedupe_key(change_id, workspace);

        // Should be workspace:change_id format
        assert_eq!(dedupe_key, "feature-branch:abc123xyz");

        // Same inputs should produce same output
        let dedupe_key2 = compute_dedupe_key(change_id, workspace);
        assert_eq!(dedupe_key, dedupe_key2);

        // Different workspace should produce different dedupe_key
        let dedupe_key3 = compute_dedupe_key(change_id, "other-workspace");
        assert_ne!(dedupe_key, dedupe_key3);
        assert_eq!(dedupe_key3, "other-workspace:abc123xyz");

        // Different change_id should produce different dedupe_key
        let dedupe_key4 = compute_dedupe_key("different", workspace);
        assert_ne!(dedupe_key, dedupe_key4);
        assert_eq!(dedupe_key4, "feature-branch:different");
    }

    #[test]
    fn test_submit_error_messages() {
        let err = SubmitError::NoBookmark;
        assert_eq!(err.to_string(), "no bookmark found in current workspace");

        let err = SubmitError::PushFailed("remote rejected".to_string());
        assert_eq!(err.to_string(), "bookmark push failed: remote rejected");

        let err = SubmitError::RemoteUnreachable("connection timeout".to_string());
        assert_eq!(err.to_string(), "remote is unreachable: connection timeout");

        let err = SubmitError::DirtyWorkspace;
        assert_eq!(err.to_string(), "workspace has uncommitted changes");

        let err = SubmitError::IdentityExtractionFailed("no change_id".to_string());
        assert_eq!(err.to_string(), "failed to extract identity: no change_id");
    }

    #[test]
    fn test_submit_response_success_schema() {
        let data = SubmitSuccessData {
            workspace: "feature-xyz".to_string(),
            bookmark: "my-feature".to_string(),
            change_id: "kxyz123".to_string(),
            head_sha: "abc123def456".to_string(),
            dedupe_key: "feature-xyz:kxyz123".to_string(),
            dry_run: false,
        };

        let response = SubmitResponse::success(data);

        // Verify schema and ok fields
        assert_eq!(response.schema, "zjj://submit-response/v1");
        assert!(response.ok);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_submit_response_error_schema() {
        let response = SubmitResponse::error("PUSH_FAILED", "Failed to push to remote");

        // Verify schema and ok fields
        assert_eq!(response.schema, "zjj://submit-response/v1");
        assert!(!response.ok);
        assert!(response.data.is_none());
        assert!(response.error.is_some());

        let error = response.error;
        assert!(error.is_some());
        let error = error;
        if let Some(err) = error {
            assert_eq!(err.code, "PUSH_FAILED");
            assert_eq!(err.message, "Failed to push to remote");
        }
    }

    #[test]
    fn test_submit_response_dry_run_schema() {
        let data = SubmitSuccessData {
            workspace: "feature-xyz".to_string(),
            bookmark: "my-feature".to_string(),
            change_id: "kxyz123".to_string(),
            head_sha: "abc123def456".to_string(),
            dedupe_key: "feature-xyz:kxyz123".to_string(),
            dry_run: true,
        };

        let response = SubmitResponse::success(data);

        assert_eq!(response.schema, "zjj://submit-response/v1");
        assert!(response.ok);

        if let Some(d) = response.data.as_ref() {
            assert!(d.dry_run);
        }
    }

    #[test]
    fn test_submit_response_json_serialization() {
        let data = SubmitSuccessData {
            workspace: "feature-xyz".to_string(),
            bookmark: "my-feature".to_string(),
            change_id: "kxyz123".to_string(),
            head_sha: "abc123".to_string(),
            dedupe_key: "feature-xyz:kxyz123".to_string(),
            dry_run: false,
        };

        let response = SubmitResponse::success(data);
        let json = response.to_json();

        assert!(json.is_ok());
        let json_str = json.ok();
        assert!(json_str.is_some());
        let json_str = json_str.unwrap_or_default();

        // Verify required fields are present
        assert!(json_str.contains("\"schema\":\"zjj://submit-response/v1\""));
        assert!(json_str.contains("\"ok\":true"));
        assert!(json_str.contains("\"workspace\":\"feature-xyz\""));
        assert!(json_str.contains("\"bookmark\":\"my-feature\""));
        assert!(json_str.contains("\"change_id\":\"kxyz123\""));
        assert!(json_str.contains("\"head_sha\":\"abc123\""));
        assert!(json_str.contains("\"dedupe_key\":\"feature-xyz:kxyz123\""));
        assert!(json_str.contains("\"dry_run\":false"));
    }

    #[test]
    fn test_submit_response_error_json_serialization() {
        let response = SubmitResponse::error("PRECONDITION_FAILED", "No bookmark found");
        let json = response.to_json();

        assert!(json.is_ok());
        let json_str = json.ok();
        assert!(json_str.is_some());
        let json_str = json_str.unwrap_or_default();

        // Verify required fields are present
        assert!(json_str.contains("\"schema\":\"zjj://submit-response/v1\""));
        assert!(json_str.contains("\"ok\":false"));
        assert!(json_str.contains("\"code\":\"PRECONDITION_FAILED\""));
        assert!(json_str.contains("\"message\":\"No bookmark found\""));

        // data should be absent (null serialized as null)
        assert!(json_str.contains("\"data\":null") || !json_str.contains("\"data\""));
    }

    #[test]
    fn test_output_dry_run_human_readable() {
        let identity = WorkspaceIdentity {
            change_id: "xyz789".to_string(),
            head_sha: "abc123".to_string(),
            bookmark_name: "feature".to_string(),
            workspace_name: "ws1".to_string(),
        };

        let result = output_dry_run(false, &identity, "ws1:xyz789");
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(0));
    }

    #[test]
    fn test_output_error_human_readable() {
        let result = output_error(
            false,
            "PRECONDITION_FAILED",
            "No bookmark found".to_string(),
            3,
        );
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(3));
    }

    #[test]
    fn test_output_error_json() {
        let result = output_error(true, "REMOTE_ERROR", "Connection timeout".to_string(), 5);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(5));
    }

    #[test]
    fn test_success_data_skip_serialization() {
        // Test that optional fields are skipped when None
        let data = SubmitSuccessData {
            workspace: "ws".to_string(),
            bookmark: "b".to_string(),
            change_id: "c".to_string(),
            head_sha: "h".to_string(),
            dedupe_key: "ws:c".to_string(),
            dry_run: true,
        };

        let response = SubmitResponse::success(data);
        let json = response.to_json();
        assert!(json.is_ok());

        let json_str = json.ok();
        assert!(json_str.is_some());
        let json_str = json_str.unwrap_or_default();

        // These fields should be present with values
        assert!(json_str.contains("\"dry_run\":true"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DIRTY WORKSPACE DETECTION TESTS (bd-1sh)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_submit_error_auto_commit_failed() {
        let err = SubmitError::AutoCommitFailed("commit failed".to_string());
        assert_eq!(err.to_string(), "auto-commit failed: commit failed");
    }

    #[test]
    fn test_submit_error_status_check_failed() {
        let err = SubmitError::StatusCheckFailed("status error".to_string());
        assert_eq!(
            err.to_string(),
            "failed to check workspace status: status error"
        );
    }

    #[test]
    fn test_dirty_workspace_error_response() {
        let response = SubmitResponse::error(
            "DIRTY_WORKSPACE",
            "Working copy has uncommitted changes.\nUse --auto-commit to commit automatically, or run 'jj commit' first.",
        );

        assert_eq!(response.schema, "zjj://submit-response/v1");
        assert!(!response.ok);
        assert!(response.data.is_none());
        assert!(response.error.is_some());

        if let Some(err) = response.error {
            assert_eq!(err.code, "DIRTY_WORKSPACE");
            assert!(err.message.contains("--auto-commit"));
            assert!(err.message.contains("jj commit"));
        }
    }

    #[test]
    fn test_submit_options_auto_commit() {
        let options_with_commit = SubmitOptions {
            format: OutputFormat::Json,
            dry_run: false,
            auto_commit: true,
            message: Some("custom message".to_string()),
        };

        assert!(options_with_commit.auto_commit);
        assert_eq!(
            options_with_commit.message,
            Some("custom message".to_string())
        );

        let options_without_commit = SubmitOptions {
            format: OutputFormat::Json,
            dry_run: false,
            auto_commit: false,
            message: None,
        };

        assert!(!options_without_commit.auto_commit);
        assert!(options_without_commit.message.is_none());
    }

    #[test]
    fn test_output_error_dirty_workspace() {
        let result = output_error(
            false,
            "DIRTY_WORKSPACE",
            "Working copy has uncommitted changes.\nUse --auto-commit to commit automatically, or run 'jj commit' first.".to_string(),
            3,
        );
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(3));
    }

    #[test]
    fn test_output_error_dirty_workspace_json() {
        let result = output_error(
            true,
            "DIRTY_WORKSPACE",
            "Working copy has uncommitted changes.\nUse --auto-commit to commit automatically, or run 'jj commit' first.".to_string(),
            3,
        );
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(3));
    }
}

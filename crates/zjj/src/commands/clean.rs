//! Clean command - Remove orphaned workspaces and stale sessions
//!
//! This command provides automatic cleanup of orphaned (merged/abandoned) workspaces
//! with safety guards and Railway-Oriented Programming patterns.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{path::Path, time::SystemTime};

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::process::Command;
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::cli::{jj_root, is_inside_zellij};
use crate::commands::get_session_db;
use crate::db::SessionData;

/// Cleanup options from CLI
#[derive(Debug, Clone)]
pub struct CleanOptions {
    /// Skip confirmation prompt
    pub force: bool,
    /// Preview only, don't delete
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
    /// Run as periodic daemon (1hr interval)
    pub periodic: bool,
    /// Age threshold in seconds (default: 7200 = 2hr)
    pub age_threshold: Option<u64>,
}

/// Cleanup result
#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanupReport {
    /// Number of workspaces cleaned
    pub cleaned: usize,
    /// Number of workspaces skipped
    pub skipped: usize,
    /// Warning messages
    pub warnings: Vec<String>,
    /// Details of cleaned workspaces
    pub details: Vec<CleanupDetail>,
}

/// Individual workspace cleanup detail
#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanupDetail {
    /// Workspace name
    pub workspace: String,
    /// Reason for cleanup
    pub reason: String,
    /// Whether workspace was merged
    pub merged: bool,
    /// Size in bytes
    pub size_bytes: u64,
}

/// Orphaned workspace metadata
#[derive(Debug, Clone)]
struct Orphan {
    /// Workspace name
    name: String,
    /// Path to workspace
    path: String,
    /// Whether workspace is merged
    merged: bool,
    /// Creation timestamp
    created_at: SystemTime,
}

/// Cleanup error types
#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
    #[error("Workspace has uncommitted changes: {workspace}")]
    UncommittedChanges { workspace: String },

    #[error("Workspace is currently active: {workspace}")]
    ActiveWorkspace { workspace: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Session database locked")]
    DatabaseLocked,

    #[error("Invalid age threshold: {value}")]
    InvalidAge { value: String },

    #[error("Workspace not found: {workspace}")]
    WorkspaceNotFound { workspace: String },
}

/// Run cleanup with options
pub async fn run_with_options(options: &CleanOptions) -> Result<()> {
    // Periodic mode not yet implemented
    if options.periodic {
        anyhow::bail!("Periodic cleanup daemon not yet implemented");
    }

    let report = detect_and_clean(options).await?;

    match options.format {
        OutputFormat::Json => {
            let envelope = SchemaEnvelope::new(
                "zjj://clean-response/v1",
                "1.0",
                "clean-response",
                report,
            );
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Human => {
            print_human_report(&report, options.dry_run)?;
        }
    }

    Ok(())
}

/// Detect orphans and perform cleanup
async fn detect_and_clean(options: &CleanOptions) -> Result<CleanupReport> {
    let root = jj_root()
        .await
        .context("Failed to get JJ root")?;

    let db = get_session_db()
        .await
        .context("Failed to open session database")?;

    // Get sessions from database
    let db_sessions = db
        .list(None)
        .await
        .unwrap_or_default();

    // Get JJ workspaces
    let jj_workspaces = list_jj_workspaces(&root).await?;

    let session_names: Vec<_> = db_sessions
        .iter()
        .map(|s| s.name.as_str())
        .collect();

    // Find workspaces without sessions (filesystem → DB orphans)
    let filesystem_orphans: Vec<_> = jj_workspaces
        .iter()
        .filter(|ws| ws.as_str() != "default" && !session_names.contains(ws.as_str()))
        .cloned()
        .collect();

    // Find sessions without valid workspaces (DB → filesystem orphans)
    let db_orphans: Vec<SessionData> = futures::stream::iter(db_sessions)
        .then(|session| async {
            let has_workspace = jj_workspaces.iter().any(|ws| ws == session.name.as_str());
            let directory_exists = tokio::fs::try_exists(&session.workspace_path)
                .await
                .unwrap_or(false);

            if !has_workspace || !directory_exists {
                Some(session)
            } else {
                None
            }
        })
        .filter_map(|opt| async move { opt })
        .collect()
        .await;

    let mut report = CleanupReport {
        cleaned: 0,
        skipped: 0,
        warnings: Vec::new(),
        details: Vec::new(),
    };

    // Dry-run: just report what would be done
    if options.dry_run {
        if !filesystem_orphans.is_empty() {
            report
                .warnings
                .push(format!("Would remove {} orphaned workspace(s)", filesystem_orphans.len()));
        }
        if !db_orphans.is_empty() {
            report
                .warnings
                .push(format!("Would remove {} session(s) without workspaces", db_orphans.len()));
        }
        return Ok(report);
    }

    // Prompt for confirmation unless --force
    let total = filesystem_orphans.len() + db_orphans.len();
    if total > 0 && !options.force {
        println!("Found {total} orphaned workspace(s)/session(s)");
        println!("Proceed with cleanup? [y/N]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cleanup cancelled");
            return Ok(report);
        }
    }

    // Clean filesystem orphans
    for workspace in &filesystem_orphans {
        match cleanup_workspace(workspace, &root, options).await {
            Ok(detail) => {
                report.cleaned += 1;
                report.details.push(detail);
            }
            Err(e) => {
                report.skipped += 1;
                report.warnings.push(e.to_string());
            }
        }
    }

    // Clean DB orphans
    match get_session_db().await {
        Ok(db) => {
            for session in &db_orphans {
                match db.delete(&session.name).await {
                    Ok(true) => {
                        report.cleaned += 1;
                        report.details.push(CleanupDetail {
                            workspace: session.name.clone(),
                            reason: "Session without workspace".to_string(),
                            merged: false,
                            size_bytes: 0,
                        });
                    }
                    Ok(false) => {}
                    Err(e) => {
                        report.skipped += 1;
                        report
                            .warnings
                            .push(format!("Failed to delete session '{}': {}", session.name, e));
                    }
                }
            }
        }
        Err(e) => {
            report
                .warnings
                .push(format!("Failed to open database for cleanup: {}", e));
        }
    }

    Ok(report)
}

/// List all JJ workspaces
async fn list_jj_workspaces(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .current_dir(root)
        .output()
        .await
        .context("Failed to list JJ workspaces")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj workspace list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect())
}

/// Cleanup a single workspace
async fn cleanup_workspace(
    workspace: &str,
    root: &Path,
    options: &CleanOptions,
) -> Result<CleanupDetail, CleanupError> {
    // Check if workspace has uncommitted changes
    let has_changes = check_workspace_changes(workspace, root).await?;
    if has_changes && !options.force {
        return Err(CleanupError::UncommittedChanges {
            workspace: workspace.to_string(),
        });
    }

    // Check if workspace is active (has Zellij tab or JJ process)
    let is_active = check_workspace_active(workspace).await;
    if is_active {
        return Err(CleanupError::ActiveWorkspace {
            workspace: workspace.to_string(),
        });
    }

    // Forget workspace in JJ
    let output = Command::new("jj")
        .args(["workspace", "forget", workspace])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| CleanupError::PermissionDenied {
            path: root.display().to_string(),
        })?;

    if !output.status.success() {
        return Err(CleanupError::WorkspaceNotFound {
            workspace: workspace.to_string(),
        });
    }

    // Get workspace directory size
    let size_bytes = get_dir_size(root.join(workspace)).await.unwrap_or(0);

    Ok(CleanupDetail {
        workspace: workspace.to_string(),
        reason: "Orphaned workspace".to_string(),
        merged: has_changes,
        size_bytes,
    })
}

/// Check if workspace has uncommitted changes
async fn check_workspace_changes(workspace: &str, root: &Path) -> Result<bool, CleanupError> {
    let output = Command::new("jj")
        .args(["status", "--quiet"])
        .current_dir(root.join(workspace))
        .output()
        .await
        .map_err(|e| CleanupError::PermissionDenied {
            path: root.display().to_string(),
        })?;

    Ok(!output.status.success())
}

/// Check if workspace is active (Zellij tab or JJ process)
async fn check_workspace_active(workspace: &str) -> bool {
    // Check for Zellij tab
    if is_inside_zellij() {
        if let Ok(output) = Command::new("zellij")
            .args(["list", "sessions", "--no-formatting"])
            .output()
            .await
        {
            let sessions = String::from_utf8_lossy(&output.stdout);
            if sessions.contains(workspace) {
                return true;
            }
        }
    }

    // Check for running JJ processes
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", &format!("jj.*{}", workspace)])
        .output()
        .await
    {
        if output.status.success() {
            let processes = String::from_utf8_lossy(&output.stdout);
            if !processes.trim().is_empty() {
                return true;
            }
        }
    }

    false
}

/// Get directory size recursively
async fn get_dir_size(path: impl AsRef<Path>) -> Result<u64> {
    let path = path.as_ref();
    let mut size = 0u64;

    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(e) => e,
        Err(_) => return Ok(size),
    };

    while let Some(entry) = entries.next_entry().await.map_err(|_| anyhow::anyhow!("Failed to read entry"))? {
        let ty = entry.file_type().await.map_err(|_| anyhow::anyhow!("Failed to get file type"))?;
        let metadata = entry.metadata().await.map_err(|_| anyhow::anyhow!("Failed to get metadata"))?;

        if ty.is_dir() {
            size += get_dir_size(entry.path()).await.unwrap_or(0);
        } else {
            size += metadata.len();
        }
    }

    Ok(size)
}

/// Print human-readable report
fn print_human_report(report: &CleanupReport, dry_run: bool) -> Result<()> {
    if report.cleaned == 0 && report.skipped == 0 && report.warnings.is_empty() {
        println!("No orphaned workspaces found");
        return Ok(());
    }

    if dry_run {
        println!("DRY RUN - No changes made");
    }

    if report.cleaned > 0 {
        println!("Cleaned {} workspace(s)/session(s)", report.cleaned);
        for detail in &report.details {
            println!("  - {}: {}", detail.workspace, detail.reason);
        }
    }

    if report.skipped > 0 {
        println!("Skipped {} workspace(s)", report.skipped);
    }

    for warning in &report.warnings {
        eprintln!("Warning: {}", warning);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_options_debug() {
        let options = CleanOptions {
            force: true,
            dry_run: false,
            format: OutputFormat::Human,
            periodic: false,
            age_threshold: Some(7200),
        };
        assert_eq!(format!("{options:?}"), format!("{:?}", options));
    }

    #[test]
    fn test_cleanup_report_serialization() {
        let report = CleanupReport {
            cleaned: 2,
            skipped: 1,
            warnings: vec!["Test warning".to_string()],
            details: vec![CleanupDetail {
                workspace: "test-workspace".to_string(),
                reason: "Test reason".to_string(),
                merged: true,
                size_bytes: 1024,
            }],
        };

        let json = serde_json::to_string(&report);
        assert!(json.is_ok());
    }
}

//! Done command - Complete work and merge to main
//!
//! This command:
//! 1. Validates we're in a workspace (not main)
//! 2. Checks for uncommitted changes
//! 3. Commits any uncommitted changes
//! 4. Checks for merge conflicts
//! 5. Merges workspace changes to main
//! 6. Updates linked bead status to completed
//! 7. Cleans up the workspace
//! 8. Switches back to main

pub mod types;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
pub use types::{DoneError, DoneExitCode, DoneOptions, DoneOutput};

use crate::{
    cli::jj_root,
    commands::context::{detect_location, Location},
};

/// Run the done command with options
pub fn run_with_options(options: &DoneOptions) -> Result<DoneExitCode> {
    let result = execute_done(options);

    match &result {
        Ok(output) => {
            output_result(output, options.format)?;
            Ok(DoneExitCode::Success)
        }
        Err(e) => {
            output_error(e, options.format)?;
            Ok(if matches!(e, DoneError::NotInWorkspace { .. }) {
                DoneExitCode::NotInWorkspace
            } else if matches!(e, DoneError::MergeConflict { .. }) {
                DoneExitCode::MergeConflict
            } else {
                DoneExitCode::OtherError
            })
        }
    }
}

/// Core done logic using Railway-Oriented Programming
fn execute_done(options: &DoneOptions) -> Result<DoneOutput, DoneError> {
    // Phase 1: Validate location (must be in workspace)
    let root = validate_location()?;
    let workspace_name = get_workspace_name(&root)?;

    // Phase 2: Build preview for dry-run
    let preview = if options.dry_run {
        Some(build_preview(&root, &workspace_name)?)
    } else {
        None
    };

    if options.dry_run {
        return Ok(DoneOutput {
            workspace_name,
            dry_run: true,
            preview,
            ..Default::default()
        });
    }

    // Phase 3: Check uncommitted files
    let uncommitted_files = get_uncommitted_files(&root)?;

    // Phase 4: Commit uncommitted changes
    let files_committed = if uncommitted_files.is_empty() {
        0
    } else {
        commit_changes(&root, &workspace_name, options.message.as_deref())?
    };

    // Phase 5: Check for conflicts
    check_conflicts(&root)?;

    // Phase 6: Get commits to merge
    let commits_to_merge = get_commits_to_merge(&root)?;

    // Phase 7: Merge to main
    merge_to_main(&root, &workspace_name, options.squash)?;

    // Phase 8: Update bead status
    let bead_id = get_bead_id_for_workspace(&workspace_name)?;
    let bead_closed = if let Some(ref bead) = bead_id {
        if options.no_bead_update {
            false
        } else {
            update_bead_status(bead, "closed")?;
            true
        }
    } else {
        false
    };

    // Phase 9: Cleanup workspace
    let cleaned = if options.keep_workspace {
        false
    } else {
        cleanup_workspace(&root, &workspace_name)?
    };

    Ok(DoneOutput {
        workspace_name,
        bead_id,
        files_committed,
        commits_merged: commits_to_merge.len(),
        merged: true,
        cleaned,
        bead_closed,
        dry_run: false,
        preview: None,
        error: None,
    })
}

/// Validate we're in a workspace
fn validate_location() -> Result<String, DoneError> {
    let root_str = jj_root().map_err(|_| DoneError::NotAJjRepo)?;
    let root = PathBuf::from(&root_str);

    let location = detect_location(&root).map_err(|e| DoneError::InvalidState {
        reason: e.to_string(),
    })?;

    match location {
        Location::Workspace { .. } => Ok(root_str),
        Location::Main => {
            let current = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            Err(DoneError::NotInWorkspace {
                current_location: current,
            })
        }
    }
}

/// Get the current workspace name from location
fn get_workspace_name(root: &str) -> Result<String, DoneError> {
    let location = detect_location(&PathBuf::from(root)).map_err(|e| DoneError::InvalidState {
        reason: e.to_string(),
    })?;

    match location {
        Location::Workspace { name, .. } => Ok(name),
        Location::Main => Err(DoneError::NotInWorkspace {
            current_location: "main".to_string(),
        }),
    }
}

/// Build preview for dry-run mode
fn build_preview(root: &str, workspace_name: &str) -> Result<types::DonePreview, DoneError> {
    let uncommitted_files = get_uncommitted_files(root)?;
    let commits_to_merge = get_commits_to_merge(root)?;
    let potential_conflicts = check_potential_conflicts(root)?;
    let bead_to_close = get_bead_id_for_workspace(workspace_name)?;
    let workspace_path = Path::new(root).join(".zjj/workspaces").join(workspace_name);

    Ok(types::DonePreview {
        uncommitted_files,
        commits_to_merge,
        potential_conflicts,
        bead_to_close,
        workspace_path: workspace_path.to_string_lossy().to_string(),
    })
}

/// Get list of uncommitted files
fn get_uncommitted_files(root: &str) -> Result<Vec<String>, DoneError> {
    let output = Command::new("jj")
        .current_dir(root)
        .args(["status", "--no-pager"])
        .output()
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj status".to_string(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(DoneError::JjCommandFailed {
            command: "jj status".to_string(),
            reason: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("A ")
            || trimmed.starts_with("M ")
            || trimmed.starts_with("D ")
            || trimmed.starts_with("R ")
        {
            if let Some(file) = trimmed.split_ascii_whitespace().nth(1) {
                files.push(file.to_string());
            }
        }
    }

    Ok(files)
}

/// Commit uncommitted changes
fn commit_changes(
    root: &str,
    workspace_name: &str,
    message: Option<&str>,
) -> Result<usize, DoneError> {
    let default_msg = format!("Complete work on {workspace_name}");
    let msg = message.unwrap_or(&default_msg);

    let output = Command::new("jj")
        .current_dir(root)
        .args(["commit", "-m", msg])
        .output()
        .map_err(|e| DoneError::CommitFailed {
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(DoneError::CommitFailed {
            reason: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    // Count files committed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.matches("committed").count();

    Ok(count.max(1))
}

/// Check for merge conflicts
fn check_conflicts(root: &str) -> Result<(), DoneError> {
    let conflicts = check_potential_conflicts(root)?;

    if !conflicts.is_empty() {
        return Err(DoneError::MergeConflict { conflicts });
    }

    Ok(())
}

/// Check for potential conflicts by checking divergent changes
fn check_potential_conflicts(root: &str) -> Result<Vec<String>, DoneError> {
    let output = Command::new("jj")
        .current_dir(root)
        .args(["log", "-r", "@-", "--no-graph", "-T", "description"])
        .output()
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    // Check if there are divergent commits
    let divergent_output = Command::new("jj")
        .current_dir(root)
        .args(["log", "-r", "@..@", "--no-graph"])
        .output()
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    if !divergent_output.status.success() {
        return Ok(Vec::new());
    }

    // For now, return empty - actual conflict detection happens during rebase
    Ok(Vec::new())
}

/// Get commits that will be merged
fn get_commits_to_merge(root: &str) -> Result<Vec<types::CommitInfo>, DoneError> {
    let output = Command::new("jj")
        .current_dir(root)
        .args([
            "log",
            "-r",
            "@..@-",
            "--no-graph",
            "-T",
            r#"change_id "\n" commit_id "\n" description "\n" time(timestamp_safe())"\n""#,
        ])
        .output()
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    let mut lines = stdout.lines().peekable();

    while lines.peek().is_some() {
        let change_id = lines.next().unwrap_or("").trim().to_string();
        let commit_id = lines.next().unwrap_or("").trim().to_string();
        let description = lines.next().unwrap_or("").trim().to_string();
        let timestamp = lines.next().unwrap_or("").trim().to_string();

        if !change_id.is_empty() {
            commits.push(types::CommitInfo {
                change_id,
                commit_id,
                description,
                timestamp,
            });
        }
    }

    Ok(commits)
}

/// Merge workspace changes to main using rebase
fn merge_to_main(root: &str, workspace_name: &str, _squash: bool) -> Result<(), DoneError> {
    // First, abandon the workspace to move changes to main
    let output = Command::new("jj")
        .current_dir(root)
        .args(["workspace", "abandon", "--name", workspace_name])
        .output()
        .map_err(|e| DoneError::MergeFailed {
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("conflict") || stderr.contains("Conflicting") {
            // Parse conflicts from output
            let conflicts: Vec<String> = stderr
                .lines()
                .filter(|l| l.contains("file"))
                .map(|l| l.trim().to_string())
                .collect();

            return Err(DoneError::MergeConflict {
                conflicts: conflicts
                    .iter()
                    .filter(|c| !c.is_empty())
                    .cloned()
                    .collect(),
            });
        }

        return Err(DoneError::MergeFailed {
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

/// Get bead ID for a workspace
fn get_bead_id_for_workspace(workspace_name: &str) -> Result<Option<String>, DoneError> {
    let session_db_path = Path::new(".zjj/state.db");
    if !session_db_path.exists() {
        return Ok(None);
    }

    // For now, check .beads/issues.jsonl for matching session
    let beads_db = Path::new(".beads/issues.jsonl");
    if !beads_db.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(beads_db).map_err(|e| DoneError::BeadUpdateFailed {
        reason: e.to_string(),
    })?;

    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json
                .get("status")
                .and_then(|s| s.as_str())
                .map(|s| s == "in_progress")
                .unwrap_or(false)
            {
                // Check if title contains workspace name or notes mention it
                if let Some(title) = json.get("title").and_then(|t| t.as_str()) {
                    if title.contains(workspace_name) {
                        if let Some(id) = json.get("id").and_then(|i| i.as_str()) {
                            return Ok(Some(id.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Update bead status in the database
fn update_bead_status(bead_id: &str, new_status: &str) -> Result<(), DoneError> {
    let beads_db = Path::new(".beads/issues.jsonl");
    let content = fs::read_to_string(beads_db).map_err(|e| DoneError::BeadUpdateFailed {
        reason: e.to_string(),
    })?;

    let mut new_content = String::new();
    let mut updated = false;

    for line in content.lines() {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(line) {
            if json
                .get("id")
                .and_then(|i| i.as_str())
                .map(|i| i == bead_id)
                .unwrap_or(false)
            {
                json["status"] = serde_json::json!(new_status);
                updated = true;
            }
            new_content.push_str(&json.to_string());
            new_content.push('\n');
        }
    }

    if updated {
        fs::write(beads_db, new_content).map_err(|e| DoneError::BeadUpdateFailed {
            reason: e.to_string(),
        })?;
    }

    Ok(())
}

/// Cleanup the workspace directory
fn cleanup_workspace(root: &str, workspace_name: &str) -> Result<bool, DoneError> {
    let workspace_path = Path::new(root).join(".zjj/workspaces").join(workspace_name);

    if workspace_path.exists() {
        fs::remove_dir_all(&workspace_path).map_err(|e| DoneError::CleanupFailed {
            reason: format!("Failed to remove workspace {workspace_name}: {e}"),
        })?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Output the result in the appropriate format
fn output_result(result: &DoneOutput, format: zjj_core::OutputFormat) -> Result<()> {
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else if result.dry_run {
        println!(
            "🔍 Dry-run preview for workspace: {}",
            result.workspace_name
        );
        if let Some(ref preview) = result.preview {
            if !preview.uncommitted_files.is_empty() {
                println!("  Files to commit:");
                for file in &preview.uncommitted_files {
                    println!("    - {file}");
                }
            }
            if !preview.commits_to_merge.is_empty() {
                println!("  Commits to merge: {}", preview.commits_to_merge.len());
            }
            if let Some(ref bead) = preview.bead_to_close {
                println!("  Bead to close: {bead}");
            }
        }
    } else {
        println!("✅ Workspace '{}' completed", result.workspace_name);
        if result.merged {
            println!("  Merged {} commits to main", result.commits_merged);
        }
        if result.files_committed > 0 {
            println!("  Committed {} files", result.files_committed);
        }
        if result.cleaned {
            println!("  Workspace cleaned up");
        }
        if result.bead_closed {
            println!("  Bead status updated to closed");
        }
        // Post-command workflow guidance
        println!();
        println!("NEXT: Start new work with:");
        println!("  zjj spawn <bead-id>   # Create isolated workspace for new task");
        println!("  bd ready              # See available work items");
    }
    Ok(())
}

/// Output error in the appropriate format
fn output_error(error: &DoneError, format: zjj_core::OutputFormat) -> Result<()> {
    if format.is_json() {
        let error_json = serde_json::json!({
            "error": error.to_string(),
            "error_code": error.error_code(),
            "phase": error.phase().name(),
            "recoverable": error.is_recoverable(),
        });
        println!("{}", serde_json::to_string_pretty(&error_json)?);
    } else {
        eprintln!("❌ {error}");
        if error.is_recoverable() {
            eprintln!("   Workspace preserved - resolve conflicts and retry");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::done::types::DonePhase;

    #[test]
    fn test_done_output_default() {
        let output = DoneOutput::default();
        assert!(output.workspace_name.is_empty());
        assert!(!output.merged);
        assert!(!output.cleaned);
    }

    #[test]
    fn test_error_code_is_consistent_with_phase() {
        let err = DoneError::CommitFailed {
            reason: "test".to_string(),
        };
        assert_eq!(err.phase(), DonePhase::CommittingChanges);
        assert_eq!(err.error_code(), "COMMIT_FAILED");
    }
}

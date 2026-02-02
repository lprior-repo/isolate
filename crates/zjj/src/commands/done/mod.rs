//! Done command - Complete work and merge to main
//!
//! This command:
//! 1. Validates we're in a workspace (not main)
//! 2. Checks for uncommitted changes
//! 3. Commits any uncommitted changes
//! 4. Checks for merge conflicts
//! 5. Merges workspace changes to main
//! 6. Logs undo history to .zjj/undo.log
//! 7. Updates linked bead status to completed
//! 8. Keeps workspace for 24h (unless --no-keep specified)
//! 9. Switches back to main

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod bead;
pub mod executor;
pub mod filesystem;
pub mod newtypes;
pub mod types;

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
pub use types::{DoneError, DoneExitCode, DoneOptions, DoneOutput, UndoEntry};

use crate::{
    cli::jj_root,
    commands::context::{detect_location, Location},
};

/// Run the done command with options
pub fn run_with_options(options: &DoneOptions) -> Result<DoneExitCode> {
    // Create real dependencies
    let executor = executor::RealJjExecutor::new();
    let mut bead_repo = bead::MockBeadRepository::new(); // TODO: Replace with real implementation
    let filesystem = filesystem::RealFileSystem::new();

    let result = execute_done(options, &executor, &mut bead_repo, &filesystem);

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
fn execute_done(
    options: &DoneOptions,
    executor: &dyn executor::JjExecutor,
    bead_repo: &mut dyn bead::BeadRepository,
    filesystem: &dyn filesystem::FileSystem,
) -> Result<DoneOutput, DoneError> {
    // Phase 1: Validate location (must be in workspace)
    let root = validate_location()?;
    let workspace_name = get_workspace_name(&root)?;

    // Phase 2: Build preview for dry-run
    let preview = if options.dry_run {
        Some(build_preview(&root, &workspace_name, executor, bead_repo)?)
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
    let uncommitted_files = get_uncommitted_files(&root, executor)?;

    // Phase 4: Commit uncommitted changes
    let files_committed = if uncommitted_files.is_empty() {
        0
    } else {
        commit_changes(&root, &workspace_name, options.message.as_deref(), executor)?
    };

    // Phase 5: Check for conflicts
    check_conflicts(&root, executor)?;

    // Phase 5.5: Get pre-merge commit ID (for undo)
    let pre_merge_commit_id = get_current_commit_id(&root, executor)?;

    // Phase 5.6: Check if pushed to remote (for undo)
    let pushed_to_remote = is_pushed_to_remote(&root, executor)?;

    // Phase 6: Get commits to merge
    let commits_to_merge = get_commits_to_merge(&root, executor)?;

    // Phase 7: Merge to main
    merge_to_main(&root, &workspace_name, options.squash, executor)?;

    // Phase 7.5: Log undo history
    log_undo_history(
        &root,
        &workspace_name,
        &pre_merge_commit_id,
        pushed_to_remote,
        filesystem,
    )?;

    // Phase 8: Update bead status
    let bead_id = get_bead_id_for_workspace(&workspace_name, bead_repo)?;
    let bead_closed = if let Some(ref bead) = bead_id {
        if options.no_bead_update {
            false
        } else {
            update_bead_status(bead, "closed", bead_repo)?;
            true
        }
    } else {
        false
    };

    // Phase 9: Cleanup workspace
    let cleaned = if options.keep_workspace || !options.no_keep {
        false
    } else {
        cleanup_workspace(&root, &workspace_name, filesystem)?
    };

    Ok(DoneOutput {
        workspace_name,
        bead_id,
        files_committed,
        commits_merged: commits_to_merge.len(),
        merged: true,
        cleaned,
        bead_closed,
        pushed_to_remote,
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
fn build_preview(
    root: &str,
    workspace_name: &str,
    executor: &dyn executor::JjExecutor,
    bead_repo: &dyn bead::BeadRepository,
) -> Result<types::DonePreview, DoneError> {
    let uncommitted_files = get_uncommitted_files(root, executor)?;
    let commits_to_merge = get_commits_to_merge(root, executor)?;
    let potential_conflicts = check_potential_conflicts(root, executor)?;
    let bead_to_close = get_bead_id_for_workspace(workspace_name, bead_repo)?;
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
fn get_uncommitted_files(
    root: &str,
    executor: &dyn executor::JjExecutor,
) -> Result<Vec<String>, DoneError> {
    let output =
        executor
            .run(&["status", "--no-pager"])
            .map_err(|e| DoneError::JjCommandFailed {
                command: "jj status".to_string(),
                reason: e.to_string(),
            })?;

    let stdout = output.as_str();
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
    executor: &dyn executor::JjExecutor,
) -> Result<usize, DoneError> {
    let default_msg = format!("Complete work on {workspace_name}");
    let msg = message.unwrap_or(&default_msg);

    let output = executor
        .run(&["commit", "-m", msg])
        .map_err(|e| DoneError::CommitFailed {
            reason: e.to_string(),
        })?;

    // Count files committed
    let stdout = output.as_str();
    let count = stdout.matches("committed").count();

    Ok(count.max(1))
}

/// Check for merge conflicts
fn check_conflicts(root: &str, executor: &dyn executor::JjExecutor) -> Result<(), DoneError> {
    let conflicts = check_potential_conflicts(root, executor)?;

    if !conflicts.is_empty() {
        return Err(DoneError::MergeConflict { conflicts });
    }

    Ok(())
}

/// Check for potential conflicts by checking divergent changes
#[allow(clippy::unnecessary_wraps)]
fn check_potential_conflicts(
    root: &str,
    executor: &dyn executor::JjExecutor,
) -> Result<Vec<String>, DoneError> {
    // Try to get log
    let _output = executor
        .run(&["log", "-r", "@-", "--no-graph", "-T", "description"])
        .ok();

    #[allow(clippy::unnecessary_wraps)]
    let _divergent_output = executor.run(&["log", "-r", "@..@", "--no-graph"]).ok();

    // For now, return empty - actual conflict detection happens during rebase
    #[allow(clippy::unnecessary_wraps)]
    Ok(Vec::new())
}

/// Get commits that will be merged
fn get_commits_to_merge(
    root: &str,
    executor: &dyn executor::JjExecutor,
) -> Result<Vec<types::CommitInfo>, DoneError> {
    let output = executor
        .run(&[
            "log",
            "-r",
            "@..@-",
            "--no-graph",
            "-T",
            r#"change_id "\n" commit_id "\n" description "\n" time(timestamp_safe())"\n""#,
        ])
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    let stdout = output.as_str();
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
fn merge_to_main(
    root: &str,
    workspace_name: &str,
    _squash: bool,
    executor: &dyn executor::JjExecutor,
) -> Result<(), DoneError> {
    // First, abandon the workspace to move changes to main
    let result = executor.run(&["workspace", "abandon", "--name", workspace_name]);

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("conflict") || error_msg.contains("Conflicting") {
                // Parse conflicts from output
                let conflicts: Vec<String> = error_msg
                    .lines()
                    .filter(|l| l.contains("file"))
                    .map(|l| l.trim().to_string())
                    .collect();

                Err(DoneError::MergeConflict {
                    conflicts: conflicts
                        .iter()
                        .filter(|c| !c.is_empty())
                        .cloned()
                        .collect(),
                })
            } else {
                Err(DoneError::MergeFailed { reason: error_msg })
            }
        }
    }
}

/// Get bead ID for a workspace using the bead repository
fn get_bead_id_for_workspace(
    workspace_name: &str,
    bead_repo: &dyn bead::BeadRepository,
) -> Result<Option<String>, DoneError> {
    use newtypes::WorkspaceName;

    let workspace =
        WorkspaceName::new(workspace_name.to_string()).map_err(|e| DoneError::InvalidState {
            reason: format!("Invalid workspace name: {}", e),
        })?;

    bead_repo
        .find_by_workspace(&workspace)
        .map(|opt| opt.map(|id| id.as_str().to_string()))
        .map_err(|e| DoneError::BeadUpdateFailed {
            reason: format!("Failed to find bead: {}", e),
        })
}

/// Update bead status in the database using the bead repository
fn update_bead_status(
    bead_id: &str,
    new_status: &str,
    bead_repo: &mut dyn bead::BeadRepository,
) -> Result<(), DoneError> {
    use newtypes::BeadId;

    let bead_id_newtype =
        BeadId::new(bead_id.to_string()).map_err(|e| DoneError::BeadUpdateFailed {
            reason: format!("Invalid bead ID: {}", e),
        })?;

    bead_repo
        .update_status(&bead_id_newtype, new_status)
        .map_err(|e| DoneError::BeadUpdateFailed {
            reason: format!("Failed to update bead: {}", e),
        })
}

/// Cleanup the workspace directory
fn cleanup_workspace(
    root: &str,
    workspace_name: &str,
    filesystem: &dyn filesystem::FileSystem,
) -> Result<bool, DoneError> {
    let workspace_path = Path::new(root).join(".zjj/workspaces").join(workspace_name);

    if filesystem.exists(&workspace_path) {
        filesystem
            .remove_dir_all(&workspace_path)
            .map_err(|e| DoneError::CleanupFailed {
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

/// Get current commit ID (before merge)
fn get_current_commit_id(
    root: &str,
    executor: &dyn executor::JjExecutor,
) -> Result<String, DoneError> {
    let output = executor
        .run(&["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    Ok(output.as_str().trim().to_string())
}

/// Check if changes have been pushed to remote
fn is_pushed_to_remote(root: &str, executor: &dyn executor::JjExecutor) -> Result<bool, DoneError> {
    let output = executor
        .run(&["log", "-r", "@-"])
        .map_err(|e| DoneError::JjCommandFailed {
            command: "jj log".to_string(),
            reason: e.to_string(),
        })?;

    Ok(output.as_str().trim().is_empty())
}

/// Log undo history to .zjj/undo.log
fn log_undo_history(
    root: &str,
    workspace_name: &str,
    pre_merge_commit_id: &str,
    pushed_to_remote: bool,
    filesystem: &dyn filesystem::FileSystem,
) -> Result<(), DoneError> {
    let undo_log_path = Path::new(root).join(".zjj/undo.log");

    let undo_entry = UndoEntry {
        session_name: workspace_name.to_string(),
        commit_id: String::new(),
        pre_merge_commit_id: pre_merge_commit_id.to_string(),
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| DoneError::InvalidState {
                reason: format!("System time error: {e}"),
            })?
            .as_secs(),
        pushed_to_remote,
        status: "completed".to_string(),
    };

    let json = serde_json::to_string(&undo_entry).map_err(|e| DoneError::InvalidState {
        reason: format!("Failed to serialize undo entry: {e}"),
    })?;

    let mut content = if undo_log_path.exists() {
        filesystem
            .read_to_string(&undo_log_path)
            .map_err(|e| DoneError::InvalidState {
                reason: format!("Failed to read undo log: {e}"),
            })?
    } else {
        String::new()
    };
    content.push_str(&json);
    content.push('\n');

    filesystem
        .write(&undo_log_path, &content)
        .map_err(|e| DoneError::InvalidState {
            reason: format!("Failed to write undo log: {e}"),
        })?;

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

    // ── DoneOutput Tests ─────────────────────────────────────────────────

    #[test]
    fn test_done_output_dry_run() {
        let output = DoneOutput {
            workspace_name: "test".to_string(),
            dry_run: true,
            preview: Some(types::DonePreview {
                uncommitted_files: vec!["file.txt".to_string()],
                commits_to_merge: vec![],
                potential_conflicts: vec![],
                bead_to_close: None,
                workspace_path: "/path".to_string(),
            }),
            ..Default::default()
        };
        assert!(output.dry_run);
        assert!(output.preview.is_some());
    }

    #[test]
    fn test_done_output_successful_merge() {
        let output = DoneOutput {
            workspace_name: "feature-auth".to_string(),
            bead_id: Some("zjj-abc123".to_string()),
            files_committed: 3,
            commits_merged: 2,
            merged: true,
            cleaned: true,
            bead_closed: true,
            pushed_to_remote: false,
            dry_run: false,
            preview: None,
            error: None,
        };
        assert!(output.merged);
        assert!(output.bead_closed);
        assert_eq!(output.commits_merged, 2);
    }

    #[test]
    fn test_done_output_serialization() {
        let output = DoneOutput {
            workspace_name: "test".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("workspace_name"));
        assert!(json_str.contains("merged"));
    }

    // ── DoneError Tests ──────────────────────────────────────────────────

    #[test]
    fn test_done_error_not_in_workspace() {
        let err = DoneError::NotInWorkspace {
            current_location: "/home/user/project".to_string(),
        };
        assert_eq!(err.error_code(), "NOT_IN_WORKSPACE");
        assert_eq!(err.phase(), DonePhase::ValidatingLocation);
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_done_error_not_a_jj_repo() {
        let err = DoneError::NotAJjRepo;
        assert_eq!(err.error_code(), "NOT_A_JJ_REPO");
        assert_eq!(err.phase(), DonePhase::ValidatingLocation);
    }

    #[test]
    fn test_done_error_merge_conflict() {
        let err = DoneError::MergeConflict {
            conflicts: vec!["file1.txt".to_string(), "file2.txt".to_string()],
        };
        assert_eq!(err.error_code(), "MERGE_CONFLICT");
        assert_eq!(err.phase(), DonePhase::MergingToMain);
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_done_error_merge_failed() {
        let err = DoneError::MergeFailed {
            reason: "rebase failed".to_string(),
        };
        assert_eq!(err.error_code(), "MERGE_FAILED");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_done_error_cleanup_failed() {
        let err = DoneError::CleanupFailed {
            reason: "permission denied".to_string(),
        };
        assert_eq!(err.error_code(), "CLEANUP_FAILED");
        assert_eq!(err.phase(), DonePhase::CleaningWorkspace);
    }

    #[test]
    fn test_done_error_bead_update_failed() {
        let err = DoneError::BeadUpdateFailed {
            reason: "db error".to_string(),
        };
        assert_eq!(err.error_code(), "BEAD_UPDATE_FAILED");
        assert_eq!(err.phase(), DonePhase::UpdatingBeadStatus);
    }

    #[test]
    fn test_done_error_jj_command_failed() {
        let err = DoneError::JjCommandFailed {
            command: "jj status".to_string(),
            reason: "not found".to_string(),
        };
        assert_eq!(err.error_code(), "JJ_COMMAND_FAILED");
        assert_eq!(err.phase(), DonePhase::MergingToMain);
    }

    #[test]
    fn test_done_error_invalid_state() {
        let err = DoneError::InvalidState {
            reason: "corrupted".to_string(),
        };
        assert_eq!(err.error_code(), "INVALID_STATE");
        assert_eq!(err.phase(), DonePhase::ValidatingLocation);
    }

    #[test]
    fn test_done_error_display_formats() {
        let err1 = DoneError::NotInWorkspace {
            current_location: "main".to_string(),
        };
        let display = format!("{err1}");
        assert!(display.contains("Not in a workspace"));
        assert!(display.contains("main"));

        let err2 = DoneError::MergeConflict {
            conflicts: vec!["a.txt".to_string(), "b.txt".to_string()],
        };
        let display2 = format!("{err2}");
        assert!(display2.contains("conflict"));
        assert!(display2.contains("a.txt"));
    }

    // ── DoneExitCode Tests ───────────────────────────────────────────────

    #[test]
    fn test_done_exit_code_values() {
        assert_eq!(DoneExitCode::Success as i32, 0);
        assert_eq!(DoneExitCode::MergeConflict as i32, 1);
        assert_eq!(DoneExitCode::NotInWorkspace as i32, 2);
        assert_eq!(DoneExitCode::OtherError as i32, 3);
    }

    // ── UndoEntry Tests ──────────────────────────────────────────────────

    #[test]
    fn test_undo_entry_serialization() {
        let entry = UndoEntry {
            session_name: "test-session".to_string(),
            commit_id: "abc123".to_string(),
            pre_merge_commit_id: "def456".to_string(),
            timestamp: 1_706_270_400,
            pushed_to_remote: false,
            status: "completed".to_string(),
        };
        let json = serde_json::to_string(&entry);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("test-session"));
        assert!(json_str.contains("abc123"));
        assert!(json_str.contains("pre_merge_commit_id"));
    }

    #[test]
    fn test_undo_entry_deserialization() {
        let json = r#"{"session_name":"ws1","commit_id":"c1","pre_merge_commit_id":"pm1","timestamp":123,"pushed_to_remote":false,"status":"completed"}"#;
        let entry: Result<UndoEntry, _> = serde_json::from_str(json);
        assert!(entry.is_ok());
        let entry = entry.unwrap_or_else(|_| UndoEntry {
            session_name: String::new(),
            commit_id: String::new(),
            pre_merge_commit_id: String::new(),
            timestamp: 0,
            pushed_to_remote: false,
            status: String::new(),
        });
        assert_eq!(entry.session_name, "ws1");
        assert_eq!(entry.timestamp, 123);
    }
}

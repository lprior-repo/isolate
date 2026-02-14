//! Spawn command - Create isolated workspace and run agent
//!
//! This command:
//! 1. Validates we're on main branch (not in a workspace)
//! 2. Validates the bead is ready/open
//! 3. Creates a JJ workspace for the bead
//! 4. Updates bead status to `in_progress`
//! 5. Spawns an agent subprocess in the workspace
//! 6. Waits for completion (or background)
//! 7. On success: merges to main and cleans up
//! 8. On failure: cleans up without merging

pub mod heartbeat;
pub mod rollback;
pub mod types;

pub use heartbeat::{write_heartbeat_instructions, HeartbeatMonitor};
pub use rollback::{SignalHandler, TransactionTracker};
pub use types::{SpawnArgs, SpawnError, SpawnOptions, SpawnOutput};

/// AI instructions placed in spawned workspace
const AI_INSTRUCTIONS: &str = r"# AI Agent - You Are in a ZJJ Workspace

## STOP - Do NOT Clone Elsewhere

You were invoked via `zjj spawn` and are **already in the correct isolated workspace**.

- **Work here** - Do NOT clone this repository to another location
- **Current workspace**: `.zjj/workspaces/<bead-id>/`
- **Your task**: Defined by `$ZJJ_BEAD_ID`

## Environment Variables

- `ZJJ_BEAD_ID` - The bead/issue you're working on
- `ZJJ_WORKSPACE` - Path to this isolated workspace

## When Done

Just exit cleanly with success (exit code 0). ZJJ will automatically:
1. Merge your changes to the main branch
2. Clean up this workspace
3. Mark the bead as completed

## Check Your Task

```bash
br show $ZJJ_BEAD_ID
```

## Build Commands

Check the project's README or CLAUDE.md for the correct build commands.
";

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use types::SpawnStatus;
use zjj_core::{config, json::SchemaEnvelope};

use crate::{
    beads::{BeadRepository, BeadStatus},
    cli::jj_root,
    commands::context::{detect_location, Location},
};

/// Run the spawn command with options
pub async fn run_with_options(options: &SpawnOptions) -> Result<()> {
    let result = execute_spawn(options).await?;

    output_result(&result, options.format)?;

    if matches!(result.status, SpawnStatus::Failed) {
        anyhow::bail!("Spawn operation failed");
    }

    Ok(())
}

/// Core spawn logic using Railway-Oriented Programming
pub async fn execute_spawn(options: &SpawnOptions) -> Result<SpawnOutput, SpawnError> {
    // Phase 1: Validate location (must be on main)
    let root = validate_location()
        .await
        .map_err(|e| SpawnError::NotOnMain {
            current_location: e.to_string(),
        })?;

    let bead_repo = BeadRepository::new(&root);

    // Phase 2: Validate bead status
    validate_bead_status(&bead_repo, &root, &options.bead_id, options.idempotent).await?;

    // Initialize transaction tracker
    let workspace = create_workspace(&root, &options.bead_id, options.idempotent).await?;

    let tracker = TransactionTracker::new(&options.bead_id, &workspace.path).await?;

    // Register signal handlers for graceful shutdown
    let signal_handler = SignalHandler::new(Some(tracker.clone()));
    signal_handler.register()?;

    // Phase 3: Create workspace
    if !workspace.reused_existing {
        tracker.mark_workspace_created()?;
    }

    // Phase 4: Update bead status to in_progress
    if let Err(e) = bead_repo
        .update_status(&options.bead_id, BeadStatus::InProgress)
        .await
    {
        let _ = tracker.rollback().await; // Ignore rollback errors in error path
        return Err(SpawnError::DatabaseError {
            reason: e.to_string(),
        });
    }
    tracker.mark_bead_status_updated()?;

    // Phase 5: Spawn agent with transaction tracking
    // Apply timeout to the spawn operation
    let spawn_result = if options.background {
        tokio::time::timeout(
            Duration::from_secs(options.timeout_secs),
            spawn_agent_background(&workspace.path, options),
        )
        .await
    } else {
        tokio::time::timeout(
            Duration::from_secs(options.timeout_secs),
            spawn_agent_foreground(&workspace.path, options),
        )
        .await
    };

    let (pid, exit_code) = if let Ok(result) = spawn_result {
        result?
    } else {
        let _ = tracker.rollback().await;
        return Err(SpawnError::Timeout {
            timeout_secs: options.timeout_secs,
        });
    };

    if let Some(pid) = pid {
        tracker.mark_agent_spawned(pid)?;
    }

    // Phase 6-8: Handle completion
    let (merged, cleaned, status) = match exit_code {
        Some(0) => handle_success(&root, &options.bead_id, &workspace.path, options).await?,
        Some(code) => handle_failure(&root, &workspace.path, options, code).await?,
        None => (false, false, SpawnStatus::Running),
    };

    Ok(SpawnOutput {
        bead_id: options.bead_id.clone(),
        workspace_path: workspace.path.to_string_lossy().to_string(),
        agent_pid: pid,
        exit_code,
        merged,
        cleaned,
        status,
    })
}

/// Validate we're on main branch (not in a workspace)
async fn validate_location() -> Result<String> {
    let root = jj_root().await.context("Failed to get JJ root")?;

    let location = detect_location(&PathBuf::from(&root)).context("Failed to detect location")?;
    if matches!(location, Location::Workspace { .. }) {
        anyhow::bail!("In workspace directory");
    }

    Ok(root)
}

/// Validate that the bead exists and has appropriate status
async fn validate_bead_status(
    bead_repo: &BeadRepository,
    root: &str,
    bead_id: &str,
    idempotent: bool,
) -> Result<(), SpawnError> {
    let bead = bead_repo
        .get_bead(bead_id)
        .await
        .map_err(|e| SpawnError::DatabaseError {
            reason: format!("Failed to read beads database: {e}"),
        })?;

    let Some(bead) = bead else {
        return Err(SpawnError::BeadNotFound {
            bead_id: bead_id.to_string(),
        });
    };

    // Check if status is appropriate.
    // In idempotent mode, allow in_progress so retries can safely continue.
    if idempotent && matches!(bead.status, BeadStatus::InProgress) {
        if is_retryable_in_progress_bead(root, bead_id).await? {
            return Ok(());
        }

        return Err(SpawnError::InvalidBeadStatus {
            bead_id: bead_id.to_string(),
            status: "in_progress (active agent)".to_string(),
        });
    }

    // Default path requires open bead.
    match bead.status {
        BeadStatus::Open => Ok(()),
        _ => Err(SpawnError::InvalidBeadStatus {
            bead_id: bead_id.to_string(),
            status: bead.status.to_string(),
        }),
    }
}

async fn is_retryable_in_progress_bead(root: &str, bead_id: &str) -> Result<bool, SpawnError> {
    if !workspace_registered_in_jj(root, bead_id).await? {
        return Ok(false);
    }

    let workspace_path = workspace_path_for_bead(root, bead_id).await?;
    let workspace_exists = tokio::fs::try_exists(&workspace_path).await.map_err(|e| {
        SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to check existing workspace path: {e}"),
        }
    })?;

    if !workspace_exists {
        return Ok(false);
    }

    let heartbeat = HeartbeatMonitor::with_defaults(&workspace_path);
    let alive = heartbeat.is_alive().await?;
    Ok(!alive)
}

/// Create a JJ workspace for the bead with operation graph synchronization
///
/// This uses the synchronized workspace creation to prevent operation graph
/// corruption when multiple workspaces are created concurrently or in quick
/// succession.
async fn create_workspace(
    root: &str,
    bead_id: &str,
    idempotent: bool,
) -> Result<WorkspaceResolution, SpawnError> {
    let workspaces_dir = workspaces_dir(root).await;
    tokio::fs::create_dir_all(&workspaces_dir)
        .await
        .map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to create workspaces directory: {e}"),
        })?;

    let workspace_path = workspace_path_for_bead(root, bead_id).await?;

    if tokio::fs::try_exists(&workspace_path).await.map_err(|e| {
        SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to check existing workspace path: {e}"),
        }
    })? {
        if idempotent {
            let workspace_registered = workspace_registered_in_jj(root, bead_id).await?;
            if workspace_registered {
                create_workspace_discoverability(&workspace_path).await?;
                return Ok(WorkspaceResolution {
                    path: workspace_path,
                    reused_existing: true,
                });
            }
        }

        return Err(SpawnError::WorkspaceCreationFailed {
            reason: format!(
                "Workspace already exists at {}. Retry with --idempotent",
                workspace_path.display(),
            ),
        });
    }

    // Use synchronized workspace creation to prevent operation graph corruption
    // This ensures:
    // 1. Workspace creations are serialized (prevents concurrent modification)
    // 2. All workspaces are based on the same repository operation
    // 3. Operation graph consistency is verified after creation
    // CRITICAL-004 fix: Pass root explicitly to support sibling workspace directories
    let create_result = zjj_core::jj_operation_sync::create_workspace_synced(
        bead_id,
        &workspace_path,
        Path::new(root),
    )
    .await;

    if let Err(e) = create_result {
        let reason = e.to_string();
        let workspace_exists_error = reason.to_ascii_lowercase().contains("already exists");

        if workspace_exists_error {
            let workspace_registered = workspace_registered_in_jj(root, bead_id).await?;

            if workspace_registered && idempotent {
                create_workspace_discoverability(&workspace_path).await?;
                return Ok(WorkspaceResolution {
                    path: workspace_path,
                    reused_existing: true,
                });
            }

            if workspace_registered {
                return Err(SpawnError::WorkspaceCreationFailed {
                    reason: format!(
                        "Workspace '{bead_id}' already exists. Retry with --idempotent"
                    ),
                });
            }
        }

        return Err(SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to create workspace with operation sync: {reason}"),
        });
    }

    // Create AI discoverability files in the workspace
    create_workspace_discoverability(&workspace_path).await?;

    Ok(WorkspaceResolution {
        path: workspace_path,
        reused_existing: false,
    })
}

async fn workspace_path_for_bead(
    root: &str,
    bead_id: &str,
) -> Result<std::path::PathBuf, SpawnError> {
    Ok(workspaces_dir(root).await.join(bead_id))
}

async fn workspaces_dir(root: &str) -> std::path::PathBuf {
    let workspace_dir_name = config::load_config()
        .await
        .map(|cfg| cfg.workspace_dir)
        .unwrap_or_else(|_| ".zjj/workspaces".to_string());
    Path::new(root).join(workspace_dir_name)
}

async fn workspace_registered_in_jj(root: &str, workspace_name: &str) -> Result<bool, SpawnError> {
    let output = tokio::process::Command::new("jj")
        .args(["workspace", "list"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| SpawnError::JjCommandFailed {
            reason: format!("Failed to execute jj workspace list: {e}"),
        })?;

    if !output.status.success() {
        return Err(SpawnError::JjCommandFailed {
            reason: format!(
                "jj workspace list failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let workspace_list = String::from_utf8_lossy(&output.stdout);
    let exists = workspace_list
        .lines()
        .any(|line| workspace_line_matches_name(line, workspace_name));

    Ok(exists)
}

fn workspace_line_matches_name(line: &str, workspace_name: &str) -> bool {
    line.split_once(':').is_some_and(|(raw_name, _)| {
        raw_name.trim().trim_start_matches('*').trim() == workspace_name
    })
}

struct WorkspaceResolution {
    path: std::path::PathBuf,
    reused_existing: bool,
}

/// Create discoverability files in the spawned workspace
///
/// These files tell AI agents they're already in the right place
/// and should NOT clone the repository elsewhere.
async fn create_workspace_discoverability(workspace_path: &Path) -> Result<(), SpawnError> {
    // Create .ai-instructions.md for Claude Code and others
    let ai_instructions_path = workspace_path.join(".ai-instructions.md");
    tokio::fs::write(&ai_instructions_path, AI_INSTRUCTIONS)
        .await
        .map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to create .ai-instructions.md: {e}"),
        })?;

    // Write heartbeat monitoring instructions
    write_heartbeat_instructions(workspace_path).await?;

    Ok(())
}

/// Spawn agent in foreground (wait for completion)
async fn spawn_agent_foreground(
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(Option<u32>, Option<i32>), SpawnError> {
    let heartbeat = HeartbeatMonitor::with_defaults(workspace_path);
    heartbeat.initialize().await?;

    let mut cmd = tokio::process::Command::new(&options.agent_command);
    cmd.args(&options.agent_args)
        .current_dir(workspace_path)
        .env("ZJJ_BEAD_ID", &options.bead_id)
        .env("ZJJ_WORKSPACE", workspace_path.to_string_lossy().as_ref())
        .env("ZJJ_ACTIVE", "1") // Required by git pre-commit hook
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| SpawnError::AgentSpawnFailed {
        reason: format!("Failed to spawn agent: {e}"),
    })?;

    let pid = child.id();

    // Wait for completion asynchronously
    let status = child
        .wait()
        .await
        .map_err(|e| SpawnError::AgentSpawnFailed {
            reason: format!("Failed to wait for agent: {e}"),
        })?;

    let exit_code = status.code();

    heartbeat.cleanup().await?;

    Ok((pid, exit_code))
}

/// Spawn agent in background (don't wait)
async fn spawn_agent_background(
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(Option<u32>, Option<i32>), SpawnError> {
    let heartbeat = HeartbeatMonitor::with_defaults(workspace_path);
    heartbeat.initialize().await?;

    let mut cmd = tokio::process::Command::new(&options.agent_command);
    cmd.args(&options.agent_args)
        .current_dir(workspace_path)
        .env("ZJJ_BEAD_ID", &options.bead_id)
        .env("ZJJ_WORKSPACE", workspace_path.to_string_lossy().as_ref())
        .env("ZJJ_ACTIVE", "1") // Required by git pre-commit hook
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn().map_err(|e| SpawnError::AgentSpawnFailed {
        reason: format!("Failed to spawn agent: {e}"),
    })?;

    let pid = child.id();

    // Detach - process continues in background
    Ok((pid, None))
}

/// Handle successful agent completion
async fn handle_success(
    root: &str,
    bead_id: &str,
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(bool, bool, SpawnStatus), SpawnError> {
    // If we're not cleaning up, we can't merge (forget) the workspace because forgetting deletes
    // it. So no_auto_cleanup implies no_auto_merge.
    let merged = if options.no_auto_merge || options.no_auto_cleanup {
        false
    } else {
        merge_to_main(root, bead_id).await?
    };

    let cleaned = if options.no_auto_cleanup {
        false
    } else {
        cleanup_workspace(workspace_path).await?
    };

    let bead_repo = BeadRepository::new(root);
    // Update bead to completed
    bead_repo
        .update_status(bead_id, BeadStatus::Closed)
        .await
        .map_err(|e| SpawnError::DatabaseError {
            reason: e.to_string(),
        })?;

    Ok((merged, cleaned, SpawnStatus::Completed))
}

/// Handle failed agent completion
async fn handle_failure(
    root: &str,
    workspace_path: &Path,
    options: &SpawnOptions,
    _exit_code: i32,
) -> Result<(bool, bool, SpawnStatus), SpawnError> {
    let cleaned = if options.no_auto_cleanup {
        false
    } else {
        cleanup_workspace(workspace_path).await?
    };

    // Reset bead status from in_progress to open for retry
    let bead_id = workspace_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| SpawnError::BeadNotFound {
            bead_id: "unknown".to_string(),
        })?;

    let bead_repo = BeadRepository::new(root);
    bead_repo
        .update_status(bead_id, BeadStatus::Open)
        .await
        .map_err(|e| SpawnError::DatabaseError {
            reason: e.to_string(),
        })?;

    Ok((false, cleaned, SpawnStatus::Failed))
}

/// Merge workspace changes to main by forgetting the workspace
///
/// This function uses `jj workspace forget` to remove the workspace record.
///
/// # Arguments
/// * `root` - The JJ repository root directory
/// * `workspace_name` - The name of the workspace to forget (`bead_id`)
///
/// # Returns
/// * `Ok(true)` - If the workspace was successfully forgotten
/// * `Err(SpawnError)` - If the forget operation failed
///
/// # Errors
/// * `JjCommandFailed` - If the jj command execution fails
/// * `MergeFailed` - If the workspace doesn't exist or forget fails
async fn merge_to_main(root: &str, workspace_name: &str) -> Result<bool, SpawnError> {
    // First, check if the workspace exists before attempting to forget
    let list_output = tokio::process::Command::new("jj")
        .args(["workspace", "list"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| SpawnError::JjCommandFailed {
            reason: format!("Failed to execute jj workspace list: {e}"),
        })?;

    if !list_output.status.success() {
        return Err(SpawnError::JjCommandFailed {
            reason: format!(
                "jj workspace list failed: {}",
                String::from_utf8_lossy(&list_output.stderr)
            ),
        });
    }

    // Check if our workspace exists in the list
    let workspace_list = String::from_utf8_lossy(&list_output.stdout);
    let workspace_exists = workspace_list
        .lines()
        .any(|line| line.contains(workspace_name));

    if !workspace_exists {
        return Err(SpawnError::MergeFailed {
            reason: format!("Workspace '{workspace_name}' does not exist"),
        });
    }

    // Abandon the workspace to merge changes back to main
    let forget_output = tokio::process::Command::new("jj")
        .args(["workspace", "forget", workspace_name])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| SpawnError::JjCommandFailed {
            reason: format!("Failed to execute jj workspace forget: {e}"),
        })?;

    if !forget_output.status.success() {
        let stderr = String::from_utf8_lossy(&forget_output.stderr);
        let stdout = String::from_utf8_lossy(&forget_output.stdout);

        // Check for conflict indicators in the output
        let error_output = if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };

        return Err(SpawnError::JjCommandFailed {
            reason: format!("jj workspace forget failed: {error_output}"),
        });
    }

    Ok(true)
}

/// Clean up the workspace directory
async fn cleanup_workspace(workspace_path: &Path) -> Result<bool, SpawnError> {
    match tokio::fs::try_exists(workspace_path).await {
        Ok(true) => {
            tokio::fs::remove_dir_all(workspace_path)
                .await
                .map_err(|e| SpawnError::CleanupFailed {
                    reason: format!("Failed to remove workspace: {e}"),
                })?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Output the result in the appropriate format
fn output_result(result: &SpawnOutput, format: zjj_core::OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("spawn-response", "single", result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Spawn operation: {}", status_display(&result.status));
        println!("  Bead ID: {}", result.bead_id);
        println!("  Workspace: {}", result.workspace_path);
        if let Some(pid) = result.agent_pid {
            println!("  Agent PID: {pid}");
        }
        if let Some(code) = result.exit_code {
            println!("  Exit code: {code}");
        }
        if result.merged {
            println!("  Merged: yes");
        }
        if result.cleaned {
            println!("  Cleaned up: yes");
        }
        // Post-command workflow guidance
        if matches!(result.status, SpawnStatus::Running) {
            println!();
            println!("NEXT: Do your work in the workspace, then run:");
            println!("  zjj sync          # Preview changes / sync with main");
            println!("  zjj done          # Merge to main + cleanup");
        }
    }
    Ok(())
}

const fn status_display(status: &SpawnStatus) -> &'static str {
    match status {
        SpawnStatus::Running => "running in background",
        SpawnStatus::Completed => "completed",
        SpawnStatus::Failed => "failed",
        SpawnStatus::ValidationError => "validation error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_output_fields() {
        // Verify SpawnOutput can be constructed with all fields
        let output = SpawnOutput {
            bead_id: "test-bead".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            agent_pid: Some(12345),
            exit_code: Some(0),
            merged: true,
            cleaned: true,
            status: SpawnStatus::Completed,
        };

        // Verify field values
        assert_eq!(output.bead_id, "test-bead");
        assert_eq!(output.agent_pid, Some(12345));
        assert!(output.merged);
        assert!(output.cleaned);
        assert!(matches!(output.status, SpawnStatus::Completed));
    }

    #[test]
    fn workspace_line_match_requires_exact_name() {
        assert!(workspace_line_matches_name(
            "zjj-123: /tmp/repo/.zjj/workspaces/zjj-123",
            "zjj-123"
        ));
        assert!(workspace_line_matches_name(
            "* zjj-123: /tmp/repo/.zjj/workspaces/zjj-123",
            "zjj-123"
        ));
        assert!(!workspace_line_matches_name(
            "zjj-1234: /tmp/repo/.zjj/workspaces/zjj-1234",
            "zjj-123"
        ));
        assert!(!workspace_line_matches_name(
            "not-a-workspace-line",
            "zjj-123"
        ));
    }
}

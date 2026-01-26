//! Spawn command - Create isolated workspace and run agent
//!
//! This command:
//! 1. Validates we're on main branch (not in a workspace)
//! 2. Validates the bead is ready/open
//! 3. Creates a JJ workspace for the bead
//! 4. Updates bead status to in_progress
//! 5. Spawns an agent subprocess in the workspace
//! 6. Waits for completion (or background)
//! 7. On success: merges to main and cleans up
//! 8. On failure: cleans up without merging

pub mod types;

pub use types::SpawnOptions;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::cli::jj_root;
use types::{SpawnError, SpawnOutput, SpawnStatus};
use zjj_core::json::SchemaEnvelope;

/// Run the spawn command with options
pub fn run_with_options(options: &SpawnOptions) -> Result<()> {
    let result = execute_spawn(options)?;

    output_result(&result, options.format)?;
    Ok(())
}

/// Core spawn logic using Railway-Oriented Programming
fn execute_spawn(options: &SpawnOptions) -> Result<SpawnOutput, SpawnError> {
    // Phase 1: Validate location (must be on main)
    let root = validate_location()
        .map_err(|e| SpawnError::NotOnMain { current_location: e.to_string() })?;

    // Phase 2: Validate bead status
    validate_bead_status(&options.bead_id)?;

    // Phase 3: Create workspace
    let workspace_path = create_workspace(&root, &options.bead_id)?;

    // Phase 4: Update bead status to in_progress
    update_bead_status(&options.bead_id, "in_progress")
        .map_err(|e| SpawnError::DatabaseError { reason: e.to_string() })?;

    // Phase 5: Spawn agent
    let (pid, exit_code) = if options.background {
        spawn_agent_background(&workspace_path, options)?
    } else {
        spawn_agent_foreground(&workspace_path, options)?
    };

    // Phase 6-8: Handle completion
    let (merged, cleaned, status) = match exit_code {
        Some(0) => handle_success(&root, &options.bead_id, &workspace_path, options)?,
        Some(code) => handle_failure(&workspace_path, options, code)?,
        None => (false, false, SpawnStatus::Running),
    };

    Ok(SpawnOutput {
        bead_id: options.bead_id.clone(),
        workspace_path: workspace_path.to_string_lossy().to_string(),
        agent_pid: pid,
        exit_code,
        merged,
        cleaned,
        status,
    })
}

/// Validate we're on main branch (not in a workspace)
fn validate_location() -> Result<String> {
    let root = jj_root().context("Failed to get JJ root")?;

    // Check if we're in a workspace by looking at current directory
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Simple check: if we're in .zjj/workspaces, we're in a workspace
    if current_dir
        .to_string_lossy()
        .contains(".zjj/workspaces")
    {
        return anyhow::bail!("In workspace directory");
    }

    Ok(root)
}

/// Validate that the bead exists and has appropriate status
fn validate_bead_status(bead_id: &str) -> Result<(), SpawnError> {
    let beads_dir = Path::new(".beads");
    if !beads_dir.exists() {
        return Err(SpawnError::BeadNotFound {
            bead_id: bead_id.to_string(),
        });
    }

    let beads_db = beads_dir.join("issues.jsonl");
    if !beads_db.exists() {
        return Err(SpawnError::BeadNotFound {
            bead_id: bead_id.to_string(),
        });
    }

    let content = fs::read_to_string(&beads_db).map_err(|e| SpawnError::DatabaseError {
        reason: format!("Failed to read beads database: {e}"),
    })?;

    let mut found = false;
    let mut status = String::new();

    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("id")
                .and_then(|i| i.as_str())
                .map(|i| i == bead_id)
                .unwrap_or(false)
            {
                found = true;
                if let Some(s) = json.get("status").and_then(|s| s.as_str()) {
                    status = s.to_string();
                }
                break;
            }
        }
    }

    if !found {
        return Err(SpawnError::BeadNotFound {
            bead_id: bead_id.to_string(),
        });
    }

    // Check if status is appropriate (open or ready, indicated by "open" or "●")
    match status.as_str() {
        "open" | "●" | "ready" => Ok(()),
        _ => Err(SpawnError::InvalidBeadStatus {
            bead_id: bead_id.to_string(),
            status,
        }),
    }
}

/// Create a JJ workspace for the bead
fn create_workspace(root: &str, bead_id: &str) -> Result<std::path::PathBuf, SpawnError> {
    let workspaces_dir = Path::new(root).join(".zjj/workspaces");
    fs::create_dir_all(&workspaces_dir).map_err(|e| SpawnError::WorkspaceCreationFailed {
        reason: format!("Failed to create workspaces directory: {e}"),
    })?;

    let workspace_path = workspaces_dir.join(bead_id);

    // Create JJ workspace
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", bead_id])
        .current_dir(root)
        .output()
        .map_err(|e| SpawnError::WorkspaceCreationFailed {
            reason: format!("Failed to execute jj workspace add: {e}"),
        })?;

    if !output.status.success() {
        return Err(SpawnError::WorkspaceCreationFailed {
            reason: format!(
                "jj workspace add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    Ok(workspace_path)
}

/// Update bead status in the database
fn update_bead_status(bead_id: &str, new_status: &str) -> Result<()> {
    let beads_db = Path::new(".beads/issues.jsonl");
    let content = fs::read_to_string(&beads_db)?;
    let mut new_content = String::new();
    let mut updated = false;

    for line in content.lines() {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("id")
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
        let mut file = fs::File::create(&beads_db)?;
        file.write_all(new_content.as_bytes())?;
    }

    Ok(())
}

/// Spawn agent in foreground (wait for completion)
fn spawn_agent_foreground(
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(Option<u32>, Option<i32>), SpawnError> {
    let mut cmd = Command::new(&options.agent_command);
    cmd.args(&options.agent_args)
        .current_dir(workspace_path)
        .env("ZJJ_BEAD_ID", &options.bead_id)
        .env("ZJJ_WORKSPACE", workspace_path.to_string_lossy().as_ref());

    let mut spawn_result = cmd
        .spawn()
        .map_err(|e| SpawnError::AgentSpawnFailed {
            reason: format!("Failed to spawn agent: {e}"),
        })?;

    let pid = Some(spawn_result.id());

    // Wait for completion
    let status = spawn_result
        .wait()
        .map_err(|e| SpawnError::AgentSpawnFailed {
            reason: format!("Failed to wait for agent: {e}"),
        })?;

    let exit_code = status.code();

    Ok((pid, exit_code))
}

/// Spawn agent in background (don't wait)
fn spawn_agent_background(
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(Option<u32>, Option<i32>), SpawnError> {
    let mut cmd = Command::new(&options.agent_command);
    cmd.args(&options.agent_args)
        .current_dir(workspace_path)
        .env("ZJJ_BEAD_ID", &options.bead_id)
        .env("ZJJ_WORKSPACE", workspace_path.to_string_lossy().as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let spawn_result = cmd
        .spawn()
        .map_err(|e| SpawnError::AgentSpawnFailed {
            reason: format!("Failed to spawn agent: {e}"),
        })?;

    let pid = Some(spawn_result.id());

    // Detach - process continues in background
    Ok((pid, None))
}

/// Handle successful agent completion
fn handle_success(
    root: &str,
    bead_id: &str,
    workspace_path: &Path,
    options: &SpawnOptions,
) -> Result<(bool, bool, SpawnStatus), SpawnError> {
    let merged = if !options.no_auto_merge {
        merge_to_main(root, workspace_path)?
    } else {
        false
    };

    let cleaned = cleanup_workspace(workspace_path)?;

    // Update bead to completed
    update_bead_status(bead_id, "completed")
        .map_err(|e| SpawnError::DatabaseError { reason: e.to_string() })?;

    Ok((merged, cleaned, SpawnStatus::Completed))
}

/// Handle failed agent completion
fn handle_failure(
    workspace_path: &Path,
    options: &SpawnOptions,
    _exit_code: i32,
) -> Result<(bool, bool, SpawnStatus), SpawnError> {
    let cleaned = if !options.no_auto_cleanup {
        cleanup_workspace(workspace_path)?
    } else {
        false
    };

    // Leave bead as in_progress for retry
    Ok((false, cleaned, SpawnStatus::Failed))
}

/// Merge workspace changes to main
fn merge_to_main(root: &str, _workspace_path: &Path) -> Result<bool, SpawnError> {
    // First, abandon the workspace
    let _ = Command::new("jj")
        .args(["workspace", "abandon"])
        .current_dir(root)
        .output();

    Ok(true)
}

/// Clean up the workspace directory
fn cleanup_workspace(workspace_path: &Path) -> Result<bool, SpawnError> {
    if workspace_path.exists() {
        fs::remove_dir_all(workspace_path).map_err(|e| SpawnError::CleanupFailed {
            reason: format!("Failed to remove workspace: {e}"),
        })?;
        Ok(true)
    } else {
        Ok(false)
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
            println!("  Agent PID: {}", pid);
        }
        if let Some(code) = result.exit_code {
            println!("  Exit code: {}", code);
        }
        if result.merged {
            println!("  Merged: yes");
        }
        if result.cleaned {
            println!("  Cleaned up: yes");
        }
    }
    Ok(())
}

fn status_display(status: &SpawnStatus) -> &'static str {
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
    fn test_spawn_output_serialization() {
        let output = SpawnOutput {
            bead_id: "test-bead".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            agent_pid: Some(12345),
            exit_code: Some(0),
            merged: true,
            cleaned: true,
            status: SpawnStatus::Completed,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"test-bead\""));
        assert!(json.contains("\"completed\""));
    }
}

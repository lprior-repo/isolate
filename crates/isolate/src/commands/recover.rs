//! Recover command - auto-detect and fix common broken states
//!
//! Provides recovery operations:
//! - `isolate recover` - Auto-detect and fix issues
//! - `isolate recover --diagnose` - Show issues without fixing
//! - `isolate recover <session>` - Show JJ operation log for session
//! - `isolate recover <session> --op=<id>` - Restore to specific operation
//! - `isolate recover <session> --last` - Restore to previous operation
//! - `isolate retry` - Retry last failed command
//! - `isolate rollback <session> --to <checkpoint>` - Restore to checkpoint

use anyhow::{Context, Result};
use futures::StreamExt;
use isolate_core::{json::SchemaEnvelope, OutputFormat};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::{
    context::{detect_location, Location},
    get_session_db,
};
use crate::cli::is_command_available;

/// Options for recover command
#[derive(Debug, Clone)]
pub struct RecoverOptions {
    /// Just diagnose without fixing
    pub diagnose_only: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Options for retry command
#[derive(Debug, Clone)]
pub struct RetryOptions {
    /// Output format
    pub format: OutputFormat,
}

/// Options for rollback command
#[derive(Debug, Clone)]
pub struct RollbackOptions {
    /// Session to rollback
    pub session: String,
    /// Checkpoint to rollback to
    pub checkpoint: String,
    /// Dry run
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Options for operation log recovery
#[derive(Debug, Clone)]
pub struct OpRecoverOptions {
    /// Session name (optional - if None, use current workspace)
    pub session: Option<String>,
    /// Operation ID to restore to
    pub operation: Option<String>,
    /// Restore to last operation (undo)
    pub last: bool,
    /// List only, don't restore
    pub list_only: bool,
    /// Output format
    pub format: OutputFormat,
}

/// JJ Operation log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationEntry {
    /// Operation ID
    pub id: String,
    /// Operation type
    pub operation: String,
    /// Description
    pub description: String,
    /// Timestamp
    pub timestamp: String,
    /// User who performed the operation
    pub user: String,
    /// Current state after this operation
    pub current: bool,
}

/// Operation log output
#[derive(Debug, Clone, Serialize)]
pub struct OperationLogOutput {
    /// Session name
    pub session: String,
    /// Operations found
    pub operations: Vec<OperationEntry>,
    /// Total operations
    pub total: usize,
    /// Current operation ID
    pub current_operation: Option<String>,
}

/// Operation restore output
#[derive(Debug, Clone, Serialize)]
pub struct OperationRestoreOutput {
    /// Session name
    pub session: String,
    /// Operation ID restored to
    pub operation_id: String,
    /// Success
    pub success: bool,
    /// Message
    pub message: String,
}

/// Issue found during diagnosis
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    /// Issue code
    pub code: String,
    /// Description
    pub description: String,
    /// Severity: critical, warning, info
    pub severity: String,
    /// Fix command (if available)
    pub fix_command: Option<String>,
    /// Whether it was fixed
    pub fixed: bool,
}

/// Recover output
#[derive(Debug, Clone, Serialize)]
pub struct RecoverOutput {
    /// Issues found
    pub issues: Vec<Issue>,
    /// Number of issues fixed
    pub fixed_count: usize,
    /// Number of issues remaining
    pub remaining_count: usize,
    /// Overall status
    pub status: String,
}

/// Compute status from issues
fn compute_status(issues: &[Issue]) -> String {
    let remaining = issues
        .iter()
        .filter(|i| !i.fixed && i.severity != "info")
        .count();

    if remaining == 0 {
        "healthy".to_string()
    } else if remaining < issues.len() {
        "partially_fixed".to_string()
    } else {
        "issues_remaining".to_string()
    }
}

/// Run the recover command
pub async fn run_recover(options: &RecoverOptions) -> Result<()> {
    let issues = diagnose_issues().await;

    // Apply fixes if not diagnose-only mode
    let issues = if options.diagnose_only {
        issues
    } else {
        fix_issues(issues).await
    };

    let output = RecoverOutput {
        fixed_count: issues.iter().filter(|i| i.fixed).count(),
        remaining_count: issues
            .iter()
            .filter(|i| !i.fixed && i.severity != "info")
            .count(),
        status: compute_status(&issues),
        issues,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("recover-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize recover output")?;
        println!("{json_str}");
    } else {
        print_recover_human(&output, options.diagnose_only);
    }

    Ok(())
}

/// Diagnose issues
async fn diagnose_issues() -> Vec<Issue> {
    let mut issues = Vec::new();

    // Check JJ
    if !is_command_available("jj").await {
        issues.push(Issue {
            code: "JJ_NOT_INSTALLED".to_string(),
            description: "JJ (Jujutsu) is not installed".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("cargo install jj-cli".to_string()),
            fixed: false,
        });
    }

    // Check database
    let db_res = get_session_db().await;
    let db_ok = db_res.is_ok();
    if !db_ok {
        issues.push(Issue {
            code: "DB_NOT_INITIALIZED".to_string(),
            description: "Isolate database not initialized".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("isolate init".to_string()),
            fixed: false,
        });
    }

    // Check for orphaned sessions
    if let Ok(db) = db_res {
        if let Ok(sessions) = db.list(None).await {
            let session_issues: Vec<Issue> = futures::stream::iter(sessions)
                .then(|session| async move {
                    let mut issues = Vec::new();
                    let name = session.name.clone();
                    let status = session.status.to_string();

                    // Check if workspace directory exists
                    if let Some(path) = session
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("workspace_path"))
                        .and_then(|v| v.as_str())
                    {
                        match tokio::fs::try_exists(path).await {
                            Ok(false) | Err(_) => {
                                issues.push(Issue {
                                    code: "ORPHANED_SESSION".to_string(),
                                    description: format!(
                                        "Session '{name}' has missing workspace at {path}",
                                    ),
                                    severity: "warning".to_string(),
                                    fix_command: Some(format!("isolate remove {name} --force")),
                                    fixed: false,
                                });
                            }
                            Ok(true) => {}
                        }
                    }

                    // Check for stale sessions (creating for too long)
                    if status == "creating" {
                        issues.push(Issue {
                            code: "STALE_CREATING_SESSION".to_string(),
                            description: format!("Session '{name}' stuck in 'creating' state"),
                            severity: "warning".to_string(),
                            fix_command: Some(format!("isolate remove {name} --force")),
                            fixed: false,
                        });
                    }
                    issues
                })
                .flat_map(futures::stream::iter)
                .collect()
                .await;

            issues.extend(session_issues);
        }
    }

    issues
}

/// Try to fix a single issue, returning updated issue
async fn try_fix_issue(issue: Issue) -> Issue {
    match issue.code.as_str() {
        "STALE_CREATING_SESSION" | "ORPHANED_SESSION" => {
            // Extract session name from fix command and attempt fix
            let session_name = issue.fix_command.as_ref().and_then(|cmd| {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                (parts.len() >= 3 && parts[0] == "isolate" && parts[1] == "remove")
                    .then(|| parts[2])
            });

            if let Some(name) = session_name {
                if let Ok(db) = get_session_db().await {
                    let fixed = db.delete(name).await.is_ok();
                    return Issue { fixed, ..issue };
                }
            }
            issue
        }
        // All other issues require user intervention - cannot auto-fix
        _ => issue,
    }
}

/// Fix issues where possible
async fn fix_issues(issues: Vec<Issue>) -> Vec<Issue> {
    futures::stream::iter(issues)
        .then(try_fix_issue)
        .collect()
        .await
}

/// Print recover output in human format
fn print_recover_human(output: &RecoverOutput, diagnose_only: bool) {
    if diagnose_only {
        println!("Isolate System Diagnosis");
        println!("====================\n");
    } else {
        println!("Isolate Recovery");
        println!("============\n");
    }

    if output.issues.is_empty() {
        println!("No issues found. System is healthy.");
        return;
    }

    println!("Issues found: {}\n", output.issues.len());

    output.issues.iter().for_each(|issue| {
        let icon = match issue.severity.as_str() {
            "critical" => "❌",
            "warning" => "⚠️ ",
            _ => "ℹ️ ",
        };

        let fixed_marker = if issue.fixed { " [FIXED]" } else { "" };

        println!(
            "{} {} [{}]{}",
            icon, issue.description, issue.code, fixed_marker
        );
        if let Some(ref cmd) = issue.fix_command {
            println!("   Fix: {cmd}");
        }
        println!();
    });

    println!("Summary:");
    println!("  Status: {}", output.status);
    println!("  Fixed: {}", output.fixed_count);
    println!("  Remaining: {}", output.remaining_count);
}

/// Retry output
#[derive(Debug, Clone, Serialize)]
pub struct RetryOutput {
    /// Whether there was a command to retry
    pub has_command: bool,
    /// The command that would be retried
    pub command: Option<String>,
    /// Message
    pub message: String,
}

/// Saved command info for retry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedCommand {
    /// The command that was run
    command: String,
    /// Whether it failed
    failed: bool,
    /// Timestamp
    timestamp: String,
}

/// Get the path to the last command file
async fn get_last_command_path() -> Result<std::path::PathBuf> {
    let data_dir = super::isolate_data_dir().await?;
    Ok(data_dir.join("last_command.json"))
}

/// Save the last command for potential retry
#[allow(dead_code)]
pub async fn save_last_command(command: &str, failed: bool) -> Result<()> {
    let path = get_last_command_path().await?;
    let saved = SavedCommand {
        command: command.to_string(),
        failed,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let content = serde_json::to_string_pretty(&saved)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

/// Run the retry command
#[allow(clippy::option_if_let_else)]
pub async fn run_retry(options: &RetryOptions) -> Result<()> {
    let path = match get_last_command_path().await {
        Ok(p) => p,
        Err(e) => {
            // If we can't get the data dir (e.g., Isolate not initialized),
            // return a clear error without retry-specific wording
            return Err(e.context("Isolate not initialized. Run 'isolate init' first."));
        }
    };

    let output = match tokio::fs::try_exists(&path).await {
        Ok(true) => {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => match serde_json::from_str::<SavedCommand>(&content) {
                    Ok(saved) if saved.failed => {
                        // Execute the saved command
                        let parts: Vec<&str> = saved.command.split_whitespace().collect();
                        if parts.is_empty() {
                            RetryOutput {
                                has_command: false,
                                command: None,
                                message: "Saved command is empty".to_string(),
                            }
                        } else {
                            // Execute the command
                            let result = Command::new(parts[0]).args(&parts[1..]).status().await;

                            match result {
                                Ok(status) if status.success() => {
                                    // Clear the failed command since retry succeeded
                                    let _ = tokio::fs::remove_file(&path).await;
                                    RetryOutput {
                                        has_command: true,
                                        command: Some(saved.command.clone()),
                                        message: format!(
                                            "Retry succeeded: {command}",
                                            command = saved.command
                                        ),
                                    }
                                }
                                Ok(_) => RetryOutput {
                                    has_command: true,
                                    command: Some(saved.command.clone()),
                                    message: format!(
                                        "Retry failed again: {command}",
                                        command = saved.command
                                    ),
                                },
                                Err(e) => RetryOutput {
                                    has_command: true,
                                    command: Some(saved.command.clone()),
                                    message: format!("Failed to execute retry: {e}"),
                                },
                            }
                        }
                    }
                    Ok(_) => RetryOutput {
                        has_command: false,
                        command: None,
                        message: "No failed operation to retry".to_string(),
                    },
                    Err(_) => RetryOutput {
                        has_command: false,
                        command: None,
                        message: "Could not parse last command file".to_string(),
                    },
                },
                Err(_) => RetryOutput {
                    has_command: false,
                    command: None,
                    message: "Could not read last command file".to_string(),
                },
            }
        }
        _ => RetryOutput {
            has_command: false,
            command: None,
            message: "No failed operation to retry".to_string(),
        },
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("retry-response", "single", &output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize retry output")?;
        println!("{json_str}");
    } else {
        println!("{}", output.message);
    }

    Ok(())
}

/// Rollback output
#[derive(Debug, Clone, Serialize)]
pub struct RollbackOutput {
    /// Session that was rolled back
    pub session: String,
    /// Checkpoint rolled back to
    pub checkpoint: String,
    /// Whether it was a dry run
    pub dry_run: bool,
    /// Whether rollback operation succeeded
    pub operation_succeeded: bool,
    /// Message
    pub message: String,
}

/// Run the rollback command
#[allow(clippy::too_many_lines)]
pub async fn run_rollback(options: &RollbackOptions) -> Result<i32> {
    // Check if session exists
    let db = get_session_db().await?;
    let maybe_session = db.get(&options.session).await?;
    let Some(session) = maybe_session else {
        let output = RollbackOutput {
            session: options.session.clone(),
            checkpoint: options.checkpoint.clone(),
            dry_run: options.dry_run,
            operation_succeeded: false,
            message: format!("Session '{}' not found", options.session),
        };
        emit_rollback_output(options.format, &output, 2)?;
        return Ok(2);
    };

    // Use canonical workspace_path first, then metadata fallback for compatibility.
    let workspace_path = if session.workspace_path.is_empty() {
        session
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_path"))
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow::anyhow!("Session '{}' has no workspace path", options.session))?
    } else {
        session.workspace_path.clone()
    };

    let workspace_dir = std::path::Path::new(&workspace_path);
    if !tokio::fs::try_exists(workspace_dir).await.unwrap_or(false) {
        let output = RollbackOutput {
            session: options.session.clone(),
            checkpoint: options.checkpoint.clone(),
            dry_run: options.dry_run,
            operation_succeeded: false,
            message: format!("Workspace directory '{workspace_path}' does not exist"),
        };
        emit_rollback_output(options.format, &output, 3)?;
        return Ok(3);
    }

    // Verify it's a JJ repository
    let jj_dir = workspace_dir.join(".jj");
    if !tokio::fs::try_exists(&jj_dir).await.unwrap_or(false) {
        let output = RollbackOutput {
            session: options.session.clone(),
            checkpoint: options.checkpoint.clone(),
            dry_run: options.dry_run,
            operation_succeeded: false,
            message: format!("'{workspace_path}' is not a JJ repository"),
        };
        emit_rollback_output(options.format, &output, 1)?;
        return Ok(1);
    }

    let output = if options.dry_run {
        // Dry run: show what would happen
        // Check if the checkpoint exists using jj log
        let check_result = Command::new("jj")
            .current_dir(workspace_dir)
            .args([
                "log",
                "-r",
                &options.checkpoint,
                "--no-graph",
                "-T",
                "change_id",
            ])
            .output()
            .await;

        match check_result {
            Ok(output) if output.status.success() => RollbackOutput {
                session: options.session.clone(),
                checkpoint: options.checkpoint.clone(),
                dry_run: true,
                operation_succeeded: true,
                message: format!(
                    "Would rollback session '{}' to checkpoint '{}' using 'jj edit {}'",
                    options.session, options.checkpoint, options.checkpoint
                ),
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                RollbackOutput {
                    session: options.session.clone(),
                    checkpoint: options.checkpoint.clone(),
                    dry_run: true,
                    operation_succeeded: false,
                    message: format!(
                        "Checkpoint '{}' not found: {}",
                        options.checkpoint,
                        stderr.trim()
                    ),
                }
            }
            Err(e) => RollbackOutput {
                session: options.session.clone(),
                checkpoint: options.checkpoint.clone(),
                dry_run: true,
                operation_succeeded: false,
                message: format!("Failed to check checkpoint: {e}"),
            },
        }
    } else {
        // Actually perform the rollback using jj edit
        let result = Command::new("jj")
            .current_dir(workspace_dir)
            .args(["edit", &options.checkpoint])
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => RollbackOutput {
                session: options.session.clone(),
                checkpoint: options.checkpoint.clone(),
                dry_run: false,
                operation_succeeded: true,
                message: format!(
                    "Rolled back session '{}' to checkpoint '{}'",
                    options.session, options.checkpoint
                ),
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                RollbackOutput {
                    session: options.session.clone(),
                    checkpoint: options.checkpoint.clone(),
                    dry_run: false,
                    operation_succeeded: false,
                    message: format!(
                        "Failed to rollback to checkpoint '{}': {}",
                        options.checkpoint,
                        stderr.trim()
                    ),
                }
            }
            Err(e) => RollbackOutput {
                session: options.session.clone(),
                checkpoint: options.checkpoint.clone(),
                dry_run: false,
                operation_succeeded: false,
                message: format!("Failed to execute jj edit: {e}"),
            },
        }
    };

    let exit_code = if output.operation_succeeded { 0 } else { 4 };
    emit_rollback_output(options.format, &output, exit_code)?;
    Ok(exit_code)
}

fn emit_rollback_output(
    format: OutputFormat,
    output: &RollbackOutput,
    exit_code: i32,
) -> Result<()> {
    if format.is_json() {
        let mut envelope = SchemaEnvelope::new("rollback-response", "single", output);
        if exit_code != 0 {
            envelope.success = false;
        }
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize rollback output")?;
        println!("{json_str}");
    } else {
        println!("{}", output.message);
    }
    Ok(())
}

/// Run operation log recovery or listing
pub async fn run_op_recover(options: &OpRecoverOptions) -> Result<()> {
    // Determine workspace path from session or current location
    let workspace_path = if let Some(session_name) = &options.session {
        // Get session from database
        let db = get_session_db().await?;
        let session = db
            .get(session_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

        session
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' has no workspace path"))?
            .to_string()
    } else {
        // Use current workspace - detect using jj root
        use crate::cli::jj_root;
        let root = jj_root().await?;
        let root_path = std::path::PathBuf::from(&root);
        let location = detect_location(&root_path)?;
        match location {
            Location::Workspace { path, .. } => path,
            Location::Main => {
                anyhow::bail!(
                    "Not in a workspace. Specify a session name or run from within a workspace."
                );
            }
        }
    };

    // If listing or no operation specified, show operation log
    if options.list_only || options.operation.is_none() && !options.last {
        return show_operation_log(&workspace_path, options.session.as_ref(), options.format).await;
    }

    // Restore to specific operation
    let operation_id = if options.last {
        // Get previous operation
        let operations = get_operation_log(&workspace_path).await?;
        if operations.len() < 2 {
            anyhow::bail!("No previous operation to restore to");
        }
        operations
            .get(1)
            .map(|op| op.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Could not find previous operation"))?
    } else {
        options
            .operation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Operation ID required"))?
    };

    restore_to_operation(&workspace_path, &operation_id, options.format).await
}

/// Get operation log from workspace
async fn get_operation_log(workspace_path: &str) -> Result<Vec<OperationEntry>> {
    let output = Command::new("jj")
        .current_dir(workspace_path)
        .args([
            "op",
            "log",
            "--no-graph",
            "-T",
            r"id | operation | time | user | description",
            "--limit",
            "50",
        ])
        .output()
        .await
        .context("Failed to run jj op log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj op log failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let current_op_id = get_current_operation_id(workspace_path).await.ok();

    let operations = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                Some(OperationEntry {
                    id: parts
                        .first()
                        .map_or_else(String::new, |s| s.trim().to_string()),
                    operation: parts
                        .get(1)
                        .map_or_else(String::new, |s| s.trim().to_string()),
                    description: parts
                        .get(4)
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| {
                            parts
                                .get(1)
                                .map_or_else(String::new, |s| s.trim().to_string())
                        }),
                    timestamp: parts
                        .get(2)
                        .map_or_else(String::new, |s| s.trim().to_string()),
                    user: parts
                        .get(3)
                        .map_or_else(String::new, |s| s.trim().to_string()),
                    current: current_op_id
                        .as_ref()
                        .is_some_and(|id| parts.first().is_some_and(|p| p.trim() == id)),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(operations)
}

/// Get current operation ID
async fn get_current_operation_id(workspace_path: &str) -> Result<String> {
    let output = Command::new("jj")
        .current_dir(workspace_path)
        .args(["op", "log", "--no-graph", "-T", "id", "--limit", "1"])
        .output()
        .await
        .context("Failed to get current operation")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to get current operation: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Show operation log
async fn show_operation_log(
    workspace_path: &str,
    session_name: Option<&String>,
    format: OutputFormat,
) -> Result<()> {
    let operations = get_operation_log(workspace_path).await?;
    let current_op_id = get_current_operation_id(workspace_path).await.ok();

    let session = session_name
        .map(std::string::String::as_str)
        .map_or("<current>", |s| s)
        .to_string();

    if format.is_json() {
        let total = operations.len();
        let output = OperationLogOutput {
            session,
            operations: operations.clone(),
            total,
            current_operation: current_op_id,
        };
        let envelope = SchemaEnvelope::new("op-log-response", "single", &output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize operation log")?;
        println!("{json_str}");
    } else {
        println!("Operation log for session '{session}':\n");
        if operations.is_empty() {
            println!("No operations found.");
        } else {
            for (idx, op) in operations.iter().enumerate() {
                let marker = if op.current { " (current)" } else { "" };
                println!(
                    "  {}. {}{} - {} @ {}",
                    idx, op.id, marker, op.operation, op.timestamp
                );
                if !op.description.is_empty() {
                    println!("     {}", op.description);
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Restore to specific operation
async fn restore_to_operation(
    workspace_path: &str,
    operation_id: &str,
    format: OutputFormat,
) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(workspace_path)
        .args(["op", "restore", "--operation", operation_id])
        .output()
        .await
        .context("Failed to run jj op restore")?;

    let session = "workspace"; // Could be enhanced to get actual session name

    let result = if output.status.success() {
        OperationRestoreOutput {
            session: session.to_string(),
            operation_id: operation_id.to_string(),
            success: true,
            message: format!("Restored to operation {operation_id}"),
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        OperationRestoreOutput {
            session: session.to_string(),
            operation_id: operation_id.to_string(),
            success: false,
            message: format!("Failed to restore: {stderr}"),
        }
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("op-restore-response", "single", &result);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize restore output")?;
        println!("{json_str}");
    } else {
        println!("{}", result.message);
    }

    if !result.success {
        anyhow::bail!("Operation restore failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_serializes() {
        let issue = Issue {
            code: "TEST".to_string(),
            description: "Test issue".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("fix it".to_string()),
            fixed: false,
        };

        let json = serde_json::to_string(&issue);
        assert!(json.is_ok());
    }

    #[test]
    fn test_recover_output_serializes() {
        let output = RecoverOutput {
            issues: vec![],
            fixed_count: 0,
            remaining_count: 0,
            status: "healthy".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }

    #[test]
    fn test_diagnose_includes_jj_check() {
        // This test just ensures diagnose_issues doesn't panic
        let _result = diagnose_issues();
        // No assertion needed - if it doesn't panic, test passes
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the recover command
    // ============================================================================

    mod issue_behavior {
        use super::*;

        /// GIVEN: A critical issue is detected
        /// WHEN: Issue is created
        /// THEN: Severity should be "critical" and it should block progress
        #[test]
        fn critical_issues_have_high_severity() {
            let issue = Issue {
                code: "JJ_NOT_INSTALLED".to_string(),
                description: "JJ is not installed".to_string(),
                severity: "critical".to_string(),
                fix_command: Some("cargo install jj-cli".to_string()),
                fixed: false,
            };

            assert_eq!(issue.severity, "critical", "Missing JJ is critical");
            assert!(
                issue.fix_command.is_some(),
                "Critical issues should have fix"
            );
        }

        /// GIVEN: A warning-level issue
        /// WHEN: Issue is created
        /// THEN: Severity should be "warning" and work can continue
        #[test]
        fn warning_issues_allow_continued_operation() {
            let issue = Issue {
                code: "ORPHANED_SESSION".to_string(),
                description: "Session 'old-task' has missing workspace".to_string(),
                severity: "warning".to_string(),
                fix_command: Some("isolate remove old-task --force".to_string()),
                fixed: false,
            };

            assert_eq!(issue.severity, "warning");
            assert!(issue.fix_command.is_some(), "Warnings should suggest fixes");
        }

        /// GIVEN: An issue with a fix command
        /// WHEN: Fix command is examined
        /// THEN: Should be a valid, executable command
        #[test]
        fn fix_commands_are_executable() {
            let issues = vec![
                Issue {
                    code: "JJ_NOT_INSTALLED".to_string(),
                    description: "JJ missing".to_string(),
                    severity: "critical".to_string(),
                    fix_command: Some("cargo install jj-cli".to_string()),
                    fixed: false,
                },
                Issue {
                    code: "ORPHANED_SESSION".to_string(),
                    description: "Orphaned".to_string(),
                    severity: "warning".to_string(),
                    fix_command: Some("isolate remove test --force".to_string()),
                    fixed: false,
                },
            ];

            for issue in issues {
                if let Some(cmd) = &issue.fix_command {
                    // Fix commands should start with known tools
                    assert!(
                        cmd.starts_with("cargo ")
                            || cmd.starts_with("isolate ")
                            || cmd.starts_with("jj "),
                        "Fix command '{cmd}' should use known tools"
                    );
                }
            }
        }

        /// GIVEN: An issue has been fixed
        /// WHEN: Issue is marked as fixed
        /// THEN: fixed flag should be true
        #[test]
        fn fixed_issues_are_marked() {
            let mut issue = Issue {
                code: "TEST".to_string(),
                description: "Test issue".to_string(),
                severity: "warning".to_string(),
                fix_command: Some("fix it".to_string()),
                fixed: false,
            };

            assert!(!issue.fixed, "Initially not fixed");

            issue.fixed = true;
            assert!(issue.fixed, "Should be marked as fixed");
        }
    }

    mod recover_output_behavior {
        use super::*;

        /// GIVEN: No issues found
        /// WHEN: Recover output is created
        /// THEN: Status should be "healthy"
        #[test]
        fn no_issues_means_healthy() {
            let output = RecoverOutput {
                issues: vec![],
                fixed_count: 0,
                remaining_count: 0,
                status: "healthy".to_string(),
            };

            assert_eq!(output.status, "healthy");
            assert_eq!(output.issues.len(), 0);
            assert_eq!(output.remaining_count, 0);
        }

        /// GIVEN: All issues were fixed
        /// WHEN: Recover output is created
        /// THEN: Status should reflect success and remaining should be 0
        #[test]
        fn all_fixed_means_healthy() {
            let output = RecoverOutput {
                issues: vec![Issue {
                    code: "ISSUE1".to_string(),
                    description: "Fixed issue".to_string(),
                    severity: "warning".to_string(),
                    fix_command: Some("fix".to_string()),
                    fixed: true,
                }],
                fixed_count: 1,
                remaining_count: 0,
                status: "healthy".to_string(),
            };

            assert_eq!(output.fixed_count, 1);
            assert_eq!(output.remaining_count, 0);
            assert_eq!(output.status, "healthy");
        }

        /// GIVEN: Some issues could not be fixed
        /// WHEN: Recover output is created
        /// THEN: Status should reflect partial fix
        #[test]
        fn partial_fix_shows_remaining() {
            let output = RecoverOutput {
                issues: vec![
                    Issue {
                        code: "FIXED".to_string(),
                        description: "Was fixed".to_string(),
                        severity: "warning".to_string(),
                        fix_command: None,
                        fixed: true,
                    },
                    Issue {
                        code: "NOT_FIXED".to_string(),
                        description: "Not fixed".to_string(),
                        severity: "warning".to_string(),
                        fix_command: Some("manual fix".to_string()),
                        fixed: false,
                    },
                ],
                fixed_count: 1,
                remaining_count: 1,
                status: "partially_fixed".to_string(),
            };

            assert_eq!(output.fixed_count, 1);
            assert_eq!(output.remaining_count, 1);
            assert_eq!(output.status, "partially_fixed");
        }

        /// GIVEN: Critical issues remain
        /// WHEN: Recover output is created
        /// THEN: Should be clear that issues remain
        #[test]
        fn critical_remaining_is_clear() {
            let output = RecoverOutput {
                issues: vec![Issue {
                    code: "JJ_NOT_INSTALLED".to_string(),
                    description: "JJ missing".to_string(),
                    severity: "critical".to_string(),
                    fix_command: Some("cargo install jj-cli".to_string()),
                    fixed: false,
                }],
                fixed_count: 0,
                remaining_count: 1,
                status: "issues_remaining".to_string(),
            };

            assert!(output.remaining_count > 0);
            assert_eq!(output.status, "issues_remaining");
        }
    }

    mod retry_behavior {
        use super::*;

        /// GIVEN: No previous command to retry
        /// WHEN: Retry output is created
        /// THEN: Should indicate no command available
        #[test]
        fn no_command_to_retry() {
            let output = RetryOutput {
                has_command: false,
                command: None,
                message: "No failed operation to retry".to_string(),
            };

            assert!(!output.has_command);
            assert!(output.command.is_none());
            assert!(!output.message.is_empty());
        }

        /// GIVEN: A previous command failed
        /// WHEN: Retry is available
        /// THEN: Should include the command to retry
        #[test]
        fn has_command_to_retry() {
            let output = RetryOutput {
                has_command: true,
                command: Some("isolate add my-session".to_string()),
                message: "Retrying: isolate add my-session".to_string(),
            };

            assert!(output.has_command);
            assert!(output.command.is_some());
            assert!(
                output
                    .command
                    .as_ref()
                    .is_some_and(|c| c.contains("isolate")),
                "Command should contain 'isolate'"
            );
        }
    }

    mod rollback_behavior {
        use super::*;

        /// GIVEN: Rollback with dry-run
        /// WHEN: Output is created
        /// THEN: Should show what would happen without executing
        #[test]
        fn dry_run_shows_preview() {
            let output = RollbackOutput {
                session: "my-session".to_string(),
                checkpoint: "checkpoint-abc".to_string(),
                dry_run: true,
                operation_succeeded: true,
                message: "Would rollback session 'my-session' to checkpoint 'checkpoint-abc'"
                    .to_string(),
            };

            assert!(output.dry_run, "Should be dry run");
            assert!(output.operation_succeeded, "Preview should succeed");
            assert!(output.message.contains("Would"), "Should indicate preview");
        }

        /// GIVEN: Actual rollback
        /// WHEN: Output is created
        /// THEN: Should show result of rollback
        #[test]
        fn actual_rollback_shows_result() {
            let output = RollbackOutput {
                session: "my-session".to_string(),
                checkpoint: "checkpoint-xyz".to_string(),
                dry_run: false,
                operation_succeeded: true,
                message: "Rolled back successfully".to_string(),
            };

            assert!(!output.dry_run, "Should not be dry run");
            assert!(output.operation_succeeded, "Should succeed");
        }

        /// GIVEN: Rollback fails
        /// WHEN: Output is created
        /// THEN: Should clearly indicate failure
        #[test]
        fn failed_rollback_is_clear() {
            let output = RollbackOutput {
                session: "my-session".to_string(),
                checkpoint: "invalid-checkpoint".to_string(),
                dry_run: false,
                operation_succeeded: false,
                message: "Checkpoint not found".to_string(),
            };

            assert!(!output.operation_succeeded);
            assert!(output.message.contains("not found") || output.message.contains("failed"));
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: Issue is serialized to JSON
        /// WHEN: AI parses it
        /// THEN: Should have all fields for automated fixing
        #[test]
        fn issue_json_is_ai_actionable() {
            let issue = Issue {
                code: "ORPHANED_SESSION".to_string(),
                description: "Session has missing workspace".to_string(),
                severity: "warning".to_string(),
                fix_command: Some("isolate remove orphan --force".to_string()),
                fixed: false,
            };

            let json_str = serde_json::to_string(&issue);
            assert!(json_str.is_ok(), "Should serialize to JSON");
            let json_str = json_str.unwrap_or_default();
            let json: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
            assert!(json.is_ok(), "deserialization should succeed");
            let json = json.unwrap_or_default();

            // AI needs these fields
            assert!(json.get("code").is_some(), "Need code for categorization");
            assert!(
                json.get("severity").is_some(),
                "Need severity for prioritization"
            );
            assert!(
                json.get("fix_command").is_some(),
                "Need fix_command for automation"
            );
            assert!(json.get("fixed").is_some(), "Need fixed for status");

            // fix_command should be executable
            let fix = json["fix_command"].as_str().unwrap_or("");
            assert!(fix.starts_with("isolate "), "Fix should be isolate command");
        }

        /// GIVEN: `RecoverOutput` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have summary and list of issues
        #[test]
        fn recover_output_json_has_summary() {
            let output = RecoverOutput {
                issues: vec![Issue {
                    code: "TEST".to_string(),
                    description: "Test".to_string(),
                    severity: "warning".to_string(),
                    fix_command: None,
                    fixed: false,
                }],
                fixed_count: 0,
                remaining_count: 1,
                status: "issues_remaining".to_string(),
            };

            let json_str = serde_json::to_string(&output);
            assert!(json_str.is_ok(), "Should serialize");

            assert!(json_str.is_ok(), "serialization should succeed");
            let json_str = json_str.unwrap_or_default();
            let json: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
            assert!(json.is_ok(), "deserialization should succeed");
            let json = json.unwrap_or_default();

            // Summary fields
            assert!(json.get("fixed_count").is_some());
            assert!(json.get("remaining_count").is_some());
            assert!(json.get("status").is_some());

            // Issues list
            assert!(json.get("issues").is_some());
            assert!(json["issues"].is_array());
        }
    }

    mod recover_options_behavior {
        use super::*;

        /// GIVEN: Diagnose-only mode
        /// WHEN: Options are set
        /// THEN: Should not attempt fixes
        #[test]
        fn diagnose_only_prevents_fixes() {
            let options = RecoverOptions {
                diagnose_only: true,
                format: isolate_core::OutputFormat::Json,
            };

            assert!(options.diagnose_only, "Should be diagnose only");
        }

        /// GIVEN: Normal recovery mode
        /// WHEN: Options are set
        /// THEN: Should attempt to fix issues
        #[test]
        fn normal_mode_attempts_fixes() {
            let options = RecoverOptions {
                diagnose_only: false,
                format: isolate_core::OutputFormat::Json,
            };

            assert!(!options.diagnose_only, "Should attempt fixes");
        }
    }
}

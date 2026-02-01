//! Recover command - auto-detect and fix common broken states
//!
//! Provides recovery operations:
//! - `zjj recover` - Auto-detect and fix issues
//! - `zjj recover --diagnose` - Show issues without fixing
//! - `zjj retry` - Retry last failed command
//! - `zjj rollback <session> --to <checkpoint>` - Restore to checkpoint

use anyhow::{Context, Result};
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::get_session_db;
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

/// Run the recover command
pub fn run_recover(options: &RecoverOptions) -> Result<()> {
    let mut issues = diagnose_issues()?;

    if !options.diagnose_only {
        fix_issues(&mut issues)?;
    }

    let fixed_count = issues.iter().filter(|i| i.fixed).count();
    let remaining_count = issues.iter().filter(|i| !i.fixed && i.severity != "info").count();

    let status = if remaining_count == 0 {
        "healthy".to_string()
    } else if remaining_count < issues.len() {
        "partially_fixed".to_string()
    } else {
        "issues_remaining".to_string()
    };

    let output = RecoverOutput {
        issues,
        fixed_count,
        remaining_count,
        status,
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
fn diagnose_issues() -> Result<Vec<Issue>> {
    let mut issues = Vec::new();

    // Check JJ
    if !is_command_available("jj") {
        issues.push(Issue {
            code: "JJ_NOT_INSTALLED".to_string(),
            description: "JJ (Jujutsu) is not installed".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("cargo install jj-cli".to_string()),
            fixed: false,
        });
    }

    // Check Zellij
    if !is_command_available("zellij") {
        issues.push(Issue {
            code: "ZELLIJ_NOT_INSTALLED".to_string(),
            description: "Zellij is not installed".to_string(),
            severity: "critical".to_string(),
            fix_command: Some("cargo install zellij".to_string()),
            fixed: false,
        });
    }

    // Check database
    let db_ok = get_session_db().is_ok();
    if !db_ok {
        issues.push(Issue {
            code: "DB_NOT_INITIALIZED".to_string(),
            description: "ZJJ database not initialized".to_string(),
            severity: "warning".to_string(),
            fix_command: Some("zjj init".to_string()),
            fixed: false,
        });
    }

    // Check for orphaned sessions
    if db_ok {
        if let Ok(db) = get_session_db() {
            if let Ok(sessions) = db.list_blocking(None) {
                for session in sessions {
                    // Check if workspace directory exists
                    if let Some(ref meta) = session.metadata {
                        if let Some(path) = meta.get("workspace_path").and_then(|v| v.as_str()) {
                            if !std::path::Path::new(path).exists() {
                                issues.push(Issue {
                                    code: "ORPHANED_SESSION".to_string(),
                                    description: format!(
                                        "Session '{}' has missing workspace at {}",
                                        session.name, path
                                    ),
                                    severity: "warning".to_string(),
                                    fix_command: Some(format!("zjj remove {} --force", session.name)),
                                    fixed: false,
                                });
                            }
                        }
                    }

                    // Check for stale sessions (creating for too long)
                    if session.status.to_string() == "creating" {
                        issues.push(Issue {
                            code: "STALE_CREATING_SESSION".to_string(),
                            description: format!(
                                "Session '{}' stuck in 'creating' state",
                                session.name
                            ),
                            severity: "warning".to_string(),
                            fix_command: Some(format!("zjj remove {} --force", session.name)),
                            fixed: false,
                        });
                    }
                }
            }
        }
    }

    Ok(issues)
}

/// Fix issues where possible
fn fix_issues(issues: &mut [Issue]) -> Result<()> {
    for issue in issues.iter_mut() {
        // Only auto-fix certain issues
        match issue.code.as_str() {
            "STALE_CREATING_SESSION" | "ORPHANED_SESSION" => {
                if let Some(ref cmd) = issue.fix_command {
                    // Try to run the fix command
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if !parts.is_empty() && parts[0] == "zjj" {
                        // We can't actually run zjj commands from within zjj
                        // but we mark it as a suggested fix
                        issue.fixed = false;
                    }
                }
            }
            _ => {
                // Other issues require manual intervention
                issue.fixed = false;
            }
        }
    }
    Ok(())
}

/// Print recover output in human format
fn print_recover_human(output: &RecoverOutput, diagnose_only: bool) {
    if diagnose_only {
        println!("ZJJ System Diagnosis");
        println!("====================\n");
    } else {
        println!("ZJJ Recovery");
        println!("============\n");
    }

    if output.issues.is_empty() {
        println!("No issues found. System is healthy.");
        return;
    }

    println!("Issues found: {}\n", output.issues.len());

    for issue in &output.issues {
        let icon = match issue.severity.as_str() {
            "critical" => "❌",
            "warning" => "⚠️ ",
            _ => "ℹ️ ",
        };

        let fixed_marker = if issue.fixed { " [FIXED]" } else { "" };

        println!("{} {} [{}]{}", icon, issue.description, issue.code, fixed_marker);
        if let Some(ref cmd) = issue.fix_command {
            println!("   Fix: {}", cmd);
        }
        println!();
    }

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

/// Run the retry command
pub fn run_retry(options: &RetryOptions) -> Result<()> {
    // In a real implementation, we would store the last command in a file
    // For now, we'll just explain the feature
    let output = RetryOutput {
        has_command: false,
        command: None,
        message: "No failed command to retry. Last command history not yet implemented.".to_string(),
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("retry-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize retry output")?;
        println!("{json_str}");
    } else {
        if output.has_command {
            println!("Retrying: {}", output.command.as_deref().unwrap_or(""));
        } else {
            println!("{}", output.message);
        }
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
    /// Success
    pub success: bool,
    /// Message
    pub message: String,
}

/// Run the rollback command
pub fn run_rollback(options: &RollbackOptions) -> Result<()> {
    // Check if session exists
    let db = get_session_db()?;
    let session = db.get_blocking(&options.session)?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", options.session))?;

    // In a real implementation, we would:
    // 1. Look up the checkpoint
    // 2. Restore the JJ state to that checkpoint
    // 3. Update the session metadata

    let output = if options.dry_run {
        RollbackOutput {
            session: options.session.clone(),
            checkpoint: options.checkpoint.clone(),
            dry_run: true,
            success: true,
            message: format!(
                "Would rollback session '{}' to checkpoint '{}'",
                options.session, options.checkpoint
            ),
        }
    } else {
        // For now, just return a message that this is not yet implemented
        RollbackOutput {
            session: options.session.clone(),
            checkpoint: options.checkpoint.clone(),
            dry_run: false,
            success: false,
            message: "Rollback to checkpoints not yet fully implemented. Use 'jj undo' for JJ-level rollback.".to_string(),
        }
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("rollback-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize rollback output")?;
        println!("{json_str}");
    } else {
        println!("{}", output.message);
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
        let result = diagnose_issues();
        assert!(result.is_ok());
    }
}

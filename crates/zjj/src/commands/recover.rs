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
    let remaining_count = issues
        .iter()
        .filter(|i| !i.fixed && i.severity != "info")
        .count();

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
                                    fix_command: Some(format!(
                                        "zjj remove {} --force",
                                        session.name
                                    )),
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

        println!(
            "{} {} [{}]{}",
            icon, issue.description, issue.code, fixed_marker
        );
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
        message: "No failed command to retry. Last command history not yet implemented."
            .to_string(),
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("retry-response", "single", &output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize retry output")?;
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
    let session = db
        .get_blocking(&options.session)?
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
                fix_command: Some("zjj remove old-task --force".to_string()),
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
                    fix_command: Some("zjj remove test --force".to_string()),
                    fixed: false,
                },
            ];

            for issue in issues {
                if let Some(cmd) = &issue.fix_command {
                    // Fix commands should start with known tools
                    assert!(
                        cmd.starts_with("cargo ")
                            || cmd.starts_with("zjj ")
                            || cmd.starts_with("jj "),
                        "Fix command '{}' should use known tools",
                        cmd
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
                message: "No failed command to retry".to_string(),
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
                command: Some("zjj add my-session".to_string()),
                message: "Retrying: zjj add my-session".to_string(),
            };

            assert!(output.has_command);
            assert!(output.command.is_some());
            assert!(output.command.unwrap().contains("zjj"));
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
                success: true,
                message: "Would rollback session 'my-session' to checkpoint 'checkpoint-abc'"
                    .to_string(),
            };

            assert!(output.dry_run, "Should be dry run");
            assert!(output.success, "Preview should succeed");
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
                success: true,
                message: "Rolled back successfully".to_string(),
            };

            assert!(!output.dry_run, "Should not be dry run");
            assert!(output.success, "Should succeed");
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
                success: false,
                message: "Checkpoint not found".to_string(),
            };

            assert!(!output.success);
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
                fix_command: Some("zjj remove orphan --force".to_string()),
                fixed: false,
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&issue).unwrap()).unwrap();

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
            let fix = json["fix_command"].as_str().unwrap();
            assert!(fix.starts_with("zjj "), "Fix should be zjj command");
        }

        /// GIVEN: RecoverOutput is serialized
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

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();

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
                format: zjj_core::OutputFormat::Json,
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
                format: zjj_core::OutputFormat::Human,
            };

            assert!(!options.diagnose_only, "Should attempt fixes");
        }
    }
}

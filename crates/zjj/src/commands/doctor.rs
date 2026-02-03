//! Doctor command - system health checks and auto-fix
//!
//! This command checks the health of the zjj system and can
//! automatically fix common issues.
//!
//! # Exit Codes
//!
//! The doctor command follows standard Unix conventions for exit codes:
//!
//! - **Exit 0**: System is healthy (all checks passed), or all critical issues were successfully
//!   fixed
//! - **Exit 1**: System has errors (one or more checks failed), or critical issues remain after
//!   auto-fix
//!
//! Warnings (`CheckStatus::Warn`) do not cause non-zero exit codes - only failures
//! (`CheckStatus::Fail`) do.

use std::{path::Path, process::Command};

use anyhow::Result;
use chrono::{Duration, Utc};
use zjj_core::{
    introspection::{CheckStatus, DoctorCheck, DoctorFixOutput, FixResult, UnfixableIssue},
    json::SchemaEnvelope,
    OutputFormat,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo, jj_root},
    commands::get_session_db,
    session::SessionStatus,
};

/// Doctor command JSON output (matches documented schema)
#[derive(Debug, Clone, serde::Serialize)]
struct DoctorJsonResponse {
    checks: Vec<DoctorCheck>,
    summary: DoctorSummary,
}

/// Summary of health check results
#[derive(Debug, Clone, serde::Serialize)]
struct DoctorSummary {
    passed: usize,
    warnings: usize,
    failed: usize,
}

fn check_for_recent_recovery() -> Option<String> {
    let log_path = Path::new(".zjj/recovery.log");

    if !log_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(log_path).ok()?;

    // Get last 5 lines to check for recent recovery
    let recent_lines: Vec<&str> = content.lines().rev().take(5).collect();

    if recent_lines.is_empty() {
        return None;
    }

    // Check timestamp of most recent entry
    if let Some(last_line) = recent_lines.first() {
        if let Some(timestamp_str) = last_line.split(']').next() {
            let timestamp = timestamp_str.trim_start_matches('[');
            // Parse timestamp and check if recent (within last 5 minutes)
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(dt);
                if duration.num_minutes() < 5 {
                    // Find message part (everything after '] ')
                    let message = last_line.split(']').nth(1).unwrap_or("");
                    return Some(format!("Recent recovery detected: {message}"));
                }
            }
        }
    }

    None
}

/// Run health checks
pub fn run(format: OutputFormat, fix: bool) -> Result<()> {
    let checks = run_all_checks();

    if fix {
        run_fixes(&checks, format)
    } else {
        show_health_report(&checks, format)
    }
}

/// Run all health checks
fn run_all_checks() -> Vec<DoctorCheck> {
    vec![
        check_jj_installed(),
        check_zellij_installed(),
        check_zellij_running(),
        check_jj_repo(),
        check_workspace_context(),
        check_initialized(),
        check_state_db(),
        check_orphaned_workspaces(),
        check_stale_sessions(),
        check_beads(),
        check_workflow_violations(),
    ]
}

/// Check if JJ is installed
fn check_jj_installed() -> DoctorCheck {
    let installed = is_command_available("jj");

    DoctorCheck {
        name: "JJ Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "JJ is installed".to_string()
        } else {
            "JJ is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install JJ: https://github.com/martinvonz/jj#installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if Zellij is installed
fn check_zellij_installed() -> DoctorCheck {
    let installed = is_command_available("zellij");

    DoctorCheck {
        name: "Zellij Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "Zellij is installed".to_string()
        } else {
            "Zellij is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install Zellij: https://zellij.dev/documentation/installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if Zellij is running
fn check_zellij_running() -> DoctorCheck {
    let running = is_inside_zellij();

    DoctorCheck {
        name: "Zellij Running".to_string(),
        status: if running {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if running {
            "Inside Zellij session".to_string()
        } else {
            "Not running inside Zellij".to_string()
        },
        suggestion: if running {
            None
        } else {
            Some("Start Zellij: zellij".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if current directory is a JJ repository
fn check_jj_repo() -> DoctorCheck {
    let is_repo = is_jj_repo().unwrap_or(false);

    DoctorCheck {
        name: "JJ Repository".to_string(),
        status: if is_repo {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if is_repo {
            "Current directory is a JJ repository".to_string()
        } else {
            "Current directory is not a JJ repository".to_string()
        },
        suggestion: if is_repo {
            None
        } else {
            Some("Initialize JJ: zjj init or jj git init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check workspace context - warn if in a zjj workspace
///
/// This helps AI agents understand they're already in the right place
/// and should NOT clone the repository elsewhere.
fn check_workspace_context() -> DoctorCheck {
    let current_dir = std::env::current_dir().ok();
    let in_workspace = current_dir
        .as_ref()
        .map(|p| p.to_string_lossy().contains(".zjj/workspaces"))
        .unwrap_or(false);

    // Extract bead ID if we're in a workspace
    let bead_id = current_dir.as_ref().and_then(|p| {
        p.components()
            .rev()
            .nth(1) // Parent of current dir
            .and_then(|comp| comp.as_os_str().to_str())
            .map(ToString::to_string)
    });

    DoctorCheck {
        name: "Workspace Context".to_string(),
        status: CheckStatus::Pass, // Always pass, just informational
        message: if in_workspace {
            let suffix = bead_id
                .as_ref()
                .map(|b| format!(" for {b}"))
                .map_or(String::new(), |value| value);
            format!("In zjj workspace{suffix}")
        } else {
            "Not in a zjj workspace".to_string()
        },
        suggestion: if in_workspace {
            Some("You are in an isolated workspace. Work here - DO NOT clone elsewhere. See .ai-instructions.md".to_string())
        } else {
            None
        },
        auto_fixable: false,
        details: in_workspace.then(|| {
            serde_json::json!({
                "location": current_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .map_or(String::new(), |value| value),
                "zjj_bead_id": std::env::var("ZJJ_BEAD_ID").unwrap_or_else(|_| "<not set>".to_string()),
                "zjj_workspace": std::env::var("ZJJ_WORKSPACE").unwrap_or_else(|_| "<not set>".to_string()),
            })
        }),
    }
}

/// Check if zjj is initialized
fn check_initialized() -> DoctorCheck {
    // Check for .zjj directory existence directly, without depending on JJ installation
    let zjj_dir = std::path::Path::new(".zjj");
    let config_file = zjj_dir.join("config.toml");
    let initialized = zjj_dir.exists() && config_file.exists();

    DoctorCheck {
        name: "zjj Initialized".to_string(),
        status: if initialized {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if initialized {
            ".zjj directory exists with valid config".to_string()
        } else {
            "zjj not initialized".to_string()
        },
        suggestion: if initialized {
            None
        } else {
            Some("Initialize zjj: zjj init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

fn check_state_db() -> DoctorCheck {
    // Check if recovery occurred recently BEFORE checking database
    if let Some(recovery_info) = check_for_recent_recovery() {
        return DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: format!("Database recovered: {recovery_info}"),
            suggestion: Some(
                "Recovery completed. Review .zjj/recovery.log for details.".to_string(),
            ),
            auto_fixable: false,
            details: Some(serde_json::json!({
                "recovered": true,
                "details": recovery_info
            })),
        };
    }

    // Read-only database check - don't trigger recovery in doctor mode
    // Check file existence, readability, and basic validity without opening DB
    let db_path = std::path::Path::new(".zjj/state.db");

    if !db_path.exists() {
        return DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: "Database file does not exist".to_string(),
            suggestion: Some("Run 'zjj init' to create database".to_string()),
            auto_fixable: false,
            details: None,
        };
    }

    // Check file permissions and readability
    let metadata = match db_path.metadata() {
        Ok(m) => m,
        Err(e) => {
            return DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Warn,
                message: format!("Cannot access database metadata: {e}"),
                suggestion: Some("Check file permissions".to_string()),
                auto_fixable: false,
                details: None,
            };
        }
    };

    let is_readable = metadata.permissions().readonly();

    if !is_readable {
        return DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Fail,
            message: "Database file is not readable (permission denied)".to_string(),
            suggestion: Some("Check file permissions on .zjj/state.db".to_string()),
            auto_fixable: false,
            details: Some(serde_json::json!({
                "path": db_path.display().to_string(),
                "permission_denied": true
            })),
        };
    }

    // Check file size (corrupted databases often have wrong size)
    let file_size = metadata.len();
    if file_size == 0 || file_size < 100 {
        return DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "Database file has suspicious size: {file_size} bytes (may be corrupted)"
            ),
            suggestion: Some(
                "Database may be corrupted. Run 'zjj doctor --fix' to attempt recovery."
                    .to_string(),
            ),
            auto_fixable: true,
            details: Some(serde_json::json!({
                "file_size": file_size,
                "suspicious_size": true
            })),
        };
    }

    // Basic check passed - consider database accessible and potentially healthy
    // Note: We don't verify SQLite integrity to avoid triggering recovery
    DoctorCheck {
        name: "State Database".to_string(),
        status: CheckStatus::Pass,
        message: format!("state.db is accessible ({file_size} bytes)"),
        suggestion: None,
        auto_fixable: false,
        details: Some(serde_json::json!({
            "file_size": file_size,
            "readable": true
        })),
    }
}

/// Check for orphaned workspaces
fn check_orphaned_workspaces() -> DoctorCheck {
    // Get list of sessions from DB with their workspace paths
    let db_sessions = get_session_db()
        .ok()
        .and_then(|db| db.list_blocking(None).ok())
        .unwrap_or_else(|| Vec::new());

    // Get list of JJ workspaces
    let jj_workspaces = jj_root().map_or_else(
        |_| vec![],
        |root| {
            let output = Command::new("jj")
                .args(["workspace", "list"])
                .current_dir(&root)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .filter_map(|line| {
                            // Parse workspace list output
                            // JJ workspace names end with a colon (e.g., "my-session:")
                            line.split_whitespace()
                                .next()
                                .map(|name| name.trim_end_matches(':').to_string())
                        })
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            }
        },
    );

    // Build a set of session names for quick lookup
    let session_names: std::collections::HashSet<_> =
        db_sessions.iter().map(|s| s.name.as_str()).collect();

    // Find workspaces without sessions (filesystem → DB orphans)
    let filesystem_orphans: Vec<_> = jj_workspaces
        .iter()
        .filter(|ws| ws.as_str() != "default" && !session_names.contains(ws.as_str()))
        .cloned()
        .collect();

    // Find sessions without valid workspaces (DB → filesystem orphans)
    // A session is orphaned if:
    // 1. No workspace with matching name exists in JJ, OR
    // 2. Workspace exists in JJ but the directory is missing
    let db_orphans: Vec<_> = db_sessions
        .into_iter()
        .filter(|session| {
            let has_workspace = jj_workspaces.iter().any(|ws| ws == session.name.as_str());
            let directory_exists = std::path::Path::new(&session.workspace_path).exists();

            !has_workspace || !directory_exists
        })
        .map(|session| session.name)
        .collect();

    // Merge both types of orphans
    let total_orphans = filesystem_orphans.len() + db_orphans.len();
    let orphaned_workspaces = if filesystem_orphans.is_empty() && db_orphans.is_empty() {
        None
    } else {
        Some(serde_json::json!({
            "filesystem_to_db": filesystem_orphans,
            "db_to_filesystem": db_orphans,
        }))
    };

    if total_orphans == 0 {
        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Pass,
            message: "No orphaned workspaces found".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }
    } else {
        let orphan_count_msg = if !filesystem_orphans.is_empty() && !db_orphans.is_empty() {
            format!(
                "{} workspace(s) without DB entries, {} session(s) without workspaces",
                filesystem_orphans.len(),
                db_orphans.len()
            )
        } else if !filesystem_orphans.is_empty() {
            format!(
                "{} workspace(s) without session records",
                filesystem_orphans.len()
            )
        } else {
            format!("{} session(s) with missing workspaces", db_orphans.len())
        };

        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Warn,
            message: orphan_count_msg,
            suggestion: Some("Run 'zjj doctor --fix' to clean up".to_string()),
            auto_fixable: true,
            details: orphaned_workspaces,
        }
    }
}

/// Check Beads integration
fn check_beads() -> DoctorCheck {
    let installed = is_command_available("br");

    if !installed {
        return DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads not installed (optional)".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
    }

    // Count open issues
    let output = Command::new("br").args(["list", "--status=open"]).output();

    match output {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .count();

            DoctorCheck {
                name: "Beads Integration".to_string(),
                status: CheckStatus::Pass,
                message: format!("Beads installed, {count} open issues"),
                suggestion: None,
                auto_fixable: false,
                details: None,
            }
        }
        _ => DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads installed".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
    }
}

/// Check for stale/incomplete sessions
fn check_stale_sessions() -> DoctorCheck {
    let sessions = get_session_db()
        .ok()
        .and_then(|db| db.list_blocking(None).ok())
        .unwrap_or_else(|| Vec::new());

    let stale_threshold = Duration::minutes(5);
    let now = Utc::now();

    let stale_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| {
            if s.status != SessionStatus::Creating {
                return false;
            }

            // Check if session is stale (not updated in 5 minutes)
            let updated_at_i64: i64 = match s.updated_at.try_into() {
                Ok(value) => value,
                Err(_) => i64::MAX,
            };
            let updated_at = chrono::DateTime::from_timestamp(updated_at_i64, 0).unwrap_or(now);
            let duration = now.signed_duration_since(updated_at);

            duration > stale_threshold
        })
        .map(|s| s.name.clone())
        .collect();

    if stale_sessions.is_empty() {
        DoctorCheck {
            name: "Stale Sessions".to_string(),
            status: CheckStatus::Pass,
            message: "No stale sessions detected".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }
    } else {
        let details = serde_json::json!({
            "stale_sessions": stale_sessions,
        });
        DoctorCheck {
            name: "Stale Sessions".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "{} stale/incomplete session(s) detected (not updated in 5 minutes)",
                stale_sessions.len()
            ),
            suggestion: Some(
                "Check for interrupted operations or run 'zjj remove <name>' to clean up"
                    .to_string(),
            ),
            auto_fixable: false,
            details: Some(details),
        }
    }
}

/// Check for workflow violations that may confuse AI agents
#[allow(clippy::manual_let_else)]
fn check_workflow_violations() -> DoctorCheck {
    let db = match get_session_db() {
        Ok(db) => db,
        Err(_) => {
            return DoctorCheck {
                name: "Workflow Health".to_string(),
                status: CheckStatus::Pass,
                message: "No session database".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            };
        }
    };

    let sessions = db.list_blocking(None).unwrap_or_else(|_| Vec::new());
    let active_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .collect();

    // Check if we're on main but have active workspaces
    let current_dir = std::env::current_dir().ok();
    let on_main = current_dir
        .as_ref()
        .map(|p| !p.to_string_lossy().contains(".zjj/workspaces"))
        .unwrap_or(true);

    if on_main && !active_sessions.is_empty() {
        let session_names: Vec<_> = active_sessions.iter().map(|s| s.name.clone()).collect();
        return DoctorCheck {
            name: "Workflow Health".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "On main branch but {} active workspace(s) exist",
                active_sessions.len()
            ),
            suggestion: Some(format!(
                "Work should happen in isolated workspaces. Run: zjj attach {}",
                session_names
                    .first()
                    .map(String::as_str)
                    .unwrap_or("<name>")
            )),
            auto_fixable: false,
            details: Some(serde_json::json!({
                "active_workspaces": session_names,
                "on_main": true,
                "workflow_violation": "working_on_main_with_workspaces"
            })),
        };
    }

    DoctorCheck {
        name: "Workflow Health".to_string(),
        status: CheckStatus::Pass,
        message: if active_sessions.is_empty() {
            "No active sessions - ready for new work".to_string()
        } else {
            format!(
                "{} active session(s) - work in progress",
                active_sessions.len()
            )
        },
        suggestion: if active_sessions.is_empty() {
            Some("Start work: zjj spawn <bead-id>".to_string())
        } else {
            None
        },
        auto_fixable: false,
        details: None,
    }
}

/// Show health report
///
/// # Exit Codes
/// - 0: All checks passed (healthy system)
/// - 1: One or more checks failed (unhealthy system)
/// - 2: System recovered from corruption (recovery detected)
fn show_health_report(checks: &[DoctorCheck], format: OutputFormat) -> Result<()> {
    // Calculate summary statistics
    let warnings = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();
    let errors = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    let passed = checks.len() - warnings - errors;
    let healthy = errors == 0;

    // Check if recovery occurred (any check with "recovered" in details)
    let has_recovery = checks.iter().any(|check| {
        check
            .details
            .as_ref()
            .and_then(|d| d.get("recovered"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });

    if format.is_json() {
        let response = DoctorJsonResponse {
            checks: checks.to_vec(),
            summary: DoctorSummary {
                passed,
                warnings,
                failed: errors,
            },
        };
        let envelope = SchemaEnvelope::new("doctor-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
        // If unhealthy in JSON mode, exit with 1 immediately to avoid
        // main.rs printing a second JSON error object
        if !healthy {
            std::process::exit(1);
        }
        // If recovery occurred, exit with 2
        if has_recovery {
            std::process::exit(2);
        }
        return Ok(());
    }

    println!("zjj System Health Check");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for check in checks {
        let symbol = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Warn => "⚠",
            CheckStatus::Fail => "✗",
        };

        println!("{symbol} {:<25} {}", check.name, check.message);

        if let Some(ref suggestion) = check.suggestion {
            println!("  → {suggestion}");
        }
    }

    println!();
    println!("Health: {passed} passed, {warnings} warning(s), {errors} error(s)");

    let auto_fixable = checks.iter().filter(|c| c.auto_fixable).count();
    if auto_fixable > 0 {
        println!("Some issues can be auto-fixed: zjj doctor --fix");
    }

    // Return error if system is unhealthy (has failures)
    if !healthy {
        anyhow::bail!("Health check failed: {errors} error(s) detected");
    }

    // Exit with code 2 if recovery occurred
    if has_recovery {
        std::process::exit(2);
    }

    Ok(())
}

/// Run auto-fixes
///
/// # Exit Codes
/// - 0: All critical issues were fixed or none existed
/// - 1: Critical issues remain unfixed
fn run_fixes(checks: &[DoctorCheck], format: OutputFormat) -> Result<()> {
    let mut fixed = vec![];
    let mut unable_to_fix = vec![];

    for check in checks {
        if !check.auto_fixable {
            if check.status != CheckStatus::Pass {
                unable_to_fix.push(UnfixableIssue {
                    issue: check.name.clone(),
                    reason: "Requires manual intervention".to_string(),
                    suggestion: match check.suggestion.clone() {
                        Some(value) => value,
                        None => String::new(),
                    },
                });
            }
            continue;
        }

        // Try to fix the issue
        let fix_result = match check.name.as_str() {
            "Orphaned Workspaces" => fix_orphaned_workspaces(check).map_err(|e| e.to_string()),
            "Stale Sessions" => fix_stale_sessions(check),
            _ => Err("No auto-fix available".to_string()),
        };

        match fix_result {
            Ok(action) => {
                fixed.push(FixResult {
                    issue: check.name.clone(),
                    action,
                    success: true,
                });
            }
            Err(reason) => {
                unable_to_fix.push(UnfixableIssue {
                    issue: check.name.clone(),
                    reason: format!("Fix failed: {reason}"),
                    suggestion: match check.suggestion.clone() {
                        Some(value) => value,
                        None => String::new(),
                    },
                });
            }
        }
    }

    let output = DoctorFixOutput {
        fixed,
        unable_to_fix,
    };

    // Count critical (Fail status) issues that couldn't be fixed
    let critical_unfixed = checks
        .iter()
        .filter(|c| {
            c.status == CheckStatus::Fail && !output.fixed.iter().any(|f| f.issue == c.name)
        })
        .count();

    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&output)?);
        if critical_unfixed > 0 {
            std::process::exit(1);
        }
        return Ok(());
    }

    show_fix_results(&output);

    if critical_unfixed > 0 {
        anyhow::bail!("Auto-fix completed but {critical_unfixed} critical issue(s) remain unfixed");
    }

    Ok(())
}

fn fix_stale_sessions(check: &DoctorCheck) -> Result<String, String> {
    let stale_data = check
        .details
        .as_ref()
        .and_then(|v| v.get("stale_sessions"))
        .ok_or_else(|| "No stale sessions data".to_string())?;

    let sessions = stale_data
        .as_array()
        .ok_or_else(|| "Stale sessions data is not an array".to_string())?;

    let db = get_session_db().map_err(|e| format!("Failed to open DB: {e}"))?;
    let mut removed = 0;

    for session_value in sessions {
        if let Some(session_name) = session_value.as_str() {
            match db.delete_blocking(session_name) {
                Ok(true) => removed += 1,
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!("Failed to delete stale session '{session_name}': {e}");
                }
            }
        }
    }

    if removed > 0 {
        Ok(format!("Removed {removed} stale session(s)"))
    } else {
        Err("Failed to remove any stale sessions".to_string())
    }
}

fn show_fix_results(output: &DoctorFixOutput) {
    if !output.fixed.is_empty() {
        println!("Fixed Issues:");
        output.fixed.iter().for_each(|fix| {
            let symbol = if fix.success { "✓" } else { "✗" };
            println!(
                "{symbol} {fix_issue}: {fix_action}",
                fix_issue = fix.issue,
                fix_action = fix.action
            );
        });
        println!();
    }

    if !output.unable_to_fix.is_empty() {
        println!("Unable to Fix:");
        output.unable_to_fix.iter().for_each(|issue| {
            println!(
                "✗ {issue_name}: {issue_reason}",
                issue_name = issue.issue,
                issue_reason = issue.reason
            );
            println!(
                "  → {issue_suggestion}",
                issue_suggestion = issue.suggestion
            );
        });
    }
}

/// Fix orphaned workspaces
fn fix_orphaned_workspaces(check: &DoctorCheck) -> Result<String> {
    let orphaned_data = check
        .details
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No orphaned workspaces data"))?;

    let root = jj_root()?;
    let mut filesystem_removed = 0;
    let mut db_removed = 0;

    // Fix filesystem → DB orphans (workspaces without sessions)
    if let Some(filesystem_orphans) = orphaned_data
        .get("filesystem_to_db")
        .and_then(|v| v.as_array())
    {
        for workspace in filesystem_orphans {
            if let Some(name) = workspace.as_str() {
                let result = Command::new("jj")
                    .args(["workspace", "forget", name])
                    .current_dir(&root)
                    .output()
                    .ok();

                if result.map(|r| r.status.success()).unwrap_or(false) {
                    filesystem_removed += 1;
                }
            }
        }
    }

    // Fix DB → filesystem orphans (sessions without workspaces)
    if let Some(db_orphans) = orphaned_data
        .get("db_to_filesystem")
        .and_then(|v| v.as_array())
    {
        if let Ok(db) = get_session_db() {
            for session_name in db_orphans {
                if let Some(name) = session_name.as_str() {
                    if db.delete_blocking(name).unwrap_or(false) {
                        db_removed += 1;
                    }
                }
            }
        }
    }

    let mut parts = Vec::new();
    if filesystem_removed > 0 {
        parts.push(format!(
            "Removed {filesystem_removed} orphaned workspace(s)"
        ));
    }
    if db_removed > 0 {
        parts.push(format!(
            "Deleted {db_removed} session(s) without workspaces"
        ));
    }

    Ok(if parts.is_empty() {
        "No orphans to clean up".to_string()
    } else {
        parts.join("; ")
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[test]
    #[serial]
    fn test_check_initialized_detects_zjj_directory() {
        // Create a temporary directory
        let temp_dir = TempDir::new().ok().filter(|_| true);
        let Some(temp_dir) = temp_dir else {
            return;
        };

        // Change to temp directory
        let original_dir = std::env::current_dir().ok().filter(|_| true);
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Test 1: No .zjj directory - should fail
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);
        assert_eq!(result.name, "zjj Initialized");
        assert!(result.message.contains("not initialized"));

        // Test 2: .zjj directory exists but no config.toml - should fail
        if fs::create_dir(".zjj").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);

        // Test 3: .zjj directory with config.toml - should pass
        if fs::write(".zjj/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains(".zjj directory exists"));

        // Cleanup: restore original directory
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    #[serial]
    fn test_check_initialized_independent_of_jj() {
        // This test verifies that check_initialized doesn't call jj commands
        // We test this by checking it works even without a JJ repo

        let temp_dir = TempDir::new().ok().filter(|_| true);
        let Some(temp_dir) = temp_dir else {
            return;
        };

        let original_dir = std::env::current_dir().ok().filter(|_| true);
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Create .zjj structure WITHOUT initializing a JJ repo
        if fs::create_dir(".zjj").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        if fs::write(".zjj/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }

        // Even without JJ installed/initialized, should detect .zjj
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_check_jj_installed_vs_check_initialized() {
        // Verify that JJ installation check and initialization check are separate concerns
        let jj_check = check_jj_installed();
        let init_check = check_initialized();

        // These should be independent checks
        assert_eq!(jj_check.name, "JJ Installation");
        assert_eq!(init_check.name, "zjj Initialized");

        // They should have different purposes
        assert!(jj_check.message.contains("JJ") || jj_check.message.contains("installed"));
        assert!(init_check.message.contains("zjj") || init_check.message.contains("initialized"));
    }

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_doctor_json_has_envelope() -> Result<()> {
        // Verify envelope wrapping for doctor command output
        use zjj_core::json::SchemaEnvelope;

        let response = DoctorJsonResponse {
            checks: vec![],
            summary: DoctorSummary {
                passed: 0,
                warnings: 0,
                failed: 0,
            },
        };
        let envelope = SchemaEnvelope::new("doctor-response", "single", response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_doctor_checks_wrapped() -> Result<()> {
        // Verify health check results are wrapped in envelope
        use zjj_core::json::SchemaEnvelope;

        let checks = vec![DoctorCheck {
            name: "JJ Installation".to_string(),
            status: CheckStatus::Pass,
            message: "JJ is installed".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }];
        let response = DoctorJsonResponse {
            checks,
            summary: DoctorSummary {
                passed: 1,
                warnings: 0,
                failed: 0,
            },
        };
        let envelope = SchemaEnvelope::new("doctor-response", "single", response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert!(parsed.get("success").is_some(), "Missing success field");
        assert!(parsed.get("checks").is_some(), "Missing checks field");
        assert!(parsed.get("summary").is_some(), "Missing summary field");

        Ok(())
    }

    #[test]
    fn test_doctor_summary_structure() -> Result<()> {
        // Verify summary structure matches documented schema
        use zjj_core::json::SchemaEnvelope;

        let response = DoctorJsonResponse {
            checks: vec![],
            summary: DoctorSummary {
                passed: 8,
                warnings: 2,
                failed: 1,
            },
        };
        let envelope = SchemaEnvelope::new("doctor-response", "single", response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let summary = parsed
            .get("summary")
            .ok_or_else(|| anyhow::anyhow!("summary field missing"))?;

        assert_eq!(
            summary.get("passed").and_then(serde_json::Value::as_u64),
            Some(8),
            "passed count should match"
        );
        assert_eq!(
            summary.get("warnings").and_then(serde_json::Value::as_u64),
            Some(2),
            "warnings count should match"
        );
        assert_eq!(
            summary.get("failed").and_then(serde_json::Value::as_u64),
            Some(1),
            "failed count should match"
        );

        Ok(())
    }
}

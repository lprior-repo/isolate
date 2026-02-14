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

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Duration, Utc};
use futures::{StreamExt, TryStreamExt};
use sqlx::Row;
use tokio::process::Command;
use zjj_core::{
    config::{load_config, Config},
    introspection::{CheckStatus, DoctorCheck, DoctorFixOutput, FixResult, UnfixableIssue},
    json::SchemaEnvelope,
    workspace_integrity::{CorruptionType, IntegrityValidator, ValidationResult},
    OutputFormat,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo, jj_root},
    commands::{
        add::{pending_add_operation_count, replay_pending_add_operations},
        get_session_db, workspace_utils,
    },
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

async fn check_for_recent_recovery() -> Option<String> {
    let log_path = Path::new(".zjj/recovery.log");

    if !tokio::fs::try_exists(log_path).await.is_ok_and(|v| v) {
        return None;
    }

    let content = tokio::fs::read_to_string(log_path).await.ok()?;

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
                    let message = last_line.split(']').nth(1).map_or("", |s| s);
                    return Some(format!("Recent recovery detected: {message}"));
                }
            }
        }
    }

    None
}

/// Run health checks
pub async fn run(format: OutputFormat, fix: bool, dry_run: bool, verbose: bool) -> Result<()> {
    let checks = run_all_checks().await;

    if fix {
        run_fixes(&checks, format, dry_run, verbose).await
    } else {
        show_health_report(&checks, format)
    }
}

/// Run all health checks
async fn run_all_checks() -> Vec<DoctorCheck> {
    vec![
        check_jj_installed().await,
        check_zellij_installed().await,
        check_zellij_running(),
        check_jj_repo().await,
        check_workspace_context(),
        check_initialized().await,
        check_state_db().await,
        check_workspace_integrity().await,
        check_orphaned_workspaces().await,
        check_stale_sessions().await,
        check_pending_add_operations().await,
        check_beads().await,
        check_workflow_violations().await,
    ]
}

async fn check_pending_add_operations() -> DoctorCheck {
    let Ok(db) = get_session_db().await else {
        return DoctorCheck {
            name: "Pending Add Operations".to_string(),
            status: CheckStatus::Warn,
            message: "Unable to open database for add operation journal check".to_string(),
            suggestion: Some("Run 'zjj init' and retry doctor".to_string()),
            auto_fixable: false,
            details: None,
        };
    };

    let pending_count = match pending_add_operation_count(&db).await {
        Ok(count) => count,
        Err(error) => {
            return DoctorCheck {
                name: "Pending Add Operations".to_string(),
                status: CheckStatus::Warn,
                message: format!("Failed to read add operation journal: {error}"),
                suggestion: Some("Run 'zjj doctor --fix' to retry reconciliation".to_string()),
                auto_fixable: true,
                details: None,
            }
        }
    };

    if pending_count == 0 {
        DoctorCheck {
            name: "Pending Add Operations".to_string(),
            status: CheckStatus::Pass,
            message: "No pending add operations in journal".to_string(),
            suggestion: None,
            auto_fixable: true,
            details: Some(serde_json::json!({ "pending_operations": 0 })),
        }
    } else {
        DoctorCheck {
            name: "Pending Add Operations".to_string(),
            status: CheckStatus::Fail,
            message: format!("Detected {pending_count} pending add operation(s)"),
            suggestion: Some(
                "Run 'zjj doctor --fix' to reconcile pending add operations".to_string(),
            ),
            auto_fixable: true,
            details: Some(serde_json::json!({ "pending_operations": pending_count })),
        }
    }
}

/// Check workspace integrity using the integrity validator
#[allow(clippy::too_many_lines)]
async fn check_workspace_integrity() -> DoctorCheck {
    let config = match load_config_or_error().await {
        Ok(cfg) => cfg,
        Err(e) => {
            return DoctorCheck {
                name: "Workspace Integrity".to_string(),
                status: CheckStatus::Warn,
                message: format!("Unable to load config: {e}"),
                suggestion: Some("Check .zjj/config.toml for errors".to_string()),
                auto_fixable: false,
                details: None,
            };
        }
    };

    let root = jj_root()
        .await
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());
    let Some(root) = root else {
        return DoctorCheck {
            name: "Workspace Integrity".to_string(),
            status: CheckStatus::Warn,
            message: "Unable to determine repository root".to_string(),
            suggestion: Some("Run doctor from within a JJ repository".to_string()),
            auto_fixable: false,
            details: None,
        };
    };

    let workspace_dir = if Path::new(&config.workspace_dir).is_absolute() {
        Path::new(&config.workspace_dir).to_path_buf()
    } else {
        root.join(Path::new(&config.workspace_dir))
    };

    let sessions = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    if sessions.is_empty() {
        return DoctorCheck {
            name: "Workspace Integrity".to_string(),
            status: CheckStatus::Pass,
            message: "No sessions to validate".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
    }

    let missing_session_paths = match futures::stream::iter(sessions.iter())
        .map(Ok::<_, anyhow::Error>)
        .try_filter_map(|session| async move {
            let session_name = session.name.clone();
            let workspace_path = session.workspace_path.clone();
            let exists = tokio::fs::try_exists(Path::new(&session.workspace_path))
                .await
                .map_err(|error| {
                    anyhow::anyhow!(
                        "Failed to verify workspace path '{}' for session '{}': {error}",
                        session.workspace_path,
                        session.name
                    )
                })?;

            Ok((!exists).then_some(serde_json::json!({
                "workspace": session_name,
                "path": workspace_path,
                "issue_count": 1,
                "issues": [
                    {
                        "type": "missing_directory",
                        "description": "Workspace path from session database does not exist",
                        "path": session.workspace_path,
                    }
                ]
            })))
        })
        .try_collect::<Vec<_>>()
        .await
    {
        Ok(values) => values,
        Err(error) => {
            return DoctorCheck {
                name: "Workspace Integrity".to_string(),
                status: CheckStatus::Warn,
                message: format!("Workspace path verification failed: {error}"),
                suggestion: Some("Run 'zjj doctor --fix' to retry validation".to_string()),
                auto_fixable: false,
                details: None,
            };
        }
    };

    let workspace_roots = workspace_utils::candidate_workspace_roots(&root, &config.workspace_dir);
    let validator = IntegrityValidator::new(workspace_dir);
    let names: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();

    let results = match validator.validate_all(&names).await {
        Ok(values) => values,
        Err(e) => {
            return DoctorCheck {
                name: "Workspace Integrity".to_string(),
                status: CheckStatus::Warn,
                message: format!("Integrity validation failed: {e}"),
                suggestion: Some(
                    "Run 'zjj integrity validate <workspace>' for details".to_string(),
                ),
                auto_fixable: false,
                details: None,
            };
        }
    };

    let invalid: Vec<&ValidationResult> = results.iter().filter(|r| !r.is_valid).collect();

    if invalid.is_empty() && missing_session_paths.is_empty() {
        create_pass_check()
    } else {
        let mut invalid_details = missing_session_paths;

        for validation in &invalid {
            let already_recorded_missing = invalid_details.iter().any(|entry| {
                entry
                    .get("workspace")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|workspace| workspace == validation.workspace)
            });
            let only_missing_directory = validation
                .issues
                .iter()
                .all(|issue| issue.corruption_type == CorruptionType::MissingDirectory);

            if already_recorded_missing && only_missing_directory {
                continue;
            }

            let needs_relocation = validation
                .issues
                .iter()
                .any(|issue| issue.corruption_type == CorruptionType::MissingDirectory);

            let relocated_path = if needs_relocation {
                workspace_utils::find_relocated_workspace(&validation.workspace, &workspace_roots)
                    .await
            } else {
                None
            };

            let issues = validation
                .issues
                .iter()
                .map(|issue| {
                    serde_json::json!({
                        "type": issue.corruption_type.to_string(),
                        "description": issue.description.clone(),
                        "path": issue
                            .affected_path
                            .as_ref()
                            .map(|p| p.display().to_string()),
                    })
                })
                .collect::<Vec<_>>();

            invalid_details.push(serde_json::json!({
                "workspace": validation.workspace.clone(),
                "path": validation.path.display().to_string(),
                "issue_count": validation.issues.len(),
                "issues": issues,
                "relocated_path": relocated_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
            }));
        }

        create_fail_check_with_details(&invalid_details)
    }
}

async fn load_config_or_error() -> Result<Config> {
    load_config().await.map_err(std::convert::Into::into)
}

fn create_pass_check() -> DoctorCheck {
    DoctorCheck {
        name: "Workspace Integrity".to_string(),
        status: CheckStatus::Pass,
        message: "All workspaces validated successfully".to_string(),
        suggestion: None,
        auto_fixable: false,
        details: None,
    }
}

fn create_fail_check_with_details(details: &[serde_json::Value]) -> DoctorCheck {
    let invalid_count = details.len();
    DoctorCheck {
        name: "Workspace Integrity".to_string(),
        status: CheckStatus::Fail,
        message: format!("Integrity issues found in {invalid_count} workspace(s)"),
        suggestion: Some(
            "Run 'zjj doctor --fix' to attempt recovery (or use 'zjj integrity repair --rebind <name>' when workspaces move)".to_string(),
        ),
        auto_fixable: true,
        details: Some(serde_json::json!({
            "invalid_workspaces": details
        })),
    }
}

/// Check if JJ is installed
async fn check_jj_installed() -> DoctorCheck {
    let installed = is_command_available("jj").await;

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
async fn check_zellij_installed() -> DoctorCheck {
    let installed = is_command_available("zellij").await;

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
async fn check_jj_repo() -> DoctorCheck {
    let is_repo = is_jj_repo().await.map_or(false, |v| v);

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
        .map_or(false, |v| v);

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
                "zjj_bead_id": std::env::var("ZJJ_BEAD_ID").map_or_else(|_| "<not set>".to_string(), |v| v),
                "zjj_workspace": std::env::var("ZJJ_WORKSPACE").map_or_else(|_| "<not set>".to_string(), |v| v),
            })
        }),
    }
}

/// Check if zjj is initialized
async fn check_initialized() -> DoctorCheck {
    // Check for .zjj directory existence directly, without depending on JJ installation
    let zjj_dir = std::path::Path::new(".zjj");
    let config_file = zjj_dir.join("config.toml");
    let initialized = tokio::fs::try_exists(zjj_dir).await.map_or(false, |v| v)
        && tokio::fs::try_exists(&config_file)
            .await
            .map_or(false, |v| v);

    DoctorCheck {
        name: "zjj Initialized".to_string(),
        status: if initialized {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
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

async fn check_state_db() -> DoctorCheck {
    // Check if recovery occurred recently BEFORE checking database
    if let Some(ref recovery_info) = check_for_recent_recovery().await {
        let recovery_check = create_recovery_check(recovery_info);

        // Run integrity validation after recovery to check for any corruption
        let integrity_check = run_integrity_after_recovery().await;
        if let Some(integrity_check) = integrity_check {
            return DoctorCheck {
                name: "State Database + Integrity".to_string(),
                status: CheckStatus::Warn,
                message: format!(
                    "Database recovered: {}\nIntegrity issues detected: {}",
                    recovery_info, integrity_check.message
                ),
                suggestion: Some(format!(
                    "{}\n{}",
                    recovery_check.suggestion.map_or(String::new(), |s| s),
                    integrity_check.suggestion.map_or(String::new(), |s| s)
                )),
                auto_fixable: false,
                details: Some(serde_json::json!({
                    "recovered": true,
                    "details": recovery_info,
                    "integrity_issues": integrity_check.details
                })),
            };
        }

        return recovery_check;
    }

    // Read-only database check - don't trigger recovery in doctor mode
    // Check file existence, readability, and basic validity without opening DB
    let db_path = std::path::Path::new(".zjj/state.db");

    let file_check_result = check_db_file_exists(db_path).await;
    let metadata = match file_check_result {
        Ok(m) => m,
        Err(check) => return check,
    };

    let readability_result = check_db_readable(db_path).await;
    let file_size = metadata.len();

    match readability_result {
        Ok(()) => check_db_file_size_and_integrity(file_size, db_path).await,
        Err(check) => check,
    }
}

/// Run workspace integrity validation after recovery to detect corruption
async fn run_integrity_after_recovery() -> Option<DoctorCheck> {
    let Ok(config) = load_config().await else {
        return None;
    };

    let root = jj_root()
        .await
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());
    let root = root?;

    let workspace_dir = if Path::new(&config.workspace_dir).is_absolute() {
        Path::new(&config.workspace_dir).to_path_buf()
    } else {
        root.join(Path::new(&config.workspace_dir))
    };

    let sessions = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    if sessions.is_empty() {
        return None;
    }

    let _workspace_roots = workspace_utils::candidate_workspace_roots(&root, &config.workspace_dir);
    let validator = IntegrityValidator::new(workspace_dir);
    let names: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();

    let Ok(results) = validator.validate_all(&names).await else {
        return None;
    };

    let invalid: Vec<&ValidationResult> = results.iter().filter(|r| !r.is_valid).collect();

    if invalid.is_empty() {
        return None;
    }

    Some(DoctorCheck {
        name: "Workspace Integrity After Recovery".to_string(),
        status: CheckStatus::Warn,
        message: format!(
            "{} workspace(s) have integrity issues after recovery",
            invalid.len()
        ),
        suggestion: Some(
            "Run 'zjj integrity repair --rebind' to fix relocated workspaces".to_string(),
        ),
        auto_fixable: false,
        details: Some(serde_json::json!({
            "invalid_workspaces": invalid.len(),
            "issues": invalid.iter().map(|r| {
                serde_json::json!({
                    "workspace": r.workspace,
                    "issues": r.issues.len(),
                    "corruptions": r.issues.iter().map(|i| i.corruption_type).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        })),
    })
}

/// Create a recovery check result when recent recovery is detected
fn create_recovery_check(recovery_info: &str) -> DoctorCheck {
    DoctorCheck {
        name: "State Database".to_string(),
        status: CheckStatus::Warn,
        message: format!("Database recovered: {recovery_info}"),
        suggestion: Some("Recovery completed. Review .zjj/recovery.log for details and run 'zjj backup --create' to capture the recovered state.".to_string()),
        auto_fixable: false,
        details: Some(serde_json::json!({
            "recovered": true,
            "details": recovery_info
        })),
    }
}

/// Check if database file exists
/// Returns Ok with metadata if exists, Err with `DoctorCheck` if missing
async fn check_db_file_exists(db_path: &Path) -> Result<std::fs::Metadata, DoctorCheck> {
    if !tokio::fs::try_exists(db_path).await.map_or(false, |v| v) {
        return Err(DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: "Database file does not exist".to_string(),
            suggestion: Some("Run 'zjj init' to create database".to_string()),
            auto_fixable: false,
            details: None,
        });
    }

    tokio::fs::metadata(db_path).await.map_err(|e| DoctorCheck {
        name: "State Database".to_string(),
        status: CheckStatus::Warn,
        message: format!("Cannot access database metadata: {e}"),
        suggestion: Some("Check file permissions".to_string()),
        auto_fixable: false,
        details: None,
    })
}

/// Check if database file is readable
/// Returns Ok(()) if readable, Err with `DoctorCheck` if not
async fn check_db_readable(db_path: &Path) -> Result<(), DoctorCheck> {
    tokio::fs::File::open(db_path)
        .await
        .map_err(|e| DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Fail,
            message: format!("Database file is not readable: {e}"),
            suggestion: Some("Check file permissions on .zjj/state.db".to_string()),
            auto_fixable: false,
            details: Some(serde_json::json!({
                "path": db_path.display().to_string(),
                "permission_denied": true
            })),
        })?;

    Ok(())
}

/// Check if database file size is valid (not corrupted) and run integrity check
/// Returns `DoctorCheck` with final result
async fn check_db_file_size_and_integrity(file_size: u64, db_path: &Path) -> DoctorCheck {
    // Check file size (corrupted databases often have wrong size)
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

    // Get metadata for read-only status
    let metadata = match tokio::fs::metadata(db_path).await {
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

    let is_read_only = metadata.permissions().readonly();
    let integrity_result = run_integrity_check(db_path).await;

    match integrity_result {
        Ok(integrity_details) => {
            // Integrity check passed
            DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Pass,
                message: format!("state.db is accessible and valid ({file_size} bytes)"),
                suggestion: None,
                auto_fixable: false,
                details: Some(serde_json::json!({
                    "file_size": file_size,
                    "read_only": is_read_only,
                    "integrity_check": integrity_details
                })),
            }
        }
        Err(integrity_error) => {
            // Integrity check failed
            DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Fail,
                message: format!("Database integrity check failed: {integrity_error}"),
                suggestion: Some(
                    "Database is corrupted. Run 'zjj doctor --fix' to attempt recovery."
                        .to_string(),
                ),
                auto_fixable: true,
                details: Some(serde_json::json!({
                    "file_size": file_size,
                    "read_only": is_read_only,
                    "integrity_error": integrity_error
                })),
            }
        }
    }
}

/// Run PRAGMA `integrity_check` on the database
///
/// Returns Ok with details if integrity check passes (result is "ok")
/// Returns Err with error message if integrity check fails or connection fails
async fn run_integrity_check(db_path: &Path) -> Result<String, String> {
    use sqlx::SqlitePool;

    // Open database in read-only mode to avoid triggering recovery
    let connection_string = format!("sqlite:{}?mode=ro", db_path.display());

    let pool = match SqlitePool::connect(&connection_string).await {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to open database for integrity check: {e}")),
    };

    // Run PRAGMA integrity_check
    // Returns "ok" if database is valid, or error details otherwise
    let result = match sqlx::query("PRAGMA integrity_check").fetch_one(&pool).await {
        Ok(r) => r,
        Err(e) => {
            pool.close().await;
            return Err(format!("Integrity check query failed: {e}"));
        }
    };

    // The result is a single row with a column named "integrity_check"
    let integrity_result: String = match result.try_get("integrity_check") {
        Ok(r) => r,
        Err(e) => {
            pool.close().await;
            return Err(format!("Failed to parse integrity check result: {e}"));
        }
    };

    // Close the pool
    pool.close().await;

    // Check if integrity check passed
    if integrity_result == "ok" {
        Ok("Database integrity verified".to_string())
    } else {
        Err(integrity_result)
    }
}

/// Check for orphaned workspaces
async fn check_orphaned_workspaces() -> DoctorCheck {
    // Get list of sessions from DB with their workspace paths
    let db_sessions = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    // Get list of JJ workspaces
    let jj_root_res = jj_root().await;
    let jj_workspaces = match jj_root_res {
        Err(_) => vec![],
        Ok(root) => {
            let output = Command::new("jj")
                .args(["workspace", "list"])
                .current_dir(&root)
                .output()
                .await;

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
        }
    };

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
    let db_orphans: Vec<_> = futures::stream::iter(db_sessions)
        .then(|session| {
            let jj_workspaces = &jj_workspaces;
            async move {
                let has_workspace = jj_workspaces.iter().any(|ws| ws == session.name.as_str());
                let directory_exists = tokio::fs::try_exists(&session.workspace_path)
                    .await
                    .map_or(false, |v| v);

                if !has_workspace || !directory_exists {
                    Some(session.name)
                } else {
                    None
                }
            }
        })
        .filter_map(|opt| async move { opt })
        .collect()
        .await;

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
async fn check_beads() -> DoctorCheck {
    let installed = is_command_available("br").await;

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
    let output = Command::new("br")
        .args(["list", "--status=open"])
        .output()
        .await;

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
async fn check_stale_sessions() -> DoctorCheck {
    let sessions = match get_session_db().await {
        Ok(db) => match db.list(None).await {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let stale_threshold = Duration::minutes(5);
    let now = Utc::now();

    let stale_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| {
            if s.status != SessionStatus::Creating {
                return false;
            }

            // Check if session is stale (not updated in 5 minutes)
            let updated_at_i64 = i64::try_from(s.updated_at).map_or(i64::MAX, |v| v);
            let updated_at = chrono::DateTime::from_timestamp(updated_at_i64, 0).map_or(now, |v| v);
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
async fn check_workflow_violations() -> DoctorCheck {
    let Ok(db) = get_session_db().await else {
        return DoctorCheck {
            name: "Workflow Health".to_string(),
            status: CheckStatus::Pass,
            message: "No session database".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
    };

    let sessions = match db.list(None).await {
        Ok(s) => s,
        Err(_) => Vec::new(),
    };
    let active_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .collect();

    // Check if we're on main but have active workspaces
    let current_dir = std::env::current_dir().ok();
    let on_main = current_dir
        .as_ref()
        .map(|p| !p.to_string_lossy().contains(".zjj/workspaces"))
        .map_or(true, |v| v);

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
                    .map_or("<name>", |v| v)
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
#[allow(clippy::too_many_lines)]
// Long function because: single-pass report generation with format branching
// (JSON vs human-readable). Splitting would require passing intermediate state
// through multiple functions, reducing clarity.
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
    // Note: Currently unused but kept for future diagnostic enhancement
    let _has_recovery = checks.iter().any(|check| {
        check
            .details
            .as_ref()
            .and_then(|d| d.get("recovered"))
            .and_then(serde_json::Value::as_bool)
            .map_or(false, |v| v)
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
        // Note: Recovery state is a warning, not an error. Warnings do not cause
        // non-zero exit codes per the docstring at the top of this file.
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

    // Note: Recovery state is a warning, not an error. Warnings do not cause
    // non-zero exit codes per the docstring at the top of this file.
    // Exit 0 is returned for healthy systems (even with warnings).

    Ok(())
}

/// Show what fixes would be attempted (dry-run mode)
///
/// # Returns
/// - Ok(()) after printing dry-run report
fn show_dry_run_report(checks: &[DoctorCheck], format: OutputFormat) -> Result<()> {
    let fixable_checks: Vec<&DoctorCheck> = checks
        .iter()
        .filter(|c| c.auto_fixable && c.status != CheckStatus::Pass)
        .collect();

    if format.is_json() {
        let dry_run_output = serde_json::json!({
            "dry_run": true,
            "would_fix": fixable_checks.iter().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "status": format!("{:?}", c.status),
                    "description": describe_fix(c)
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&dry_run_output)?);
    } else if fixable_checks.is_empty() {
        println!("Dry-run mode: No auto-fixable issues found");
    } else {
        println!("Dry-run mode: would fix the following:");
        println!();
        for check in fixable_checks {
            println!(
                "  • {}: {}",
                check.name,
                describe_fix(check)
                    .map_or_else(|| "No fix description available".to_string(), |v| v)
            );
        }
        println!();
        println!("No changes will be made. Run without --dry-run to apply fixes.");
    }

    Ok(())
}

/// Get human-readable description of what fix would do
///
/// # Returns
/// - Some(String) describing the fix
/// - None if no fix available
fn describe_fix(check: &DoctorCheck) -> Option<String> {
    match check.name.as_str() {
        "State Database" => {
            Some("Delete corrupted database file (will be recreated on next run)".to_string())
        }
        "Orphaned Workspaces" => {
            check.details.as_ref().map_or_else(
                || Some("Remove orphaned workspaces and stale session records".to_string()),
                |details| {
                    details.get("filesystem_to_db").and_then(|v| v.as_array()).map_or_else(
                        || Some("Remove orphaned workspaces and stale session records".to_string()),
                        |fs_orphans| {
                            let db_orphans = details
                                .get("db_to_filesystem")
                                .and_then(|v| v.as_array())
                                .map_or(0, Vec::len);
                            let fs_count = fs_orphans.len();
                            Some(format!(
                                "Remove {fs_count} orphaned workspace(s) and {db_orphans} session(s) without workspaces"
                            ))
                        },
                    )
                },
            )
        }
        "Stale Sessions" => {
            check.details.as_ref().map_or_else(
                || Some("Remove stale/incomplete session records".to_string()),
                |details| {
                    details.get("stale_sessions").and_then(|v| v.as_array()).map_or_else(
                        || Some("Remove stale/incomplete session records".to_string()),
                        |stale| Some(format!("Remove {} stale session(s)", stale.len())),
                    )
                },
            )
        }
        "Pending Add Operations" => check
            .details
            .as_ref()
            .and_then(|details| details.get("pending_operations"))
            .and_then(serde_json::Value::as_u64)
            .map(|count| format!("Replay and reconcile {count} pending add operation(s)")),
        _ => None,
    }
}

/// Run auto-fixes
///
/// # Exit Codes
/// - 0: All critical issues were fixed or none existed
/// - 1: Critical issues remain unfixed
async fn run_fixes(
    checks: &[DoctorCheck],
    format: OutputFormat,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    // If dry-run, show what would be fixed and exit
    if dry_run {
        show_dry_run_report(checks, format)?;
        return Ok(());
    }

    // Show verbose header
    if verbose && !format.is_json() {
        println!("Attempting to fix auto-fixable issues...");
        println!();
    }
    let (fixed, unable_to_fix) = futures::stream::iter(checks)
        .fold(
            (vec![], vec![]),
            |(mut fixed, mut unable_to_fix), check| async move {
                // Only report non-auto-fixable issues if they failed (not Pass/Warn)
                if !check.auto_fixable {
                    if check.status == CheckStatus::Fail {
                        unable_to_fix.push(UnfixableIssue {
                            issue: check.name.clone(),
                            reason: "Requires manual intervention".to_string(),
                            suggestion: check.suggestion.clone().map_or(String::new(), |s| s),
                        });
                    }
                    return (fixed, unable_to_fix);
                }

                // Skip auto-fixable checks that are passing
                if check.status == CheckStatus::Pass {
                    return (fixed, unable_to_fix);
                }

                // Show verbose progress
                if verbose && !format.is_json() {
                    println!("Fixing {}...", check.name);
                }

                // Try to fix the issue
                let fix_result = match check.name.as_str() {
                    "Orphaned Workspaces" => fix_orphaned_workspaces(check, dry_run).await,
                    "Stale Sessions" => fix_stale_sessions(check, dry_run).await,
                    "Pending Add Operations" => fix_pending_add_operations(check, dry_run).await,
                    "Workspace Integrity" => fix_workspace_integrity(check, dry_run).await,
                    "State Database" => fix_state_database(check, dry_run)
                        .await
                        .map_err(|e| e.to_string()),
                    _ => Err("No auto-fix available".to_string()),
                };

                match fix_result {
                    Ok(action) => {
                        if verbose && !format.is_json() {
                            println!("  ✓ {}: {}", check.name, action);
                        }
                        fixed.push(FixResult {
                            issue: check.name.clone(),
                            action,
                            success: true,
                        });
                    }
                    Err(reason) => {
                        if verbose && !format.is_json() {
                            println!("  ✗ {}: Fix failed: {}", check.name, reason);
                        }
                        unable_to_fix.push(UnfixableIssue {
                            issue: check.name.clone(),
                            reason: format!("Fix failed: {reason}"),
                            suggestion: check.suggestion.clone().map_or(String::new(), |s| s),
                        });
                    }
                }
                (fixed, unable_to_fix)
            },
        )
        .await;

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

async fn fix_stale_sessions(check: &DoctorCheck, dry_run: bool) -> Result<String, String> {
    let stale_data = check
        .details
        .as_ref()
        .and_then(|v| v.get("stale_sessions"))
        .ok_or_else(|| "No stale sessions data".to_string())?;

    let sessions = stale_data
        .as_array()
        .ok_or_else(|| "Stale sessions data is not an array".to_string())?;

    // In dry-run mode, just report what would be done
    if dry_run {
        return Ok(format!("Would remove {} stale session(s)", sessions.len()));
    }

    let db = get_session_db()
        .await
        .map_err(|e| format!("Failed to open DB: {e}"))?;

    let removed = futures::stream::iter(sessions)
        .fold(0, |mut acc, session_value| {
            let db = &db;
            async move {
                if let Some(session_name) = session_value.as_str() {
                    match db.delete(session_name).await {
                        Ok(true) => acc += 1,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!("Failed to delete stale session '{session_name}': {e}");
                        }
                    }
                }
                acc
            }
        })
        .await;

    if removed > 0 {
        Ok(format!("Removed {removed} stale session(s)"))
    } else {
        Err("Failed to remove any stale sessions".to_string())
    }
}

async fn fix_pending_add_operations(check: &DoctorCheck, dry_run: bool) -> Result<String, String> {
    let pending = check
        .details
        .as_ref()
        .and_then(|value| value.get("pending_operations"))
        .and_then(serde_json::Value::as_u64)
        .map_or(0, std::convert::identity);

    if dry_run {
        return Ok(format!(
            "Would reconcile {pending} pending add operation(s)"
        ));
    }

    let db = get_session_db()
        .await
        .map_err(|error| format!("Failed to open DB for reconciliation: {error}"))?;

    replay_pending_add_operations(&db)
        .await
        .map(|recovered| format!("Reconciled {recovered} pending add operation(s)"))
        .map_err(|error| format!("Failed to reconcile add operations: {error}"))
}

async fn fix_workspace_integrity(check: &DoctorCheck, dry_run: bool) -> Result<String, String> {
    let details = check
        .details
        .as_ref()
        .ok_or_else(|| "No workspace integrity data".to_string())?;

    let invalid_workspaces = details
        .get("invalid_workspaces")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Invalid workspaces data is not an array".to_string())?;

    if dry_run {
        return Ok(format!(
            "Would attempt to repair {} workspace(s)",
            invalid_workspaces.len()
        ));
    }

    let config = load_config_or_error()
        .await
        .map_err(|e| format!("Unable to load config: {e}"))?;

    let root = jj_root()
        .await
        .map(PathBuf::from)
        .map_err(|e| format!("Unable to determine repository root: {e}"))?;

    let workspace_dir = if Path::new(&config.workspace_dir).is_absolute() {
        Path::new(&config.workspace_dir).to_path_buf()
    } else {
        root.join(Path::new(&config.workspace_dir))
    };

    let validator = IntegrityValidator::new(workspace_dir.clone());
    let executor = zjj_core::workspace_integrity::RepairExecutor::new();
    let db = get_session_db()
        .await
        .map_err(|error| format!("Failed to open session database: {error}"))?;

    let mut fixed_count = 0;
    let mut failed_workspaces = Vec::new();

    for ws_value in invalid_workspaces {
        let ws_name = ws_value
            .get("workspace")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing workspace name".to_string())?;

        // Re-validate to get fresh state before repair
        let validation = validator
            .validate(ws_name)
            .await
            .map_err(|e| format!("Failed to validate workspace '{ws_name}': {e}"))?;

        let report_path = ws_value
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let validated_path = validation.path.display().to_string();
        let needs_db_rebind = report_path
            .as_ref()
            .is_some_and(|reported_path| reported_path != &validated_path);

        if needs_db_rebind
            && tokio::fs::try_exists(&validation.path)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to verify validated workspace path '{}' for '{ws_name}': {error}",
                        validation.path.display()
                    )
                })?
        {
            db.update_workspace_path(ws_name, &validated_path)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to rebind session '{ws_name}' to workspace path '{validated_path}': {error}"
                    )
                })?;
            fixed_count += 1;
            continue;
        }

        match executor.repair(&validation).await {
            Ok(result) if result.success => {
                fixed_count += 1;
            }
            Ok(result) => {
                failed_workspaces.push(format!("{}: {}", ws_name, result.summary));
            }
            Err(e) => {
                failed_workspaces.push(format!("{}: {}", ws_name, e));
            }
        }
    }

    if failed_workspaces.is_empty() {
        Ok(format!("Successfully repaired {fixed_count} workspace(s)"))
    } else if fixed_count > 0 {
        Ok(format!(
            "Repaired {fixed_count} workspace(s), but {} failed: {}",
            failed_workspaces.len(),
            failed_workspaces.join("; ")
        ))
    } else {
        Err(format!(
            "Failed to repair any workspaces: {}",
            failed_workspaces.join("; ")
        ))
    }
}

async fn fix_state_database(_check: &DoctorCheck, dry_run: bool) -> Result<String> {
    let db_path = std::path::Path::new(".zjj/state.db");
    if !tokio::fs::try_exists(db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check database: {e}"))?
    {
        return Ok("Database file does not exist".to_string());
    }

    // In dry-run mode, just report what would be done
    if dry_run {
        return Ok(
            "Would delete corrupted database file (will be recreated on next run)".to_string(),
        );
    }

    // Attempt to delete the corrupted database
    tokio::fs::remove_file(db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete database: {e}"))?;
    Ok("Deleted corrupted database file. It will be recreated on next run.".to_string())
}

fn show_fix_results(output: &DoctorFixOutput) {
    if !output.fixed.is_empty() {
        println!("Fixed Issues:");
        for fix in &output.fixed {
            let symbol = if fix.success { "✓" } else { "✗" };
            println!(
                "{symbol} {fix_issue}: {fix_action}",
                fix_issue = fix.issue,
                fix_action = fix.action
            );
        }
        println!();
    }

    if !output.unable_to_fix.is_empty() {
        println!("Unable to Fix:");
        for issue in &output.unable_to_fix {
            println!(
                "✗ {issue_name}: {issue_reason}",
                issue_name = issue.issue,
                issue_reason = issue.reason
            );
            println!(
                "  → {issue_suggestion}",
                issue_suggestion = issue.suggestion
            );
        }
    }
}

/// Fix orphaned workspaces
async fn fix_orphaned_workspaces(check: &DoctorCheck, dry_run: bool) -> Result<String, String> {
    let orphaned_data = check
        .details
        .as_ref()
        .ok_or_else(|| "No orphaned workspaces data".to_string())?;

    // In dry-run mode, just report what would be done
    if dry_run {
        let filesystem_count = orphaned_data
            .get("filesystem_to_db")
            .and_then(|v| v.as_array())
            .map_or(0, Vec::len);
        let db_count = orphaned_data
            .get("db_to_filesystem")
            .and_then(|v| v.as_array())
            .map_or(0, Vec::len);

        return Ok(format!(
            "Would remove {filesystem_count} orphaned workspace(s) and {db_count} session(s) without workspaces"
        ));
    }

    let root = jj_root()
        .await
        .map_err(|e| format!("Failed to get JJ root: {e}"))?;

    // Fix filesystem → DB orphans (workspaces without sessions)
    let filesystem_removed = if let Some(filesystem_orphans) = orphaned_data
        .get("filesystem_to_db")
        .and_then(|v| v.as_array())
    {
        futures::stream::iter(filesystem_orphans)
            .fold(0, |mut acc, workspace| {
                let root = &root;
                async move {
                    if let Some(name) = workspace.as_str() {
                        let result = Command::new("jj")
                            .args(["workspace", "forget", name])
                            .current_dir(root)
                            .output()
                            .await
                            .ok();

                        if result.is_some_and(|r| r.status.success()) {
                            acc += 1;
                        }
                    }
                    acc
                }
            })
            .await
    } else {
        0
    };

    // Fix DB → filesystem orphans (sessions without workspaces)
    let db_removed = if let Some(db_orphans) = orphaned_data
        .get("db_to_filesystem")
        .and_then(|v| v.as_array())
    {
        match get_session_db().await {
            Ok(db) => {
                futures::stream::iter(db_orphans)
                    .fold(0, |mut acc, session_name| {
                        let db = &db;
                        async move {
                            if let Some(name) = session_name.as_str() {
                                let should_delete = match db.get(name).await {
                                    Ok(Some(session)) => {
                                        match tokio::fs::try_exists(std::path::Path::new(
                                            &session.workspace_path,
                                        ))
                                        .await
                                        {
                                            Ok(exists) => !exists,
                                            Err(error) => {
                                                tracing::warn!(
                                                    "Failed to verify workspace path '{}' for orphan check on '{name}': {error}",
                                                    session.workspace_path
                                                );
                                                false
                                            }
                                        }
                                    }
                                    Ok(None) => false,
                                    Err(error) => {
                                        tracing::warn!(
                                            "Failed to load session '{name}' before orphan cleanup: {error}"
                                        );
                                        false
                                    }
                                };

                                if should_delete {
                                    match db.delete(name).await {
                                        Ok(true) => acc += 1,
                                        Ok(false) => {}
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to delete orphaned session '{name}': {e}"
                                            );
                                        }
                                    }
                                }
                            }
                            acc
                        }
                    })
                    .await
            }
            Err(_) => 0,
        }
    } else {
        0
    };

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
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_check_initialized_detects_zjj_directory() {
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

        // Test 1: No .zjj directory - should warn
        let result = check_initialized().await;
        assert_eq!(result.status, CheckStatus::Warn);
        assert_eq!(result.name, "zjj Initialized");
        assert!(result.message.contains("not initialized"));

        // Test 2: .zjj directory exists but no config.toml - should warn
        if tokio::fs::create_dir(".zjj").await.is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized().await;
        assert_eq!(result.status, CheckStatus::Warn);

        // Test 3: .zjj directory with config.toml - should pass
        if tokio::fs::write(".zjj/config.toml", "workspace_dir = \"test\"")
            .await
            .is_err()
        {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized().await;
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains(".zjj directory exists"));

        // Cleanup: restore original directory
        let _ = std::env::set_current_dir(original_dir);
    }

    #[tokio::test]
    #[serial]
    async fn test_check_initialized_independent_of_jj() {
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
        if tokio::fs::create_dir(".zjj").await.is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        if tokio::fs::write(".zjj/config.toml", "workspace_dir = \"test\"")
            .await
            .is_err()
        {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }

        // Even without JJ installed/initialized, should detect .zjj
        let result = check_initialized().await;
        assert_eq!(result.status, CheckStatus::Pass);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
    }

    #[tokio::test]
    async fn test_check_jj_installed_vs_check_initialized() {
        // Verify that JJ installation check and initialization check are separate concerns
        let jj_check = check_jj_installed().await;
        let init_check = check_initialized().await;

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

    #[tokio::test]
    async fn test_doctor_json_has_envelope() -> Result<()> {
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

    #[tokio::test]
    async fn test_doctor_checks_wrapped() -> Result<()> {
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

    #[tokio::test]
    async fn test_doctor_summary_structure() -> Result<()> {
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

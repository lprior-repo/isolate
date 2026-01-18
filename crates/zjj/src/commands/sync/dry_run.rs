//! Dry-run output formatting for sync operations
//!
//! This module is organized into three concerns:
//! - Operation simulation: Building plans for what would happen
//! - Result formatting: Formatting results into JSON/text
//! - Output handlers: Displaying formatted results

use std::fmt::Write;

use anyhow::{Context, Result};

use super::{branch_detection::detect_main_branch, SyncOptions};
use crate::{
    json_output::{SyncDryRunOutput, SyncDryRunPlan, SyncSessionPlan},
    session::{Session, SessionStatus},
};

// ═══════════════════════════════════════════════════════════════════════════
// OPERATION SIMULATION
// ═══════════════════════════════════════════════════════════════════════════

/// Build a dry-run plan for a single session
pub fn build_single_session_plan(name: &str, session: &Session) -> Result<SyncDryRunPlan> {
    let workspace_exists = std::path::Path::new(&session.workspace_path).exists();

    // Load config to determine target branch
    let config = zjj_core::config::load_config().context("Failed to load configuration")?;

    let (target_branch, target_source) = determine_target_branch(&config, session)?;

    let session_plan = SyncSessionPlan {
        name: name.to_string(),
        workspace_path: session.workspace_path.clone(),
        workspace_exists,
        status: format!("{:?}", session.status),
        can_sync: workspace_exists,
        skip_reason: if workspace_exists {
            None
        } else {
            Some("Workspace directory does not exist".to_string())
        },
    };

    Ok(SyncDryRunPlan {
        session_name: Some(name.to_string()),
        sessions_to_sync: vec![session_plan],
        target_branch: target_branch.clone(),
        target_branch_source: target_source,
        total_count: 1,
        operations_per_session: vec![
            format!("Rebase workspace onto {target_branch}"),
            "Update last_synced timestamp in database".to_string(),
        ],
    })
}

/// Build a dry-run plan for all sessions
pub fn build_all_sessions_plan(sessions: &[Session]) -> Result<SyncDryRunPlan> {
    // Load config to determine target branch
    let config = zjj_core::config::load_config().context("Failed to load configuration")?;

    // Try to detect target branch from first session with valid workspace
    let first_valid_workspace = sessions
        .iter()
        .find(|s| std::path::Path::new(&s.workspace_path).exists())
        .map(|s| s.workspace_path.as_str());

    let (target_branch, target_source) = match &config.main_branch {
        Some(branch) if !branch.trim().is_empty() => (branch.clone(), "config".to_string()),
        _ => first_valid_workspace.map_or_else(
            || {
                (
                    "trunk()".to_string(),
                    "default (no valid workspaces)".to_string(),
                )
            },
            |workspace_path| {
                detect_main_branch(workspace_path).map_or_else(
                    |_| {
                        (
                            "trunk()".to_string(),
                            "default (detection failed)".to_string(),
                        )
                    },
                    |branch| (branch, "auto-detected".to_string()),
                )
            },
        ),
    };

    // Functional approach: map sessions to plans and count in one pass
    let session_plans: Vec<SyncSessionPlan> = sessions
        .iter()
        .map(|session| {
            let workspace_exists = std::path::Path::new(&session.workspace_path).exists();
            let can_sync = matches!(
                session.status,
                SessionStatus::Active | SessionStatus::Paused
            ) && workspace_exists;

            let skip_reason = compute_skip_reason(&session.status, workspace_exists);

            SyncSessionPlan {
                name: session.name.clone(),
                workspace_path: session.workspace_path.clone(),
                workspace_exists,
                status: format!("{:?}", session.status),
                can_sync,
                skip_reason,
            }
        })
        .collect();

    // Count syncable sessions using functional filter
    let syncable_count = session_plans.iter().filter(|plan| plan.can_sync).count();

    // Format operations before moving target_branch
    let operations_per_session = vec![
        format!("Rebase workspace onto {target_branch}"),
        "Update last_synced timestamp in database".to_string(),
    ];

    Ok(SyncDryRunPlan {
        session_name: None,
        sessions_to_sync: session_plans,
        target_branch,                       // Move instead of clone
        target_branch_source: target_source, // Move instead of clone
        total_count: syncable_count,
        operations_per_session,
    })
}

/// Determine target branch and its source
fn determine_target_branch(
    config: &zjj_core::config::Config,
    session: &Session,
) -> Result<(String, String)> {
    match &config.main_branch {
        Some(branch) if !branch.trim().is_empty() => Ok((branch.clone(), "config".to_string())),
        _ => {
            // Try to detect main branch
            detect_main_branch(&session.workspace_path).map_or_else(
                |_| {
                    Ok((
                        "trunk()".to_string(),
                        "default (detection failed)".to_string(),
                    ))
                },
                |branch| Ok((branch, "auto-detected".to_string())),
            )
        }
    }
}

/// Compute skip reason for a session
fn compute_skip_reason(status: &SessionStatus, workspace_exists: bool) -> Option<String> {
    if !workspace_exists {
        Some("Workspace directory does not exist".to_string())
    } else if !matches!(status, SessionStatus::Active | SessionStatus::Paused) {
        Some(format!("Session status {status:?} not syncable"))
    } else {
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESULT FORMATTING
// ═══════════════════════════════════════════════════════════════════════════

/// Format plan as JSON output for single session
fn format_single_session_json(plan: &SyncDryRunPlan) -> Result<String> {
    let output = SyncDryRunOutput {
        success: true,
        dry_run: true,
        plan,
    };
    serde_json::to_string(&output).map_err(anyhow::Error::from)
}

/// Format plan as JSON output for all sessions
fn format_all_sessions_json(plan: &SyncDryRunPlan) -> Result<String> {
    let output = SyncDryRunOutput {
        success: true,
        dry_run: true,
        plan,
    };
    serde_json::to_string(&output).map_err(anyhow::Error::from)
}

/// Format plan as text output for single session
fn format_single_session_text(name: &str, session: &Session, plan: &SyncDryRunPlan) -> String {
    let workspace_exists = std::path::Path::new(&session.workspace_path).exists();

    let mut output = String::new();
    output.push_str("DRY RUN: The following sync operation would be performed:\n\n");
    writeln!(output, "Session: {name}").ok();
    writeln!(output, "  Workspace: {}", session.workspace_path).ok();
    writeln!(output, "  Workspace exists: {workspace_exists}").ok();
    writeln!(output, "  Status: {:?}", session.status).ok();
    output.push('\n');
    writeln!(
        output,
        "Target branch: {} ({})",
        plan.target_branch, plan.target_branch_source
    )
    .ok();
    output.push('\n');
    output.push_str("Operations:\n");
    writeln!(
        output,
        "  1. jj rebase -d {} (in workspace)",
        plan.target_branch
    )
    .ok();
    output.push_str("  2. Update last_synced timestamp in database\n");
    output.push('\n');
    output.push_str("To execute, run without --dry-run flag:\n");
    writeln!(output, "  zjj sync {name}").ok();

    output
}

/// Format plan as text output for all sessions
fn format_all_sessions_text(sessions: &[Session], plan: &SyncDryRunPlan) -> String {
    let mut output = String::new();
    output.push_str("DRY RUN: The following sync operations would be performed:\n\n");
    writeln!(
        output,
        "Target branch: {} ({})",
        plan.target_branch, plan.target_branch_source
    )
    .ok();
    writeln!(output, "Total sessions: {}", sessions.len()).ok();
    writeln!(output, "Syncable sessions: {}", plan.total_count).ok();
    output.push('\n');

    output.push_str("Sessions:\n");
    plan.sessions_to_sync.iter().for_each(|sp| {
        let status_icon = if sp.can_sync { "✓" } else { "✗" };
        writeln!(output, "  {status_icon} {} ({})", sp.name, sp.status).ok();
        if let Some(ref reason) = sp.skip_reason {
            writeln!(output, "    Skip reason: {reason}").ok();
        }
    });

    output.push('\n');
    output.push_str("Operations per session:\n");
    writeln!(output, "  1. jj rebase -d {}", plan.target_branch).ok();
    output.push_str("  2. Update last_synced timestamp\n");
    output.push('\n');
    output.push_str("To execute, run without --dry-run flag:\n");
    output.push_str("  zjj sync\n");

    output
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/// Output a dry-run plan for a single session sync
pub fn output_single_session_dry_run(
    name: &str,
    session: &Session,
    options: SyncOptions,
) -> Result<()> {
    let plan = build_single_session_plan(name, session)?;

    if options.json {
        let json_str = format_single_session_json(&plan)?;
        println!("{json_str}");
    } else {
        let text = format_single_session_text(name, session, &plan);
        println!("{text}");
    }

    Ok(())
}

/// Output a dry-run plan for syncing all sessions
pub fn output_all_sessions_dry_run(sessions: &[Session], options: SyncOptions) -> Result<()> {
    let plan = build_all_sessions_plan(sessions)?;

    if options.json {
        let json_str = format_all_sessions_json(&plan)?;
        println!("{json_str}");
    } else {
        let text = format_all_sessions_text(sessions, &plan);
        println!("{text}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_skip_reason_no_workspace() {
        let reason = compute_skip_reason(&SessionStatus::Active, false);
        assert!(reason.is_some());
        if let Some(r) = reason {
            assert!(r.contains("does not exist"));
        }
    }

    #[test]
    fn test_compute_skip_reason_invalid_status() {
        let reason = compute_skip_reason(&SessionStatus::Creating, true);
        assert!(reason.is_some());
        if let Some(r) = reason {
            assert!(r.contains("not syncable"));
        }
    }

    #[test]
    fn test_compute_skip_reason_valid() {
        let reason = compute_skip_reason(&SessionStatus::Active, true);
        assert!(reason.is_none());
    }

    #[test]
    fn test_compute_skip_reason_paused_valid() {
        let reason = compute_skip_reason(&SessionStatus::Paused, true);
        assert!(reason.is_none());
    }
}

//! Auto-fix implementations
//!
//! This module contains functions that automatically fix common issues
//! detected by health checks. Uses `im::Vector` for efficient accumulation
//! during the fold operation with structural sharing.

use std::process::Command;

use anyhow::Result;
use im::Vector;
use zjj_core::introspection::{
    CheckStatus, DoctorCheck, DoctorFixOutput, FixResult, UnfixableIssue,
};

use crate::cli::jj_root;

/// Run auto-fixes for all checks
///
/// Returns a tuple of (fixed issues, unfixable issues).
/// Uses `im::Vector` internally for efficient structural sharing during fold,
/// converting to `Vec` at the API boundary for compatibility.
pub async fn run_all(checks: &[DoctorCheck]) -> (Vec<FixResult>, Vec<UnfixableIssue>) {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    // Functional fold: accumulate fixed and unfixable results in one pass
    // Using im::Vector for efficient structural sharing during fold
    let (fixed, unable_to_fix): (Vector<FixResult>, Vector<UnfixableIssue>) = checks.iter().fold(
        (Vector::new(), Vector::new()),
        |(mut fixed, mut unable_to_fix), check| {
            if !check.auto_fixable {
                if check.status != CheckStatus::Pass {
                    unable_to_fix.push_back(UnfixableIssue {
                        issue: check.name.clone(),
                        reason: "Requires manual intervention".to_string(),
                        suggestion: check.suggestion.clone().unwrap_or_default(),
                    });
                }
                return (fixed, unable_to_fix);
            }

            // Try to fix the issue
            match check.name.as_str() {
                "Orphaned Workspaces" => match fix_orphaned_workspaces(check) {
                    Ok(action) => {
                        fixed.push_back(FixResult {
                            issue: check.name.clone(),
                            action,
                            success: true,
                        });
                    }
                    Err(e) => {
                        unable_to_fix.push_back(UnfixableIssue {
                            issue: check.name.clone(),
                            reason: format!("Fix failed: {e}"),
                            suggestion: check.suggestion.clone().unwrap_or_default(),
                        });
                    }
                },
                _ => {
                    unable_to_fix.push_back(UnfixableIssue {
                        issue: check.name.clone(),
                        reason: "No auto-fix available".to_string(),
                        suggestion: check.suggestion.clone().unwrap_or_default(),
                    });
                }
            }

            (fixed, unable_to_fix)
        },
    );

    // Convert im::Vector to Vec at the API boundary
    (
        fixed.into_iter().collect(),
        unable_to_fix.into_iter().collect(),
    )
}

/// Create fix output from results
pub const fn create_output(
    fixed: Vec<FixResult>,
    unable_to_fix: Vec<UnfixableIssue>,
) -> DoctorFixOutput {
    DoctorFixOutput {
        fixed,
        unable_to_fix,
    }
}

/// Fix orphaned workspaces by removing them
fn fix_orphaned_workspaces(check: &DoctorCheck) -> Result<String> {
    let orphaned = check
        .details
        .as_ref()
        .and_then(|d| d.get("orphaned_workspaces"))
        .and_then(|w| w.as_array())
        .ok_or_else(|| anyhow::anyhow!("No orphaned workspaces data"))?;

    let root = jj_root()?;

    let removed_count = orphaned
        .iter()
        .filter_map(|workspace| {
            let name = workspace.as_str()?;

            let result = Command::new("jj")
                .args(["workspace", "forget", name])
                .current_dir(&root)
                .output()
                .ok()?;

            if result.status.success() {
                Some(name)
            } else {
                None
            }
        })
        .count();

    Ok(format!("Removed {removed_count} orphaned workspace(s)"))
}

//! Repository-level health checks
//!
//! This module contains checks for JJ repository state and database integrity.
//! Checks whether the repository is properly configured and accessible.

use std::process::Command;

use im;
use zjj_core::introspection::{CheckStatus, DoctorCheck};

use crate::{
    cli::{is_jj_repo, jj_root},
    commands::get_session_db,
};

/// Check if current directory is a JJ repository
pub fn check_jj_repo() -> DoctorCheck {
    // Handle the result without unwrap - convert error to false for graceful fallback
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
            Some("Initialize JJ: jjz init or jj git init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check state database health
pub async fn check_state_db() -> DoctorCheck {
    match get_session_db().await {
        Err(_) => DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: "State database not accessible".to_string(),
            suggestion: Some("Initialize jjz: jjz init".to_string()),
            auto_fixable: false,
            details: None,
        },
        Ok(db) => match db.list(None).await {
            Ok(sessions) => DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Pass,
                message: format!("state.db is healthy ({} sessions)", sessions.len()),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            Err(e) => DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Warn,
                message: format!("Database exists but error reading: {e}"),
                suggestion: Some("Database may be corrupted".to_string()),
                auto_fixable: false,
                details: None,
            },
        },
    }
}

/// Check for orphaned workspaces
pub async fn check_orphaned_workspaces() -> DoctorCheck {
    // Get list of JJ workspaces
    let jj_workspaces = get_jj_workspaces().unwrap_or_default();

    // Get list of sessions from DB
    let session_names = get_session_names().await.unwrap_or_default();

    // Find workspaces without sessions (excluding 'default')
    let orphaned: im::Vector<_> = jj_workspaces
        .into_iter()
        .filter(|ws| ws != "default" && !session_names.contains(ws))
        .collect();

    if orphaned.is_empty() {
        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Pass,
            message: "No orphaned workspaces found".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }
    } else {
        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "Found {} workspace(s) without session records",
                orphaned.len()
            ),
            suggestion: Some("Run 'jjz doctor --fix' to clean up".to_string()),
            auto_fixable: true,
            details: Some(serde_json::json!({
                "orphaned_workspaces": orphaned,
            })),
        }
    }
}

/// Get list of JJ workspaces from the repository
fn get_jj_workspaces() -> Option<Vec<String>> {
    jj_root().ok().and_then(|root| {
        let output = Command::new("jj")
            .args(["workspace", "list"])
            .current_dir(&root)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let workspaces = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                // Parse workspace list output and normalize:
                // JJ outputs "workspace:" format, we need to strip the trailing colon
                line.split_whitespace()
                    .next()
                    .map(|ws| ws.trim_end_matches(':').to_string())
            })
            .collect::<Vec<_>>();

        Some(workspaces)
    })
}

/// Get session names from the database
async fn get_session_names() -> Option<im::Vector<String>> {
    get_session_db()
        .await
        .ok()
        .and_then(|db| {
            // Use tokio::spawn to run the async operation properly
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(db.list(None))
            })
            .ok()
        })
        .map(|sessions| {
            sessions
                .into_iter()
                .map(|s| s.name)
                .collect::<im::Vector<_>>()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_jj_repo_returns_valid_check() {
        let check = check_jj_repo();
        assert_eq!(check.name, "JJ Repository");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Fail);
        assert!(!check.message.is_empty());
    }

    #[test]
    async fn test_check_state_db_returns_valid_check() {
        let check = check_state_db().await;
        assert_eq!(check.name, "State Database");
        assert!(
            check.status == CheckStatus::Pass
                || check.status == CheckStatus::Warn
                || check.status == CheckStatus::Fail
        );
        assert!(!check.message.is_empty());
    }

    #[test]
    async fn test_check_orphaned_workspaces_returns_valid_check() {
        let check = check_orphaned_workspaces().await;
        assert_eq!(check.name, "Orphaned Workspaces");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Warn);
        assert!(!check.message.is_empty());
    }
}

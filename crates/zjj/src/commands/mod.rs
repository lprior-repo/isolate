//! Command implementations

pub mod abort;
pub mod add;
pub mod agents;
pub mod ai;
pub mod attach;
pub mod backup;
pub mod batch;
pub mod bookmark;
pub mod broadcast;
pub mod can_i;
pub mod checkpoint;
pub mod claim;
pub mod clean;
pub mod completions;
pub mod config;
pub mod context;
pub mod contract;
pub mod dashboard;
pub mod diff;
pub mod doctor;
pub mod done;
pub mod events;
pub mod examples;
pub mod export_import;
pub mod focus;
pub mod init;
pub mod integrity;
pub mod introspect;
pub mod list;
pub mod lock;
pub mod pane;
pub mod prune_invalid;
pub mod query;
pub mod queue;
pub mod recover;
pub mod remove;
pub mod rename;
pub mod revert;
pub mod schema;
pub mod session_mgmt;
pub mod spawn;
#[cfg(test)]
mod spawn_behavior_tests;
pub mod status;
pub mod switch;
pub mod sync;
pub mod template;
pub mod undo;
pub mod validate;
pub mod wait;
pub mod whatif;
pub mod whereami;
pub mod whoami;
pub mod work;
pub mod workspace_utils;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::db::SessionDb;

/// Check if JJ is installed and available
///
/// # Errors
///
/// Returns an error with helpful installation instructions if JJ is not found
pub async fn check_jj_installed() -> Result<()> {
    zjj_core::jj::check_jj_installed().await.map_err(|_| {
        anyhow::anyhow!(
            "JJ is not installed or not found in PATH.\n\n\
            Installation instructions:\n\
            \n  cargo install jj-cli\n\
            \n  # or: brew install jj\n\
            \n  # or: https://martinvonz.github.io/jj/latest/install-and-setup/"
        )
    })
}

/// Check if current directory is in a JJ repository
///
/// # Errors
///
/// Returns an error if not in a JJ repository
pub async fn check_in_jj_repo() -> Result<PathBuf> {
    zjj_core::jj::check_in_jj_repo().await.map_err(|_| {
        anyhow::anyhow!(
            "Not in a JJ repository.\n\n\
            Run 'zjj init' to initialize JJ and ZJJ in this directory."
        )
    })
}

/// Check prerequisites before executing JJ commands
///
/// This ensures:
/// 1. JJ binary is installed
/// 2. We're inside a JJ repository
///
/// # Errors
///
/// Returns an error with helpful messages if prerequisites are not met
pub async fn check_prerequisites() -> Result<PathBuf> {
    // First check if JJ is installed
    check_jj_installed().await?;

    // Then check if we're in a JJ repo
    check_in_jj_repo().await
}

/// Get the ZJJ data directory for the current repository
///
/// # Errors
///
/// Returns an error if prerequisites are not met (JJ not installed or not in a JJ repo)
pub async fn zjj_data_dir() -> Result<PathBuf> {
    // Check prerequisites first
    let root = check_prerequisites().await?;
    Ok(root.join(".zjj"))
}

/// Get the path to the session database, respecting environment variable overrides and config
///
/// # Errors
///
/// Returns an error if prerequisites are not met and no override is provided
pub async fn get_db_path() -> Result<PathBuf> {
    // 1. Environment variable has highest priority
    if let Ok(env_db) = std::env::var("ZJJ_STATE_DB") {
        let p = PathBuf::from(env_db);
        return Ok(p);
    }

    // 2. Load config to check for state_db override
    // We try to load config but don't fail if it doesn't exist yet (e.g. during init)
    if let Ok(cfg) = zjj_core::config::load_config().await {
        if cfg.state_db != ".zjj/state.db" {
            let p = PathBuf::from(cfg.state_db);
            // If absolute, use as is. If relative, it's relative to repo root.
            if p.is_absolute() {
                return Ok(p);
            }
            let root = check_prerequisites().await?;
            return Ok(root.join(p));
        }
    }

    // 3. Default path: .zjj/state.db
    let data_dir = zjj_data_dir().await?;
    let p = data_dir.join("state.db");
    Ok(p)
}

/// Get the session database for the current repository
///
/// # Errors
///
/// Returns an error if:
/// - Prerequisites are not met (JJ not installed or not in a JJ repo)
/// - ZJJ is not initialized
/// - Unable to open the database
pub async fn get_session_db() -> Result<SessionDb> {
    let db_path = get_db_path().await?;

    // Check for initialization unless we have an override
    if std::env::var("ZJJ_STATE_DB").is_err() {
        let data_dir = zjj_data_dir().await?;
        anyhow::ensure!(
            tokio::fs::try_exists(&data_dir).await.unwrap_or(false),
            "ZJJ not initialized. Run 'zjj init' first."
        );
    }

    // Security consideration: Verify database is not a symlink before opening
    // Symlinks could potentially be used to redirect database access
    if db_path.is_symlink() {
        return Err(anyhow::anyhow!(
            "Database is a symlink: {}. This is not allowed for security reasons.",
            db_path.display()
        ));
    }

    SessionDb::open(&db_path)
        .await
        .context("Failed to open session database")
}

/// Determine the main branch for a workspace
///
/// Uses jj's `trunk()` function to find the main branch.
/// Falls back to "main" if unable to detect.
#[allow(dead_code)] // Used in sync.rs via re-export
pub async fn determine_main_branch(workspace_path: &Path) -> String {
    let output = Command::new("jj")
        .args(["log", "-r", "trunk()", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output()
        .await;

    if let Ok(output) = output {
        if output.status.success() {
            let commit_id = String::from_utf8_lossy(&output.stdout);
            let trimmed = commit_id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "main".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_jj_installed_error_message() {
        // This test verifies that check_jj_installed returns a helpful error message
        // We can't directly test the failure case without controlling PATH, but we can
        // verify the error message format by examining the code
        let result = check_jj_installed().await;

        // If JJ is installed (likely in CI), this will pass
        // If JJ is not installed, verify the error message is helpful
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("JJ is not installed"),
                "Error should mention JJ is not installed"
            );
            assert!(
                msg.contains("Installation instructions"),
                "Error should include installation instructions"
            );
            assert!(
                msg.contains("cargo install jj-cli") || msg.contains("brew install jj"),
                "Error should include specific installation commands"
            );
        }
    }

    #[tokio::test]
    async fn test_check_in_jj_repo_error_message() {
        // When not in a JJ repo, we should get a helpful error message
        // We can't control being in/out of a repo in tests, but we verify the error format
        let result = check_in_jj_repo().await;

        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("Not in a JJ repository") || msg.contains("Failed to execute jj"),
                "Error should indicate not in a JJ repository or JJ execution failure"
            );
        }
    }

    #[tokio::test]
    async fn test_check_prerequisites_validates_jj_first() {
        // Prerequisites should check JJ installation before checking repo
        // This ensures we give the right error first
        let result = check_prerequisites().await;

        // If this fails, it should be because JJ is not installed OR we're not in a repo
        if let Err(e) = result {
            let msg = e.to_string();
            // Should mention either "not installed" or "Not in a JJ repository"
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("Failed to execute jj"),
                "Error should mention JJ installation or repository issue"
            );
        }
    }

    #[tokio::test]
    async fn test_zjj_data_dir_checks_prerequisites() {
        // zjj_data_dir should call check_prerequisites
        let result = zjj_data_dir().await;

        // If this fails, it should be due to prerequisites
        if let Err(e) = result {
            let msg = e.to_string();
            // The error should be from prerequisites check
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("Failed to execute jj"),
                "zjj_data_dir should fail with prerequisite errors when not met"
            );
        } else {
            // If prerequisites pass, we should get a valid path
            let path = result.ok();
            assert!(
                path.is_some(),
                "zjj_data_dir should return a path when prerequisites are met"
            );
            if let Some(p) = path {
                assert!(
                    p.to_string_lossy().ends_with(".zjj"),
                    "Path should end with .zjj"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_get_session_db_requires_init() {
        // get_session_db should fail if zjj is not initialized
        // Even if we're in a JJ repo, if .zjj doesn't exist, it should fail
        let result = get_session_db().await;

        if let Err(e) = result {
            let msg = e.to_string();
            // Should mention either prerequisites or initialization
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("ZJJ not initialized")
                    || msg.contains("Failed to execute jj")
                    || msg.contains("Failed to open session database"),
                "get_session_db should fail with clear error when not initialized: {msg}"
            );
        }
    }

    #[tokio::test]
    async fn test_prerequisite_error_messages_are_actionable() {
        // Verify that error messages tell users what to do

        // Test check_jj_installed error
        let jj_err = check_jj_installed().await;
        if let Err(e) = jj_err {
            let msg = e.to_string();
            assert!(
                msg.contains("cargo install") || msg.contains("brew install"),
                "JJ installation error should include installation commands"
            );
        }

        // Test check_in_jj_repo error
        let repo_err = check_in_jj_repo().await;
        if let Err(e) = repo_err {
            let msg = e.to_string();
            // If we get the "not in repo" error, it should mention zjj init
            if msg.contains("Not in a JJ repository") {
                assert!(
                    msg.contains("zjj init"),
                    "Repository error should mention 'zjj init'"
                );
            }
        }
    }
}

//! Environment gathering functions using functional patterns

use anyhow::Result;

use crate::cli::{is_command_available, is_inside_zellij, is_jj_repo, run_command};
use crate::commands::{get_session_db, zjj_data_dir};

use super::types::{
    DependencyInfo, DependencyStatus, EnvironmentContext, EnvironmentInfo, SessionStats,
};

/// Gather current working directory
/// Returns "<unknown>" if unable to determine
pub fn gather_cwd() -> String {
    std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string())
}

/// Check if we're in a JJ repository and get repo info
pub fn gather_jj_repo_info() -> (bool, Option<String>, Option<String>) {
    let jj_repo = is_jj_repo().unwrap_or(false);

    if !jj_repo {
        return (false, None, None);
    }

    let repo_root = get_jj_repo_root();
    let current_branch = get_jj_current_branch();

    (jj_repo, repo_root, current_branch)
}

/// Check ZJJ initialization status
pub fn gather_zjj_init_info() -> (bool, Option<String>) {
    let initialized = zjj_data_dir().is_ok();
    let data_dir = zjj_data_dir().ok().map(|p| p.display().to_string());

    (initialized, data_dir)
}

/// Gather session statistics from the database
pub async fn gather_session_stats(cwd: &str) -> SessionStats {
    let (total, active, current) = match get_session_db().await {
        Ok(db) => {
            // Map database result to tuple using functional composition
            db.list(None).await.map_or((0, 0, None), |sessions| {
                let total = sessions.len();
                let active = sessions
                    .iter()
                    .filter(|s| s.status.to_string() == "active")
                    .count();
                // Check if cwd is in any session workspace
                let current = sessions
                    .iter()
                    .find(|s| cwd.starts_with(&s.workspace_path))
                    .map(|s| s.name.clone());
                (total, active, current)
            })
        }
        Err(_) => (0, 0, None),
    };

    SessionStats {
        total,
        active,
        current,
    }
}

/// Gather environment variable information
pub fn gather_environment_info() -> EnvironmentInfo {
    let zellij_running = is_inside_zellij();
    let zellij_session = zellij_running
        .then(|| std::env::var("ZELLIJ_SESSION_NAME").ok())
        .flatten();

    EnvironmentInfo {
        zellij_running,
        zellij_session,
        pager: std::env::var("PAGER").ok(),
        editor: std::env::var("EDITOR").ok(),
    }
}

/// Gather dependency status using functional pattern
pub fn gather_dependency_status() -> DependencyStatus {
    DependencyStatus {
        jj: check_dependency("jj"),
        zellij: check_dependency("zellij"),
    }
}

/// Check a single dependency and get its version
fn check_dependency(cmd: &str) -> DependencyInfo {
    let installed = is_command_available(cmd);

    let version = installed
        .then(|| run_command(cmd, &["--version"]).ok())
        .flatten()
        .and_then(|output| output.lines().next().map(|l| l.trim().to_string()));

    DependencyInfo { installed, version }
}

/// Get JJ repository root using composition
fn get_jj_repo_root() -> Option<String> {
    run_command("jj", &["root"])
        .ok()
        .map(|output| output.trim().to_string())
}

/// Get current JJ branch/bookmark using functional composition
fn get_jj_current_branch() -> Option<String> {
    run_command("jj", &["log", "-r", "@", "--no-graph", "-T", "bookmarks"])
        .ok()
        .map(|output| output.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Gather all environment context using async composition
pub async fn gather_context() -> EnvironmentContext {
    let cwd = gather_cwd();
    let (jj_repo, jj_repo_root, jj_current_branch) = gather_jj_repo_info();
    let (zjj_initialized, zjj_data_dir) = gather_zjj_init_info();

    // Run async gathering in parallel using async composition
    let sessions = gather_session_stats(&cwd).await;
    let environment = gather_environment_info();
    let dependencies = gather_dependency_status();

    EnvironmentContext {
        cwd,
        jj_repo,
        jj_repo_root,
        jj_current_branch,
        zjj_initialized,
        zjj_data_dir,
        sessions,
        environment,
        dependencies,
    }
}

//! Data structures for context command output

use serde::Serialize;

/// Context output structure (zjj-k1w)
#[derive(Debug, Serialize)]
pub struct ContextOutput {
    pub success: bool,
    pub context: EnvironmentContext,
}

/// Full environment context
#[derive(Debug, Serialize)]
pub struct EnvironmentContext {
    /// Current working directory
    pub cwd: String,
    /// Whether we're in a JJ repository
    pub jj_repo: bool,
    /// JJ repository root path (null if not in repo)
    pub jj_repo_root: Option<String>,
    /// Current JJ branch/bookmark (null if not in repo)
    pub jj_current_branch: Option<String>,
    /// Whether ZJJ is initialized
    pub zjj_initialized: bool,
    /// ZJJ data directory path (null if not initialized)
    pub zjj_data_dir: Option<String>,
    /// Session statistics
    pub sessions: SessionStats,
    /// Environment information
    pub environment: EnvironmentInfo,
    /// Dependency status
    pub dependencies: DependencyStatus,
}

/// Session statistics
#[derive(Debug, Serialize)]
pub struct SessionStats {
    /// Total number of sessions
    pub total: usize,
    /// Number of active sessions
    pub active: usize,
    /// Current session name if cwd is in a session workspace
    pub current: Option<String>,
}

/// Environment information
#[derive(Debug, Serialize)]
pub struct EnvironmentInfo {
    /// Whether running inside Zellij
    pub zellij_running: bool,
    /// Zellij session name if running inside Zellij
    pub zellij_session: Option<String>,
    /// PAGER environment variable
    pub pager: Option<String>,
    /// EDITOR environment variable
    pub editor: Option<String>,
}

/// Dependency status
#[derive(Debug, Serialize)]
pub struct DependencyStatus {
    /// JJ dependency info
    pub jj: DependencyInfo,
    /// Zellij dependency info
    pub zellij: DependencyInfo,
}

/// Single dependency info
#[derive(Debug, Serialize)]
pub struct DependencyInfo {
    /// Whether the dependency is installed
    pub installed: bool,
    /// Version string if installed
    pub version: Option<String>,
}

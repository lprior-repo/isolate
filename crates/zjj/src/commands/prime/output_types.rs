//! Output type definitions for prime command
//!
//! This module defines all the data structures used to serialize
//! prime context output to JSON or markdown formats.

use serde::Serialize;

/// Prime output for JSON mode
#[derive(Debug, Serialize)]
pub struct PrimeOutput {
    pub jj_status: JjStatus,
    pub zjj_status: ZjjStatus,
    pub sessions: Vec<SessionInfo>,
    pub commands: CommandCategories,
    pub beads_status: BeadsStatus,
    pub workflows: Vec<WorkflowSection>,
}

/// JJ repository status
#[derive(Debug, Serialize)]
pub struct JjStatus {
    pub in_repo: bool,
    pub repo_root: Option<String>,
    pub current_bookmark: Option<String>,
    pub has_changes: bool,
    pub change_summary: Option<String>,
}

/// ZJJ initialization status
#[derive(Debug, Serialize)]
pub struct ZjjStatus {
    pub initialized: bool,
    pub data_dir: Option<String>,
    pub total_sessions: usize,
    pub active_sessions: usize,
}

/// Session information
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub name: String,
    pub status: String,
    pub workspace_path: String,
    pub zellij_tab: String,
}

/// Commands organized by category
#[derive(Debug, Serialize)]
pub struct CommandCategories {
    pub session_lifecycle: Vec<CommandRef>,
    pub workspace_sync: Vec<CommandRef>,
    pub system: Vec<CommandRef>,
    pub introspection: Vec<CommandRef>,
    pub utilities: Vec<CommandRef>,
}

/// Single command reference
#[derive(Debug, Serialize)]
pub struct CommandRef {
    pub name: String,
    pub description: String,
}

/// Beads integration status
#[derive(Debug, Serialize)]
pub struct BeadsStatus {
    pub available: bool,
    pub beads_dir: Option<String>,
    pub command_available: bool,
}

/// Workflow section
#[derive(Debug, Serialize)]
pub struct WorkflowSection {
    pub title: String,
    pub steps: Vec<String>,
}

//! Type definitions for the dashboard
//!
//! Contains all core types used throughout the dashboard module,
//! including session data, app state, and dialog types.

use std::time::Instant;

use zjj_core::watcher::BeadsStatus;

use crate::session::Session;

/// Session data enriched with JJ changes and beads counts
#[derive(Debug, Clone)]
pub struct SessionData {
    pub session: Session,
    pub changes: Option<usize>,
    pub beads: BeadsStatus,
}

/// Dashboard application state
#[derive(Debug)]
pub struct DashboardApp {
    /// All session data grouped by status (Creating, Active, Paused, Completed, Failed)
    pub sessions_by_status: Vec<Vec<SessionData>>,
    /// Currently selected column (0=Creating, 1=Active, 2=Paused, 3=Completed, 4=Failed)
    pub selected_column: usize,
    /// Currently selected row within the column
    pub selected_row: usize,
    /// Terminal width for responsive layout
    pub terminal_width: u16,
    /// Last time data was refreshed
    pub last_update: Instant,
    /// Whether to quit the application
    pub should_quit: bool,
    /// Confirmation dialog state
    pub confirm_dialog: Option<ConfirmDialog>,
    /// Input dialog state
    pub input_dialog: Option<InputDialog>,
}

/// Confirmation dialog for destructive actions
#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub message: String,
    pub action: ConfirmAction,
}

/// Action to perform on confirmation
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    RemoveSession(String),
}

/// Input dialog for adding new sessions
#[derive(Debug, Clone)]
pub struct InputDialog {
    pub prompt: String,
    pub input: String,
    pub action: InputAction,
}

/// Action to perform with input
#[derive(Debug, Clone)]
pub enum InputAction {
    AddSession,
}

/// Column titles for the kanban board
pub const COLUMN_TITLES: [&str; 5] = ["Creating", "Active", "Paused", "Completed", "Failed"];

/// Number of columns in the kanban board
pub const COLUMN_COUNT: usize = 5;

/// Minimum terminal width for horizontal layout
pub const WIDE_TERMINAL_THRESHOLD: u16 = 120;

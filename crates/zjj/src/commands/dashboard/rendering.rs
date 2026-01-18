//! UI rendering orchestration for the dashboard
//!
//! Main entry point for rendering the dashboard UI.
//! Delegates to specialized modules for layout, widgets, and formatting.
//!
//! Module structure:
//! - `layout`: Terminal layout calculations and responsive mode selection
//! - `widgets`: Ratatui widget builders for kanban, dialogs, status bar
//! - `formatting`: Text formatting and styling for domain data

use super::layout;
use super::widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::types::DashboardApp;

/// Render the main dashboard UI
///
/// Orchestrates the rendering pipeline:
/// 1. Split frame into main content and status bar areas
/// 2. Render kanban board in main area with responsive layout
/// 3. Render status bar with help text
/// 4. Conditionally render active dialogs (input/confirm)
///
/// Pure function: Reads app state, renders widgets.
pub fn render_ui(f: &mut Frame, app: &DashboardApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    // Main content area - kanban board
    layout::render_kanban(f, app, chunks[0]);

    // Status bar
    widgets::render_status_bar(f, app, chunks[1]);

    // Dialogs (functional Option handling - no unwraps)
    if let Some(dialog) = app.input_dialog.as_ref() {
        widgets::render_input_dialog(f, dialog);
    }

    if let Some(dialog) = app.confirm_dialog.as_ref() {
        widgets::render_confirm_dialog(f, dialog);
    }
}

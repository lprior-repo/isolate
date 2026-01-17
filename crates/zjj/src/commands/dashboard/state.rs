//! App state management
//!
//! Handles all state operations for the dashboard including
//! initialization, refresh, navigation, and dialog management.
//!
//! This module separates concerns into:
//! - **Initialization**: `new()` sets up initial app state
//! - **Updates**: `refresh_sessions()` fetches and groups session data
//! - **State Transitions**: Navigation methods that modify selection state
//! - **Event Handling**: Dialog show/hide methods

use std::{path::Path, time::Instant};

use anyhow::{Context, Result};
use zjj_core::watcher::{query_beads_status, BeadsStatus};

use super::types::{
    ConfirmAction, ConfirmDialog, DashboardApp, InputAction, InputDialog, SessionData, COLUMN_COUNT,
};
use crate::{commands::get_session_db, session::SessionStatus};

impl DashboardApp {
    /// Create a new dashboard app
    ///
    /// # Errors
    /// Returns error if terminal size cannot be determined or session refresh fails
    pub async fn new() -> Result<Self> {
        let (width, _) = crossterm::terminal::size().context("Failed to get terminal size")?;

        let mut app = Self {
            sessions_by_status: create_empty_columns(),
            selected_column: 1, // Start on "Active"
            selected_row: 0,
            terminal_width: width,
            last_update: Instant::now(),
            should_quit: false,
            confirm_dialog: None,
            input_dialog: None,
        };

        app.refresh_sessions().await?;
        Ok(app)
    }
}

// ============================================================================
// UPDATE OPERATIONS: Refresh and data grouping
// ============================================================================

impl DashboardApp {
    /// Refresh session data from database
    ///
    /// Fetches all sessions, enriches them with metadata (changes, beads status),
    /// groups them by status, and adjusts selection if needed.
    ///
    /// # Errors
    /// Returns error if database access fails
    pub async fn refresh_sessions(&mut self) -> Result<()> {
        let db = get_session_db().await?;
        let sessions = db.list(None).await?;

        let grouped = group_sessions_by_status(sessions).await?;

        self.sessions_by_status = grouped;
        self.last_update = Instant::now();

        // Adjust selection if out of bounds
        self.adjust_selection();

        Ok(())
    }

    /// Get the currently selected session
    #[must_use]
    pub fn get_selected_session(&self) -> Option<&SessionData> {
        self.sessions_by_status
            .get(self.selected_column)
            .and_then(|column| column.get(self.selected_row))
    }
}

// ============================================================================
// STATE TRANSITIONS: Navigation and selection management
// ============================================================================

impl DashboardApp {
    /// Move selection left (to previous status column)
    pub fn move_left(&mut self) {
        if self.selected_column > 0 {
            self.selected_column = self.selected_column.saturating_sub(1);
            self.adjust_selection();
        }
    }

    /// Move selection right (to next status column)
    pub fn move_right(&mut self) {
        if self.selected_column < 4 {
            self.selected_column = self.selected_column.saturating_add(1);
            self.adjust_selection();
        }
    }

    /// Move selection up (to previous row in column)
    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row = self.selected_row.saturating_sub(1);
        }
    }

    /// Move selection down (to next row in column)
    pub fn move_down(&mut self) {
        let max_row = self
            .sessions_by_status
            .get(self.selected_column)
            .map_or(0, |column| column.len().saturating_sub(1));

        if self.selected_row < max_row {
            self.selected_row = self.selected_row.saturating_add(1);
        }
    }

    /// Adjust selection to stay within bounds after column changes
    ///
    /// When moving to a different column, the current row may no longer be valid.
    /// This ensures the row index doesn't exceed the number of items in the new column.
    pub fn adjust_selection(&mut self) {
        let max_row = self
            .sessions_by_status
            .get(self.selected_column)
            .map_or(0, |column| column.len().saturating_sub(1));

        if self.selected_row > max_row {
            self.selected_row = max_row;
        }
    }
}

// ============================================================================
// EVENT HANDLING: Dialog management
// ============================================================================

impl DashboardApp {
    /// Show dialog to add a new session
    ///
    /// Creates and displays an input dialog for entering a session name.
    pub fn show_add_dialog(&mut self) {
        self.input_dialog = Some(InputDialog {
            prompt: "Enter session name:".to_string(),
            input: String::new(),
            action: InputAction::AddSession,
        });
    }

    /// Show dialog to confirm session removal
    ///
    /// Creates and displays a confirmation dialog before removing a session.
    pub fn show_remove_dialog(&mut self, name: String) {
        self.confirm_dialog = Some(ConfirmDialog {
            message: format!("Remove session '{name}'?"),
            action: ConfirmAction::RemoveSession(name),
        });
    }
}

// ============================================================================
// UPDATE HELPERS: Data enrichment and grouping
// ============================================================================

/// Group sessions by status using functional composition
///
/// Enriches all sessions concurrently, then groups them by status using
/// a functional fold. This separates the concerns of data enrichment and
/// data organization.
///
/// # Errors
/// Returns error if database access fails
async fn group_sessions_by_status(
    sessions: Vec<crate::session::Session>,
) -> Result<Vec<Vec<SessionData>>> {
    // Functional pipeline: Enrich all sessions concurrently, then group by status
    let enriched_sessions =
        futures::future::join_all(sessions.into_iter().map(enrich_session_with_metadata)).await;

    // Functional fold: Group enriched sessions by status using iterator combinators
    let grouped =
        enriched_sessions
            .into_iter()
            .fold(create_empty_columns(), |mut acc, session_data| {
                let column_idx = status_to_column_index(&session_data.session.status);
                // Safe: column_idx is guaranteed to be < COLUMN_COUNT by exhaustive match
                if let Some(column) = acc.get_mut(column_idx) {
                    column.push(session_data);
                }
                acc
            });

    Ok(grouped)
}

/// Enrich session with workspace changes and beads status
///
/// Pure functional transformation: Session -> `SessionData`
/// Uses Railway-Oriented Programming for error handling.
///
/// Queries the workspace for uncommitted changes and checks beads
/// status, providing sensible defaults if operations fail.
async fn enrich_session_with_metadata(session: crate::session::Session) -> SessionData {
    let workspace_path = Path::new(&session.workspace_path);

    // Functional pipeline: Query changes using Option combinators
    let changes = workspace_path
        .exists()
        .then(|| zjj_core::jj::workspace_status(workspace_path).ok())
        .flatten()
        .map(|status| status.change_count());

    // Railway-Oriented: Handle error case by providing default
    let beads = query_beads_status(workspace_path)
        .await
        .unwrap_or(BeadsStatus::NoBeads);

    SessionData {
        session,
        changes,
        beads,
    }
}

/// Convert session status to column index
///
/// Exhaustive match ensures all statuses map to valid column indices.
/// This is a pure mapping function with no side effects.
const fn status_to_column_index(status: &SessionStatus) -> usize {
    match status {
        SessionStatus::Creating => 0,
        SessionStatus::Active => 1,
        SessionStatus::Paused => 2,
        SessionStatus::Completed => 3,
        SessionStatus::Failed => 4,
    }
}

/// Create empty column structure for session grouping
///
/// Pure function: Always returns `COLUMN_COUNT` empty vectors.
/// Used to initialize the session grouping structure.
fn create_empty_columns() -> Vec<Vec<SessionData>> {
    (0..COLUMN_COUNT).map(|_| Vec::new()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_column_mapping() {
        assert_eq!(status_to_column_index(&SessionStatus::Creating), 0);
        assert_eq!(status_to_column_index(&SessionStatus::Active), 1);
        assert_eq!(status_to_column_index(&SessionStatus::Paused), 2);
        assert_eq!(status_to_column_index(&SessionStatus::Completed), 3);
        assert_eq!(status_to_column_index(&SessionStatus::Failed), 4);
    }

    #[test]
    fn test_column_navigation() {
        let mut selected_column = 1;

        // Move right
        if selected_column < 4 {
            selected_column += 1;
        }
        assert_eq!(selected_column, 2);

        // Move right again
        if selected_column < 4 {
            selected_column += 1;
        }
        assert_eq!(selected_column, 3);

        // Move left
        if selected_column > 0 {
            selected_column -= 1;
        }
        assert_eq!(selected_column, 2);
    }

    #[test]
    fn test_row_navigation_bounds() {
        let sessions_in_column: usize = 5;
        let mut selected_row = 0;

        // Move down
        let max_row = sessions_in_column.saturating_sub(1);
        if selected_row < max_row {
            selected_row += 1;
        }
        assert_eq!(selected_row, 1);

        // Try to move down past bounds
        selected_row = 4;
        if selected_row < max_row {
            selected_row += 1;
        }
        assert_eq!(selected_row, 4); // Should not exceed max_row

        // Move up
        selected_row = selected_row.saturating_sub(1);
        assert_eq!(selected_row, 3);
    }

    #[test]
    fn test_create_empty_columns() {
        let columns = create_empty_columns();
        assert_eq!(columns.len(), COLUMN_COUNT);
        assert!(columns.iter().all(Vec::is_empty));
    }
}

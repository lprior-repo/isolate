//! Layout and geometry calculations for the dashboard UI
//!
//! Handles terminal layout calculations, including:
//! - Responsive layout selection (horizontal vs vertical)
//! - Column layout for kanban board
//! - Centered dialog positioning
//! - Terminal width-based mode selection

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::types::{DashboardApp, COLUMN_TITLES, WIDE_TERMINAL_THRESHOLD};
use crate::commands::dashboard::widgets;

/// Render kanban board with responsive layout selection
///
/// Chooses horizontal or vertical layout based on terminal width.
/// Pure function: Reads app state, dispatches to appropriate layout.
pub fn render_kanban(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let is_wide = area.width >= WIDE_TERMINAL_THRESHOLD;

    if is_wide {
        render_kanban_horizontal(f, app, area);
    } else {
        render_kanban_vertical(f, app, area);
    }
}

/// Render kanban board horizontally (5 equal columns for wide screens)
///
/// Creates a 5-column equal-width layout for terminal width >= 120.
/// Pure function: Transforms area into column rects, delegates rendering.
fn render_kanban_horizontal(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Functional iteration: Render each column using iterator combinators
    COLUMN_TITLES
        .iter()
        .enumerate()
        .for_each(|(idx, title)| widgets::render_column(f, app, columns[idx], idx, title));
}

/// Render kanban board vertically (single column for narrow screens)
///
/// Shows only the currently selected column for terminal width < 120.
/// Pure function: Selects column based on app state.
fn render_kanban_vertical(f: &mut Frame, app: &DashboardApp, area: Rect) {
    // Safe column title lookup using get() instead of indexing
    let title = get_column_title(app.selected_column);
    widgets::render_column(f, app, area, app.selected_column, title);
}

/// Get column title by index, with safe fallback
///
/// Pure function: Always returns a valid &str without panicking
pub fn get_column_title(column_idx: usize) -> &'static str {
    COLUMN_TITLES.get(column_idx).copied().unwrap_or("Unknown")
}

/// Create a centered rectangle for dialogs
///
/// Pure function: Calculates centered Rect using safe arithmetic.
/// Uses saturating arithmetic to prevent overflow on small terminals.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_y).saturating_div(2)),
            Constraint::Percentage(percent_y),
            Constraint::Percentage(100u16.saturating_sub(percent_y).saturating_div(2)),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100u16.saturating_sub(percent_x).saturating_div(2)),
            Constraint::Percentage(percent_x),
            Constraint::Percentage(100u16.saturating_sub(percent_x).saturating_div(2)),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let full_area = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(50, 50, full_area);

        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }

    #[test]
    fn test_layout_mode_selection() {
        // Wide screen
        let wide_width = 120;
        let is_wide = wide_width >= WIDE_TERMINAL_THRESHOLD;
        assert!(is_wide);

        // Narrow screen
        let narrow_width = 80;
        let is_narrow = narrow_width < WIDE_TERMINAL_THRESHOLD;
        assert!(is_narrow);
    }

    #[test]
    fn test_get_column_title_safe() {
        // Valid indices
        assert_eq!(get_column_title(0), "Creating");
        assert_eq!(get_column_title(1), "Active");
        assert_eq!(get_column_title(4), "Failed");

        // Out of bounds - safe fallback
        assert_eq!(get_column_title(10), "Unknown");
        assert_eq!(get_column_title(100), "Unknown");
    }
}

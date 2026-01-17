//! Text formatting and styling for dashboard display
//!
//! Handles conversion of domain data (sessions, beads) into styled UI text.
//! Pure functions: No side effects, functional Option/enum handling.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use zjj_core::watcher::BeadsStatus;

use super::types::SessionData;

/// Format a session as a styled list item
///
/// Pure function: SessionData -> ListItem with conditional styling.
/// Combines session name, branch, changes count, and beads status.
pub fn format_session_item(session_data: &SessionData, is_selected: bool) -> ListItem<'_> {
    let session = &session_data.session;

    // Functional Option handling: No unwrap_or, use map_or_else with lazy evaluation
    let changes_str = session_data
        .changes
        .map_or_else(|| "-".to_string(), |c| c.to_string());

    let beads_str = format_beads_status(&session_data.beads);

    // Safe Option handling with unwrap_or_else providing default
    let branch = session.branch.as_deref().unwrap_or("-");

    let line = Line::from(vec![
        Span::styled(
            format!("{:<15}", session.name),
            if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
        Span::raw(format!(" {branch} ")),
        Span::styled(
            format!("Δ{changes_str} "),
            Style::default().fg(Color::Green),
        ),
        Span::styled(format!("B{beads_str}"), Style::default().fg(Color::Blue)),
    ]);

    ListItem::new(line)
}

/// Format beads status for display
///
/// Pure function: BeadsStatus -> String transformation.
/// Maps enum variants to human-readable strings.
pub fn format_beads_status(beads: &BeadsStatus) -> String {
    match beads {
        BeadsStatus::NoBeads => "-".to_string(),
        BeadsStatus::Counts {
            open,
            in_progress,
            blocked,
            ..
        } => format!("{open}/{in_progress}/{blocked}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_status_formatting() {
        let beads = BeadsStatus::Counts {
            open: 5,
            in_progress: 3,
            blocked: 2,
            closed: 10,
        };

        let formatted = format_beads_status(&beads);
        assert_eq!(formatted, "5/3/2");

        let no_beads = BeadsStatus::NoBeads;
        let formatted_none = format_beads_status(&no_beads);
        assert_eq!(formatted_none, "-");
    }
}

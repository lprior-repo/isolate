//! Widget builders for the dashboard UI
//!
//! Constructs ratatui widgets (List, Paragraph, Block) with appropriate
//! styling and content. All functions are pure with no side effects.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::formatting;
use super::layout;
use super::types::{ConfirmDialog, DashboardApp, InputDialog, SessionData};

/// Render a single kanban column as a List widget
///
/// Pure function: Creates styled List widget with session items.
/// Highlights the column border if it's the selected column.
pub fn render_column(
    f: &mut Frame,
    app: &DashboardApp,
    area: Rect,
    column_idx: usize,
    title: &str,
) {
    let sessions = app
        .sessions_by_status
        .get(column_idx)
        .map_or(&[] as &[SessionData], |s| s.as_slice());

    let is_selected = column_idx == app.selected_column;

    // Functional pipeline: Transform sessions into ListItems
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(idx, session_data)| {
            let is_row_selected = is_selected && idx == app.selected_row;
            formatting::format_session_item(session_data, is_row_selected)
        })
        .collect();

    let border_style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ({}) ", title, sessions.len()))
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(list, area);
}

/// Render status bar with help text and last update time
///
/// Pure function: Creates Paragraph widget with keybinding help.
/// Displays navigation hints and last refresh timestamp.
pub fn render_status_bar(f: &mut Frame, app: &DashboardApp, area: Rect) {
    let help_text = vec![
        Span::raw("hjkl/arrows:"),
        Span::styled(" navigate ", Style::default().fg(Color::Gray)),
        Span::raw("Enter:"),
        Span::styled(" focus ", Style::default().fg(Color::Gray)),
        Span::raw("d:"),
        Span::styled(" delete ", Style::default().fg(Color::Gray)),
        Span::raw("a:"),
        Span::styled(" add ", Style::default().fg(Color::Gray)),
        Span::raw("r:"),
        Span::styled(" refresh ", Style::default().fg(Color::Gray)),
        Span::raw("q:"),
        Span::styled(" quit ", Style::default().fg(Color::Gray)),
        Span::raw(format!(
            "| Last update: {:?} ago",
            app.last_update.elapsed()
        )),
    ];

    let paragraph = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL).title(" Help "));

    f.render_widget(paragraph, area);
}

/// Render input dialog for user text entry
///
/// Pure function: Creates centered dialog widget for input.
/// Displays prompt and styled text input area.
pub fn render_input_dialog(f: &mut Frame, dialog: &InputDialog) {
    let area = layout::centered_rect(60, 20, f.area());

    let text = vec![
        Line::from(dialog.prompt.as_str()),
        Line::from(""),
        Line::from(Span::styled(
            &dialog.input,
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

/// Render confirmation dialog for destructive actions
///
/// Pure function: Creates centered confirmation dialog widget.
/// Displays message and instructions (Y to confirm, N to cancel).
pub fn render_confirm_dialog(f: &mut Frame, dialog: &ConfirmDialog) {
    let area = layout::centered_rect(60, 20, f.area());

    let text = vec![
        Line::from(dialog.message.as_str()),
        Line::from(""),
        Line::from(Span::styled(
            "Press Y to confirm, N to cancel",
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(paragraph, area);
}

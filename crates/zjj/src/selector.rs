//! Interactive session selector TUI
//!
//! Provides a simple list selection interface for choosing a session
//! when no name is provided on the command line.

use std::{
    io::{self, Stdout},
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use crate::session::{Session, SessionStatus};

/// Select a session interactively from a list
pub async fn select_session(sessions: &[Session]) -> Result<Option<Session>> {
    if sessions.is_empty() {
        return Ok(None);
    }

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Run app
    let result = run_selector(&mut terminal, sessions).await;

    // Restore terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    result
}

struct SelectorState {
    list_state: ListState,
    selected_index: usize,
}

async fn run_selector(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    sessions: &[Session],
) -> Result<Option<Session>> {
    let mut state = SelectorState {
        list_state: ListState::default(),
        selected_index: 0,
    };
    state.list_state.select(Some(0));

    loop {
        terminal.draw(|f| draw_ui(f, sessions, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        return Ok(Some(sessions[state.selected_index].clone()));
                        // Note: Clone necessary here as we need owned Session for return
                        // Future optimization: Consider Arc<Session> for shared ownership
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.selected_index > 0 {
                            state.selected_index -= 1;
                            state.list_state.select(Some(state.selected_index));
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.selected_index < sessions.len() - 1 {
                            state.selected_index += 1;
                            state.list_state.select(Some(state.selected_index));
                        }
                    }
                    _ => {}
                }
            }
        }
        // Small sleep to avoid busy-waiting if poll returns immediately
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn draw_ui(f: &mut Frame, sessions: &[Session], state: &mut SelectorState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());

    // Calculate centering for list
    let list_area = centered_rect(60, 60, chunks[0]);

    let items: Vec<ListItem> = sessions
        .iter()
        .map(|session| {
            let status_color = match session.status {
                SessionStatus::Active => Color::Green,
                SessionStatus::Creating => Color::Yellow,
                SessionStatus::Paused => Color::Blue,
                SessionStatus::Completed => Color::Gray,
                SessionStatus::Failed => Color::Red,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{session.name:<20}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{session.status:10}"),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!(" {}", session.branch.as_deref().unwrap_or("-"))),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select Session "),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Black)
                .bg(Color::White),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, list_area, &mut state.list_state);

    // Help text
    let help_text = vec![
        Span::raw("Navigate: "),
        Span::styled("Up/Down/j/k", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | Select: "),
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | Cancel: "),
        Span::styled("Esc/q", Style::default().add_modifier(Modifier::BOLD)),
    ];

    let help = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(help, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

//! Terminal management
//!
//! Handles terminal setup, event loop, and file watching
//! for the dashboard application.

use std::{
    io::{self, Stdout},
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use zjj_core::{
    config::Config,
    watcher::{FileWatcher, WatchEvent},
};

use super::{events::handle_key_event, rendering::render_ui, types::DashboardApp};
use crate::commands::get_session_db;

/// Setup terminal for TUI rendering
///
/// # Errors
/// Returns error if terminal setup fails
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("Failed to create terminal")
}

/// Cleanup terminal after TUI exits
///
/// # Errors
/// Returns error if terminal cleanup fails
pub fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")
}

/// Main application event loop
///
/// # Errors
/// Returns error if rendering, event handling, or refresh fails
pub async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut DashboardApp,
    watcher_rx: &mut Option<tokio::sync::mpsc::Receiver<WatchEvent>>,
    refresh_interval: Duration,
) -> Result<()> {
    let mut last_refresh = Instant::now();

    loop {
        // Render UI
        terminal.draw(|f| render_ui(f, app))?;

        // Check for quit
        if app.should_quit {
            break;
        }

        // Poll for events with timeout
        let event_result = tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(100)))
            .await
            .context("Failed to join event polling task")?;

        if event_result? {
            let event = tokio::task::spawn_blocking(event::read)
                .await
                .context("Failed to join event reading task")??;

            match event {
                Event::Key(key) => {
                    handle_key_event(app, key).await?;
                }
                Event::Resize(width, height) => {
                    app.terminal_width = width;
                    terminal.resize(Rect::new(0, 0, width, height))?;
                }
                _ => {}
            }
        }

        // Check file watcher
        if let Some(rx) = watcher_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    WatchEvent::BeadsChanged { .. } => {
                        app.refresh_sessions().await?;
                    }
                }
            }
        }

        // Auto-refresh
        if last_refresh.elapsed() >= refresh_interval {
            app.refresh_sessions().await?;
            last_refresh = Instant::now();
        }
    }

    Ok(())
}

/// Setup file watcher for beads database changes
///
/// # Errors
/// Returns error if file watcher setup fails
pub async fn setup_file_watcher(
    config: &Config,
) -> Result<tokio::sync::mpsc::Receiver<WatchEvent>> {
    // Get all workspace paths from sessions
    let db = get_session_db().await?;
    let sessions = db.list(None).await?;

    let workspaces: Vec<PathBuf> = sessions
        .into_iter()
        .map(|s| PathBuf::from(s.workspace_path))
        .collect();

    FileWatcher::watch_workspaces(&config.watch, workspaces).context("Failed to setup file watcher")
}

//! Interactive TUI dashboard with kanban view
//!
//! Displays sessions organized by status in a kanban-style layout with:
//! - Real-time updates from beads database changes
//! - Vim-style keyboard navigation (hjkl)
//! - Session management actions (focus, add, remove)
//! - Responsive layout based on terminal width
//!
//! # Architecture
//!
//! The dashboard module is organized into focused submodules:
//! - `types`: Core type definitions
//! - `state`: Application state management
//! - `rendering`: UI rendering with ratatui
//! - `events`: Keyboard input handling
//! - `actions`: Session operations (focus, add, remove)
//! - `terminal`: Terminal setup and event loop

mod actions;
mod events;
mod formatting;
mod layout;
mod rendering;
mod state;
mod terminal;
mod types;
mod widgets;

use std::time::Duration;

use anyhow::{Context, Result};
use terminal::{cleanup_terminal, run_event_loop, setup_file_watcher, setup_terminal};
use types::DashboardApp;
use zjj_core::config::load_config;

/// Run the interactive dashboard
///
/// # Errors
/// Returns error if:
/// - Not in a JJ repository
/// - Terminal setup fails
/// - Event loop encounters errors
/// - Terminal cleanup fails
pub async fn run() -> Result<()> {
    // Check if we're in a JJ repo
    let _root = crate::cli::jj_root().context("Not in a JJ repository. Run 'jjz init' first.")?;

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Load config
    let config = load_config().context("Failed to load configuration")?;

    // Create app state
    let mut app = DashboardApp::new().await?;

    // Setup file watcher if enabled
    let mut watcher_rx = if config.watch.enabled {
        setup_file_watcher(&config).await.ok()
    } else {
        None
    };

    // Main event loop
    let result = run_event_loop(
        &mut terminal,
        &mut app,
        &mut watcher_rx,
        Duration::from_millis(u64::from(config.dashboard.refresh_ms)),
    )
    .await;

    // Cleanup terminal
    cleanup_terminal(&mut terminal)?;

    result
}

// Re-export types for testing

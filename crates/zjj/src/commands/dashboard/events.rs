//! Event handling
//!
//! Processes keyboard input and manages dialog interactions
//! using functional patterns and proper error handling.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    actions::{add_session, focus_session, remove_session},
    types::{ConfirmAction, ConfirmDialog, DashboardApp, InputAction, InputDialog},
};

/// Handle keyboard input
///
/// # Errors
/// Returns error if any action (focus, add, remove) fails
pub async fn handle_key_event(app: &mut DashboardApp, key: KeyEvent) -> Result<()> {
    // Handle dialogs first
    if let Some(dialog) = app.input_dialog.take() {
        return handle_input_dialog(app, dialog, key).await;
    }

    if let Some(dialog) = app.confirm_dialog.take() {
        return handle_confirm_dialog(app, dialog, key).await;
    }

    // Normal key handling
    handle_normal_key(app, key).await
}

/// Handle normal keyboard input (no active dialogs)
async fn handle_normal_key(app: &mut DashboardApp, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.move_left();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.move_right();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
        }
        KeyCode::Char('r') => {
            app.refresh_sessions().await?;
        }
        KeyCode::Char('a') => {
            app.show_add_dialog();
        }
        KeyCode::Char('d') => {
            if let Some(session) = app.get_selected_session() {
                app.show_remove_dialog(session.session.name.clone());
            }
        }
        KeyCode::Enter => {
            if let Some(session) = app.get_selected_session() {
                focus_session(&session.session).await?;
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        _ => {}
    }

    Ok(())
}

/// Handle input dialog events
///
/// # Errors
/// Returns error if the add session action fails
async fn handle_input_dialog(
    app: &mut DashboardApp,
    mut dialog: InputDialog,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let input = dialog.input.clone();
            let action = dialog.action.clone();

            match action {
                InputAction::AddSession => {
                    if !input.is_empty() {
                        add_session(&input).await?;
                        app.refresh_sessions().await?;
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Dialog already taken, just return
        }
        KeyCode::Char(c) => {
            dialog.input.push(c);
            app.input_dialog = Some(dialog);
        }
        KeyCode::Backspace => {
            dialog.input.pop();
            app.input_dialog = Some(dialog);
        }
        _ => {
            app.input_dialog = Some(dialog);
        }
    }

    Ok(())
}

/// Handle confirmation dialog events
///
/// # Errors
/// Returns error if the remove session action fails
async fn handle_confirm_dialog(
    app: &mut DashboardApp,
    dialog: ConfirmDialog,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Char('y' | 'Y') => match dialog.action {
            ConfirmAction::RemoveSession(name) => {
                remove_session(&name).await?;
                app.refresh_sessions().await?;
            }
        },
        KeyCode::Char('n' | 'N') | KeyCode::Esc => {
            // Dialog already taken, just return
        }
        _ => {
            // Restore dialog if other key pressed
            app.confirm_dialog = Some(dialog);
        }
    }

    Ok(())
}

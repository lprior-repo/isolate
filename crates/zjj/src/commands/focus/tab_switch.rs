//! Tab switching operations for the focus command
//!
//! Handles both inside-Zellij tab switching and outside-Zellij attach operations.
//! Uses Result<T> for all error handling - zero unwrap/panic design.

use anyhow::Result;

use crate::cli::{attach_to_zellij_session, is_inside_zellij, run_command};

/// Result of attempting to switch to a session's tab
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabSwitchResult {
    /// Successfully switched to tab (only possible inside Zellij)
    Switched,
    /// Tab info prepared but not switched (outside Zellij)
    /// User will attach to Zellij and land in session
    Attached,
}

impl TabSwitchResult {
    /// Determine if we actually switched tabs
    pub fn did_switch(&self) -> bool {
        matches!(self, Self::Switched)
    }
}

/// Switch to a session's tab
///
/// Behavior depends on current context:
/// - **Inside Zellij**: Use `zellij action go-to-tab-name` to switch immediately
/// - **Outside Zellij**: Output tab info and attach to Zellij session
///   (user will land in session and can navigate to desired tab)
///
/// # Arguments
/// * `tab_name` - Name of the Zellij tab to switch to
/// * `session_name` - Session name (for display purposes)
///
/// # Returns
/// * `Ok(TabSwitchResult::Switched)` - Successfully switched to tab (inside Zellij)
/// * `Ok(TabSwitchResult::Attached)` - Attached to Zellij session (outside Zellij)
/// * `Err(e)` - If tab switching failed
///
/// # Note on async
/// This function is async to maintain consistent public API contracts with callers,
/// even though the current implementation is synchronous. This follows Railway-Oriented
/// Programming patterns where the interface specifies the effect (async) regardless of
/// internal implementation details.
#[allow(clippy::unused_async)]
pub async fn switch_to_tab(tab_name: &str, session_name: &str) -> Result<TabSwitchResult> {
    if is_inside_zellij() {
        switch_tab_inside_zellij(tab_name, session_name)
    } else {
        switch_tab_outside_zellij(tab_name, session_name)
    }
}

/// Switch to tab from inside Zellij using action command
///
/// Uses `zellij action go-to-tab-name` to navigate to the tab.
fn switch_tab_inside_zellij(tab_name: &str, _session_name: &str) -> Result<TabSwitchResult> {
    run_command("zellij", &["action", "go-to-tab-name", tab_name])?;
    Ok(TabSwitchResult::Switched)
}

/// Prepare for tab switch from outside Zellij by attaching to session
///
/// When outside Zellij, we:
/// 1. Display the tab name
/// 2. Attach to the Zellij session (note: this exec's the process, so we never return)
/// 3. User lands in the session
///
/// Note: This function may not return due to exec into Zellij.
fn switch_tab_outside_zellij(tab_name: &str, session_name: &str) -> Result<TabSwitchResult> {
    println!("Session '{session_name}' is in tab '{tab_name}'");
    println!("Attaching to Zellij session...");

    // This exec's into Zellij - may not return
    attach_to_zellij_session(None)?;

    // If we get here, exec failed
    Ok(TabSwitchResult::Attached)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_switch_result_switched() {
        let result = TabSwitchResult::Switched;
        assert!(result.did_switch());
    }

    #[test]
    fn test_tab_switch_result_attached() {
        let result = TabSwitchResult::Attached;
        assert!(!result.did_switch());
    }
}

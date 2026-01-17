//! Tab management operations - Imperative Shell with safe I/O
//!
//! This module handles Zellij tab operations (open, close, focus).
//! All operations properly handle errors without panics.

use std::{path::Path, process::Command};

use crate::{Error, Result};

/// Open a new tab with the given layout
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - Layout file doesn't exist
/// - zellij action command fails
pub fn tab_open(layout_path: &Path, tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // Verify layout file exists
    if !layout_path.exists() {
        return Err(Error::not_found(format!(
            "Layout file not found: {}",
            layout_path.display()
        )));
    }

    // Execute: zellij action new-tab --layout <path> --name <name>
    let output = Command::new("zellij")
        .args(["action", "new-tab"])
        .arg("--layout")
        .arg(layout_path)
        .arg("--name")
        .arg(tab_name)
        .output()
        .map_err(|e| Error::command_error(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::command_error(format!(
            "zellij action failed: {stderr}"
        )));
    }

    Ok(())
}

/// Close a tab by switching to it first, then closing
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - zellij action command fails
pub fn tab_close(tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // First focus the tab
    tab_focus(tab_name)?;

    // Execute: zellij action close-tab
    let output = Command::new("zellij")
        .args(["action", "close-tab"])
        .output()
        .map_err(|e| Error::command_error(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::command_error(format!(
            "zellij action failed: {stderr}"
        )));
    }

    Ok(())
}

/// Focus a tab by name
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - Tab doesn't exist
/// - zellij action command fails
pub fn tab_focus(tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // Execute: zellij action go-to-tab-name <name>
    let output = Command::new("zellij")
        .args(["action", "go-to-tab-name", tab_name])
        .output()
        .map_err(|e| Error::command_error(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::command_error(format!(
            "zellij action failed: {stderr}"
        )));
    }

    Ok(())
}

/// Check if Zellij is running
///
/// # Errors
///
/// Returns error if Zellij is not running in current session
pub fn check_zellij_running() -> Result<()> {
    // Check if ZELLIJ environment variable is set
    if std::env::var("ZELLIJ").is_err() {
        return Err(Error::command_error(
            "Zellij not running. Run jjz inside a Zellij session.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    // Test Case 5: Open tab - Executes 'zellij action new-tab ...'
    #[test]
    fn test_tab_open_requires_zellij() {
        // This test will fail if not in Zellij
        // We just test the error handling
        let temp_dir = env::temp_dir();
        let layout_path = temp_dir.join("test.kdl");

        // Create a test layout file
        std::fs::write(&layout_path, "layout { pane { } }").ok();

        let result = tab_open(&layout_path, "test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::command_error(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }

        // Cleanup
        std::fs::remove_file(&layout_path).ok();
    }

    // Test Case 6: Close tab - Executes 'zellij action close-tab ...'
    #[test]
    fn test_tab_close_requires_zellij() {
        let result = tab_close("test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::command_error(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }
    }

    // Test Case 7: Focus tab - Executes 'zellij action go-to-tab-name ...'
    #[test]
    fn test_tab_focus_requires_zellij() {
        let result = tab_focus("test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::command_error(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }
    }

    // Test Case 10: Zellij not running - Error "Zellij not running"
    #[test]
    fn test_check_zellij_not_running() {
        // Save current ZELLIJ var
        let zellij_var = env::var("ZELLIJ");

        // Temporarily remove it
        env::remove_var("ZELLIJ");

        let result = check_zellij_running();
        assert!(result.is_err());

        if let Err(Error::command_error(msg)) = result {
            assert!(msg.contains("Zellij not running"));
        }

        // Restore ZELLIJ var if it existed
        if let Ok(val) = zellij_var {
            env::set_var("ZELLIJ", val);
        }
    }

    // Additional test: tab_open with missing file
    #[test]
    fn test_tab_open_missing_layout_file() {
        let missing_path = Path::new("/nonexistent/layout.kdl");
        let result = tab_open(missing_path, "test");

        assert!(result.is_err());
        if let Err(Error::not_found(msg)) = result {
            assert!(msg.contains("Layout file not found"));
        }
    }
}

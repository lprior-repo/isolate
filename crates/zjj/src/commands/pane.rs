#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

//! Pane focus and navigation within Zellij sessions
//!
//! This module provides functionality for:
//! - Focusing specific panes by name or ID
//! - Listing all panes in a session
//! - Cycling to the next pane
//! - Directional navigation (up, down, left, right)

use std::{io::Write, process::Command};

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{cli::run_command, commands::get_session_db, json::output_json_error_and_exit};

/// Direction for pane navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Parse direction from string
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(anyhow::anyhow!(
                "Invalid direction: '{s}'. Valid values: up, down, left, right"
            )),
        }
    }

    /// Get Zellij action argument for this direction
    const fn as_zellij_arg(self) -> &'static str {
        match self {
            Self::Up => "move-focus up",
            Self::Down => "move-focus down",
            Self::Left => "move-focus left",
            Self::Right => "move-focus right",
        }
    }
}

/// Pane information
#[derive(Debug, Clone, Serialize)]
pub struct PaneInfo {
    /// Pane ID (Zellij internal identifier)
    pub id: String,
    /// Pane title/name
    pub title: String,
    /// Whether this pane is currently focused
    pub focused: bool,
    /// Pane command (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Options for pane focus command
#[derive(Debug, Clone)]
pub struct PaneFocusOptions {
    /// Output format
    pub format: OutputFormat,
}

/// Options for pane list command
#[derive(Debug, Clone)]
pub struct PaneListOptions {
    /// Output format
    pub format: OutputFormat,
}

/// Options for pane next command
#[derive(Debug, Clone)]
pub struct PaneNextOptions {
    /// Output format
    pub format: OutputFormat,
}

/// JSON output for pane focus
#[derive(Debug, Serialize)]
struct PaneFocusOutput {
    pub session: String,
    pub pane: String,
    pub message: String,
}

/// JSON output for pane list
#[derive(Debug, Serialize)]
struct PaneListOutput {
    pub session: String,
    pub panes: Vec<PaneInfo>,
    pub focused: Option<String>,
}

/// JSON output for pane next
#[derive(Debug, Serialize)]
struct PaneNextOutput {
    pub session: String,
    pub message: String,
}

/// Check if Zellij is running
fn check_zellij_running() -> Result<()> {
    if std::env::var("ZELLIJ").is_err() {
        return Err(anyhow::anyhow!(
            "Zellij not running. Run zjj inside a Zellij session."
        ));
    }
    Ok(())
}

/// Focus a pane by ID or name in the given session
pub fn pane_focus(
    session_name: &str,
    pane_identifier: Option<&str>,
    options: &PaneFocusOptions,
) -> Result<()> {
    check_zellij_running()?;

    let db = get_session_db()?;
    let session = db
        .get_blocking(session_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

    let zellij_tab = session.zellij_tab;

    // First, focus the session's tab
    run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

    let pane_id = if let Some(pane) = pane_identifier {
        pane.to_string()
    } else {
        // No pane specified, this is an error
        if options.format.is_json() {
            output_json_error_and_exit(&anyhow::anyhow!(
                "Pane identifier is required for focus command"
            ));
        }
        return Err(anyhow::anyhow!(
            "Pane identifier is required. Use 'zjj pane list {session_name}' to see available panes."
        ));
    };

    // Focus the specific pane
    let output = Command::new("zellij")
        .args(["action", "focus-pane-id", &pane_id])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute zellij: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if options.format.is_json() {
            output_json_error_and_exit(&anyhow::anyhow!(
                "Failed to focus pane '{pane_id}': {stderr}"
            ));
        }
        return Err(anyhow::anyhow!(
            "Failed to focus pane '{pane_id}': {stderr}"
        ));
    }

    if options.format.is_json() {
        let output = PaneFocusOutput {
            session: session_name.to_string(),
            pane: pane_id.clone(),
            message: format!("Focused pane '{pane_id}' in session '{session_name}'"),
        };
        let envelope = SchemaEnvelope::new("pane-focus-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(std::io::stdout(), "Focused pane '{pane_id}' in session '{session_name}'")?;
    }

    Ok(())
}

/// List all panes in a session
pub fn pane_list(session_name: &str, options: &PaneListOptions) -> Result<()> {
    check_zellij_running()?;

    let db = get_session_db()?;
    let session = db
        .get_blocking(session_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

    let zellij_tab = session.zellij_tab;

    // Focus the session's tab first
    run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

    // Get pane list from Zellij
    let output = Command::new("zellij")
        .args(["action", "list-pane-ids"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute zellij: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to list panes: {stderr}"));
    }

    let panes_output = String::from_utf8_lossy(&output.stdout);
    let pane_ids: Vec<String> = panes_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();

    // Parse pane info (simplified - actual implementation would get more details)
    let panes: Vec<PaneInfo> = pane_ids
        .iter()
        .enumerate()
        .map(|(index, id)| PaneInfo {
            id: id.clone(),
            title: format!("Pane {}", index + 1),
            focused: index == 0,
            command: None,
        })
        .collect();

    let focused = panes.first().map(|p| p.id.clone());

    if options.format.is_json() {
        let output = PaneListOutput {
            session: session_name.to_string(),
            panes,
            focused,
        };
        let envelope = SchemaEnvelope::new("pane-list-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        let mut stdout = std::io::stdout();
        writeln!(stdout, "Panes in session '{session_name}':")?;
        for pane in &panes {
            let focused_mark = if pane.focused { "*" } else { " " };
            writeln!(stdout, " {focused_mark} {} - {}", pane.id, pane.title)?;
        }
    }

    Ok(())
}

/// Cycle to the next pane in a session
pub fn pane_next(session_name: &str, options: &PaneNextOptions) -> Result<()> {
    check_zellij_running()?;

    let db = get_session_db()?;
    let session = db
        .get_blocking(session_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

    let zellij_tab = session.zellij_tab;

    // Focus the session's tab first
    run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

    // Move focus to next pane
    let output = Command::new("zellij")
        .args(["action", "move-focus-or-tab"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute zellij: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if options.format.is_json() {
            output_json_error_and_exit(&anyhow::anyhow!("Failed to move to next pane: {stderr}"));
        }
        return Err(anyhow::anyhow!("Failed to move to next pane: {stderr}"));
    }

    if options.format.is_json() {
        let output = PaneNextOutput {
            session: session_name.to_string(),
            message: format!("Moved to next pane in session '{session_name}'"),
        };
        let envelope = SchemaEnvelope::new("pane-next-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(std::io::stdout(), "Moved to next pane in session '{session_name}'")?;
    }

    Ok(())
}

/// Navigate in a direction within a session
pub fn pane_navigate(
    session_name: &str,
    direction: Direction,
    options: &PaneFocusOptions,
) -> Result<()> {
    check_zellij_running()?;

    let db = get_session_db()?;
    let session = db
        .get_blocking(session_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;

    let zellij_tab = session.zellij_tab;

    // Focus the session's tab first
    run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

    // Move focus in the specified direction
    let direction_arg = direction.as_zellij_arg();
    let output = Command::new("zellij")
        .args(["action"])
        .arg(direction_arg)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute zellij: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if options.format.is_json() {
            output_json_error_and_exit(&anyhow::anyhow!("Failed to move focus: {stderr}"));
        }
        return Err(anyhow::anyhow!("Failed to move focus: {stderr}"));
    }

    if options.format.is_json() {
        let output = PaneFocusOutput {
            session: session_name.to_string(),
            pane: format!("{direction:?}"),
            message: format!("Moved focus {direction:?} in session '{session_name}'"),
        };
        let envelope = SchemaEnvelope::new("pane-navigate-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(std::io::stdout(), "Moved focus {direction:?} in session '{session_name}'")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Red Queen Attack: Invalid direction input
    // Category: Input Boundary Attacks
    // Severity: P2 (UX issue)
    // Test: Direction::parse should reject invalid inputs with clear error message
    #[test]
    fn test_direction_parse_valid() {
        assert_eq!(Direction::parse("up").unwrap(), Direction::Up);
        assert_eq!(Direction::parse("UP").unwrap(), Direction::Up);
        assert_eq!(Direction::parse("Down").unwrap(), Direction::Down);
        assert_eq!(Direction::parse("LEFT").unwrap(), Direction::Left);
        assert_eq!(Direction::parse("Right").unwrap(), Direction::Right);
    }

    // Red Queen Attack: Diagonal direction (not supported)
    // Category: Input Boundary Attacks
    // Severity: P2 (UX issue)
    // Test: Should return error for unsupported direction
    #[test]
    fn test_direction_parse_invalid() {
        let result = Direction::parse("diagonal");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid direction"));
        assert!(err
            .to_string()
            .contains("Valid values: up, down, left, right"));
    }

    // Red Queen Attack: Empty direction
    // Category: Input Boundary Attacks
    // Severity: P2 (UX issue)
    // Test: Should return error for empty input
    #[test]
    fn test_direction_parse_empty() {
        let result = Direction::parse("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid direction"));
    }

    // Red Queen Attack: Non-standard direction
    // Category: Input Boundary Attacks
    // Severity: P2 (UX issue)
    // Test: Should return clear error message for any non-standard direction
    #[test]
    fn test_direction_parse_non_standard() {
        let result = Direction::parse("sideways");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("sideways"));
        assert!(err.to_string().contains("Valid values"));
    }

    // Red Queen Attack: Direction to Zellij argument mapping
    // Category: Happy Path Verification
    // Severity: P1 (Wrong output)
    // Test: All directions should map to correct Zellij action
    #[test]
    fn test_direction_as_zellij_arg() {
        assert_eq!(Direction::Up.as_zellij_arg(), "move-focus up");
        assert_eq!(Direction::Down.as_zellij_arg(), "move-focus down");
        assert_eq!(Direction::Left.as_zellij_arg(), "move-focus left");
        assert_eq!(Direction::Right.as_zellij_arg(), "move-focus right");
    }

    // Red Queen Attack: Case insensitive direction parsing
    // Category: Happy Path Verification
    // Severity: P2 (UX issue)
    // Test: Users might type uppercase directions
    #[test]
    fn test_direction_case_insensitive() {
        assert_eq!(Direction::parse("UP").unwrap(), Direction::Up);
        assert_eq!(Direction::parse("DOWN").unwrap(), Direction::Down);
        assert_eq!(Direction::parse("LEFT").unwrap(), Direction::Left);
        assert_eq!(Direction::parse("RIGHT").unwrap(), Direction::Right);
        assert_eq!(Direction::parse("Up").unwrap(), Direction::Up);
        assert_eq!(Direction::parse("DoWn").unwrap(), Direction::Down);
    }

    // Red Queen Attack: PaneInfo serialization
    // Category: Output Contract Attacks
    // Severity: P1 (Broken contract)
    // Test: PaneInfo should serialize to valid JSON
    #[test]
    fn test_pane_info_serialization() {
        let pane = PaneInfo {
            id: "1".to_string(),
            title: "Main".to_string(),
            focused: true,
            command: Some("bash".to_string()),
        };

        let json = serde_json::to_string(&pane);
        assert!(json.is_ok());
        let json_str = json.unwrap();

        // Verify JSON contains all required fields
        assert!(json_str.contains("\"id\""));
        assert!(json_str.contains("\"title\""));
        assert!(json_str.contains("\"focused\""));
        assert!(json_str.contains("\"command\""));
    }

    // Red Queen Attack: PaneInfo with None command
    // Category: Output Contract Attacks
    // Severity: P1 (Broken contract)
    // Test: command field should be skipped when None
    #[test]
    fn test_pane_info_without_command_serialization() {
        let pane = PaneInfo {
            id: "2".to_string(),
            title: "Sidebar".to_string(),
            focused: false,
            command: None,
        };

        let json = serde_json::to_string(&pane).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // command field should be omitted (null or absent)
        let command = parsed.get("command");
        assert!(command.is_none() || command.unwrap().is_null());
    }

    // Red Queen Attack: PaneFocusOutput with SchemaEnvelope
    // Category: Output Contract Attacks
    // Severity: P1 (Broken contract)
    // Test: Output should wrap in SchemaEnvelope for consistency
    #[test]
    fn test_pane_focus_output_wrapped_in_envelope() {
        let output = PaneFocusOutput {
            session: "test-session".to_string(),
            pane: "1".to_string(),
            message: "Focused".to_string(),
        };

        let envelope = SchemaEnvelope::new("pane-focus-response", "single", output);
        let json = serde_json::to_string(&envelope).unwrap();

        // Verify SchemaEnvelope fields are present
        assert!(json.contains("\"$schema\""));
        assert!(json.contains("\"_schema_version\""));
        assert!(json.contains("\"schema_type\""));
        assert!(json.contains("pane-focus-response"));
    }

    // Red Queen Attack: PaneListOutput with panes
    // Category: Output Contract Attacks
    // Severity: P1 (Broken contract)
    // Test: Should serialize list of panes correctly
    #[test]
    fn test_pane_list_output_serialization() {
        let output = PaneListOutput {
            session: "my-session".to_string(),
            panes: vec![
                PaneInfo {
                    id: "1".to_string(),
                    title: "Main".to_string(),
                    focused: true,
                    command: None,
                },
                PaneInfo {
                    id: "2".to_string(),
                    title: "Terminal".to_string(),
                    focused: false,
                    command: Some("bash".to_string()),
                },
            ],
            focused: Some("1".to_string()),
        };

        let envelope = SchemaEnvelope::new("pane-list-response", "single", output);
        let json = serde_json::to_string(&envelope).unwrap();

        // Verify panes array is present
        assert!(json.contains("\"panes\""));
        assert!(json.contains("\"focused\""));
    }

    // Red Queen Attack: PaneNextOutput serialization
    // Category: Output Contract Attacks
    // Severity: P1 (Broken contract)
    // Test: Should serialize next operation response
    #[test]
    fn test_pane_next_output_serialization() {
        let output = PaneNextOutput {
            session: "test".to_string(),
            message: "Moved to next pane".to_string(),
        };

        let envelope = SchemaEnvelope::new("pane-next-response", "single", output);
        let json = serde_json::to_string(&envelope).unwrap();

        assert!(json.contains("\"session\""));
        assert!(json.contains("\"message\""));
    }
}

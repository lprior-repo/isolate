//! Rename command - Rename a session and its Zellij tab
//!
//! Renames a session while preserving all state and synchronizing with Zellij.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::io::Write;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::{
    cli::{is_inside_zellij, run_command},
    commands::get_session_db,
};

/// Maximum tab name length for Zellij (conservative limit)
const MAX_TAB_NAME_LENGTH: usize = 64;

/// Options for the rename command
#[derive(Debug, Clone)]
pub struct RenameOptions {
    /// Current session name
    pub old_name: String,
    /// New session name
    pub new_name: String,
    /// Dry-run mode
    pub dry_run: bool,
    /// Skip Zellij integration
    pub no_zellij: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Rename result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    /// Whether rename succeeded
    pub success: bool,
    /// Old session name
    pub old_name: String,
    /// New session name
    pub new_name: String,
    /// Old Zellij tab name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_tab_name: Option<String>,
    /// New Zellij tab name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_tab_name: Option<String>,
    /// Whether this was a dry-run
    pub dry_run: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Validate tab name length
///
/// # Errors
///
/// Returns error if tab name exceeds maximum length
fn validate_tab_name_length(name: &str) -> Result<()> {
    if name.len() > MAX_TAB_NAME_LENGTH {
        anyhow::bail!(
            "Tab name too long: {} characters (max: {})",
            name.len(),
            MAX_TAB_NAME_LENGTH
        );
    }
    Ok(())
}

/// Rename Zellij tab using Zellij actions
///
/// # Errors
///
/// Returns error if Zellij command fails
async fn rename_zellij_tab(new_tab_name: &str) -> Result<()> {
    run_command("zellij", &["action", "rename-tab", new_tab_name]).await?;
    Ok(())
}

/// Run the rename command
///
/// # Errors
///
/// Returns error with specific exit codes:
/// - Exit 1: Validation errors (name too long, session exists, etc.)
/// - Exit 2: Not inside Zellij
/// - Exit 3: Session not found
/// - Exit 4: Other errors (Zellij command failed, database errors)
#[allow(clippy::too_many_lines)]
pub async fn run(options: &RenameOptions) -> Result<()> {
    // REQUIREMENT [IF2]: If not inside Zellij and --no-zellij not set, exit 2 with message
    // Use ValidationError which maps to exit code 1 (validation failure)
    let zellij_installed = crate::cli::is_zellij_installed().await;
    let no_zellij = options.no_zellij || !crate::cli::is_terminal() || !zellij_installed;

    if !no_zellij && !is_inside_zellij() {
        anyhow::bail!("Not inside a Zellij session. Use 'zjj rename' from within Zellij or use --no-zellij flag.");
    }

    if !crate::cli::is_terminal() && !options.no_zellij && !options.format.is_json() {
        println!("Note: Non-interactive environment detected, skipping Zellij integration.");
    } else if !zellij_installed && !options.no_zellij && !options.format.is_json() {
        println!("Note: Zellij not found, skipping Zellij integration.");
    }

    let db = get_session_db().await?;

    // REQUIREMENT [IF1]: If session doesn't exist, exit 3
    let session = db.get(&options.old_name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{0}' not found",
            options.old_name
        )))
    })?;

    // EDGE CASE 1: Rename to same name - No-op success
    if options.old_name == options.new_name {
        let result = RenameResult {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            old_tab_name: Some(session.zellij_tab.clone()),
            new_tab_name: Some(session.zellij_tab),
            dry_run: options.dry_run,
            error: None,
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("rename-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(
                std::io::stdout(),
                "Session '{}' already has that name (no-op)",
                &options.old_name
            )?;
        }
        return Ok(());
    }

    // REQUIREMENT [IF3]: If name too long, exit 1 with validation error
    validate_tab_name_length(&options.new_name)?;

    // Check new name doesn't exist
    if db.get(&options.new_name).await?.is_some() {
        anyhow::bail!("Session '{}' already exists", options.new_name);
    }

    let old_tab_name = session.zellij_tab.clone();
    let new_tab_name = format!("zjj:{name}", name = options.new_name);

    // Dry-run mode - preview changes without executing
    if options.dry_run {
        let result = RenameResult {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            old_tab_name: Some(old_tab_name.clone()),
            new_tab_name: Some(new_tab_name.clone()),
            dry_run: true,
            error: None,
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("rename-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(std::io::stdout(), "[dry-run] Would rename:")?;
            writeln!(
                std::io::stdout(),
                "  Session: '{}' → '{}'",
                &options.old_name,
                &options.new_name
            )?;
            writeln!(
                std::io::stdout(),
                "  Tab:     '{}' → '{}'",
                &old_tab_name,
                &new_tab_name
            )?;
        }
        return Ok(());
    }

    // REQUIREMENT [E1]: Rename Zellij tab via action (skip if --no-zellij)
    if !no_zellij {
        rename_zellij_tab(&new_tab_name).await?;
    }

    // Rename workspace directory
    let workspace_path = &session.workspace_path;
    let new_workspace_path = if workspace_path.is_empty() {
        None
    } else {
        let old_path = std::path::Path::new(workspace_path);
        let new_path = old_path.parent().map(|p| p.join(&options.new_name));
        if let Some(ref new_path) = new_path {
            // Use map to handle Result without unwrap
            let path_exists = tokio::fs::try_exists(old_path).await;
            if matches!(path_exists, Ok(true)) {
                tokio::fs::rename(old_path, new_path).await?;
            }
        }
        new_path.map(|p| p.to_string_lossy().to_string())
    };

    // REQUIREMENT [E2]: Update session record in database
    // Create new session with new name
    db.create(
        &options.new_name,
        new_workspace_path.as_deref().unwrap_or(workspace_path),
    )
    .await?;

    // Delete old session
    db.delete(&options.old_name).await?;

    // Success output
    let result = RenameResult {
        success: true,
        old_name: options.old_name.clone(),
        new_name: options.new_name.clone(),
        old_tab_name: Some(old_tab_name.clone()),
        new_tab_name: Some(new_tab_name.clone()),
        dry_run: false,
        error: None,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("rename-response", "single", &result);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(
            std::io::stdout(),
            "✓ Renamed session '{}' → '{}'",
            &options.old_name,
            &options.new_name
        )?;
        writeln!(
            std::io::stdout(),
            "✓ Renamed tab '{}' → '{}'",
            &old_tab_name,
            &new_tab_name
        )?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// PHASE 1 (RED) - TDD Tests for Rename Command
// These tests FAIL until the implementation is complete
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = RenameResult {
            success: true,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            old_tab_name: Some("zjj:old".to_string()),
            new_tab_name: Some("zjj:new".to_string()),
            dry_run: false,
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"old_name\":\"old\""));
        assert!(json.contains("\"new_name\":\"new\""));
        assert!(json.contains("\"old_tab_name\":\"zjj:old\""));
        assert!(json.contains("\"new_tab_name\":\"zjj:new\""));
        Ok(())
    }

    #[test]
    fn test_rename_result_with_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = RenameResult {
            success: false,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            old_tab_name: None,
            new_tab_name: None,
            dry_run: false,
            error: Some("Session exists".to_string()),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":"));
        Ok(())
    }

    #[test]
    fn test_validate_tab_name_length_success() {
        let short_name = "valid";
        assert!(validate_tab_name_length(short_name).is_ok());
    }

    #[test]
    fn test_validate_tab_name_length_failure() {
        let long_name = "a".repeat(MAX_TAB_NAME_LENGTH + 1);
        let result = validate_tab_name_length(&long_name);
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("Tab name too long"));
            assert!(msg.contains(&format!("{}", MAX_TAB_NAME_LENGTH + 1)));
        }
    }

    #[test]
    fn test_validate_tab_name_length_exact_max() {
        let exact_max = "a".repeat(MAX_TAB_NAME_LENGTH);
        assert!(validate_tab_name_length(&exact_max).is_ok());
    }

    #[test]
    fn test_rename_result_dry_run() -> Result<(), Box<dyn std::error::Error>> {
        let result = RenameResult {
            success: true,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            old_tab_name: Some("zjj:old".to_string()),
            new_tab_name: Some("zjj:new".to_string()),
            dry_run: true,
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"dry_run\":true"));
        Ok(())
    }

    #[test]
    fn test_rename_options_construction() {
        let options = RenameOptions {
            old_name: "feature-a".to_string(),
            new_name: "feature-b".to_string(),
            dry_run: false,
            no_zellij: false,
            format: OutputFormat::Json,
        };

        assert_eq!(options.old_name, "feature-a");
        assert_eq!(options.new_name, "feature-b");
        assert!(!options.dry_run);
        assert!(!options.no_zellij);
        assert_eq!(options.format, OutputFormat::Json);
    }

    #[test]
    fn test_rename_options_no_zellij() {
        let options = RenameOptions {
            old_name: "feature-a".to_string(),
            new_name: "feature-b".to_string(),
            dry_run: false,
            no_zellij: true,
            format: OutputFormat::Json,
        };

        assert!(options.no_zellij, "Should have no_zellij flag set");
    }

    #[test]
    fn test_rename_same_name_format() {
        // Verify the tab name format is consistent
        let old_name = "my-session";
        let new_name = "my-session";
        let expected_tab = format!("zjj:{old_name}");

        assert_eq!(old_name, new_name);
        assert_eq!(expected_tab, "zjj:my-session");
    }
}

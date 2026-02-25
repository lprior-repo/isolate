//! Rename command - Rename a session
//!
//! Renames a session while preserving all state.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::io::Write;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;

/// Maximum session name length (conservative limit)
const MAX_NAME_LENGTH: usize = 64;

/// Options for the rename command
#[derive(Debug, Clone)]
pub struct RenameOptions {
    /// Current session name
    pub old_name: String,
    /// New session name
    pub new_name: String,
    /// Dry-run mode
    pub dry_run: bool,
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
    /// Whether this was a dry-run
    pub dry_run: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Validate session name length
///
/// # Errors
///
/// Returns error if name exceeds maximum length
fn validate_name_length(name: &str) -> Result<()> {
    if name.len() > MAX_NAME_LENGTH {
        anyhow::bail!(
            "Session name too long: {} characters (max: {})",
            name.len(),
            MAX_NAME_LENGTH
        );
    }
    Ok(())
}

/// Run the rename command
///
/// # Errors
///
/// Returns error with specific exit codes:
/// - Exit 1: Validation errors (name too long, session exists, etc.)
/// - Exit 3: Session not found
/// - Exit 4: Other errors (database errors)
#[allow(clippy::too_many_lines)]
pub async fn run(options: &RenameOptions) -> Result<()> {
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
    validate_name_length(&options.new_name)?;

    // Check new name doesn't exist
    if db.get(&options.new_name).await?.is_some() {
        anyhow::bail!("Session '{}' already exists", options.new_name);
    }

    // Dry-run mode - preview changes without executing
    if options.dry_run {
        let result = RenameResult {
            success: true,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
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
        }
        return Ok(());
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
            dry_run: false,
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"old_name\":\"old\""));
        assert!(json.contains("\"new_name\":\"new\""));
        Ok(())
    }

    #[test]
    fn test_rename_result_with_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = RenameResult {
            success: false,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
            dry_run: false,
            error: Some("Session exists".to_string()),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":"));
        Ok(())
    }

    #[test]
    fn test_validate_name_length_success() {
        let short_name = "valid";
        assert!(validate_name_length(short_name).is_ok());
    }

    #[test]
    fn test_validate_name_length_failure() {
        let long_name = "a".repeat(MAX_NAME_LENGTH + 1);
        let result = validate_name_length(&long_name);
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("Session name too long"));
            assert!(msg.contains(&format!("{}", MAX_NAME_LENGTH + 1)));
        }
    }

    #[test]
    fn test_validate_name_length_exact_max() {
        let exact_max = "a".repeat(MAX_NAME_LENGTH);
        assert!(validate_name_length(&exact_max).is_ok());
    }

    #[test]
    fn test_rename_result_dry_run() -> Result<(), Box<dyn std::error::Error>> {
        let result = RenameResult {
            success: true,
            old_name: "old".to_string(),
            new_name: "new".to_string(),
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
            format: OutputFormat::Json,
        };

        assert_eq!(options.old_name, "feature-a");
        assert_eq!(options.new_name, "feature-b");
        assert!(!options.dry_run);
        assert_eq!(options.format, OutputFormat::Json);
    }
}

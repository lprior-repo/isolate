//! Rename command - Rename a session
//!
//! Renames a session while preserving all state.

use std::io::Write;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;

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

/// Run the rename command
pub fn run(options: &RenameOptions) -> Result<()> {
    let db = get_session_db()?;

    // Check old session exists
    let session = db
        .get_blocking(&options.old_name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", &options.old_name))?;

    // Check new name doesn't exist
    if db.get_blocking(&options.new_name)?.is_some() {
        let result = RenameResult {
            success: false,
            old_name: options.old_name.clone(),
            new_name: options.new_name.clone(),
            dry_run: options.dry_run,
            error: Some(format!("Session '{}' already exists", &options.new_name)),
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("rename-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
            return Ok(());
        }
        anyhow::bail!("Session '{}' already exists", &options.new_name);
    }

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
            writeln!(
                std::io::stdout(),
                "[dry-run] Would rename '{}' to '{}'",
                &options.old_name,
                &options.new_name
            )?;
        }
        return Ok(());
    }

    // Perform the rename by creating new session with new name and deleting old
    // 1. Rename JJ workspace directory
    let workspace_path = &session.workspace_path;
    let new_workspace_path = if workspace_path.is_empty() {
        None
    } else {
        let old_path = std::path::Path::new(workspace_path);
        let new_path = old_path.parent().map(|p| p.join(&options.new_name));
        if let Some(ref new_path) = new_path {
            if old_path.exists() {
                std::fs::rename(old_path, new_path)?;
            }
        }
        new_path.map(|p| p.to_string_lossy().to_string())
    };

    // 2. Rename Zellij tab
    let _ = std::process::Command::new("zellij")
        .args(["action", "rename-tab", &format!("zjj:{}", options.new_name)])
        .output();

    // 3. Create new session and delete old
    // Create new session with new name
    db.create_blocking(
        &options.new_name,
        new_workspace_path.as_deref().unwrap_or(workspace_path),
    )?;

    // Delete old session
    db.delete_blocking(&options.old_name)?;

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
            "✓ Renamed '{}' to '{}'",
            &options.old_name,
            &options.new_name
        )?;
    }

    Ok(())
}

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
}

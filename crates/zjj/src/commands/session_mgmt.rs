//! Session management commands - Pause, Resume, Clone
//!
//! Additional session lifecycle management operations.

use std::io::Write;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::{
    commands::get_session_db,
    session::{SessionStatus, SessionUpdate},
};

// ═══════════════════════════════════════════════════════════════════════════
// PAUSE
// ═══════════════════════════════════════════════════════════════════════════

/// Options for the pause command
#[derive(Debug, Clone)]
pub struct PauseOptions {
    /// Session to pause
    pub session: String,
    /// Output format
    pub format: OutputFormat,
}

/// Pause result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResult {
    pub success: bool,
    pub session: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run the pause command
pub fn run_pause(options: &PauseOptions) -> Result<()> {
    let db = get_session_db()?;

    // Check session exists
    let session = db
        .get_blocking(&options.session)?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", &options.session))?;

    let prev_status = session.status.to_string();

    // Update status to paused
    let update = SessionUpdate {
        status: Some(SessionStatus::Paused),
        state: None,
        branch: None,
        last_synced: None,
        metadata: None,
    };
    db.update_blocking(&options.session, update)?;

    let result = PauseResult {
        success: true,
        session: options.session.clone(),
        status: "paused".to_string(),
        error: None,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("pause-response", "single", &result);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(std::io::stdout(), "✓ Paused session '{}'", &options.session)?;
        writeln!(std::io::stdout(), "  Previous status: {prev_status}")?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// RESUME
// ═══════════════════════════════════════════════════════════════════════════

/// Options for the resume command
#[derive(Debug, Clone)]
pub struct ResumeOptions {
    /// Session to resume
    pub session: String,
    /// Output format
    pub format: OutputFormat,
}

/// Resume result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResult {
    pub success: bool,
    pub session: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run the resume command
pub fn run_resume(options: &ResumeOptions) -> Result<()> {
    let db = get_session_db()?;

    // Check session exists
    let session = db
        .get_blocking(&options.session)?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", &options.session))?;

    if session.status != SessionStatus::Paused {
        let result = ResumeResult {
            success: false,
            session: options.session.clone(),
            status: session.status.to_string(),
            error: Some(format!(
                "Session is not paused (status: {})",
                session.status
            )),
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("resume-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
            return Ok(());
        }
        anyhow::bail!("Session is not paused (status: {})", session.status);
    }

    // Update status to active
    let update = SessionUpdate {
        status: Some(SessionStatus::Active),
        state: None,
        branch: None,
        last_synced: None,
        metadata: None,
    };
    db.update_blocking(&options.session, update)?;

    let result = ResumeResult {
        success: true,
        session: options.session.clone(),
        status: "active".to_string(),
        error: None,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("resume-response", "single", &result);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(
            std::io::stdout(),
            "✓ Resumed session '{}'",
            &options.session
        )?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CLONE
// ═══════════════════════════════════════════════════════════════════════════

/// Options for the clone command
#[derive(Debug, Clone)]
pub struct CloneOptions {
    /// Session to clone
    pub source: String,
    /// Name for the cloned session
    pub target: String,
    /// Dry-run mode
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Clone result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneResult {
    pub success: bool,
    pub source: String,
    pub target: String,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run the clone command
#[allow(clippy::too_many_lines)]
pub fn run_clone(options: &CloneOptions) -> Result<()> {
    let db = get_session_db()?;

    // Check source exists
    let source_session = db
        .get_blocking(&options.source)?
        .ok_or_else(|| anyhow::anyhow!("Source session '{}' not found", &options.source))?;

    // Check target doesn't exist
    if db.get_blocking(&options.target)?.is_some() {
        let result = CloneResult {
            success: false,
            source: options.source.clone(),
            target: options.target.clone(),
            dry_run: options.dry_run,
            workspace_path: None,
            error: Some(format!(
                "Target session '{}' already exists",
                &options.target
            )),
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("clone-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
            return Ok(());
        }
        anyhow::bail!("Target session '{}' already exists", &options.target);
    }

    if options.dry_run {
        let result = CloneResult {
            success: true,
            source: options.source.clone(),
            target: options.target.clone(),
            dry_run: true,
            workspace_path: None,
            error: None,
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("clone-response", "single", &result);
            let json_str = serde_json::to_string_pretty(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(
                std::io::stdout(),
                "[dry-run] Would clone '{}' to '{}'",
                &options.source,
                &options.target
            )?;
        }
        return Ok(());
    }

    // Create new workspace from source
    let source_path = &source_session.workspace_path;
    let new_workspace_path = if source_path.is_empty() {
        None
    } else {
        let source_path = std::path::Path::new(source_path);
        let new_path = source_path.parent().map(|p| p.join(&options.target));

        if let Some(new_path) = &new_path {
            let Some(new_path_str) = new_path.to_str() else {
                anyhow::bail!("Invalid workspace path (non-UTF8)");
            };
            // Use jj to create a new workspace at the same commit
            let output = std::process::Command::new("jj")
                .args(["workspace", "add", new_path_str])
                .current_dir(source_path)
                .output()?;

            if !output.status.success() {
                anyhow::bail!(
                    "Failed to create workspace: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        new_path.map(|p| p.to_string_lossy().to_string())
    };

    // Create session in database
    db.create_blocking(
        &options.target,
        new_workspace_path.as_deref().map_or("", |value| value),
    )?;

    let result = CloneResult {
        success: true,
        source: options.source.clone(),
        target: options.target.clone(),
        dry_run: false,
        workspace_path: new_workspace_path.clone(),
        error: None,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("clone-response", "single", &result);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        writeln!(
            std::io::stdout(),
            "✓ Cloned '{}' to '{}'",
            &options.source,
            &options.target
        )?;
        if let Some(path) = new_workspace_path {
            writeln!(std::io::stdout(), "  Workspace: {path}")?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pause_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = PauseResult {
            success: true,
            session: "test".to_string(),
            status: "paused".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"status\":\"paused\""));
        Ok(())
    }

    #[test]
    fn test_resume_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = ResumeResult {
            success: true,
            session: "test".to_string(),
            status: "active".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"status\":\"active\""));
        Ok(())
    }

    #[test]
    fn test_clone_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = CloneResult {
            success: true,
            source: "orig".to_string(),
            target: "copy".to_string(),
            dry_run: false,
            workspace_path: Some("/path/to/workspace".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"source\":\"orig\""));
        assert!(json.contains("\"target\":\"copy\""));
        Ok(())
    }
}

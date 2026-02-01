//! Session management commands - Pause, Resume, Clone
//!
//! Additional session lifecycle management operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;
use crate::session::{SessionStatus, SessionUpdate};

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
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", options.session))?;

    let prev_status = session.status.to_string();

    // Update status to paused
    let update = SessionUpdate {
        status: Some(SessionStatus::Paused),
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
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("✓ Paused session '{}'", options.session);
        println!("  Previous status: {prev_status}");
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
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", options.session))?;

    if session.status != SessionStatus::Paused {
        let result = ResumeResult {
            success: false,
            session: options.session.clone(),
            status: session.status.to_string(),
            error: Some(format!("Session is not paused (status: {})", session.status)),
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("resume-response", "single", &result);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            return Ok(());
        } else {
            anyhow::bail!("Session is not paused (status: {})", session.status);
        }
    }

    // Update status to active
    let update = SessionUpdate {
        status: Some(SessionStatus::Active),
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
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("✓ Resumed session '{}'", options.session);
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
pub fn run_clone(options: &CloneOptions) -> Result<()> {
    let db = get_session_db()?;

    // Check source exists
    let source_session = db
        .get_blocking(&options.source)?
        .ok_or_else(|| anyhow::anyhow!("Source session '{}' not found", options.source))?;

    // Check target doesn't exist
    if db.get_blocking(&options.target)?.is_some() {
        let result = CloneResult {
            success: false,
            source: options.source.clone(),
            target: options.target.clone(),
            dry_run: options.dry_run,
            workspace_path: None,
            error: Some(format!("Target session '{}' already exists", options.target)),
        };

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("clone-response", "single", &result);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            return Ok(());
        } else {
            anyhow::bail!("Target session '{}' already exists", options.target);
        }
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
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("[dry-run] Would clone '{}' to '{}'", options.source, options.target);
        }
        return Ok(());
    }

    // Create new workspace from source
    let source_path = &source_session.workspace_path;
    let new_workspace_path = if !source_path.is_empty() {
        let source_path = std::path::Path::new(source_path);
        let new_path = source_path.parent().map(|p| p.join(&options.target));

        if let Some(new_path) = &new_path {
            // Use jj to create a new workspace at the same commit
            let output = std::process::Command::new("jj")
                .args(["workspace", "add", new_path.to_str().unwrap_or_default()])
                .current_dir(source_path)
                .output()?;

            if !output.status.success() {
                anyhow::bail!("Failed to create workspace: {}", String::from_utf8_lossy(&output.stderr));
            }
        }

        new_path.map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    // Create session in database
    db.create_blocking(
        &options.target,
        new_workspace_path.as_deref().unwrap_or_default(),
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
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("✓ Cloned '{}' to '{}'", options.source, options.target);
        if let Some(path) = new_workspace_path {
            println!("  Workspace: {path}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pause_result_serialization() {
        let result = PauseResult {
            success: true,
            session: "test".to_string(),
            status: "paused".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"status\":\"paused\""));
    }

    #[test]
    fn test_resume_result_serialization() {
        let result = ResumeResult {
            success: true,
            session: "test".to_string(),
            status: "active".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_clone_result_serialization() {
        let result = CloneResult {
            success: true,
            source: "orig".to_string(),
            target: "copy".to_string(),
            dry_run: false,
            workspace_path: Some("/path/to/workspace".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"source\":\"orig\""));
        assert!(json.contains("\"target\":\"copy\""));
    }
}

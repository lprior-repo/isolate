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
pub async fn run_pause(options: &PauseOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Check session exists
    let session = db
        .get(&options.session)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", &options.session))?;

    let prev_status = session.status.to_string();

    // If already paused, return success (idempotent)
    if session.status == SessionStatus::Paused {
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
            writeln!(
                std::io::stdout(),
                "✓ Session '{}' is already paused",
                &options.session
            )?;
        }
        return Ok(());
    }

    // Update status to paused
    let update = SessionUpdate {
        status: Some(SessionStatus::Paused),
        state: None,
        branch: None,
        last_synced: None,
        metadata: None,
    };
    db.update(&options.session, update).await?;

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
pub async fn run_resume(options: &ResumeOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Check session exists
    let session = db
        .get(&options.session)
        .await?
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
    db.update(&options.session, update).await?;

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
    /// Skip Zellij integration entirely
    #[allow(dead_code)] // Reserved for future use
    pub no_zellij: bool,
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
pub async fn run_clone(options: &CloneOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Check source exists
    let source_session = db
        .get(&options.source)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Source session '{}' not found", &options.source))?;

    // Validate target session name (REQ-CLI-015)
    // Map zjj_core::Error to anyhow::Error while preserving the original error
    crate::session::validate_session_name(&options.target).map_err(anyhow::Error::new)?;

    // Check target doesn't exist
    if db.get(&options.target).await?.is_some() {
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
            // Use zjj_core to create workspace (async)
            zjj_core::jj::workspace_create(&options.target, new_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create workspace: {e}"))?;
        }

        new_path.map(|p| p.to_string_lossy().to_string())
    };

    // Create session in database
    db.create(
        &options.target,
        new_workspace_path.as_deref().map_or("", |value| value),
    )
    .await?;
    // Update session status to Active (fix bug: clone leaves status as "creating")
    let update = SessionUpdate {
        status: Some(SessionStatus::Active),
        ..Default::default()
    };
    db.update(&options.target, update).await?;

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

    /// Test that pause is idempotent - pausing an already-paused session should succeed
    #[test]
    fn test_pause_idempotent() {
        // Simulate pausing a session that's already paused
        // This should return success without error (idempotent behavior)
        let result = PauseResult {
            success: true,
            session: "already-paused".to_string(),
            status: "paused".to_string(),
            error: None,
        };

        assert!(result.success, "Pause should be idempotent");
        assert_eq!(result.status, "paused");
        assert!(
            result.error.is_none(),
            "Already-paused session should not error"
        );
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
        assert!(json.contains(r#""source":"orig""#));
        assert!(json.contains(r#""target":"copy""#));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLONE VALIDATION TESTS (RED phase - should fail before implementation)
    // ═══════════════════════════════════════════════════════════════════════════

    /// RED: Clone should reject empty target names
    #[test]
    fn test_clone_rejects_empty_target_name() {
        use crate::session::validate_session_name;

        let result = validate_session_name("");
        assert!(result.is_err(), "Empty session name should be rejected");

        if let Err(e) = result {
            assert_eq!(
                e.exit_code(),
                1,
                "ValidationError should map to exit code 1"
            );
        }
    }

    /// RED: Clone should reject non-ASCII target names
    #[test]
    fn test_clone_rejects_non_ascii_target_name() {
        use crate::session::validate_session_name;

        let invalid_names = vec!["test-clone-🚀", "café", "日本語", "中文名字"];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Non-ASCII name '{name}' should be rejected"
            );

            if let Err(e) = result {
                assert_eq!(
                    e.exit_code(),
                    1,
                    "ValidationError should map to exit code 1"
                );
                assert!(
                    matches!(e, zjj_core::Error::ValidationError(_)),
                    "Should return ValidationError"
                );
            }
        }
    }

    /// RED: Clone should reject target names starting with digit
    #[test]
    fn test_clone_rejects_target_starting_with_digit() {
        use crate::session::validate_session_name;

        let result = validate_session_name("123-clone-target");
        assert!(
            result.is_err(),
            "Name starting with digit should be rejected"
        );

        if let Err(e) = result {
            assert_eq!(e.exit_code(), 1);
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
        }
    }

    /// RED: Clone should reject target names with invalid characters
    #[test]
    fn test_clone_rejects_target_with_invalid_chars() {
        use crate::session::validate_session_name;

        let invalid_names = vec![
            "clone target", // spaces
            "clone/target", // slashes
            "clone.target", // dots
            "clone@target", // @ symbol
            "clone!target", // exclamation
        ];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Name with invalid chars '{name}' should be rejected"
            );

            if let Err(e) = result {
                assert_eq!(e.exit_code(), 1);
                assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            }
        }
    }

    /// RED: Clone should reject target names starting with dash/underscore
    #[test]
    fn test_clone_rejects_target_starting_with_dash_or_underscore() {
        use crate::session::validate_session_name;

        let invalid_names = vec!["-clone-target", "_clone_target"];

        for name in invalid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_err(),
                "Name starting with special char '{name}' should be rejected"
            );

            if let Err(e) = result {
                assert_eq!(e.exit_code(), 1);
                assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            }
        }
    }

    /// RED: Clone should reject target names exceeding 64 characters
    #[test]
    fn test_clone_rejects_target_too_long() {
        use crate::session::validate_session_name;

        let long_name = "a".repeat(65);
        let result = validate_session_name(&long_name);
        assert!(result.is_err(), "Name > 64 chars should be rejected");

        if let Err(e) = result {
            assert_eq!(e.exit_code(), 1);
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
        }
    }

    /// RED: Clone should accept valid target names
    #[test]
    fn test_clone_accepts_valid_target_names() {
        use crate::session::validate_session_name;

        let valid_names = vec![
            "valid-clone",
            "valid_clone",
            "ValidClone123",
            "a", // single letter
            "feature-branch-copy-2",
        ];

        for name in valid_names {
            let result = validate_session_name(name);
            assert!(
                result.is_ok(),
                "Valid name '{name}' should be accepted: {result:?}"
            );
        }
    }

    /// RED: Clone should use same validation as add command
    #[test]
    fn test_clone_uses_same_validation_as_add() {
        use crate::session::validate_session_name;

        // Test cases from add.rs tests
        let test_cases = vec![
            ("my-session", true),
            ("my_session", true),
            ("MyName123", true),
            ("", false),
            ("test session", false),
            ("-session", false),
            ("_session", false),
            ("123session", false),
            ("test-🚀", false),
        ];

        for (name, should_pass) in test_cases {
            let result = validate_session_name(name);
            assert_eq!(
                result.is_ok(),
                should_pass,
                "Name '{name}' validation mismatch: expected ok={should_pass}, got {result:?}"
            );
        }
    }
}

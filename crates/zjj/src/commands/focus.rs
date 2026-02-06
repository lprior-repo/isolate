//! Switch to a session's Zellij tab

use anyhow::Result;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::get_session_db,
    json::FocusOutput,
};

/// Options for the focus command
#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    /// Output format
    pub format: OutputFormat,
    /// Skip Zellij integration entirely (for non-TTY environments)
    pub no_zellij: bool,
}

/// Run the focus command with options
pub async fn run_with_options(name: Option<&str>, options: &FocusOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Resolve the name (either provided or selected interactively)
    let resolved_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Interactive selection
        let sessions = db.list(None).await?;

        if sessions.is_empty() {
            if options.format.is_json() {
                return Err(anyhow::anyhow!("No sessions found"));
            }
            println!("No sessions found. Create one with 'zjj add <name>'.");
            return Ok(());
        }

        if let Some(session) = crate::selector::select_session(&sessions)? {
            session.name
        } else {
            return Ok(()); // User cancelled
        }
    };

    // Get the session (we might need to fetch it again if it was provided by name)
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(&resolved_name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{resolved_name}' not found"
        )))
    })?;

    let zellij_tab = session.zellij_tab;

    if options.no_zellij {
        // Skip Zellij integration - just print info
        if options.format.is_json() {
            let output = FocusOutput {
                name: resolved_name.clone(),
                zellij_tab: zellij_tab.clone(),
                message: format!(
                    "Session '{resolved_name}' is in tab '{zellij_tab}' (Zellij disabled)"
                ),
            };
            let envelope = SchemaEnvelope::new("focus-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Session '{resolved_name}' is in tab '{zellij_tab}'");
            println!("Workspace path: {}", session.workspace_path);
        }
    } else if is_inside_zellij() {
        // Inside Zellij: Switch to the tab
        run_command("zellij", &["action", "go-to-tab-name", &zellij_tab]).await?;

        if options.format.is_json() {
            let output = FocusOutput {
                name: resolved_name.clone(),
                zellij_tab,
                message: format!("Switched to session '{resolved_name}'"),
            };
            let envelope = SchemaEnvelope::new("focus-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Switched to session '{resolved_name}'");
        }
    } else {
        // Outside Zellij: Attach to the Zellij session
        // User will land in session and can navigate to desired tab
        if options.format.is_json() {
            let output = FocusOutput {
                name: resolved_name.clone(),
                zellij_tab: zellij_tab.clone(),
                message: format!(
                    "Session '{resolved_name}' is in tab '{zellij_tab}'. Attaching to Zellij session..."
                ),
            };
            let envelope = SchemaEnvelope::new("focus-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Session '{resolved_name}' is in tab '{zellij_tab}'");
            println!("Attaching to Zellij session...");
        }
        attach_to_zellij_session(None)?;
        // Note: This never returns - we exec into Zellij
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::db::SessionDb;

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[tokio::test]
    async fn test_focus_session_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Try to get a non-existent session
        let result = db.get("nonexistent").await?;
        assert!(result.is_none());

        // Verify the error message format when session not found
        let session_name = "nonexistent";
        let result = db
            .get(session_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"));

        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Session 'nonexistent' not found");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create a session
        let session = db.create("test-session", "/tmp/test").await?;

        // Verify we can retrieve it
        let retrieved = db.get("test-session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, session.name);
        assert_eq!(retrieved_session.zellij_tab, "zjj:test-session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_hyphens() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create a session with hyphens in the name
        let _session = db.create("my-test-session", "/tmp/my-test").await?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test-session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test-session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test-session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_underscores() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create a session with underscores in the name
        let _session = db.create("my_test_session", "/tmp/my_test").await?;

        // Verify we can retrieve it
        let retrieved = db.get("my_test_session").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my_test_session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my_test_session");

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_session_with_mixed_special_chars() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create a session with mixed special characters
        let _session = db.create("my-test_123", "/tmp/my-test_123").await?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test_123").await?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test_123");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test_123");

        Ok(())
    }

    #[tokio::test]
    async fn test_zellij_tab_format() -> Result<()> {
        let (db, _dir) = setup_test_db().await?;

        // Create sessions and verify tab name format
        let session1 = db.create("session1", "/tmp/s1").await?;
        assert_eq!(session1.zellij_tab, "zjj:session1");

        let session2 = db.create("my-session", "/tmp/s2").await?;
        assert_eq!(session2.zellij_tab, "zjj:my-session");

        let session3 = db.create("test_session_123", "/tmp/s3").await?;
        assert_eq!(session3.zellij_tab, "zjj:test_session_123");

        Ok(())
    }

    #[tokio::test]
    async fn test_is_inside_zellij_detection() {
        // Save original value
        let original = std::env::var("ZELLIJ").ok();

        // Test when ZELLIJ env var is not set
        std::env::remove_var("ZELLIJ");
        assert!(!is_inside_zellij());

        // Test when ZELLIJ env var is set
        std::env::set_var("ZELLIJ", "1");
        assert!(is_inside_zellij());

        // Restore original value
        if let Some(val) = original {
            std::env::set_var("ZELLIJ", val);
        } else {
            std::env::remove_var("ZELLIJ");
        }
    }

    // Phase 1 RED tests: Focus JSON output should be wrapped with SchemaEnvelope

    #[tokio::test]
    async fn test_focus_json_has_envelope() -> Result<()> {
        use crate::json::FocusOutput;

        // Create sample FocusOutput
        let output = FocusOutput {
            name: "test-session".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            message: "Switched to session".to_string(),
        };

        // Wrap with SchemaEnvelope (this is what the command actually prints)
        let envelope = SchemaEnvelope::new("focus-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify SchemaEnvelope fields are present
        assert!(
            parsed.get("$schema").is_some(),
            "JSON output should have $schema field"
        );
        assert!(
            parsed.get("_schema_version").is_some(),
            "JSON output should have _schema_version field"
        );
        assert!(
            parsed.get("schema_type").is_some(),
            "JSON output should have schema_type field"
        );

        // Verify schema_type is "single"
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single"),
            "schema_type should be 'single' for FocusOutput"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_focus_schema_format() -> Result<()> {
        use crate::json::FocusOutput;

        // Create sample output
        let output = FocusOutput {
            name: "my-feature".to_string(),
            zellij_tab: "zjj:my-feature".to_string(),
            message: "Focused".to_string(),
        };

        // Wrap with SchemaEnvelope
        let envelope = SchemaEnvelope::new("focus-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify $schema format matches zjj://<command>/v1 pattern
        let schema_value = parsed.get("$schema").and_then(|v| v.as_str());
        assert!(
            schema_value.is_some(),
            "$schema field should be present and be a string"
        );
        let Some(schema) = schema_value else {
            return Ok(());
        };
        assert!(
            schema.starts_with("zjj://"),
            "$schema should start with 'zjj://', got: {schema}"
        );
        assert!(
            schema.ends_with("/v1"),
            "$schema should end with '/v1', got: {schema}"
        );
        assert!(
            schema.contains("focus"),
            "$schema should contain 'focus' for focus command, got: {schema}"
        );

        // Verify _schema_version is "1.0"
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0"),
            "_schema_version should be '1.0'"
        );

        Ok(())
    }
}

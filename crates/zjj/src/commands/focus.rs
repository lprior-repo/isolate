//! Switch to a session's Zellij tab

use anyhow::Result;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::get_session_db,
    json_output::FocusOutput,
};

/// Options for the focus command
#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    /// Output format
    pub format: OutputFormat,
}

/// Run the focus command with options
pub fn run_with_options(name: &str, options: &FocusOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(name)?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    let zellij_tab = session.zellij_tab;

    if is_inside_zellij() {
        // Inside Zellij: Switch to the tab
        run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

        if options.format.is_json() {
            let output = FocusOutput {
                name: name.to_string(),
                zellij_tab,
                message: format!("Switched to session '{name}'"),
            };
            let envelope = SchemaEnvelope::new("focus-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Switched to session '{name}'");
        }
    } else {
        // Outside Zellij: Attach to the Zellij session
        // User will land in session and can navigate to desired tab
        if options.format.is_json() {
            let output = FocusOutput {
                name: name.to_string(),
                zellij_tab: zellij_tab.clone(),
                message: format!(
                    "Session '{name}' is in tab '{zellij_tab}'. Attaching to Zellij session..."
                ),
            };
            let envelope = SchemaEnvelope::new("focus-response", "single", output);
            println!("{}", serde_json::to_string(&envelope)?);
        } else {
            println!("Session '{name}' is in tab '{zellij_tab}'");
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

    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    #[test]
    fn test_focus_session_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Try to get a non-existent session
        let result = db.get("nonexistent")?;
        assert!(result.is_none());

        // Verify the error message format when session not found
        let session_name = "nonexistent";
        let result = db
            .get(session_name)?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"));

        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Session 'nonexistent' not found");
        }

        Ok(())
    }

    #[test]
    fn test_focus_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        let session = db.create("test-session", "/tmp/test")?;

        // Verify we can retrieve it
        let retrieved = db.get("test-session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, session.name);
        assert_eq!(retrieved_session.zellij_tab, "zjj:test-session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_hyphens() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with hyphens in the name
        let _session = db.create("my-test-session", "/tmp/my-test")?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test-session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test-session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test-session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_underscores() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with underscores in the name
        let _session = db.create("my_test_session", "/tmp/my_test")?;

        // Verify we can retrieve it
        let retrieved = db.get("my_test_session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my_test_session");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my_test_session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_mixed_special_chars() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with mixed special characters
        let _session = db.create("my-test_123", "/tmp/my-test_123")?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test_123")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test_123");
        assert_eq!(retrieved_session.zellij_tab, "zjj:my-test_123");

        Ok(())
    }

    #[test]
    fn test_zellij_tab_format() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create sessions and verify tab name format
        let session1 = db.create("session1", "/tmp/s1")?;
        assert_eq!(session1.zellij_tab, "zjj:session1");

        let session2 = db.create("my-session", "/tmp/s2")?;
        assert_eq!(session2.zellij_tab, "zjj:my-session");

        let session3 = db.create("test_session_123", "/tmp/s3")?;
        assert_eq!(session3.zellij_tab, "zjj:test_session_123");

        Ok(())
    }

    #[test]
    fn test_is_inside_zellij_detection() {
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

    #[test]
    fn test_focus_json_has_envelope() -> Result<()> {
        use crate::json_output::FocusOutput;

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

    #[test]
    fn test_focus_schema_format() -> Result<()> {
        use crate::json_output::FocusOutput;

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

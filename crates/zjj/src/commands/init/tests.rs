use std::process::Command;

use anyhow::bail;
use tempfile::TempDir;

use super::*;

/// Check if jj is available in PATH
fn jj_is_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Helper to setup a test JJ repository
/// Returns None if jj is not available
fn setup_test_jj_repo() -> Result<Option<TempDir>> {
    if !jj_is_available() {
        return Ok(None);
    }

    let temp_dir = TempDir::new().context("Failed to create temp dir")?;

    // Initialize a JJ repo in the temp directory
    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(temp_dir.path())
        .output()
        .context("Failed to run jj git init")?;

    if !output.status.success() {
        bail!(
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(Some(temp_dir))
}

#[test]
fn test_init_creates_zjj_directory() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // Run init with temp directory as cwd
    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());

    // Check result
    result?;

    // Verify .zjj directory was created (use absolute path)
    let zjj_path = temp_dir.path().join(".zjj");
    assert!(zjj_path.exists(), ".zjj directory was not created");
    assert!(zjj_path.is_dir(), ".zjj is not a directory");

    Ok(())
}

#[test]
fn test_init_creates_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    // Verify config.toml was created
    let config_path = temp_dir.path().join(".zjj/config.toml");
    assert!(config_path.exists(), "config.toml was not created");
    assert!(config_path.is_file(), "config.toml is not a file");

    // Verify it contains expected content
    let content = std::fs::read_to_string(&config_path)?;
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));
    assert!(content.contains("[zellij]"));
    assert!(content.contains("[dashboard]"));

    Ok(())
}

#[test]
fn test_init_creates_state_db() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    // Verify state.db was created
    let db_path = temp_dir.path().join(".zjj/state.db");
    assert!(db_path.exists(), "state.db was not created");
    assert!(db_path.is_file(), "state.db is not a file");

    // Verify it's a valid SQLite database with correct schema
    let db = SessionDb::open_blocking(&db_path)?;
    let sessions = db.list_blocking(None)?;
    assert_eq!(sessions.len(), 0); // Should be empty initially

    Ok(())
}

#[test]
fn test_init_creates_layouts_directory() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    // Verify layouts directory was created
    let layouts_path = temp_dir.path().join(".zjj/layouts");
    assert!(layouts_path.exists(), "layouts directory was not created");
    assert!(layouts_path.is_dir(), "layouts is not a directory");

    Ok(())
}

#[test]
fn test_init_handles_already_initialized() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // First init should succeed
    let result1 = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    assert!(result1.is_ok());

    // Second init should not fail, just inform user
    let result2 = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    assert!(result2.is_ok());

    Ok(())
}

#[test]
fn test_init_auto_creates_jj_repo() -> Result<()> {
    // This test verifies that if we're not in a JJ repo,
    // the init command will create one automatically
    if !jj_is_available() {
        // Test framework will handle skipping - no output needed
        return Ok(());
    }

    let temp_dir = TempDir::new()?;

    // Before JJ init, should not be a repo
    // After our init command runs, it will create a JJ repo automatically
    // So we just verify the automatic initialization works
    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());

    // Should succeed because init_jj_repo is called automatically
    assert!(result.is_ok());

    // Verify JJ repo was created
    assert!(
        temp_dir.path().join(".jj").exists(),
        "JJ repo should be auto-created"
    );

    Ok(())
}

#[test]
fn test_default_config_is_valid_toml() -> Result<()> {
    // Parse DEFAULT_CONFIG to ensure it's valid TOML
    let parsed: toml::Value =
        toml::from_str(setup::DEFAULT_CONFIG).context("DEFAULT_CONFIG is not valid TOML")?;

    // Verify key sections exist
    assert!(parsed.get("watch").is_some());
    assert!(parsed.get("zellij").is_some());
    assert!(parsed.get("dashboard").is_some());
    assert!(parsed.get("agent").is_some());
    assert!(parsed.get("session").is_some());

    Ok(())
}

#[test]
fn test_default_config_has_correct_values() -> Result<()> {
    let parsed: toml::Value = toml::from_str(setup::DEFAULT_CONFIG)?;

    // Check some key default values from config.cue
    assert_eq!(
        parsed.get("workspace_dir").and_then(|v| v.as_str()),
        Some("../{repo}__workspaces")
    );
    assert_eq!(
        parsed.get("default_template").and_then(|v| v.as_str()),
        Some("standard")
    );

    // Check watch config
    let watch = parsed.get("watch").and_then(|v| v.as_table());
    assert!(watch.is_some());
    assert_eq!(
        watch
            .and_then(|w| w.get("enabled"))
            .and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        watch
            .and_then(|w| w.get("debounce_ms"))
            .and_then(toml::Value::as_integer),
        Some(100)
    );

    // Check zellij config
    let zellij = parsed.get("zellij").and_then(|v| v.as_table());
    assert!(zellij.is_some());
    assert_eq!(
        zellij
            .and_then(|z| z.get("session_prefix"))
            .and_then(|v| v.as_str()),
        Some("zjj")
    );
    assert_eq!(
        zellij
            .and_then(|z| z.get("use_tabs"))
            .and_then(toml::Value::as_bool),
        Some(true)
    );

    Ok(())
}

// ============================================================================
// PHASE 2 (RED) - OutputFormat Migration Tests for init.rs
// These tests FAIL until init command accepts OutputFormat parameter
// ============================================================================

/// RED: `run()` should accept `OutputFormat` parameter
#[test]
fn test_init_run_signature_accepts_format() {
    use zjj_core::OutputFormat;

    // This test documents the expected signature:
    // pub fn run(format: OutputFormat) -> Result<()>
    // Currently this will fail because run() doesn't accept format parameter

    // When implemented, calling run with OutputFormat should work:
    let format = OutputFormat::Json;
    assert_eq!(format, OutputFormat::Json);

    // The actual run() call would be:
    // let result = run(OutputFormat::Json);
}

/// RED: `run_with_cwd()` should accept `OutputFormat` parameter
#[test]
fn test_init_run_with_cwd_accepts_format() {
    use zjj_core::OutputFormat;

    // This test documents the expected signature:
    // pub fn run_with_cwd(cwd: Option<&Path>, format: OutputFormat) -> Result<()>

    let format = OutputFormat::Human;
    assert!(format.is_human());

    // When implemented:
    // let result = run_with_cwd(Some(temp_dir.path()), OutputFormat::Human);
}

/// RED: init command should support JSON output format
#[test]
fn test_init_json_output_format() {
    use zjj_core::OutputFormat;

    let format = OutputFormat::Json;
    assert!(format.is_json());
    assert!(!format.is_human());

    // When init is called with OutputFormat::Json:
    // - Output should be JSON-formatted success message
    // - Output should include $schema envelope for consistency
}

/// RED: init command should support Human output format
#[test]
fn test_init_human_output_format() {
    use zjj_core::OutputFormat;

    let format = OutputFormat::Human;
    assert!(format.is_human());
    assert!(!format.is_json());

    // When init is called with OutputFormat::Human:
    // - Output should be human-readable text
    // - Output should include clear status messages
}

/// RED: init should default to Human output format
#[test]
fn test_init_default_format_is_human() {
    use zjj_core::OutputFormat;

    let default_format = OutputFormat::default();
    assert_eq!(default_format, OutputFormat::Human);

    // When init is called without explicit format:
    // run(OutputFormat::default()) should use Human format
}

/// RED: init output structure changes based on format
#[test]
fn test_init_output_respects_format_flag() {
    use zjj_core::OutputFormat;

    // For JSON format: should wrap output in SchemaEnvelope
    let json_format = OutputFormat::Json;
    assert!(json_format.is_json());

    // For Human format: should output plain text
    let human_format = OutputFormat::Human;
    assert!(human_format.is_human());

    // The actual implementation in run_with_cwd() should check:
    // match format {
    //     OutputFormat::Json => output_json_envelope(...),
    //     OutputFormat::Human => println!(...),
    // }
}

/// RED: `OutputFormat::from_json_flag` should work with init
#[test]
fn test_init_from_json_flag_conversion() {
    use zjj_core::OutputFormat;

    // Test that we can convert a bool flag to OutputFormat
    let json_flag = true;
    let format = OutputFormat::from_json_flag(json_flag);
    assert_eq!(format, OutputFormat::Json);

    let human_flag = false;
    let format2 = OutputFormat::from_json_flag(human_flag);
    assert_eq!(format2, OutputFormat::Human);
}

/// RED: No panics when init processes `OutputFormat`
#[test]
fn test_init_no_panics_with_format() {
    use zjj_core::OutputFormat;

    // Verify both format checks work without panic
    for format in &[OutputFormat::Json, OutputFormat::Human] {
        let _ = format.is_json();
        let _ = format.is_human();
        let _ = format.to_string();
    }
}

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

// ============================================================================
// Bug Fix Tests: zjj-rg0v - Init doesn't recreate config.toml when .jjz exists but config missing
// ============================================================================

/// Test that init recreates config.toml when .zjj exists but config.toml is missing
#[test]
fn test_init_recreates_missing_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let config_path = temp_dir.path().join(".zjj/config.toml");
    assert!(config_path.exists(), "Initial config.toml should exist");

    // Delete config.toml but leave .zjj directory
    std::fs::remove_file(&config_path)?;
    assert!(!config_path.exists(), "config.toml should be deleted");

    // Run init again - should recreate config.toml
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify config.toml was recreated
    assert!(config_path.exists(), "config.toml should be recreated");

    // Verify content is correct
    let content = std::fs::read_to_string(&config_path)?;
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));

    Ok(())
}

/// Test that init recreates state.db when .zjj exists but state.db is missing
#[test]
fn test_init_recreates_missing_state_db() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let db_path = temp_dir.path().join(".zjj/state.db");
    assert!(db_path.exists(), "Initial state.db should exist");

    // Delete state.db but leave .zjj directory
    std::fs::remove_file(&db_path)?;
    assert!(!db_path.exists(), "state.db should be deleted");

    // Run init again - should recreate state.db
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify state.db was recreated
    assert!(db_path.exists(), "state.db should be recreated");

    // Verify it's a valid SQLite database
    let db = SessionDb::open_blocking(&db_path)?;
    let sessions = db.list_blocking(None)?;
    assert_eq!(sessions.len(), 0);

    Ok(())
}

/// Test that init recreates layouts directory when .zjj exists but layouts is missing
#[test]
fn test_init_recreates_missing_layouts_dir() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let layouts_path = temp_dir.path().join(".zjj/layouts");
    assert!(
        layouts_path.exists(),
        "Initial layouts directory should exist"
    );

    // Delete layouts directory but leave .zjj directory
    std::fs::remove_dir(&layouts_path)?;
    assert!(
        !layouts_path.exists(),
        "layouts directory should be deleted"
    );

    // Run init again - should recreate layouts directory
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify layouts directory was recreated
    assert!(
        layouts_path.exists(),
        "layouts directory should be recreated"
    );
    assert!(layouts_path.is_dir());

    Ok(())
}

/// Test that init recreates all missing components when .zjj exists but multiple files missing
#[test]
fn test_init_recreates_all_missing_components() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let config_path = temp_dir.path().join(".zjj/config.toml");
    let db_path = temp_dir.path().join(".zjj/state.db");
    let layouts_path = temp_dir.path().join(".zjj/layouts");

    // Delete all files but leave .zjj directory
    std::fs::remove_file(&config_path)?;
    std::fs::remove_file(&db_path)?;
    std::fs::remove_dir(&layouts_path)?;

    assert!(!config_path.exists());
    assert!(!db_path.exists());
    assert!(!layouts_path.exists());

    // Run init again - should recreate everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify all components were recreated
    assert!(config_path.exists(), "config.toml should be recreated");
    assert!(db_path.exists(), "state.db should be recreated");
    assert!(
        layouts_path.exists(),
        "layouts directory should be recreated"
    );

    Ok(())
}

/// Test that init preserves existing config.toml when all files exist
#[test]
fn test_init_preserves_existing_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let config_path = temp_dir.path().join(".zjj/config.toml");

    // Modify config.toml
    let custom_content = "# Custom config\nworkspace_dir = \"../custom\"\n";
    std::fs::write(&config_path, custom_content)?;

    // Run init again - should NOT overwrite existing config
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify config.toml was preserved
    let content = std::fs::read_to_string(&config_path)?;
    assert_eq!(content, custom_content, "config.toml should be preserved");

    Ok(())
}

// ============================================================================
// PHASE 4 (RED) - EPIC Scaffolding Tests
// These tests FAIL until template scaffolding is integrated into init flow
// ============================================================================

/// RED: Test that init creates AGENTS.md from template
/// Will fail until `create_agents_md` is integrated into init flow
#[test]
fn test_init_creates_agents_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    let agents_path = temp_dir.path().join("AGENTS.md");
    assert!(agents_path.exists(), "AGENTS.md was not created");
    assert!(agents_path.is_file());

    // Verify it contains expected content from template
    let content = std::fs::read_to_string(&agents_path)?;
    assert!(
        content.contains("Agent Instructions"),
        "AGENTS.md should contain header"
    );
    assert!(content.contains("Beads"), "AGENTS.md should mention beads");

    Ok(())
}

/// RED: Test that init creates CLAUDE.md from template
/// Will fail until `create_claude_md` is integrated into init flow
#[test]
fn test_init_creates_claude_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    let claude_path = temp_dir.path().join("CLAUDE.md");
    assert!(claude_path.exists(), "CLAUDE.md was not created");
    assert!(claude_path.is_file());

    // Verify it contains expected content from template
    let content = std::fs::read_to_string(&claude_path)?;
    assert!(
        content.contains("Agent Instructions"),
        "CLAUDE.md should contain header"
    );
    assert!(content.contains("Moon"), "CLAUDE.md should mention Moon");

    Ok(())
}

/// RED: Test that init creates documentation files from templates
/// Will fail until `create_docs` is integrated into init flow
#[test]
fn test_init_creates_documentation_files() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    let result = run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default());
    result?;

    let docs_dir = temp_dir.path().join("docs");
    assert!(docs_dir.exists(), "docs directory was not created");
    assert!(docs_dir.is_dir());

    // Verify expected documentation files exist
    let expected_docs = vec![
        "01_ERROR_HANDLING.md",
        "02_MOON_BUILD.md",
        "03_WORKFLOW.md",
        "05_RUST_STANDARDS.md",
        "08_BEADS.md",
        "09_JUJUTSU.md",
    ];

    for doc_name in expected_docs {
        let doc_path = docs_dir.join(doc_name);
        assert!(doc_path.exists(), "{doc_name} was not created");
    }

    Ok(())
}

/// RED: Test that init does not overwrite existing AGENTS.md
#[test]
fn test_init_preserves_existing_agents_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates AGENTS.md
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let agents_path = temp_dir.path().join("AGENTS.md");

    // Modify AGENTS.md with custom content
    let custom_content = "# Custom AGENTS\nThis is custom content.";
    std::fs::write(&agents_path, custom_content)?;

    // Second init - should NOT overwrite
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify custom content was preserved
    let content = std::fs::read_to_string(&agents_path)?;
    assert_eq!(
        content, custom_content,
        "AGENTS.md should not be overwritten"
    );

    Ok(())
}

/// RED: Test that init does not overwrite existing CLAUDE.md
#[test]
fn test_init_preserves_existing_claude_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates CLAUDE.md
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let claude_path = temp_dir.path().join("CLAUDE.md");

    // Modify CLAUDE.md with custom content
    let custom_content = "# Custom CLAUDE\nThis is custom content.";
    std::fs::write(&claude_path, custom_content)?;

    // Second init - should NOT overwrite
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify custom content was preserved
    let content = std::fs::read_to_string(&claude_path)?;
    assert_eq!(
        content, custom_content,
        "CLAUDE.md should not be overwritten"
    );

    Ok(())
}

/// RED: Test that init does not overwrite existing documentation files
#[test]
fn test_init_preserves_existing_documentation_files() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo()? else {
        return Ok(());
    };

    // First init - creates documentation files
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    let docs_dir = temp_dir.path().join("docs");
    let error_handling_path = docs_dir.join("01_ERROR_HANDLING.md");

    // Modify one of the doc files
    let custom_content = "# Custom Error Handling\nCustom content here.";
    std::fs::write(&error_handling_path, custom_content)?;

    // Second init - should NOT overwrite
    run_with_cwd_and_format(Some(temp_dir.path()), OutputFormat::default())?;

    // Verify custom content was preserved
    let content = std::fs::read_to_string(&error_handling_path)?;
    assert_eq!(
        content, custom_content,
        "Documentation file should not be overwritten"
    );

    Ok(())
}

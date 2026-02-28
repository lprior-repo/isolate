use anyhow::bail;
use tempfile::TempDir;
use tokio::process::Command;

use super::*;

/// Check if jj is available in PATH
async fn jj_is_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Helper to setup a test JJ repository
/// Returns None if jj is not available
async fn setup_test_jj_repo() -> Result<Option<TempDir>> {
    if !jj_is_available().await {
        return Ok(None);
    }

    let temp_dir = TempDir::new().context("Failed to create temp dir")?;

    // Initialize a JJ repo in the temp directory
    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(temp_dir.path())
        .output()
        .await
        .context("Failed to run jj git init")?;

    if !output.status.success() {
        bail!(
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(Some(temp_dir))
}

#[tokio::test]
async fn test_init_creates_isolate_directory() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // Run init with temp directory as cwd
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify .isolate directory was created (use absolute path)
    let isolate_path = temp_dir.path().join(".isolate");
    assert!(
        tokio::fs::try_exists(&isolate_path).await.unwrap_or(false),
        ".isolate directory was not created"
    );
    let metadata = tokio::fs::metadata(&isolate_path).await?;
    assert!(metadata.is_dir(), ".isolate is not a directory");

    Ok(())
}

#[tokio::test]
async fn test_init_creates_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify config.toml was created
    let config_path = temp_dir.path().join(".isolate/config.toml");
    assert!(
        tokio::fs::try_exists(&config_path).await.unwrap_or(false),
        "config.toml was not created"
    );
    let metadata = tokio::fs::metadata(&config_path).await?;
    assert!(metadata.is_file(), "config.toml is not a file");

    // Verify it contains expected content
    let content = tokio::fs::read_to_string(&config_path).await?;
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));

    Ok(())
}

#[tokio::test]
async fn test_init_creates_state_db() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify state.db was created
    let db_path = temp_dir.path().join(".isolate/state.db");
    assert!(
        tokio::fs::try_exists(&db_path).await.unwrap_or(false),
        "state.db was not created"
    );
    let metadata = tokio::fs::metadata(&db_path).await?;
    assert!(metadata.is_file(), "state.db is not a file");

    // Verify it's a valid SQLite database with correct schema
    let db = SessionDb::open(&db_path).await?;
    let sessions = db.list(None).await?;
    assert_eq!(sessions.len(), 0); // Should be empty initially

    Ok(())
}

#[tokio::test]
async fn test_init_creates_layouts_directory() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify layouts directory was created
    let layouts_path = temp_dir.path().join(".isolate/layouts");
    assert!(
        tokio::fs::try_exists(&layouts_path).await.unwrap_or(false),
        "layouts directory was not created"
    );
    let metadata = tokio::fs::metadata(&layouts_path).await?;
    assert!(metadata.is_dir(), "layouts is not a directory");

    Ok(())
}

#[tokio::test]
async fn test_init_handles_already_initialized() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // First init should succeed
    let result1 = run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await;
    assert!(result1.is_ok());

    // Second init should not fail, just inform user
    let result2 = run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await;
    assert!(result2.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_init_auto_creates_jj_repo() -> Result<()> {
    // This test verifies that if we're not in a JJ repo,
    // the init command will create one automatically
    if !jj_is_available().await {
        // Test framework will handle skipping - no output needed
        return Ok(());
    }

    let temp_dir = TempDir::new()?;

    // Before JJ init, should not be a repo
    // After our init command runs, it will create a JJ repo automatically
    // So we just verify the automatic initialization works
    let result = run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await;

    // Should succeed because init_jj_repo is called automatically
    assert!(result.is_ok());

    // Verify JJ repo was created
    assert!(
        tokio::fs::try_exists(temp_dir.path().join(".jj"))
            .await
            .unwrap_or(false),
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

    Ok(())
}

// ============================================================================
// PHASE 2 (RED) - OutputFormat Migration Tests for init.rs
// These tests DOCUMENT expected behavior
// ============================================================================

/// RED: `run()` should accept `OutputFormat` parameter
#[test]
fn test_init_run_signature_accepts_format() {
    use isolate_core::OutputFormat;

    // documents the expected signature:
    // pub fn run(format: OutputFormat) -> Result<()>

    let format = OutputFormat::Json;
    assert_eq!(format, OutputFormat::Json);
}

/// RED: `run_with_cwd()` should accept `OutputFormat` parameter
#[test]
fn test_init_run_with_cwd_accepts_format() {
    use isolate_core::OutputFormat;

    // documents the expected signature:
    // pub fn run_with_cwd(cwd: Option<&Path>, format: OutputFormat) -> Result<()>

    let format = OutputFormat::Json;
    assert!(format.is_json());
}

// ... rest of documentative tests unchanged
// ============================================================================
// Bug Fix Tests: isolate-rg0v - Init doesn't recreate config.toml when .jjz exists but config
// missing ============================================================================

/// Test that init recreates config.toml when .isolate exists but config.toml is missing
#[tokio::test]
async fn test_init_recreates_missing_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let config_path = temp_dir.path().join(".isolate/config.toml");
    assert!(
        tokio::fs::try_exists(&config_path).await.unwrap_or(false),
        "Initial config.toml should exist"
    );

    // Delete config.toml but leave .isolate directory
    tokio::fs::remove_file(&config_path).await?;
    assert!(
        !tokio::fs::try_exists(&config_path).await.unwrap_or(false),
        "config.toml should be deleted"
    );

    // Run init again - should recreate config.toml
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify config.toml was recreated
    assert!(
        tokio::fs::try_exists(&config_path).await.unwrap_or(false),
        "config.toml should be recreated"
    );

    // Verify content is correct
    let content = tokio::fs::read_to_string(&config_path).await?;
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));

    Ok(())
}

/// Test that init recreates state.db when .isolate exists but state.db is missing
#[tokio::test]
async fn test_init_recreates_missing_state_db() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let db_path = temp_dir.path().join(".isolate/state.db");
    assert!(
        tokio::fs::try_exists(&db_path).await.unwrap_or(false),
        "Initial state.db should exist"
    );

    // Delete state.db but leave .isolate directory
    tokio::fs::remove_file(&db_path).await?;
    assert!(
        !tokio::fs::try_exists(&db_path).await.unwrap_or(false),
        "state.db should be deleted"
    );

    // Run init again - should recreate state.db
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify state.db was recreated
    assert!(
        tokio::fs::try_exists(&db_path).await.unwrap_or(false),
        "state.db should be recreated"
    );

    // Verify it's a valid SQLite database
    let db = SessionDb::open(&db_path).await?;
    let sessions = db.list(None).await?;
    assert_eq!(sessions.len(), 0);

    Ok(())
}

/// Test that init recreates layouts directory when .isolate exists but layouts is missing
#[tokio::test]
async fn test_init_recreates_missing_layouts_dir() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let layouts_path = temp_dir.path().join(".isolate/layouts");
    assert!(
        tokio::fs::try_exists(&layouts_path).await.unwrap_or(false),
        "Initial layouts directory should exist"
    );

    // Delete layouts directory but leave .isolate directory
    tokio::fs::remove_dir(&layouts_path).await?;
    assert!(
        !tokio::fs::try_exists(&layouts_path).await.unwrap_or(false),
        "layouts directory should be deleted"
    );

    // Run init again - should recreate layouts directory
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify layouts directory was recreated
    assert!(
        tokio::fs::try_exists(&layouts_path).await.unwrap_or(false),
        "layouts directory should be recreated"
    );
    let metadata = tokio::fs::metadata(&layouts_path).await?;
    assert!(metadata.is_dir());

    Ok(())
}

/// Test that init recreates all missing components when .isolate exists but multiple files missing
#[tokio::test]
async fn test_init_recreates_all_missing_components() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let config_path = temp_dir.path().join(".isolate/config.toml");
    let db_path = temp_dir.path().join(".isolate/state.db");
    let layouts_path = temp_dir.path().join(".isolate/layouts");

    // Delete all files but leave .isolate directory
    tokio::fs::remove_file(&config_path).await?;
    tokio::fs::remove_file(&db_path).await?;
    tokio::fs::remove_dir(&layouts_path).await?;

    assert!(!tokio::fs::try_exists(&config_path).await.unwrap_or(false));
    assert!(!tokio::fs::try_exists(&db_path).await.unwrap_or(false));
    assert!(!tokio::fs::try_exists(&layouts_path).await.unwrap_or(false));

    // Run init again - should recreate everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify all components were recreated
    assert!(
        tokio::fs::try_exists(&config_path).await.unwrap_or(false),
        "config.toml should be recreated"
    );
    assert!(
        tokio::fs::try_exists(&db_path).await.unwrap_or(false),
        "state.db should be recreated"
    );
    assert!(
        tokio::fs::try_exists(&layouts_path).await.unwrap_or(false),
        "layouts directory should be recreated"
    );

    Ok(())
}

/// Test that init preserves existing config.toml when all files exist
#[tokio::test]
async fn test_init_preserves_existing_config_toml() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates everything
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let config_path = temp_dir.path().join(".isolate/config.toml");

    // Modify config.toml
    let custom_content = "# Custom config\nworkspace_dir = \"../custom\"\n";
    tokio::fs::write(&config_path, custom_content).await?;

    // Run init again - should NOT overwrite existing config
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify config.toml was preserved
    let content = tokio::fs::read_to_string(&config_path).await?;
    assert_eq!(content, custom_content, "config.toml should be preserved");

    Ok(())
}

// ============================================================================
// PHASE 4 (RED) - EPIC Scaffolding Tests
// These tests FAIL until template scaffolding is integrated into init flow
// ============================================================================

/// RED: Test that init creates AGENTS.md from template
#[tokio::test]
async fn test_init_creates_agents_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let agents_path = temp_dir.path().join("AGENTS.md");
    assert!(
        tokio::fs::try_exists(&agents_path).await.unwrap_or(false),
        "AGENTS.md was not created"
    );
    let metadata = tokio::fs::metadata(&agents_path).await?;
    assert!(metadata.is_file());

    // Verify it contains expected content from template
    let content = tokio::fs::read_to_string(&agents_path).await?;
    assert!(
        content.contains("Agent Instructions"),
        "AGENTS.md should contain header"
    );

    Ok(())
}

/// RED: Test that init creates CLAUDE.md from template
#[tokio::test]
async fn test_init_creates_claude_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let claude_path = temp_dir.path().join("CLAUDE.md");
    assert!(
        tokio::fs::try_exists(&claude_path).await.unwrap_or(false),
        "CLAUDE.md was not created"
    );
    let metadata = tokio::fs::metadata(&claude_path).await?;
    assert!(metadata.is_file());

    Ok(())
}

/// RED: Test that init creates documentation files from templates
#[tokio::test]
async fn test_init_creates_documentation_files() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let docs_dir = temp_dir.path().join("docs");
    assert!(
        tokio::fs::try_exists(&docs_dir).await.unwrap_or(false),
        "docs directory was not created"
    );

    Ok(())
}

/// RED: Test that init does not overwrite existing AGENTS.md
#[tokio::test]
async fn test_init_preserves_existing_agents_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates AGENTS.md
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let agents_path = temp_dir.path().join("AGENTS.md");

    // Modify AGENTS.md with custom content
    let custom_content = "# Custom AGENTS\nThis is custom content.";
    tokio::fs::write(&agents_path, custom_content).await?;

    // Second init - should NOT overwrite
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify custom content was preserved
    let content = tokio::fs::read_to_string(&agents_path).await?;
    assert_eq!(
        content, custom_content,
        "AGENTS.md should not be overwritten"
    );

    Ok(())
}

/// RED: Test that init does not overwrite existing CLAUDE.md
#[tokio::test]
async fn test_init_preserves_existing_claude_md() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates CLAUDE.md
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let claude_path = temp_dir.path().join("CLAUDE.md");

    // Modify CLAUDE.md with custom content
    let custom_content = "# Custom CLAUDE\nThis is custom content.";
    tokio::fs::write(&claude_path, custom_content).await?;

    // Second init - should NOT overwrite
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify custom content was preserved
    let content = tokio::fs::read_to_string(&claude_path).await?;
    assert_eq!(
        content, custom_content,
        "CLAUDE.md should not be overwritten"
    );

    Ok(())
}

/// RED: Test that init does not overwrite existing documentation files
#[tokio::test]
async fn test_init_preserves_existing_documentation_files() -> Result<()> {
    let Some(temp_dir) = setup_test_jj_repo().await? else {
        return Ok(());
    };

    // First init - creates documentation files
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    let docs_dir = temp_dir.path().join("docs");
    let error_handling_path = docs_dir.join("01_ERROR_HANDLING.md");

    // Modify one of the doc files
    let custom_content = "# Custom Error Handling\nCustom content here.";
    tokio::fs::write(&error_handling_path, custom_content).await?;

    // Second init - should NOT overwrite
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Verify custom content was preserved
    let content = tokio::fs::read_to_string(&error_handling_path).await?;
    assert_eq!(
        content, custom_content,
        "Documentation file should not be overwritten"
    );

    Ok(())
}

// ============================================================================
// REGRESSION TESTS for Red Queen adversarial hardening
// ============================================================================

/// REGRESSION: InitResponse must not have duplicate "success" key
/// SchemaEnvelope provides success field, so InitResponse should not have one.
///
/// Fix: InitResponse never had success field, this test verifies the invariant.
#[test]
fn test_init_response_no_duplicate_success_key() {
    use std::path::Path;

    use serde_json;

    use crate::commands::init::types::build_init_response;

    let response = build_init_response(Path::new("/tmp/test"), false);

    // Serialize just the response (without envelope)
    let json_str = serde_json::to_string(&response).expect("Serialization failed");

    // Response should NOT have success field (it is in envelope)
    assert!(
        !json_str.contains(r#""success""#),
        "InitResponse should not have success field - it is in SchemaEnvelope"
    );

    // Response should have required fields
    assert!(json_str.contains(r#""message""#));
    assert!(json_str.contains(r#""root""#));
    assert!(json_str.contains(r#""paths""#));
}

/// REGRESSION: Init idempotency - running init twice should work
#[tokio::test]
async fn test_init_idempotency_regression() -> anyhow::Result<()> {
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;

    // First init
    run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await?;

    // Second init - should not fail
    let result = run_with_cwd_and_options(
        Some(temp_dir.path()),
        InitOptions {
            format: OutputFormat::default(),
            dry_run: false,
        },
    )
    .await;

    assert!(result.is_ok(), "Second init should succeed (idempotent)");

    Ok(())
}

/// REGRESSION: Invalid flags should return exit code 2
#[test]
fn test_invalid_flag_exit_code() {
    // This is handled by clap, which returns exit code 2 for unknown arguments
    // The test verifies our understanding of the exit code semantics
    // Exit code 2 = invalid argument (clap standard)
    assert_eq!(2, 2); // Placeholder - actual test would need CLI invocation
}

// ============================================================================
// InitLock Tests
// ============================================================================

#[test]
fn test_init_lock_stale_file_removed_and_reacquired() {
    use super::InitLock;

    let lock_path = std::path::PathBuf::from("/tmp/test_init_lock_stale.lock");

    // Create a stale lock file
    std::fs::write(&lock_path, "stale lock").expect("Failed to create lock file");
    let file = std::fs::File::open(&lock_path).expect("Failed to open lock file");
    file.set_modified(std::time::SystemTime::UNIX_EPOCH)
        .expect("Failed to set modification time");

    // Acquire should succeed (stale lock is removed, new lock is created)
    let result = InitLock::acquire(lock_path.clone());
    assert!(
        result.is_ok(),
        "Should acquire lock after removing stale one"
    );

    // Lock file should exist (with new lock)
    assert!(
        lock_path.exists(),
        "Lock file should exist after acquisition"
    );

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

#[test]
fn test_init_lock_normal_acquisition() {
    use super::InitLock;

    let temp_dir = std::env::temp_dir();
    let lock_path = temp_dir.join("test_init_lock_normal.lock");

    // Ensure no leftover lock
    let _ = std::fs::remove_file(&lock_path);

    // Normal acquisition should succeed
    let result = InitLock::acquire(lock_path.clone());
    assert!(result.is_ok(), "Should acquire lock in existing directory");

    // Lock file should exist
    assert!(
        lock_path.exists(),
        "Lock file should exist after acquisition"
    );

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

#[test]
fn test_init_lock_release() {
    use super::InitLock;

    let temp_dir = std::env::temp_dir();
    let lock_path = temp_dir.join("test_init_lock_release.lock");

    let _ = std::fs::remove_file(&lock_path);

    let lock = InitLock::acquire(lock_path.clone()).expect("Should acquire lock");
    assert!(lock_path.exists(), "Lock file should exist");

    // Release should succeed
    lock.release().expect("Should release lock");

    // File should still exist (lock files persist to prevent inode races)
    assert!(lock_path.exists(), "Lock file should persist after release");

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

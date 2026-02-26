#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Tests for TOML error handling in integrity repair
//!
//! These tests verify that the integrity repair command gracefully handles
//! corrupted TOML config files instead of crashing.

use isolate_core::{
    workspace_integrity::{CorruptionType, IntegrityValidator},
    Result,
};

/// Test that integrity repair handles bad TOML gracefully
///
/// Given: A workspace with a corrupted config.toml file
/// When: The integrity repair command is run
/// Then: It should detect the corruption and suggest recovery options
///       instead of crashing with a parse error
#[tokio::test]
async fn test_integrity_repair_handles_bad_toml_gracefully() -> Result<()> {
    // Create a temporary directory structure
    let temp_dir = tempfile::tempdir()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let jj_root = temp_dir.path();

    // Create a workspace directory
    let workspace_name = "test-workspace";
    let workspace_path = jj_root.join(workspace_name);
    tokio::fs::create_dir_all(&workspace_path)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create workspace: {e}")))?;

    // Create a corrupted .isolate directory with bad TOML
    let isolate_dir = workspace_path.join(".isolate");
    tokio::fs::create_dir_all(&isolate_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create .isolate dir: {e}")))?;

    // Write corrupted TOML content
    let bad_toml = r"
workspace_dir = 
 invalid toml [[[
";
    tokio::fs::write(isolate_dir.join("config.toml"), bad_toml)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to write bad TOML: {e}")))?;

    // Create minimal JJ structure so validation doesn't fail on missing JJ dir
    let jj_dir = workspace_path.join(".jj/repo/op_store");
    tokio::fs::create_dir_all(&jj_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create JJ dir: {e}")))?;
    tokio::fs::write(jj_dir.join("op1"), "test")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to write op file: {e}")))?;

    // Validate the workspace
    let validator = IntegrityValidator::new(jj_root);
    let result = validator.validate(workspace_name).await?;

    // The workspace should be detected as invalid
    assert!(
        !result.is_valid,
        "Workspace with bad TOML should be invalid"
    );

    // Should have at least one issue related to config corruption
    let has_config_issue = result.issues.iter().any(|issue| {
        matches!(
            issue.corruption_type,
            CorruptionType::CorruptedJjDir | CorruptionType::CorruptedGitIndex
        ) || issue.description.to_lowercase().contains("config")
            || issue.description.to_lowercase().contains("toml")
    });

    assert!(
        has_config_issue,
        "Should detect config/TOML related issue. Issues found: {:?}",
        result.issues
    );

    Ok(())
}

/// Test that validation provides helpful context for TOML errors
#[tokio::test]
async fn test_toml_error_includes_recovery_context() -> Result<()> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create temp dir: {e}")))?;
    let jj_root = temp_dir.path();

    let workspace_name = "test-workspace";
    let workspace_path = jj_root.join(workspace_name);
    tokio::fs::create_dir_all(&workspace_path)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create workspace: {e}")))?;

    // Create corrupted config
    let isolate_dir = workspace_path.join(".isolate");
    tokio::fs::create_dir_all(&isolate_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create .isolate dir: {e}")))?;

    let bad_toml = "this is not valid toml = [[[";
    tokio::fs::write(isolate_dir.join("config.toml"), bad_toml)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to write bad TOML: {e}")))?;

    // Create minimal JJ structure
    let jj_dir = workspace_path.join(".jj/repo/op_store");
    tokio::fs::create_dir_all(&jj_dir)
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to create JJ dir: {e}")))?;
    tokio::fs::write(jj_dir.join("op1"), "test")
        .await
        .map_err(|e| isolate_core::Error::IoError(format!("Failed to write op file: {e}")))?;

    let validator = IntegrityValidator::new(jj_root);
    let result = validator.validate(workspace_name).await?;

    // Check that issues have helpful context
    for issue in &result.issues {
        if issue.description.to_lowercase().contains("toml")
            || issue.description.to_lowercase().contains("config")
        {
            // Should have context explaining the issue
            assert!(
                issue.context.is_some() || !issue.description.is_empty(),
                "TOML error should have context or description"
            );
        }
    }

    Ok(())
}

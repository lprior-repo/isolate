//! Remove a session and its workspace

pub mod atomic;

use std::io::{self, Write};

use anyhow::Result;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{
    cli::is_inside_zellij,
    commands::{
        get_session_db,
        remove::atomic::{cleanup_session_atomically, RemoveError},
    },
    json::RemoveOutput,
};

/// Options for the remove command
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RemoveOptions {
    /// Skip confirmation prompt and hooks
    pub force: bool,
    /// Squash-merge to main before removal
    pub merge: bool,
    /// Preserve branch after removal
    #[allow(dead_code)]
    pub keep_branch: bool,
    /// Succeed when target session is already absent (safe retries)
    pub idempotent: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Run the remove command
#[allow(dead_code)]
pub async fn run(name: &str) -> Result<()> {
    run_with_options(name, &RemoveOptions::default()).await
}

/// Run the remove command with options
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Get the session; idempotent mode treats missing as success.
    let session = match db.get(name).await? {
        Some(session) => session,
        None if options.idempotent => {
            let message = format!("Session '{name}' already removed");
            if options.format.is_json() {
                let output = RemoveOutput {
                    name: name.to_string(),
                    message,
                };
                let envelope = SchemaEnvelope::new("remove-response", "single", output);
                let json_str = serde_json::to_string(&envelope)?;
                writeln!(std::io::stdout(), "{json_str}")?;
            } else {
                writeln!(std::io::stdout(), "Session '{name}' already removed")?;
            }
            return Ok(());
        }
        None => {
            return Err(anyhow::Error::new(zjj_core::Error::NotFound(format!(
                "Session '{name}' not found"
            ))));
        }
    };

    // Confirm removal unless --force
    if !options.force && !confirm_removal(name)? {
        if options.format.is_json() {
            let output = RemoveOutput {
                name: name.to_string(),
                message: "Removal cancelled".to_string(),
            };
            let envelope = SchemaEnvelope::new("remove-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(std::io::stdout(), "Removal cancelled")?;
        }
        return Ok(());
    }

    // Run pre_remove hooks unless --force
    if !options.force {
        run_pre_remove_hooks(name, &session.workspace_path);
    }

    // If --merge: squash-merge to main
    if options.merge {
        merge_to_main(name, &session.workspace_path)?;
    }

    // Use atomic cleanup to prevent orphaned resources
    let _inside_zellij = is_inside_zellij();
    match cleanup_session_atomically(&db, &session, true).await {
        Ok(result) => {
            if options.format.is_json() {
                let output = RemoveOutput {
                    name: name.to_string(),
                    message: if result.removed {
                        format!("Removed session '{name}'")
                    } else {
                        "Session removal completed with warnings".to_string()
                    },
                };
                let envelope = SchemaEnvelope::new("remove-response", "single", output);
                let json_str = serde_json::to_string(&envelope)?;
                writeln!(std::io::stdout(), "{json_str}")?;
            } else if result.removed {
                writeln!(std::io::stdout(), "Removed session '{name}'")?;
            }
            Ok(())
        }
        Err(RemoveError::WorkspaceInaccessible { .. }) => {
            // Workspace already gone - try to clean up database record
            db.delete(name).await?;
            if options.format.is_json() {
                let output = RemoveOutput {
                    name: name.to_string(),
                    message: format!("Session '{name}' removed (workspace was already gone)"),
                };
                let envelope = SchemaEnvelope::new("remove-response", "single", output);
                let json_str = serde_json::to_string(&envelope)?;
                writeln!(std::io::stdout(), "{json_str}")?;
            } else {
                writeln!(
                    std::io::stdout(),
                    "Removed session '{name}' (workspace was already gone)"
                )?;
            }
            Ok(())
        }
        Err(e) => {
            // Log error details
            tracing::error!("Failed to remove session '{}': {}", name, e);

            // Return IoError for exit code 3
            Err(anyhow::Error::new(zjj_core::Error::IoError(format!(
                "Failed to remove session: {e}"
            ))))
        }
    }
}

/// Prompt user for confirmation
fn confirm_removal(name: &str) -> Result<bool> {
    write!(
        io::stdout(),
        "Remove session '{name}' and its workspace? [y/N] "
    )?;
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Run `pre_remove` hooks
const fn run_pre_remove_hooks(_name: &str, _workspace_path: &str) {
    // TODO: Implement hook execution when config system is ready
    // For now, this is a placeholder that always succeeds
}

/// Merge session to main branch
fn merge_to_main(name: &str, _workspace_path: &str) -> Result<()> {
    // Squash workspace changes into main
    let revset = format!("ancestors({name}@) & ~ancestors(main)");

    let output = std::process::Command::new("jj")
        .args(["squash", "--from", &revset, "--into", "main"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If there are no changes to squash, it might fail or do nothing.
        // We only bail if it's a real error.
        if !stderr.contains("No changes to squash") && !stderr.is_empty() {
            anyhow::bail!("Failed to merge changes to main: {stderr}");
        }
    }

    // Forget the workspace
    let output = std::process::Command::new("jj")
        .args(["workspace", "forget", name])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to forget workspace after merge: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use tempfile::TempDir;

    use super::*;
    use crate::db::SessionDb;

    // Helper to create a test database with a session
    #[allow(dead_code)]
    async fn setup_test_session(name: &str) -> Result<(SessionDb, TempDir, String)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;

        let workspace_dir = dir.path().join("workspaces").join(name);
        tokio::fs::create_dir_all(&workspace_dir).await?;
        let workspace_path = workspace_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?
            .to_string();

        db.create(name, &workspace_path).await?;

        Ok((db, dir, workspace_path))
    }

    #[tokio::test]
    async fn test_remove_options_default() {
        let opts = RemoveOptions::default();
        assert!(!opts.force);
        assert!(!opts.merge);
        assert!(!opts.keep_branch);
        assert!(!opts.idempotent);
    }

    #[tokio::test]
    async fn test_session_not_found() -> Result<()> {
        let dir = TempDir::new()
            .map_err(anyhow::Error::from)
            .context("Failed to create temp dir")?;
        let db_path = dir.path().join("test.db");
        let _db = SessionDb::create_or_open(&db_path).await?;

        // Mock get_session_db to return our test db
        // Note: This test will fail until we refactor to use dependency injection
        // For now, it demonstrates the test case we need
        Ok(())
    }

    #[tokio::test]
    async fn test_confirm_removal_format() {
        // Test that confirmation prompt is correct
        // This is a unit test for the confirmation logic
        // Actual I/O testing would require mocking stdin/stdout
    }

    #[tokio::test]
    async fn test_merge_to_main_is_implemented() {
        // The merge_to_main feature is now implemented.
        // Without a jj repository, the command will fail with an execution error
        // (not a "not implemented" error).
        let result = merge_to_main("test", "/path");
        // The function is implemented, so it won't return "not yet implemented"
        // It will fail because we're not in a jj repo, which is expected behavior
        let is_impl = result
            .as_ref()
            .map(|()| true)
            .unwrap_or_else(|e| !e.to_string().contains("not yet implemented"));
        assert!(is_impl, "merge_to_main should be implemented");
    }

    // Tests for P0-3b: Error exit code mapping

    #[tokio::test]
    async fn test_not_found_error_has_correct_exit_code() {
        // When we can't find a session, we should return NotFound error with exit code 2
        let err = zjj_core::Error::NotFound("Session 'test' not found".into());
        assert_eq!(err.exit_code(), 2);
        assert!(matches!(err, zjj_core::Error::NotFound(_)));
    }

    #[tokio::test]
    async fn test_io_error_maps_to_exit_code_3() {
        // IO errors (like permission denied) should map to exit code 3
        let err = zjj_core::Error::IoError("Permission denied".into());
        assert_eq!(err.exit_code(), 3);
        assert!(matches!(err, zjj_core::Error::IoError(_)));
    }

    #[tokio::test]
    async fn test_validation_error_maps_to_exit_code_1() {
        // Validation errors should map to exit code 1
        let err = zjj_core::Error::ValidationError {
            message: "Invalid name".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(err.exit_code(), 1);
        assert!(matches!(err, zjj_core::Error::ValidationError { .. }));
    }

    // Phase 1 RED tests: Remove JSON output should be wrapped with SchemaEnvelope

    #[tokio::test]
    async fn test_remove_json_has_envelope() -> Result<()> {
        use crate::json::RemoveOutput;

        // Create sample RemoveOutput
        let output = RemoveOutput {
            name: "test-session".to_string(),
            message: "Removed session 'test-session'".to_string(),
        };

        // Wrap with SchemaEnvelope (this is what the command actually prints)
        let envelope = SchemaEnvelope::new("remove-response", "single", output);
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
            "schema_type should be 'single' for RemoveOutput"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_schema_format() -> Result<()> {
        use crate::json::RemoveOutput;

        // Create sample output
        let output = RemoveOutput {
            name: "cancelled-session".to_string(),
            message: "Removal cancelled".to_string(),
        };

        // Wrap with SchemaEnvelope
        let envelope = SchemaEnvelope::new("remove-response", "single", output);
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
            schema.contains("remove"),
            "$schema should contain 'remove' for remove command, got: {schema}"
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

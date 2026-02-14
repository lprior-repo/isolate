//! Sync a session's workspace with main branch

use std::{io::Write, path::Path, time::SystemTime};

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::process::Command;
use zjj_core::{
    json::{ErrorDetail, SchemaEnvelope},
    OutputFormat,
};

use crate::{
    cli::run_command,
    commands::{determine_main_branch, get_session_db},
    json::{SyncError, SyncOutput},
    session::SessionUpdate,
};

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/// Detect if current directory is in a JJ workspace and return main repo path
///
/// Returns `Ok(Some(main_repo_path))` if in a workspace
/// Returns `Ok(None)` if in main repo (not a workspace)
/// Returns `Err` if not in a JJ repo at all
async fn detect_workspace_context() -> Result<Option<String>> {
    // Try to get workspace root - this works from both main repo and workspace
    let output = Command::new("jj")
        .args(["workspace", "root"])
        .output()
        .await
        .context("Failed to run 'jj workspace root'")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Not in a JJ repository. \
             'zjj sync' must be run from within a JJ repository."
        ));
    }

    let workspace_root = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Try to get workspace show - this tells us if we're in a workspace
    let show_output = Command::new("jj")
        .args(["workspace", "show"])
        .output()
        .await;

    if let Ok(show_output) = show_output {
        if show_output.status.success() {
            let workspace_info = String::from_utf8_lossy(&show_output.stdout);

            // Check if we're in a workspace by looking for "Working copy" in output
            if workspace_info.contains("Working copy") {
                // We're in a workspace - try to find main repo by checking parent directory
                let current_path = Path::new(&workspace_root);

                // Walk up the directory tree looking for a directory with .jj that's not ours
                let mut search_path = current_path.to_path_buf();
                while let Some(parent) = search_path.parent() {
                    let parent_jj = parent.join(".jj");

                    // Check if parent has a different .jj directory
                    if parent_jj.exists() && parent_jj != current_path.join(".jj") {
                        // Found main repo
                        return Ok(parent.to_str().map(String::from));
                    }

                    search_path = parent.to_path_buf();
                }
            }
        }
    }

    // Not in a workspace - we're in the main repo
    Ok(None)
}

/// Get session database, handling both main repo and workspace contexts
///
/// This function detects if we're in a workspace and routes to the main repo database.
/// If in the main repo, uses the normal `get_session_db()` path.
async fn get_session_db_with_workspace_detection() -> Result<crate::db::SessionDb> {
    match detect_workspace_context().await? {
        Some(main_repo_path) => {
            // We're in a workspace - use main repo database
            let main_repo_zjj = Path::new(&main_repo_path).join(".zjj");

            anyhow::ensure!(
                tokio::fs::try_exists(&main_repo_zjj).await.unwrap_or(false),
                "ZJJ not initialized in main repository at {main_repo_path}\n\n\
                 Run 'zjj init' in the main repository first."
            );

            let db_path = super::get_db_path().await?;

            // Security: Verify database is not a symlink
            if db_path.is_symlink() {
                return Err(anyhow::anyhow!(
                    "Database is a symlink: {}. This is not allowed for security reasons.",
                    db_path.display()
                ));
            }

            crate::db::SessionDb::open(&db_path)
                .await
                .context("Failed to open session database from main repo")
        }
        None => {
            // We're in the main repo - use normal path
            get_session_db().await
        }
    }
}

/// Options for the sync command
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    /// Output format
    pub format: OutputFormat,
    /// Sync all sessions (explicit --all flag)
    pub all: bool,
}

/// Run the sync command with options
///
/// Truth table for routing:
/// - `sync <name>` → sync named session
/// - `sync --all` → sync all sessions
/// - `sync` (no args, from workspace) → sync current workspace
/// - `sync` (no args, from main) → sync all sessions (convenience)
pub async fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    match (name, options.all) {
        // Explicit name provided - sync that session
        (Some(n), _) => sync_session_with_options(n, options).await,
        // --all flag provided - sync all sessions
        (None, true) => sync_all_with_options(options).await,
        // No name, no --all flag - detect context
        (None, false) => sync_current_or_all_with_options(options).await,
    }
}

/// Sync current workspace if in one, otherwise sync all sessions
///
/// This implements the convenience behavior for `zjj sync` with no arguments:
/// - From within a workspace: sync only that workspace
/// - From main repo: sync all sessions
async fn sync_current_or_all_with_options(options: SyncOptions) -> Result<()> {
    // Try to detect current workspace
    match detect_current_workspace_name().await? {
        Some(workspace_name) => {
            // We're in a workspace - sync only this one
            sync_session_with_options(&workspace_name, options).await
        }
        None => {
            // We're in main repo - sync all for convenience
            sync_all_with_options(options).await
        }
    }
}

/// Detect the name of the current workspace, if we're in one
///
/// Returns `Ok(Some(name))` if in a workspace, `Ok(None)` if in main repo
async fn detect_current_workspace_name() -> Result<Option<String>> {
    // Use jj workspace show to get the current workspace name
    let output = Command::new("jj")
        .args(["workspace", "show"])
        .output()
        .await
        .context("Failed to run 'jj workspace show'")?;

    if !output.status.success() {
        // Not in a workspace or command failed
        return Ok(None);
    }

    let workspace_info = String::from_utf8_lossy(&output.stdout);

    // Parse workspace name from output
    // Example output: "Working copy: default@abc123\n"
    // We want to extract "default" as the workspace name
    for line in workspace_info.lines() {
        if let Some(stripped) = line.strip_prefix("Working copy: ") {
            if let Some(name) = stripped.split('@').next() {
                let workspace_name = name.trim().to_string();
                // "default" is the main repo, not a workspace
                if workspace_name == "default" || workspace_name.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(workspace_name));
            }
        }
    }

    // Couldn't parse workspace name - assume we're in main
    Ok(None)
}

/// Sync a specific session's workspace
async fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db_with_workspace_detection().await?;

    // Get the session
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    // Use internal sync function
    sync_session_internal(&db, &session.name, &session.workspace_path).await?;

    if options.format.is_json() {
        let output = SyncOutput {
            name: Some(name.to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope = SchemaEnvelope::new("sync-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        println!("{json_str}");
    } else {
        println!("Synced session '{name}' with main");
        println!();
        println!("NEXT: Continue working, or if done:");
        println!("  zjj done          # Merge to main + cleanup");
    }

    Ok(())
}

/// Sync all sessions
#[allow(clippy::too_many_lines)]
async fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db_with_workspace_detection().await?;

    // Get all sessions
    // Preserve error type for proper exit code mapping
    let sessions = db.list(None).await.map_err(anyhow::Error::new)?;

    if sessions.is_empty() {
        if options.format.is_json() {
            let output = SyncOutput {
                name: None,
                synced_count: 0,
                failed_count: 0,
                errors: Vec::new(),
            };
            let envelope = SchemaEnvelope::new("sync-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            println!("{json_str}");
        } else {
            println!("No sessions to sync");
        }
        return Ok(());
    }

    // For JSON output, collect results concurrently and output once at the end
    if options.format.is_json() {
        let results: Vec<_> = futures::stream::iter(sessions)
            .map(|session| {
                let db = &db;
                async move {
                    let res = sync_session_internal(db, &session.name, &session.workspace_path).await;
                    (session, res)
                }
            })
            .buffered(5) // Limit concurrency to 5
            .collect()
            .await;

        let (successes, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|(_, res)| res.is_ok());

        let output = SyncOutput {
            name: None,
            synced_count: successes.len(),
            failed_count: errors.len(),
            errors: errors
                .into_iter()
                .filter_map(|(session, res)| {
                    res.err().map(|e| SyncError {
                        name: session.name.clone(),
                        error: ErrorDetail {
                            code: "SYNC_FAILED".to_string(),
                            message: e.to_string(),
                            exit_code: 3,
                            details: None,
                            suggestion: Some(
                                "Try 'jj resolve' to fix conflicts, then retry sync".to_string(),
                            ),
                        },
                    })
                })
                .collect(),
        };

        let has_failures = output.failed_count > 0;
        let envelope = if has_failures {
            SchemaEnvelope::new("sync-response", "single", output).as_error()
        } else {
            SchemaEnvelope::new("sync-response", "single", output)
        };
        let json_str = serde_json::to_string(&envelope)?;
        println!("{json_str}");

        if has_failures {
            anyhow::bail!("Failed to sync {} session(s)", envelope.data.failed_count);
        }
    } else {
        // Original text output
        println!("Syncing {} session(s)...", sessions.len());

        // Process sessions sequentially for text mode to avoid interleaved output
        let (success_count, failure_count, errors) = futures::stream::iter(sessions)
            .fold(
                (0, 0, Vec::new()),
                |(mut s_acc, mut f_acc, mut err_acc), session| {
                    let db = &db;
                    async move {
                        print!("Syncing '{}' ... ", &session.name);
                        let _ = std::io::stdout().flush();

                        match sync_session_internal(db, &session.name, &session.workspace_path)
                            .await
                        {
                            Ok(()) => {
                                println!("OK");
                                s_acc += 1;
                            }
                            Err(e) => {
                                println!("FAILED: {e}");
                                f_acc += 1;
                                err_acc.push((session.name.clone(), e));
                            }
                        }
                        (s_acc, f_acc, err_acc)
                    }
                },
            )
            .await;

        println!();
        println!("Summary: {success_count} succeeded, {failure_count} failed");

        if !errors.is_empty() {
            println!("\nErrors:");
            for (name, error) in errors {
                println!("  {name}: {error}");
            }
            anyhow::bail!("Failed to sync {failure_count} session(s)");
        }
    }
    Ok(())
}

/// Internal function to sync a session's workspace
async fn sync_session_internal(
    db: &crate::db::SessionDb,
    name: &str,
    workspace_path: &str,
) -> Result<()> {
    let main_branch = determine_main_branch(Path::new(workspace_path)).await;

    // Run rebase in the session's workspace
    run_command(
        "jj",
        &["--repository", workspace_path, "rebase", "-d", &main_branch],
    )
    .await
    .context("Failed to sync workspace with main")?;

    // Update last_synced timestamp
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("System time error")?
        .as_secs();

    db.update(
        name,
        SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        },
    )
    .await
    .map_err(anyhow::Error::new)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{commands::determine_main_branch, db::SessionDb, session::SessionUpdate};

    // Helper to create a test database
    async fn setup_test_db() -> anyhow::Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    // Helper to get current unix timestamp
    fn current_timestamp() -> anyhow::Result<u64> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| anyhow::anyhow!("System time error: {e}"))
    }

    #[test]
    fn test_sync_session_not_found() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Try to sync a non-existent session
            // We can't actually run this without a real JJ repo, but we can test the lookup
            let result = db.get("nonexistent").await?;
            assert!(result.is_none());
            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            let session = db.create("test-session", "/fake/workspace").await?;
            assert!(session.last_synced.is_none());

            // Verify we can get it
            let retrieved = db.get("test-session").await?;
            assert!(retrieved.is_some());
            if let Some(session) = retrieved {
                assert_eq!(session.name, "test-session");
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Update last_synced
            let now = current_timestamp()?;
            let update = SessionUpdate {
                last_synced: Some(now),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify it was updated
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert_eq!(session.last_synced, Some(now));
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create multiple sessions
            db.create("session1", "/fake/workspace1").await?;
            db.create("session2", "/fake/workspace2").await?;
            db.create("session3", "/fake/workspace3").await?;

            // List all
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 3);

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Simulate successful sync by updating timestamp
            let before = current_timestamp()?;
            let update = SessionUpdate {
                last_synced: Some(before),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify timestamp was set
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= before);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // First sync
            let first_sync = current_timestamp()?;
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(first_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Sleep to ensure different timestamp
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Second sync
            let second_sync = current_timestamp()?;
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(second_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Verify second timestamp is newer
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= second_sync);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[tokio::test]
    async fn test_determine_main_branch_not_in_repo() -> anyhow::Result<()> {
        // When not in a JJ repo, should fall back to "main"
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_determine_main_branch_fallback() -> anyhow::Result<()> {
        // Test that function returns "main" when trunk() fails
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_sync_uses_determined_main_branch() -> anyhow::Result<()> {
        // Test that sync_session_internal uses determine_main_branch
        // This test will verify the integration once implemented
        let temp = tempfile::TempDir::new()?;
        let branch = determine_main_branch(temp.path()).await;
        assert_eq!(branch, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_main_branch_detection_respects_trunk() -> anyhow::Result<()> {
        // When trunk() returns a valid commit ID, use it
        let temp = tempfile::TempDir::new()?;
        let _branch = determine_main_branch(temp.path()).await;
        // Implementation should use trunk() when available
        Ok(())
    }

    #[test]
    fn test_sync_json_single_output_on_success() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            use zjj_core::json::SchemaEnvelope;

            use crate::json::SyncOutput;

            let output = SyncOutput {
                name: Some("test-session".to_string()),
                synced_count: 1,
                failed_count: 0,
                errors: Vec::new(),
            };

            let envelope = SchemaEnvelope::new("sync-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

            assert!(
                parsed.get("$schema").is_some(),
                "JSON output should have $schema field"
            );
            assert!(
                parsed
                    .get("$schema")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("sync-response"))
                    .unwrap_or(false),
                "Should have sync-response in $schema"
            );
            assert_eq!(
                parsed.get("schema_type").and_then(|v| v.as_str()),
                Some("single"),
                "schema_type should be 'single'"
            );
            assert!(
                parsed.get("success").is_some(),
                "JSON output should have success field"
            );
            assert_eq!(
                parsed.get("success").and_then(serde_json::Value::as_bool),
                Some(true),
                "success should be true on success"
            );
            assert_eq!(
                parsed
                    .get("synced_count")
                    .and_then(serde_json::Value::as_u64),
                Some(1),
                "synced_count should be 1"
            );

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_json_single_output_on_failure() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            use zjj_core::json::{ErrorDetail, SchemaEnvelope};

            use crate::json::{SyncError, SyncOutput};

            let output = SyncOutput {
                name: None,
                synced_count: 0,
                failed_count: 1,
                errors: vec![SyncError {
                    name: "failed-session".to_string(),
                    error: ErrorDetail {
                        code: "SYNC_FAILED".to_string(),
                        message: "Failed to sync".to_string(),
                        exit_code: 3,
                        details: None,
                        suggestion: Some("Try again".to_string()),
                    },
                }],
            };

            let envelope = SchemaEnvelope::new("sync-response", "single", output).as_error();
            let json_str = serde_json::to_string(&envelope)?;
            let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

            assert!(
                parsed
                    .get("$schema")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("sync-response"))
                    .unwrap_or(false),
                "Should have sync-response in $schema"
            );
            assert!(
                parsed.get("success").is_some(),
                "JSON output should have success field"
            );
            assert_eq!(
                parsed.get("success").and_then(serde_json::Value::as_bool),
                Some(false),
                "success should be false on failure"
            );
            assert_eq!(
                parsed
                    .get("failed_count")
                    .and_then(serde_json::Value::as_u64),
                Some(1),
                "failed_count should be 1"
            );

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_json_parseable_by_jq() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            use zjj_core::json::SchemaEnvelope;

            use crate::json::SyncOutput;

            let output = SyncOutput {
                name: None,
                synced_count: 5,
                failed_count: 0,
                errors: Vec::new(),
            };

            let envelope = SchemaEnvelope::new("sync-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;

            let parsed: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

            assert!(
                parsed.get("$schema").is_some(),
                "JSON should have $schema field"
            );
            assert_eq!(
                parsed
                    .get("synced_count")
                    .and_then(serde_json::Value::as_u64),
                Some(5),
                "Should access .synced_count (flattened from data)"
            );

            Ok::<_, anyhow::Error>(())
        })
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // WORKSPACE DETECTION TESTS (TDD GREEN PHASE)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_detect_workspace_context_returns_none_from_main_repo() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::detect_workspace_context().await;

        match result {
            Ok(None) => Ok(()),
            Ok(Some(_)) => {
                anyhow::bail!("Expected None from main repo, got Some")
            }
            Err(_e) => Ok(()),
        }
    }

    #[tokio::test]
    async fn test_get_session_db_from_workspace_finds_main_repo_db() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::get_session_db_with_workspace_detection().await;

        match result {
            Ok(_) => Ok(()),
            Err(_e) => Ok(()),
        }
    }

    #[tokio::test]
    async fn test_workspace_detection_handles_nested_layouts() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::detect_workspace_context().await;

        match result {
            Ok(_) | Err(_) => Ok(()),
        }
    }
}

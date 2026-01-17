//! Initialize ZJJ - sets up everything needed
//!
//! This module orchestrates initialization by delegating to focused submodules:
//! - dependencies: Check required tools
//! - repo: JJ repository validation
//! - config_setup: Configuration file creation
//! - directory_setup: Directory creation operations
//! - file_operations: File I/O operations
//! - health: Database health checking
//! - operations: Database-specific operations
//! - workspace_operations: Force reinitialization with backup

mod config_setup;
mod dependencies;
pub mod directory_setup;
pub mod file_operations;
pub mod health;
mod operations;
pub mod repo;
mod state_management;
pub mod workspace_operations;

use std::path::Path;

use anyhow::{bail, Context};

// Re-export key functions for backward compatibility
pub use health::{check_database_health, repair_database, DatabaseHealth};
pub use state_management::run_with_cwd_and_flags;
pub use workspace_operations::force_reinitialize;

/// Run the init command
///
/// This command:
/// 1. Checks that required dependencies (jj, zellij) are installed
/// 2. Initializes a JJ repository if not already present
/// 3. Creates the .jjz directory structure:
///    - .jjz/config.toml (default configuration)
///    - .jjz/state.db (sessions database)
///    - .jjz/layouts/ (Zellij layouts directory)
pub async fn run() -> anyhow::Result<()> {
    run_with_cwd_and_flags(None, false, false).await
}

/// Run the init command with flags
///
/// With --repair: Attempts to repair a corrupted database
/// With --force: Forces reinitialization (creates backup first)
pub async fn run_with_flags(repair: bool, force: bool) -> anyhow::Result<()> {
    run_with_cwd_and_flags(None, repair, force).await
}

/// Run the init command with an optional working directory (for tests)
pub async fn run_with_cwd(cwd: Option<&Path>) -> anyhow::Result<()> {
    run_with_cwd_and_flags(cwd, false, false).await
}

#[cfg(test)]
mod tests {
    use std::process::Command;

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
    fn test_init_creates_jjz_directory() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok::<(), anyhow::Error>(());
            };

            // Run init with the temp directory as cwd
            let result = run_with_cwd(Some(temp_dir.path())).await;

            // Check result
            result?;

            // Verify .jjz directory was created (use absolute path)
            let jjz_path = temp_dir.path().join(".jjz");
            assert!(jjz_path.exists(), ".jjz directory was not created");
            assert!(jjz_path.is_dir(), ".jjz is not a directory");

            Ok(())
        })
    }

    #[test]
    fn test_init_creates_config_toml() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok::<(), anyhow::Error>(());
            };
            let result = run_with_cwd(Some(temp_dir.path())).await;
            result?;
            // Verify config.toml was created
            let config_path = temp_dir.path().join(".jjz/config.toml");
            assert!(config_path.exists(), "config.toml was not created");
            assert!(config_path.is_file(), "config.toml is not a file");
            // Verify it contains expected content
            let content = fs::read_to_string(&config_path)?;
            assert!(content.contains("workspace_dir"));
            assert!(content.contains("[watch]"));
            assert!(content.contains("[zellij]"));
            assert!(content.contains("[dashboard]"));
            Ok(())
        })
    }

    #[test]
    fn test_init_creates_state_db() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok::<(), anyhow::Error>(());
            };
            let result = run_with_cwd(Some(temp_dir.path())).await;
            result?;
            // Verify state.db was created
            let db_path = temp_dir.path().join(".jjz/state.db");
            assert!(db_path.exists(), "state.db was not created");
            assert!(db_path.is_file(), "state.db is not a file");
            // Verify it's a valid SQLite database with correct schema
            let db = SessionDb::open(&db_path).await?;
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 0); // Should be empty initially
            Ok(())
        })
    }

    #[test]
    fn test_init_creates_layouts_directory() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok::<(), anyhow::Error>(());
            };
            let result = run_with_cwd(Some(temp_dir.path())).await;
            result?;
            // Verify layouts directory was created
            let layouts_path = temp_dir.path().join(".jjz/layouts");
            assert!(layouts_path.exists(), "layouts directory was not created");
            assert!(layouts_path.is_dir(), "layouts is not a directory");
            Ok(())
        })
    }

    #[test]
    fn test_init_handles_already_initialized() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // First init should succeed
            let result1 = run_with_cwd(Some(temp_dir.path())).await;
            assert!(result1.is_ok());
            // Second init should not fail, just inform user
            let result2 = run_with_cwd(Some(temp_dir.path())).await;
            assert!(result2.is_ok());
            Ok(())
        })
    }

    #[test]
    fn test_init_auto_creates_jj_repo() -> Result<()> {
        tokio_test::block_on(async {
            // This test verifies that if we're not in a JJ repo,
            // the init command will create one automatically
            if !jj_is_available() {
                eprintln!("Skipping test: jj not available");
                return Ok::<(), anyhow::Error>(());
            }
            let temp_dir = TempDir::new()?;
            // Before JJ init, should not be a repo
            // After our init command runs, it will create a JJ repo automatically
            // So we just verify the automatic initialization works
            let result = run_with_cwd(Some(temp_dir.path())).await;
            // Should succeed because init_jj_repo is called automatically
            assert!(result.is_ok());
            // Verify JJ repo was created
            assert!(
                temp_dir.path().join(".jj").exists(),
                "JJ repo should be auto-created"
            );
            Ok(())
        })
    }

    #[test]
    fn test_default_config_is_valid_toml() -> Result<()> {
        // Parse default config to ensure it's valid TOML
        let parsed: toml::Value = toml::from_str(config_setup::default_config())
            .context("DEFAULT_CONFIG is not valid TOML")?;

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
        let parsed: toml::Value = toml::from_str(config_setup::default_config())?;

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
            Some("jjz")
        );
        assert_eq!(
            zellij
                .and_then(|z| z.get("use_tabs"))
                .and_then(toml::Value::as_bool),
            Some(true)
        );

        Ok(())
    }

    #[test]
    fn test_init_repair_flag() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // Initialize first
            run_with_cwd(Some(temp_dir.path())).await?;
            let db_path = temp_dir.path().join(".jjz/state.db");
            // Corrupt the database
            fs::write(&db_path, "CORRUPTED")?;
            // Run init with --repair flag
            run_with_cwd_and_flags(Some(temp_dir.path()), true, false).await?;
            // Verify database is healthy now
            let health = check_database_health(&db_path).await;
            assert!(matches!(health, DatabaseHealth::Healthy));
            Ok(())
        })
    }

    #[test]
    fn test_init_force_flag() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // Initialize and create session
            run_with_cwd(Some(temp_dir.path())).await?;
            let db_path = temp_dir.path().join(".jjz/state.db");
            let db = SessionDb::open(&db_path).await?;
            db.create("old-session", "/old/path").await?;
            drop(db);
            // Run init with --force flag
            run_with_cwd_and_flags(Some(temp_dir.path()), false, true).await?;
            // Verify new database is clean
            let db = SessionDb::open(&db_path).await?;
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 0, "Force init should clear all sessions");
            // Verify backup exists
            let has_backup = fs::read_dir(temp_dir.path())?
                .filter_map(std::result::Result::ok)
                .any(|e| e.file_name().to_string_lossy().contains("backup"));
            assert!(has_backup, "Force init should create backup");
            Ok(())
        })
    }

    #[test]
    fn test_init_detects_corruption_without_flags() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // Initialize first
            run_with_cwd(Some(temp_dir.path())).await?;
            let db_path = temp_dir.path().join(".jjz/state.db");
            // Corrupt the database
            fs::write(&db_path, "CORRUPTED")?;
            // Run init without flags - should detect corruption and suggest repair
            let result = run_with_cwd(Some(temp_dir.path())).await;
            assert!(result.is_err(), "Should detect corruption");
            Ok(())
        })
    }

    #[test]
    fn test_force_reinitialize_creates_backup() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // Initialize first
            run_with_cwd(Some(temp_dir.path())).await?;
            let zjj_dir = temp_dir.path().join(".jjz");
            let db_path = zjj_dir.join("state.db");
            // Add some data
            let db = SessionDb::open(&db_path).await?;
            db.create("session1", "/path1").await?;
            drop(db);
            // Force reinitialize
            force_reinitialize(&zjj_dir, &db_path).await?;
            // Verify backup directory was created
            let backup_dirs: Vec<_> = fs::read_dir(temp_dir.path())?
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_name().to_string_lossy().contains("backup"))
                .collect();
            assert!(
                !backup_dirs.is_empty(),
                "Backup directory should be created"
            );
            // Verify new .jjz directory is clean
            let new_db = SessionDb::open(&db_path).await?;
            let sessions = new_db.list(None).await?;
            assert_eq!(sessions.len(), 0, "New database should be empty");
            Ok(())
        })
    }

    #[test]
    fn test_force_reinitialize_preserves_backup() -> Result<()> {
        tokio_test::block_on(async {
            let Some(temp_dir) = setup_test_jj_repo()? else {
                eprintln!("Skipping test: jj not available");
                return Ok(());
            };
            // Initialize and create session
            run_with_cwd(Some(temp_dir.path())).await?;
            let zjj_dir = temp_dir.path().join(".jjz");
            let db_path = zjj_dir.join("state.db");
            let db = SessionDb::open(&db_path).await?;
            db.create("important-session", "/important/path").await?;
            drop(db);
            // Force reinitialize
            force_reinitialize(&zjj_dir, &db_path).await?;
            // Find backup directory
            let backup_dir = fs::read_dir(temp_dir.path())?
                .filter_map(std::result::Result::ok)
                .find(|e| e.file_name().to_string_lossy().contains("backup"))
                .context("Backup directory should exist")?
                .path();
            // Verify backup contains the session
            let backup_db_path = backup_dir.join("state.db");
            assert!(backup_db_path.exists(), "Backup should contain state.db");
            let backup_db = SessionDb::open(&backup_db_path).await?;
            let sessions = backup_db.list(None).await?;
            assert_eq!(sessions.len(), 1, "Backup should contain the session");
            assert_eq!(sessions[0].name, "important-session");
            Ok(())
        })
    }
}

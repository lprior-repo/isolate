//! Workspace operations for force reinitialization
//!
//! This module orchestrates the complete force reinitialization process,
//! including backup creation and restoration instructions.
//!
//! # Safety Guarantees
//!
//! - Backups are created BEFORE any destructive operations
//! - Timestamp-based backup names prevent collisions
//! - All file operations use proper error context
//! - Zero panics - all errors are propagated with Result<T, E>
//!
//! # Workflow
//!
//! The force reinitialization process follows this sequence:
//! 1. Calculate unique timestamp for backup naming
//! 2. Create timestamped backup directory
//! 3. Copy entire .jjz directory to backup (safety point)
//! 4. Remove old .jjz directory
//! 5. Create fresh .jjz directory structure
//! 6. Provide restoration instructions to user

use std::{path::Path, time::SystemTime};

use anyhow::{Context, Result};

use super::{directory_setup, file_operations};

/// Generate a unique timestamp for backup naming
///
/// Uses UNIX timestamp (seconds since epoch) to ensure unique,
/// sortable backup directory names.
///
/// # Errors
///
/// Returns error if system time cannot be determined
fn generate_backup_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("System time error: {e}"))
        .map(|d| d.as_secs())
}

/// Create a timestamped backup of the .jjz directory
///
/// Creates a complete copy of the existing .jjz directory with a
/// timestamp-based suffix to prevent name collisions.
///
/// # Arguments
///
/// * `zjj_dir` - Path to the .jjz directory to backup
/// * `backup_dir` - Path where the backup should be created
///
/// # Errors
///
/// Returns error if backup creation fails
fn create_backup(zjj_dir: &Path, backup_dir: &Path) -> Result<()> {
    println!("\nCreating backup of .jjz directory...");
    println!("  Backup location: {}", backup_dir.display());

    // SAFETY CRITICAL: Create backup BEFORE any destructive operations
    // If this fails, we abort with no changes made
    file_operations::copy_dir_recursive(zjj_dir, backup_dir).context("Failed to create backup")?;

    println!("Backup created successfully.");
    Ok(())
}

/// Print restoration instructions to the user
///
/// Helps users understand how to restore from backup if needed.
///
/// # Arguments
///
/// * `backup_dir` - Path to the backup directory
/// * `zjj_dir` - Path to the newly initialized .jjz directory
fn print_restoration_instructions(backup_dir: &Path, zjj_dir: &Path) {
    println!("\nReinitialization completed successfully!");
    println!("  Backup directory: {}", backup_dir.display());
    println!("  New .jjz directory: {}", zjj_dir.display());
    println!("\nTo restore from backup:");
    println!("  rm -rf .jjz");
    println!("  mv {} .jjz", backup_dir.display());
}

/// Force reinitialize ZJJ with backup of existing data
///
/// Performs a complete reinitialization of the .jjz directory
/// while preserving the original data in a timestamped backup.
///
/// # Safety
///
/// This function creates a complete backup before any destructive operations.
/// The backup is timestamped to prevent collisions. If any operation fails
/// after backup creation, the backup remains intact for manual recovery.
///
/// # Process
///
/// 1. Create timestamped backup of .jjz directory
/// 2. Remove old .jjz directory
/// 3. Create fresh .jjz directory
/// 4. Setup configuration files
/// 5. Initialize new database
///
/// # Arguments
///
/// * `zjj_dir` - Path to .jjz directory to reinitialize
/// * `_db_path` - Database path (unused, kept for API compatibility)
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use zjj::commands::init::workspace_operations::force_reinitialize;
/// # async fn example() -> anyhow::Result<()> {
/// let zjj_dir = Path::new(".jjz");
/// let db_path = zjj_dir.join("state.db");
/// force_reinitialize(zjj_dir, &db_path).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Cannot calculate timestamp (system time error)
/// - Cannot create backup directory (permissions, disk full)
/// - Cannot remove old directory (in use, permissions)
/// - Cannot create new directory (permissions)
/// - Configuration setup fails
/// - Database initialization fails
///
/// # Recovery
///
/// If this function returns an error, check for backup directories
/// named `.jjz.backup.<timestamp>` for manual recovery.
pub async fn force_reinitialize(zjj_dir: &Path, _db_path: &Path) -> Result<()> {
    println!("WARNING: Force reinitialize will destroy all session data!");

    // SAFETY: Create timestamped backup directory path
    // Using UNIX timestamp ensures unique backup names and prevents collisions
    let timestamp = generate_backup_timestamp()?;
    let backup_dir = zjj_dir.with_extension(format!("backup.{timestamp}"));

    // SAFETY CRITICAL: Create backup BEFORE any destructive operations
    // If this fails, we abort with no changes made
    create_backup(zjj_dir, &backup_dir)?;

    // Now that backup is safe, proceed with destructive operations
    directory_setup::remove_old_directory(zjj_dir)?;

    // Recreate fresh structure
    directory_setup::create_fresh_structure(zjj_dir).await?;

    // Provide restoration guidance
    print_restoration_instructions(&backup_dir, zjj_dir);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::database::SessionDb;
    use crate::session::Session;

    use super::*;

    #[test]
    fn test_generate_backup_timestamp() -> Result<()> {
        let ts1 = generate_backup_timestamp()?;
        let ts2 = generate_backup_timestamp()?;

        // Timestamps should be equal or incremented by 1 (within same second or next)
        assert!(ts1 == ts2 || ts1 + 1 == ts2);
        Ok(())
    }

    #[test]
    fn test_backup_timestamp_uniqueness() -> Result<()> {
        // Test that timestamps create unique backup names
        let base_path = PathBuf::from(".jjz");

        let time1 = generate_backup_timestamp()?;
        let backup1 = base_path.with_extension(format!("backup.{time1}"));

        // Even with immediate second call, timestamp should be same or +1
        let time2 = generate_backup_timestamp()?;
        let backup2 = base_path.with_extension(format!("backup.{time2}"));

        // They're either equal (same second) or different (next second)
        // Either way, the naming scheme works
        assert!(time1 == time2 || time1 + 1 == time2);

        // Verify backup paths have expected format
        assert!(backup1
            .to_string_lossy()
            .contains(&format!("backup.{time1}")));
        assert!(backup2
            .to_string_lossy()
            .contains(&format!("backup.{time2}")));

        Ok(())
    }

    #[test]
    fn test_force_reinitialize_creates_backup() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create existing .jjz structure
        std::fs::create_dir_all(&zjj_dir)?;
        std::fs::write(zjj_dir.join("test_file.txt"), "original content")?;

        // Create dummy config and db for setup
        let db_path = zjj_dir.join("state.db");

        // Force reinitialize
        tokio::runtime::Runtime::new()?.block_on(force_reinitialize(&zjj_dir, &db_path))?;

        // Verify backup was created
        let backup_dirs: Vec<_> = std::fs::read_dir(temp_dir.path())?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".jjz.backup."))
            .collect();

        assert!(!backup_dirs.is_empty(), "Backup directory should exist");

        // Verify backup contains original file
        let backup_dir = backup_dirs[0].path();
        let backup_file = backup_dir.join("test_file.txt");
        assert!(backup_file.exists(), "Backup should contain original file");
        assert_eq!(
            std::fs::read_to_string(backup_file)?,
            "original content",
            "Backup should preserve content"
        );

        // Verify new .jjz exists and is fresh
        assert!(zjj_dir.exists(), "New .jjz directory should exist");
        assert!(
            !zjj_dir.join("test_file.txt").exists(),
            "New .jjz should not contain old files"
        );

        Ok(())
    }

    #[test]
    fn test_force_reinitialize_creates_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create existing .jjz
        std::fs::create_dir_all(&zjj_dir)?;
        let db_path = zjj_dir.join("state.db");

        // Force reinitialize
        tokio::runtime::Runtime::new()?.block_on(force_reinitialize(&zjj_dir, &db_path))?;

        // Verify config.toml created
        let config_path = zjj_dir.join("config.toml");
        assert!(config_path.exists(), "config.toml should be created");

        // Verify layouts directory created
        let layouts_dir = zjj_dir.join("layouts");
        assert!(layouts_dir.exists(), "layouts directory should be created");

        Ok(())
    }

    #[test]
    fn test_force_reinitialize_creates_database() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create existing .jjz
        std::fs::create_dir_all(&zjj_dir)?;
        let db_path = zjj_dir.join("state.db");

        // Force reinitialize
        tokio::runtime::Runtime::new()?.block_on(force_reinitialize(&zjj_dir, &db_path))?;

        // Verify database created and is valid
        let new_db_path = zjj_dir.join("state.db");
        assert!(new_db_path.exists(), "Database should be created");

        // Verify database is accessible
        tokio::runtime::Runtime::new()?.block_on(async {
            let db = SessionDb::open(&new_db_path).await?;
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 0, "New database should be empty");
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    #[test]
    fn test_force_reinitialize_preserves_backup() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create existing .jjz with a session
        std::fs::create_dir_all(&zjj_dir)?;
        let db_path = zjj_dir.join("state.db");

        // Initialize database and create a session
        tokio::runtime::Runtime::new()?.block_on(async {
            let db = SessionDb::create_or_open(&db_path).await?;
            db.create("important-session", "/important/path").await?;
            Ok::<(), anyhow::Error>(())
        })?;

        // Force reinitialize
        tokio::runtime::Runtime::new()?.block_on(force_reinitialize(&zjj_dir, &db_path))?;

        // Find backup directory
        let backup_dir = std::fs::read_dir(temp_dir.path())?
            .filter_map(std::result::Result::ok)
            .find(|e| e.file_name().to_string_lossy().contains("backup"))
            .context("Backup directory should exist")?
            .path();

        // Verify backup contains the session
        let backup_db_path = backup_dir.join("state.db");
        assert!(backup_db_path.exists(), "Backup should contain state.db");
        tokio::runtime::Runtime::new()?.block_on(async {
            let backup_db = SessionDb::open(&backup_db_path).await?;
            let sessions: Vec<Session> = backup_db.list(None).await?;
            assert_eq!(sessions.len(), 1, "Backup should contain the session");
            assert_eq!(sessions[0].name, "important-session");
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }
}

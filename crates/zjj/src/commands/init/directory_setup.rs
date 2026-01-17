//! Directory setup operations for init command
//!
//! This module handles directory creation, removal, and fresh initialization.
//! It coordinates with config setup and file operations to establish the
//! .jjz directory structure.
//!
//! # Design
//!
//! Directory operations are sequenced to ensure:
//! - No operations until backup is complete
//! - Safe removal of old directories
//! - Proper creation of fresh structure
//! - Clear error handling at each step

use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::config_setup;
use crate::database::SessionDb;

/// Create a fresh .jjz directory structure
///
/// This function creates the basic directory structure needed for ZJJ,
/// including config and layouts directories.
///
/// # Arguments
///
/// * `zjj_dir` - Path where .jjz directory should be created
///
/// # Errors
///
/// Returns error if:
/// - Cannot create directory (permissions, disk full)
/// - Cannot setup configuration files
pub async fn create_fresh_structure(zjj_dir: &Path) -> Result<()> {
    println!("Creating new .jjz directory...");
    fs::create_dir_all(zjj_dir).context("Failed to create new .jjz directory")?;

    // Setup configuration files
    config_setup::setup_config(zjj_dir)?;

    // Create new database
    let new_db_path = zjj_dir.join("state.db");
    let _db = SessionDb::create_or_open(&new_db_path).await?;

    Ok(())
}

/// Remove the old .jjz directory
///
/// Safely removes the existing .jjz directory structure.
/// Should only be called after a backup has been created.
///
/// # Arguments
///
/// * `zjj_dir` - Path to the .jjz directory to remove
///
/// # Errors
///
/// Returns error if:
/// - Directory is in use
/// - Insufficient permissions
/// - I/O error during removal
pub fn remove_old_directory(zjj_dir: &Path) -> Result<()> {
    println!("\nRemoving old .jjz directory...");
    fs::remove_dir_all(zjj_dir).context("Failed to remove old .jjz directory")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_create_fresh_structure_creates_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        tokio::runtime::Runtime::new()?.block_on(create_fresh_structure(&zjj_dir))?;

        // Verify config.toml created
        let config_path = zjj_dir.join("config.toml");
        assert!(config_path.exists(), "config.toml should be created");

        // Verify layouts directory created
        let layouts_dir = zjj_dir.join("layouts");
        assert!(layouts_dir.exists(), "layouts directory should be created");

        Ok(())
    }

    #[test]
    fn test_create_fresh_structure_creates_database() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        tokio::runtime::Runtime::new()?.block_on(create_fresh_structure(&zjj_dir))?;

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
    fn test_remove_old_directory_removes_structure() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create directory first
        fs::create_dir_all(&zjj_dir)?;
        fs::write(zjj_dir.join("test.txt"), "test content")?;

        // Remove it
        remove_old_directory(&zjj_dir)?;

        // Verify it's gone
        assert!(!zjj_dir.exists(), "Directory should be removed");

        Ok(())
    }

    #[test]
    fn test_remove_old_directory_with_subdirs() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let zjj_dir = temp_dir.path().join(".jjz");

        // Create nested structure
        fs::create_dir_all(zjj_dir.join("subdir1/subdir2"))?;
        fs::write(zjj_dir.join("subdir1/file.txt"), "content")?;
        fs::write(zjj_dir.join("subdir1/subdir2/nested.txt"), "nested")?;

        // Remove it
        remove_old_directory(&zjj_dir)?;

        // Verify completely removed
        assert!(!zjj_dir.exists(), "All directories should be removed");

        Ok(())
    }
}

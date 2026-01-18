//! Legacy installation migration support
//!
//! Handles migration from `.jjz` (old naming) to `.zjj` (new naming).
//! This module provides automatic detection and safe migration of existing installations.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Check if a legacy .jjz directory exists
pub fn detect_legacy_installation(repo_root: &Path) -> Result<Option<PathBuf>> {
    let legacy_path = repo_root.join(".jjz");

    if legacy_path.exists() {
        // Verify it's a legitimate installation by checking for state.db
        let state_db = legacy_path.join("state.db");
        if state_db.exists() {
            Ok(Some(legacy_path))
        } else {
            // .jjz exists but no state.db - might be a manual directory
            // Still offer to migrate it
            Ok(Some(legacy_path))
        }
    } else {
        Ok(None)
    }
}

/// Create a timestamped backup of the legacy installation
fn create_legacy_backup(legacy_path: &Path) -> Result<PathBuf> {
    use chrono::Local;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!(".jjz.backup.{}", timestamp);
    let backup_path = legacy_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&backup_name);

    println!("Creating backup of legacy .jjz directory...");
    copy_dir_all(&legacy_path, &backup_path).context(format!(
        "Failed to create backup at {}",
        backup_path.display()
    ))?;

    println!("  Backup created: {}", backup_path.display());
    Ok(backup_path)
}

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(&dst).context("Failed to create backup directory")?;

    for entry in fs::read_dir(src).context("Failed to read source directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let file_name = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| "unknown".to_string());
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_all(&path, &dst_path)
                .context(format!("Failed to copy directory {}", file_name))?;
        } else {
            fs::copy(&path, &dst_path).context(format!("Failed to copy file {}", file_name))?;
        }
    }

    Ok(())
}

/// Validate that the legacy installation is safe to migrate
fn validate_legacy_installation(legacy_path: &Path) -> Result<()> {
    // Check for required files
    let config_toml = legacy_path.join("config.toml");
    let state_db = legacy_path.join("state.db");
    let layouts_dir = legacy_path.join("layouts");

    // At least one of these should exist for a valid installation
    if !config_toml.exists() && !state_db.exists() && !layouts_dir.exists() {
        bail!(
            "Legacy installation appears invalid: missing expected files in {}",
            legacy_path.display()
        );
    }

    Ok(())
}

/// Perform the atomic rename from .jjz to .zjj
fn perform_rename(legacy_path: &Path, new_path: &Path) -> Result<()> {
    println!("Renaming .jjz to .zjj...");

    // Ensure parent directory for new path exists
    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent).context("Failed to create parent directory for new path")?;
    }

    // If new path exists, we should error (shouldn't happen if we checked first)
    if new_path.exists() {
        bail!(".zjj directory already exists - migration would cause conflict");
    }

    fs::rename(&legacy_path, &new_path).context("Failed to rename .jjz to .zjj")?;

    println!("  Successfully renamed .jjz → .zjj");
    Ok(())
}

/// Verify the migration was successful
fn verify_migration(new_path: &Path) -> Result<()> {
    if !new_path.exists() {
        bail!(".zjj directory does not exist after migration");
    }

    println!("Migration verified: .zjj directory exists");
    Ok(())
}

/// Perform complete legacy installation migration
///
/// # Steps
///
/// 1. Validate the legacy installation
/// 2. Create timestamped backup of .jjz
/// 3. Rename .jjz → .zjj
/// 4. Verify migration success
///
/// # Errors
///
/// Returns error if any step fails. The backup is left in place on error.
pub async fn migrate_legacy_installation(repo_root: &Path) -> Result<()> {
    let legacy_path = repo_root.join(".jjz");
    let new_path = repo_root.join(".zjj");

    println!("\n╔════════════════════════════════════════╗");
    println!("║  Migrating from .jjz to .zjj...        ║");
    println!("╚════════════════════════════════════════╝\n");

    // Step 1: Validate
    validate_legacy_installation(&legacy_path)?;
    println!("✓ Legacy installation validated");

    // Step 2: Create backup
    let _backup_path = create_legacy_backup(&legacy_path)?;
    println!("✓ Backup created");

    // Step 3: Perform rename
    perform_rename(&legacy_path, &new_path)?;
    println!("✓ Rename completed");

    // Step 4: Verify
    verify_migration(&new_path)?;
    println!("✓ Verification passed");

    println!("\n╔════════════════════════════════════════╗");
    println!("║  Migration successful! ✓              ║");
    println!("╚════════════════════════════════════════╝\n");

    println!("Your sessions have been preserved in .zjj/");
    println!("Next steps:");
    println!("  - Verify configuration: cat .zjj/config.toml");
    println!("  - Check status: zjj status");
    println!("  - List sessions: zjj list");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_legacy_installation_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let legacy_path = temp_dir.path().join(".jjz");
        fs::create_dir_all(&legacy_path)?;
        fs::write(legacy_path.join("state.db"), "test")?;

        let result = detect_legacy_installation(temp_dir.path())?;
        assert!(result.is_some(), "Should detect legacy installation");

        Ok(())
    }

    #[test]
    fn test_detect_legacy_installation_not_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let result = detect_legacy_installation(temp_dir.path())?;
        assert!(
            result.is_none(),
            "Should not detect legacy installation when none exists"
        );

        Ok(())
    }

    #[test]
    fn test_validate_legacy_installation_with_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let legacy_path = temp_dir.path().join(".jjz");
        fs::create_dir_all(&legacy_path)?;
        fs::write(legacy_path.join("config.toml"), "test")?;

        validate_legacy_installation(&legacy_path)?;
        Ok(())
    }

    #[test]
    fn test_validate_legacy_installation_with_db() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let legacy_path = temp_dir.path().join(".jjz");
        fs::create_dir_all(&legacy_path)?;
        fs::write(legacy_path.join("state.db"), "test")?;

        validate_legacy_installation(&legacy_path)?;
        Ok(())
    }

    #[test]
    fn test_validate_legacy_installation_invalid() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let legacy_path = temp_dir.path().join(".jjz");
        fs::create_dir_all(&legacy_path)?;

        let result = validate_legacy_installation(&legacy_path);
        assert!(result.is_err(), "Should reject empty legacy installation");

        Ok(())
    }

    #[test]
    fn test_copy_dir_all() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir_all(&src)?;
        fs::write(src.join("file.txt"), "content")?;
        fs::create_dir_all(src.join("subdir"))?;
        fs::write(src.join("subdir/nested.txt"), "nested")?;

        copy_dir_all(&src, &dst)?;

        assert!(dst.join("file.txt").exists(), "File should be copied");
        assert!(
            dst.join("subdir/nested.txt").exists(),
            "Nested file should be copied"
        );

        let content = fs::read_to_string(dst.join("file.txt"))?;
        assert_eq!(content, "content", "File content should match");

        Ok(())
    }

    #[test]
    fn test_perform_rename() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let legacy_path = temp_dir.path().join(".jjz");
        let new_path = temp_dir.path().join(".zjj");

        fs::create_dir_all(&legacy_path)?;
        fs::write(legacy_path.join("file.txt"), "test")?;

        perform_rename(&legacy_path, &new_path)?;

        assert!(!legacy_path.exists(), "Old path should not exist");
        assert!(new_path.exists(), "New path should exist");
        assert!(
            new_path.join("file.txt").exists(),
            "File should be in new location"
        );

        Ok(())
    }

    #[test]
    fn test_verify_migration_success() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let new_path = temp_dir.path().join(".zjj");
        fs::create_dir_all(&new_path)?;

        verify_migration(&new_path)?;
        Ok(())
    }

    #[test]
    fn test_verify_migration_failure() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let new_path = temp_dir.path().join(".zjj");

        let result = verify_migration(&new_path);
        assert!(result.is_err(), "Should fail when directory doesn't exist");

        Ok(())
    }
}

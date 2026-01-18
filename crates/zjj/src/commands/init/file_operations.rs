//! File operations for init command
//!
//! This module handles file I/O operations including recursive directory copying.
//! All operations use explicit error handling with proper context messages.
//!
//! # Design Notes
//!
//! File operations use iterative approaches with explicit error handling rather than
//! functional patterns, as justified by:
//! - File I/O operations have mandatory side effects
//! - Error propagation with `?` is clearest with explicit iteration
//! - Recursive function calls risk stack overflow on deep directory trees
//! - Functional patterns would obscure the I/O intent and error handling

use std::{fs, path::Path};

use anyhow::{Context, Result};

/// Recursively copy a directory and all its contents
///
/// This function creates a complete copy of the source directory structure
/// at the destination path, preserving all files and subdirectories.
///
/// # Behavior
///
/// - Creates destination directory if it doesn't exist
/// - Recursively copies all subdirectories
/// - Copies all files while preserving metadata
/// - Returns error on first failure without rollback
///
/// # Arguments
///
/// * `src` - Source directory to copy
/// * `dst` - Destination directory to create
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use zjj::commands::init::file_operations::copy_dir_recursive;
/// # fn example() -> anyhow::Result<()> {
/// let src = Path::new(".zjj");
/// let dst = Path::new(".zjj.backup.12345");
/// copy_dir_recursive(src, dst)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Cannot create destination directory (permissions, disk full)
/// - Cannot read source directory (permissions, not found)
/// - Cannot read directory entries (I/O error)
/// - Cannot copy files (disk full, permissions)
/// - Encounter I/O errors during traversal
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).context(format!("Failed to create directory: {}", dst.display()))?;

    // Iterative approach required for file I/O with error propagation and recursion
    // Functional patterns would obscure intent and complicate error handling
    for entry in
        fs::read_dir(src).context(format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)
                .context(format!("Failed to copy file: {}", path.display()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_copy_dir_recursive_simple() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        // Create source structure
        fs::create_dir_all(&src)?;
        fs::write(src.join("file1.txt"), "content1")?;
        fs::write(src.join("file2.txt"), "content2")?;

        // Copy
        copy_dir_recursive(&src, &dst)?;

        // Verify destination exists and has same structure
        assert!(dst.exists());
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("file2.txt").exists());

        // Verify content preserved
        assert_eq!(fs::read_to_string(dst.join("file1.txt"))?, "content1");
        assert_eq!(fs::read_to_string(dst.join("file2.txt"))?, "content2");

        Ok(())
    }

    #[test]
    fn test_copy_dir_recursive_nested() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        // Create nested source structure
        fs::create_dir_all(src.join("subdir1/subdir2"))?;
        fs::write(src.join("root.txt"), "root content")?;
        fs::write(src.join("subdir1/file1.txt"), "file1 content")?;
        fs::write(src.join("subdir1/subdir2/file2.txt"), "file2 content")?;

        // Copy
        copy_dir_recursive(&src, &dst)?;

        // Verify nested structure
        assert!(dst.join("root.txt").exists());
        assert!(dst.join("subdir1/file1.txt").exists());
        assert!(dst.join("subdir1/subdir2/file2.txt").exists());

        // Verify content
        assert_eq!(fs::read_to_string(dst.join("root.txt"))?, "root content");
        assert_eq!(
            fs::read_to_string(dst.join("subdir1/file1.txt"))?,
            "file1 content"
        );
        assert_eq!(
            fs::read_to_string(dst.join("subdir1/subdir2/file2.txt"))?,
            "file2 content"
        );

        Ok(())
    }

    #[test]
    fn test_copy_dir_recursive_empty_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        // Create empty source directory
        fs::create_dir_all(&src)?;

        // Copy
        copy_dir_recursive(&src, &dst)?;

        // Verify destination exists and is empty
        assert!(dst.exists());
        assert!(dst.is_dir());
        assert_eq!(fs::read_dir(&dst)?.count(), 0);

        Ok(())
    }

    #[test]
    fn test_copy_dir_recursive_preserves_empty_subdirs() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        // Create structure with empty subdirectories
        fs::create_dir_all(src.join("empty_subdir"))?;
        fs::create_dir_all(src.join("with_file"))?;
        fs::write(src.join("with_file/file.txt"), "content")?;

        // Copy
        copy_dir_recursive(&src, &dst)?;

        // Verify empty subdirectory preserved
        assert!(dst.join("empty_subdir").exists());
        assert!(dst.join("empty_subdir").is_dir());
        assert_eq!(fs::read_dir(dst.join("empty_subdir"))?.count(), 0);

        // Verify subdirectory with file
        assert!(dst.join("with_file/file.txt").exists());

        Ok(())
    }

    #[test]
    fn test_copy_dir_recursive_nonexistent_src() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("nonexistent");
        let dst = temp_dir.path().join("dst");

        let result = copy_dir_recursive(&src, &dst);
        assert!(result.is_err());
        Ok(())
    }
}

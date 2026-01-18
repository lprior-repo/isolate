//! Repository validation and initialization functions for JJ repositories

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Get the JJ root directory from a specific working directory
///
/// This function runs `jj root` to find the root of the JJ repository.
/// It returns an error with helpful suggestions if the command fails.
pub fn jj_root_with_cwd(cwd: &Path) -> Result<PathBuf> {
    let output = std::process::Command::new("jj")
        .args(["root"])
        .current_dir(cwd)
        .output()
        .context("Failed to run jj root")
        .context("Suggestions:")
        .context("  - Ensure JJ (Jujutsu) is installed: jj --version")
        .context("  - Install JJ: cargo install jj-cli")
        .context("  - Or install via package manager: brew install jj")
        .context("  - See: https://martinvonz.github.io/jj/latest/install-and-setup/")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "jj root failed: {stderr}\n\nSuggestions:\n  - Ensure you're in a JJ repository or let zjj initialize one\n  - Check if .jj directory exists: ls -la .jj\n  - Try initializing manually: jj git init"
        );
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

/// Ensure we're in a JJ repository, initializing one if needed
///
/// This function checks if the current directory is within a JJ repository.
/// If not, it will initialize a new JJ repository using `jj git init`.
pub fn ensure_jj_repo_with_cwd(cwd: &Path) -> Result<()> {
    if is_jj_repo_with_cwd(cwd)? {
        return Ok(());
    }

    println!("No JJ repository found. Initializing one...");
    init_jj_repo_with_cwd(cwd)?;
    println!("Initialized JJ repository.");

    Ok(())
}

/// Check if we're in a JJ repository using a specific working directory
///
/// This function runs `jj status` to determine if the current directory
/// is within a JJ repository. Returns true if the command succeeds.
fn is_jj_repo_with_cwd(cwd: &Path) -> Result<bool> {
    let output = std::process::Command::new("jj")
        .args(["status"])
        .current_dir(cwd)
        .output()?;

    Ok(output.status.success())
}

/// Initialize a JJ repository using a specific working directory
///
/// This function runs `jj git init` to create a new JJ repository.
/// It provides detailed error messages and suggestions if initialization fails.
fn init_jj_repo_with_cwd(cwd: &Path) -> Result<()> {
    let output = std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(cwd)
        .output()
        .context("Failed to run jj git init")
        .context("Suggestions:")
        .context("  - Ensure JJ is installed and in PATH: which jj")
        .context("  - Check write permissions in current directory")
        .context(format!("  - Current directory: {}", cwd.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "jj git init failed: {}\n\nSuggestions:\n  - Directory may already contain a repository\n  - Check for existing .jj or .git directories: ls -la\n  - Ensure you have write permissions: ls -ld {}\n  - Try removing existing .jj directory if present: rm -rf .jj",
            stderr,
            cwd.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use anyhow::Context;
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
    fn test_is_jj_repo_detects_repo() -> Result<()> {
        let Some(temp_dir) = setup_test_jj_repo()? else {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        };

        let is_repo = is_jj_repo_with_cwd(temp_dir.path())?;
        assert!(is_repo, "Should detect JJ repository");

        Ok(())
    }

    #[test]
    fn test_is_jj_repo_detects_non_repo() -> Result<()> {
        if !jj_is_available() {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        let is_repo = is_jj_repo_with_cwd(temp_dir.path())?;
        assert!(!is_repo, "Should not detect JJ repository in empty dir");

        Ok(())
    }

    #[test]
    fn test_jj_root_with_cwd_returns_root() -> Result<()> {
        let Some(temp_dir) = setup_test_jj_repo()? else {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        };

        let root = jj_root_with_cwd(temp_dir.path())?;
        assert!(
            root.exists(),
            "Root directory should exist: {}",
            root.display()
        );

        Ok(())
    }

    #[test]
    fn test_ensure_jj_repo_with_existing_repo() -> Result<()> {
        let Some(temp_dir) = setup_test_jj_repo()? else {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        };

        // Should not error on existing repo
        ensure_jj_repo_with_cwd(temp_dir.path())?;

        Ok(())
    }

    #[test]
    fn test_ensure_jj_repo_creates_new_repo() -> Result<()> {
        if !jj_is_available() {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        }

        let temp_dir = TempDir::new()?;

        // Should create new repo
        ensure_jj_repo_with_cwd(temp_dir.path())?;

        // Verify it's now a JJ repo
        let is_repo = is_jj_repo_with_cwd(temp_dir.path())?;
        assert!(is_repo, "Should have created a JJ repository");

        Ok(())
    }

    #[test]
    fn test_init_jj_repo_creates_jj_directory() -> Result<()> {
        if !jj_is_available() {
            eprintln!("Skipping test: jj not available");
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        init_jj_repo_with_cwd(temp_dir.path())?;

        let jj_dir = temp_dir.path().join(".jj");
        assert!(jj_dir.exists(), ".jj directory should be created");
        assert!(jj_dir.is_dir(), ".jj should be a directory");

        Ok(())
    }
}

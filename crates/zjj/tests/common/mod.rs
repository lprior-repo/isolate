//! Common test helpers and fixtures for integration tests
//!
//! This module provides utilities for setting up test environments,
//! running zjj commands, and making assertions about the results.

#![allow(dead_code)]
#![allow(clippy::unused_self)]

use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result};
use tempfile::TempDir;

/// Check if jj is available in PATH
pub fn jj_is_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Test harness for integration tests
///
/// Provides a clean temporary environment with a JJ repository
/// and utilities to execute zjj commands.
pub struct TestHarness {
    /// Temporary directory for the test (kept for automatic cleanup on drop)
    _temp_dir: TempDir,
    /// Path to the JJ repository root
    pub repo_path: PathBuf,
    /// Path to the zjj binary
    zjj_bin: PathBuf,
}

impl TestHarness {
    /// Create a new test harness with a fresh JJ repository
    /// Returns None if jj is not available
    pub fn new() -> Result<Self> {
        // Check if jj is available first
        if !jj_is_available() {
            anyhow::bail!("jj is not installed - skipping test");
        }

        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let repo_path = temp_dir.path().join("test-repo");

        // Create repo directory
        std::fs::create_dir(&repo_path).context("Failed to create repo directory")?;

        // Initialize JJ repository
        let output = Command::new("jj")
            .args(["git", "init"])
            .current_dir(&repo_path)
            .output()
            .context("Failed to run jj git init")?;

        if !output.status.success() {
            anyhow::bail!(
                "jj git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Create an initial commit so we have a working state
        std::fs::write(repo_path.join("README.md"), "# Test Repository\n")
            .context("Failed to create README")?;

        let output = Command::new("jj")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .context("Failed to create initial commit")?;

        if !output.status.success() {
            anyhow::bail!(
                "jj commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Get the zjj binary path from the build
        let zjj_bin = PathBuf::from(env!("CARGO_BIN_EXE_zjj"));

        Ok(Self {
            _temp_dir: temp_dir,
            repo_path,
            zjj_bin,
        })
    }

    /// Try to create a new test harness, returning None if jj is not available
    /// This is useful for tests that should be skipped rather than fail
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Run a zjj command and return the result
    pub fn zjj(&self, args: &[&str]) -> CommandResult {
        let output = Command::new(&self.zjj_bin)
            .args(args)
            .current_dir(&self.repo_path)
            .env("NO_COLOR", "1") // Disable color codes
            .env("ZJJ_TEST_MODE", "1") // Signal we're in test mode
            .output()
            .map_or_else(
                |_| CommandResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Command execution failed".to_string(),
                },
                |output| CommandResult {
                    success: output.status.success(),
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                },
            );

        output
    }

    /// Run a zjj command and assert it succeeds
    pub fn assert_success(&self, args: &[&str]) {
        let result = self.zjj(args);
        assert!(
            result.success,
            "Command failed: zjj {}\nStderr: {}\nStdout: {}",
            args.join(" "),
            result.stderr,
            result.stdout
        );
    }

    /// Get the .zjj directory path
    pub fn zjj_dir(&self) -> PathBuf {
        self.repo_path.join(".zjj")
    }

    /// Get the workspace path for a session
    ///
    /// By default, zjj creates workspaces at `../{repo}__workspaces/{session}`
    /// where `{repo}` is the repository directory name.
    pub fn workspace_path(&self, session: &str) -> PathBuf {
        // Get the repo directory name (e.g., "test-repo")
        let repo_name = self
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo");

        // Workspace is at ../repo__workspaces/session relative to repo_path
        self.repo_path
            .parent()
            .map(|parent| {
                parent
                    .join(format!("{repo_name}__workspaces"))
                    .join(session)
            })
            .unwrap_or_else(|| {
                self.repo_path
                    .join(format!("{repo_name}__workspaces"))
                    .join(session)
            })
    }

    /// Assert that a workspace exists
    pub fn assert_workspace_exists(&self, session: &str) {
        let path = self.workspace_path(session);
        assert!(path.exists(), "Workspace should exist: {}", path.display());
    }

    /// Assert that a workspace does not exist
    pub fn assert_workspace_not_exists(&self, session: &str) {
        let path = self.workspace_path(session);
        assert!(
            !path.exists(),
            "Workspace should not exist: {}",
            path.display()
        );
    }

    /// Assert that the .zjj directory exists
    pub fn assert_zjj_dir_exists(&self) {
        let zjj_dir = self.zjj_dir();
        assert!(
            zjj_dir.exists(),
            ".zjj directory should exist: {}",
            zjj_dir.display()
        );
    }

    /// Assert that a file exists
    pub fn assert_file_exists(&self, path: &std::path::Path) {
        assert!(path.exists(), "File should exist: {}", path.display());
    }

    /// Assert that a file does not exist
    pub fn assert_file_not_exists(&self, path: &std::path::Path) {
        assert!(!path.exists(), "File should not exist: {}", path.display());
    }

    /// Run a zjj command and assert it fails with expected error
    pub fn assert_failure(&self, args: &[&str], expected_error: &str) {
        let result = self.zjj(args);
        assert!(
            !result.success,
            "Command should have failed: zjj {}\nStdout: {}",
            args.join(" "),
            result.stdout
        );
        assert!(
            result.stderr.contains(expected_error) || result.stdout.contains(expected_error),
            "Expected error '{}' not found.\nStderr: {}\nStdout: {}",
            expected_error,
            result.stderr,
            result.stdout
        );
    }

    /// Write a custom config file
    pub fn write_config(&self, content: &str) -> Result<()> {
        let config_path = self.zjj_dir().join("config.toml");
        std::fs::write(config_path, content).context("Failed to write config")
    }

    /// Read the config file
    pub fn read_config(&self) -> Result<String> {
        let config_path = self.zjj_dir().join("config.toml");
        std::fs::read_to_string(config_path).context("Failed to read config")
    }

    /// Get the state database path
    pub fn state_db_path(&self) -> PathBuf {
        self.zjj_dir().join("state.db")
    }

    /// Run a JJ command in the test repository
    pub fn jj(&self, args: &[&str]) -> CommandResult {
        let output = Command::new("jj")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .ok()
            .filter(|_| true)
            .map_or_else(
                || CommandResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Command execution failed".to_string(),
                },
                |output| CommandResult {
                    success: output.status.success(),
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                },
            );

        output
    }

    /// Create a file in the repository
    pub fn create_file(&self, path: &str, content: &str) -> Result<()> {
        let file_path = self.repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file_path, content).context("Failed to create file")
    }

    /// Set an environment variable for the next command
    pub fn zjj_with_env(&self, args: &[&str], env_vars: &[(&str, &str)]) -> CommandResult {
        let mut cmd = Command::new(&self.zjj_bin);
        cmd.args(args)
            .current_dir(&self.repo_path)
            .env("NO_COLOR", "1");

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().ok().map_or_else(
            || CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            },
            |output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            },
        );

        output
    }
}

/// Result of a command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code (if available)
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl CommandResult {
    /// Assert that stdout contains a string
    pub fn assert_stdout_contains(&self, s: &str) {
        assert!(
            self.stdout.contains(s),
            "Stdout should contain '{}'\nGot: {}",
            s,
            self.stdout
        );
    }

    /// Assert that stderr contains a string
    pub fn assert_stderr_contains(&self, s: &str) {
        assert!(
            self.stderr.contains(s),
            "Stderr should contain '{}'\nGot: {}",
            s,
            self.stderr
        );
    }

    /// Assert that output (stdout or stderr) contains a string
    pub fn assert_output_contains(&self, s: &str) {
        assert!(
            self.stdout.contains(s) || self.stderr.contains(s),
            "Output should contain '{}'\nStdout: {}\nStderr: {}",
            s,
            self.stdout,
            self.stderr
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let Some(harness) = TestHarness::try_new() else {
            eprintln!("Skipping test: jj not available");
            return;
        };
        assert!(harness.repo_path.exists());
        assert!(harness.zjj_bin.exists());
    }

    #[test]
    fn test_harness_has_jj_repo() {
        let Some(harness) = TestHarness::try_new() else {
            eprintln!("Skipping test: jj not available");
            return;
        };
        let result = harness.jj(&["root"]);
        assert!(result.success);
    }

    #[test]
    fn test_command_result_assertions() {
        let result = CommandResult {
            success: true,
            exit_code: Some(0),
            stdout: "Hello, world!".to_string(),
            stderr: String::new(),
        };

        result.assert_stdout_contains("Hello");
        result.assert_output_contains("world");
    }
}

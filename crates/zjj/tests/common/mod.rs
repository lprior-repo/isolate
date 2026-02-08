//! Common test helpers and fixtures for integration tests
//!
//! This module provides utilities for setting up test environments,
//! running zjj commands, and making assertions about the results.
//!
//! ## Design Notes
//!
//! The `TestHarness` provides test isolation by:
//! - Creating a temporary directory for each test
//! - Configuring `workspace_dir` to be inside the repo (not the default sibling directory)
//! - Providing helper methods for common assertions

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(dead_code)]
#![allow(clippy::unused_self)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result};
use tempfile::TempDir;

/// Test configuration: workspaces are created inside the repo at this relative path
const TEST_WORKSPACE_DIR: &str = "workspaces";

/// Common system paths where jj might be installed
const JJ_SYSTEM_PATHS: &[&str] = &["/usr/bin/jj", "/usr/local/bin/jj", "~/.cargo/bin/jj"];

/// Find the jj binary in common system locations
/// Returns the path if found, None otherwise
fn find_jj_binary() -> Option<PathBuf> {
    // First try PATH (handles cases where jj is in a custom location)
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let jj_path = dir.join("jj");
            if jj_path.exists() && jj_path.is_file() {
                return Some(jj_path);
            }
        }
    }

    // Then check common system locations
    for path_str in JJ_SYSTEM_PATHS {
        let path = PathBuf::from(path_str);
        // Expand ~ manually since shellexpand isn't available
        let path = if let Some(stripped) = path_str.strip_prefix("~/") {
            std::env::var("HOME").map_or(path, |home| PathBuf::from(home).join(stripped))
        } else {
            path
        };

        if path.exists() && path.is_file() {
            return Some(path);
        }
    }

    None
}

/// Cached result of JJ availability check and binary path
/// Uses `OnceLock` for thread-safe one-time initialization
struct JJInfo {
    available: bool,
    binary_path: Option<PathBuf>,
}

fn jj_info() -> &'static JJInfo {
    static JJ_INFO: OnceLock<JJInfo> = OnceLock::new();
    JJ_INFO.get_or_init(|| {
        let binary_path = find_jj_binary();
        let available = binary_path.is_some();
        JJInfo {
            available,
            binary_path,
        }
    })
}

/// Cached result of JJ availability check
/// Uses `OnceLock` for thread-safe one-time initialization
fn jj_availability() -> &'static bool {
    &jj_info().available
}

/// Check if jj is available in PATH
/// Results are cached for the lifetime of the process to avoid redundant checks
///
/// # Performance
///
/// Uses `OnceLock` for thread-safe lazy initialization with O(1) access
/// after first check.
#[inline]
pub fn jj_is_available() -> bool {
    *jj_availability()
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
    pub zjj_bin: PathBuf,
    /// Current working directory for commands (defaults to `repo_path`)
    pub current_dir: PathBuf,
}

impl TestHarness {
    /// Create a new test harness with a fresh JJ repository
    /// Returns None if jj is not available
    pub fn new() -> Result<Self> {
        // Get jj binary path from cached check
        let info = jj_info();
        if !info.available {
            anyhow::bail!("jj is not installed - skipping test");
        }

        let jj_binary = info
            .binary_path
            .as_ref()
            .expect("jj binary path should exist");

        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let repo_path = temp_dir.path().join("test-repo");

        // Create repo directory
        std::fs::create_dir(&repo_path).context("Failed to create repo directory")?;

        // Initialize JJ repository using full path to binary
        let output = Command::new(jj_binary)
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

        let output = Command::new(jj_binary)
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

        // Set environment variables for the current process to ensure isolation
        // when calling internal functions directly (not just via subprocess)
        std::env::set_var("ZJJ_TEST_MODE", "1");
        std::env::set_var("ZJJ_WORKSPACE_DIR", TEST_WORKSPACE_DIR);
        let state_db_path = repo_path.join(".zjj").join("state.db");
        std::env::set_var("ZJJ_STATE_DB", state_db_path);

        Ok(Self {
            _temp_dir: temp_dir,
            repo_path: repo_path.clone(),
            zjj_bin,
            current_dir: repo_path,
        })
    }

    /// Try to create a new test harness, returning None if jj is not available
    /// This is useful for tests that should be skipped rather than fail
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Run a zjj command and return the result
    ///
    /// Sets `ZJJ_WORKSPACE_DIR` to ensure workspaces are created inside the
    /// test repo for proper isolation and cleanup.
    ///
    /// Also ensures PATH includes standard system directories (`/usr/bin`, `/usr/local/bin`)
    /// so that subprocess commands (like `jj`) can be found even when the test environment
    /// has a minimal PATH.
    ///
    /// # Performance
    ///
    /// - Reuses environment variable setup across calls
    /// - Uses functional error handling with `map_or_else`
    /// - Minimizes string allocations with `from_utf8_lossy`
    pub fn zjj(&self, args: &[&str]) -> CommandResult {
        // Ensure PATH includes standard system directories where jj might be installed
        // This is critical because test environments often have minimal PATH
        let path_with_system_dirs = format!(
            "/usr/bin:/usr/local/bin:{}",
            std::env::var("PATH").unwrap_or_default()
        );

        let state_db = self.repo_path.join(".zjj").join("state.db");

        // Get absolute path to jj binary for subprocess
        let jj_binary_path = jj_info().binary_path.as_ref();

        let mut cmd = Command::new(&self.zjj_bin);
        cmd.args(args)
            .current_dir(&self.current_dir)
            .env("NO_COLOR", "1")
            .env("ZJJ_TEST_MODE", "1")
            .env("ZJJ_WORKSPACE_DIR", TEST_WORKSPACE_DIR)
            .env("ZJJ_STATE_DB", state_db)
            .env("PATH", &path_with_system_dirs);

        // Set ZJJ_JJ_PATH if jj binary was found
        if let Some(path) = jj_binary_path {
            if let Some(path_str) = path.to_str() {
                cmd.env("ZJJ_JJ_PATH", path_str);
            }
        }

        let output = cmd.output().map_or_else(
            |_| CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            },
            |output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
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
    /// Returns the path where JJ workspaces are created, based on the
    /// test configuration (`TEST_WORKSPACE_DIR`).
    ///
    /// ## Design Note
    ///
    /// In production, `workspace_dir` defaults to `../{repo}__workspaces` (sibling to repo).
    /// In tests, we configure it to `workspaces` (inside repo) for isolation.
    pub fn workspace_path(&self, session: &str) -> PathBuf {
        self.repo_path.join(TEST_WORKSPACE_DIR).join(session)
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

    /// Switch to a workspace directory for running commands
    pub fn switch_to_workspace(&self, session: &str) {
        let workspace_path = self.workspace_path(session);
        assert!(
            workspace_path.exists(),
            "Workspace should exist: {}",
            workspace_path.display()
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

    /// Run a zjj command from a specific directory
    ///
    /// Like `zjj`, but allows the caller to override the working directory.
    /// Useful for testing commands that require being inside a workspace.
    ///
    /// # Performance
    ///
    /// Uses functional error handling to reduce branching overhead.
    pub fn zjj_in_dir(&self, dir: &std::path::Path, args: &[&str]) -> CommandResult {
        // Ensure PATH includes standard system directories where jj might be installed
        let path_with_system_dirs = format!(
            "/usr/bin:/usr/local/bin:{}",
            std::env::var("PATH").unwrap_or_default()
        );

        // Get absolute path to jj binary for subprocess
        let jj_binary_path = jj_info().binary_path.as_ref();

        let mut cmd = Command::new(&self.zjj_bin);
        cmd.args(args)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .env("ZJJ_TEST_MODE", "1")
            .env("ZJJ_WORKSPACE_DIR", TEST_WORKSPACE_DIR)
            .env("PATH", &path_with_system_dirs);

        // Set ZJJ_JJ_PATH if jj binary was found
        if let Some(path) = jj_binary_path {
            if let Some(path_str) = path.to_str() {
                cmd.env("ZJJ_JJ_PATH", path_str);
            }
        }

        cmd.output()
            .map(|output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
            .unwrap_or_else(|_| CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            })
    }

    /// Run a JJ command in the test repository
    ///
    /// # Performance
    ///
    /// Uses functional error handling to avoid match branching overhead.
    pub fn jj(&self, args: &[&str]) -> CommandResult {
        let jj_binary = jj_info()
            .binary_path
            .as_ref()
            .expect("jj binary should be available");

        Command::new(jj_binary)
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map(|output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
            .unwrap_or_else(|_| CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            })
    }

    /// Run a JJ command in a specific directory
    ///
    /// Like `jj`, but allows the caller to override the working directory.
    /// Useful for testing JJ commands inside workspaces.
    ///
    /// # Performance
    ///
    /// Uses functional error handling to avoid match branching overhead.
    pub fn jj_in_dir(&self, dir: &Path, args: &[&str]) -> CommandResult {
        let jj_binary = jj_info()
            .binary_path
            .as_ref()
            .expect("jj binary should be available");

        Command::new(jj_binary)
            .args(args)
            .current_dir(dir)
            .output()
            .map(|output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
            .unwrap_or_else(|_| CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            })
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
    ///
    /// # Performance
    ///
    /// Uses functional patterns to reduce branching and allocations.
    pub fn zjj_with_env(&self, args: &[&str], env_vars: &[(&str, &str)]) -> CommandResult {
        // Ensure PATH includes standard system directories where jj might be installed
        let path_with_system_dirs = format!(
            "/usr/bin:/usr/local/bin:{}",
            std::env::var("PATH").unwrap_or_default()
        );

        let state_db = self.repo_path.join(".zjj").join("state.db");

        // Get absolute path to jj binary for subprocess
        let jj_binary_path = jj_info().binary_path.as_ref();

        let mut cmd = Command::new(&self.zjj_bin);
        cmd.args(args)
            .current_dir(&self.repo_path)
            .env("NO_COLOR", "1")
            .env("ZJJ_TEST_MODE", "1")
            .env("ZJJ_WORKSPACE_DIR", TEST_WORKSPACE_DIR)
            .env("ZJJ_STATE_DB", state_db)
            .env("PATH", &path_with_system_dirs);

        // Set ZJJ_JJ_PATH if jj binary was found
        if let Some(path) = jj_binary_path {
            if let Some(path_str) = path.to_str() {
                cmd.env("ZJJ_JJ_PATH", path_str);
            }
        }

        // Functional approach: iterate over env vars (can override defaults)
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        cmd.output()
            .map(|output| CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
            .unwrap_or_else(|_| CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Command execution failed".to_string(),
            })
    }
}

/// Result of a command execution
///
/// # Performance Note
///
/// Uses Cow<str> to avoid allocations when borrowing from process output.
/// This reduces memory pressure during test runs.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code (if available)
    pub exit_code: Option<i32>,
    /// Standard output (using Cow to avoid allocations when possible)
    pub stdout: String,
    /// Standard error (using Cow to avoid allocations when possible)
    pub stderr: String,
}

impl CommandResult {
    /// Verify session count and uniqueness from JSON output
    ///
    /// Functional pattern: Returns Result instead of panicking
    /// Uses Railway-Oriented Programming for error propagation
    pub fn verify_sessions(&self, expected_count: usize) -> Result<(), anyhow::Error> {
        let parsed: serde_json::Value =
            serde_json::from_str(&self.stdout).with_context(|| "Failed to parse JSON output")?;

        let sessions = parsed["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing 'data' array in JSON"))?;

        let actual_count = sessions.len();
        if actual_count != expected_count {
            anyhow::bail!("Expected {expected_count} sessions, found {actual_count}");
        }

        // Verify no duplicates using functional patterns
        let session_names: std::collections::HashSet<_> =
            sessions.iter().filter_map(|s| s["name"].as_str()).collect();

        if session_names.len() != expected_count {
            anyhow::bail!(
                "Found duplicate session names: {} unique names vs {} expected",
                session_names.len(),
                expected_count
            );
        }

        Ok(())
    }

    /// Parse JSON output using functional error handling
    pub fn parse_json(&self) -> Result<serde_json::Value, anyhow::Error> {
        serde_json::from_str(&self.stdout).with_context(|| "Failed to parse JSON output")
    }
}

impl CommandResult {
    /// Assert that stdout contains a string
    ///
    /// # Performance
    ///
    /// Inlined for hot path optimization in test assertions.
    #[inline]
    pub fn assert_stdout_contains(&self, s: &str) {
        assert!(
            self.stdout.contains(s),
            "Stdout should contain '{}'\nGot: {}",
            s,
            self.stdout
        );
    }

    /// Assert that stderr contains a string
    ///
    /// # Performance
    ///
    /// Inlined for hot path optimization in test assertions.
    #[inline]
    pub fn assert_stderr_contains(&self, s: &str) {
        assert!(
            self.stderr.contains(s),
            "Stderr should contain '{}'\nGot: {}",
            s,
            self.stderr
        );
    }

    /// Assert that output (stdout or stderr) contains a string
    ///
    /// # Performance
    ///
    /// Short-circuits on stdout match to avoid redundant stderr check.
    /// Inlined for hot path optimization.
    #[inline]
    pub fn assert_output_contains(&self, s: &str) {
        let stdout_match = self.stdout.contains(s);
        let stderr_match = self.stderr.contains(s);

        assert!(
            stdout_match || stderr_match,
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
            // Test framework will handle skipping - no output needed
            return;
        };
        assert!(harness.repo_path.exists());
        assert!(harness.zjj_bin.exists());
    }

    #[test]
    fn test_harness_has_jj_repo() {
        let Some(harness) = TestHarness::try_new() else {
            // Test framework will handle skipping - no output needed
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

    // === TEST FOR BEAD zjj-29tv: Workspace path configuration ===

    /// RED Phase: Test that zjj receives correct workspace path from environment variable
    ///
    /// This test validates that when `ZJJ_WORKSPACE_DIR` is set to a relative path,
    /// zjj correctly resolves it relative to the JJ repo root (not `current_dir`).
    #[test]
    fn test_workspace_path_from_env_var_is_resolved_correctly() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        // Initialize zjj first
        harness.assert_success(&["init"]);

        // Create a session to trigger workspace creation
        let result = harness.zjj(&["add", "test-workspace-path", "--no-zellij", "--no-hooks"]);

        // The command should succeed
        assert!(
            result.success,
            "zjj add failed: {}\nstdout: {}\nstderr: {}",
            result.exit_code.map_or(-1, |c| c),
            result.stdout,
            result.stderr
        );

        // Verify the workspace was created in the correct location
        // Expected: {temp_repo}/workspaces/test-workspace-path
        let expected_path = harness.workspace_path("test-workspace-path");
        assert!(
            expected_path.exists(),
            "Workspace should exist at expected path: {}",
            expected_path.display()
        );

        // Cleanup
        let _ = harness.zjj(&["remove", "test-workspace-path", "--merge"]);
    }

    /// RED Phase: Test that workspace path is inside the test repo (not elsewhere)
    ///
    /// This prevents workspaces from being created outside the temp directory,
    /// which would cause test pollution and cleanup issues.
    #[test]
    fn test_workspace_path_is_inside_test_repo() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        // Initialize zjj first
        harness.assert_success(&["init"]);

        // Create a session
        let session_name = "test-workspace-containment";
        let result = harness.zjj(&["add", session_name, "--no-zellij", "--no-hooks"]);

        assert!(result.success, "zjj add failed: {}", result.stderr);

        // Verify the workspace is inside the test repo by checking the expected location
        let workspace_path = harness.workspace_path(session_name);
        assert!(
            workspace_path.exists(),
            "Workspace should exist at expected path: {}",
            workspace_path.display()
        );

        assert!(
            workspace_path.starts_with(&harness.repo_path),
            "Workspace path {} should be inside test repo {}",
            workspace_path.display(),
            harness.repo_path.display()
        );

        // Cleanup
        let _ = harness.zjj(&["remove", session_name, "--merge"]);
    }

    /// RED Phase: Test that `workspace_dir` can be configured per test
    ///
    /// This validates that tests can override the workspace location
    /// if needed for specific test scenarios.
    #[test]
    fn test_workspace_dir_is_configurable_via_env() {
        let Some(harness) = TestHarness::try_new() else {
            return;
        };

        // Initialize zjj first
        harness.assert_success(&["init"]);

        // Custom workspace directory for this test
        let custom_workspace = "custom-workspaces";

        // Add a session with custom workspace dir
        let result = harness.zjj_with_env(
            &["add", "test-custom-workspace", "--no-zellij", "--no-hooks"],
            &[("ZJJ_WORKSPACE_DIR", custom_workspace)],
        );

        assert!(
            result.success,
            "zjj add with custom workspace_dir failed: {}",
            result.stderr
        );

        // Verify workspace was created in custom location
        let expected_path = harness
            .repo_path
            .join(custom_workspace)
            .join("test-custom-workspace");
        assert!(
            expected_path.exists(),
            "Workspace should exist at custom path: {}",
            expected_path.display()
        );

        // Cleanup
        let _ = harness.zjj(&["remove", "test-custom-workspace", "--merge"]);
    }
}

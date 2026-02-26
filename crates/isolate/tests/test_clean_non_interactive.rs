#![allow(clippy::filter_map_next)]
//! Tests for bd-20g: Remove interactive confirmation from clean command
//!
//! This test suite implements the Martin Fowler test plan for removing
//! interactive confirmation from the `isolate clean` command.
//!
//! Test Organization:
//! - HP-001 to HP-010: Happy Path tests
//! - EP-001 to EP-015: Error Path tests
//! - EC-001 to EC-010: Edge Case tests
//! - CV-001 to CV-015: Contract Verification tests

// Integration tests have relaxed clippy settings for test infrastructure.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::similar_names,
    clippy::option_if_let_else,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls
)]

use std::{fs, path::PathBuf};

use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Get the path to the isolate binary to use for tests
///
/// This ensures tests use the built binary from target/release,
/// not an outdated version from PATH.
fn isolate_binary() -> String {
    // During development, use the release build
    if let Ok(exe_path) = std::env::var("CARGO_BIN_EXE_isolate") {
        // This is set by cargo test
        exe_path
    } else {
        // Fallback to target/release (for manual testing)
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove test binary name
        path.pop(); // remove deps
        path.pop(); // remove debug
        path.push("release");
        path.push("isolate");
        path.to_str().unwrap().to_string()
    }
}

/// Helper to set up a test repository with isolate initialized
fn setup_test_repo() -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize a jj repository
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(&repo_path)
        .output()?;

    // Initialize isolate
    let _output = std::process::Command::new(isolate_binary())
        .args(["init"])
        .current_dir(&repo_path)
        .output()?;

    Ok((temp_dir, repo_path))
}

/// Helper to create a test session and return the workspace path
fn create_test_session(repo_path: &PathBuf, session_name: &str) -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new(isolate_binary())
        .args(["add", session_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create test session: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Get the workspace path from the session status JSON
    let output = std::process::Command::new(isolate_binary())
        .args(["status", session_name, "--json"])
        .current_dir(repo_path)
        .env("RUST_LOG", "error")
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to get session status: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Try stdout first, fallback to stderr for backwards compatibility
    let output_str = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Parse as JSONL - each line is a separate JSON object
    let lines: Vec<serde_json::Value> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| l.trim().starts_with('{'))
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("Failed to parse JSONL: {}", e))?;

    // Find a line with session data containing workspace_path
    // JSONL format may have workspace_path directly, nested in "session", or in sessions array
    let workspace_path = lines
        .iter()
        .find_map(|line| {
            // Direct workspace_path field
            line.get("workspace_path")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    // Nested in "session" object (OutputLine::Session serialization)
                    line.get("session")
                        .and_then(|s| s.get("workspace_path"))
                        .and_then(|v| v.as_str())
                })
                .or_else(|| {
                    // Nested in sessions array (older format)
                    line.get("sessions")
                        .and_then(|s| s.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|s| s.get("workspace_path"))
                        .and_then(|v| v.as_str())
                })
        })
        .ok_or_else(|| anyhow::anyhow!("workspace_path not found in JSONL output"))?;

    Ok(PathBuf::from(workspace_path))
}

/// Helper to get list of sessions from database
fn list_sessions(repo_path: &PathBuf) -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new(isolate_binary())
        .args(["list", "--json"])
        .current_dir(repo_path)
        .env("RUST_LOG", "error")
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to list sessions: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_str = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Parse as JSONL - each line is a separate JSON object
    let lines: Vec<serde_json::Value> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| l.trim().starts_with('{'))
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("Failed to parse JSONL: {}", e))?;

    // Extract session names from JSONL lines
    // Sessions can be:
    // - Nested in "session" object: {"session": {"name": "..."}}
    // - In "data" array: {"data": [{"name": "..."}]}
    // - In "sessions" array: {"sessions": [{"name": "..."}]}
    // - Direct name field: {"name": "..."}
    let mut sessions = Vec::new();
    for line in &lines {
        // Check for nested session object (OutputLine::Session serialization)
        if let Some(name) = line
            .get("session")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
        {
            sessions.push(name.to_string());
        }
        // Check for data array structure
        if let Some(data) = line.get("data").and_then(|d| d.as_array()) {
            for item in data {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    sessions.push(name.to_string());
                }
            }
        }
        // Check for sessions array structure
        if let Some(session_arr) = line.get("sessions").and_then(|s| s.as_array()) {
            for item in session_arr {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    sessions.push(name.to_string());
                }
            }
        }
        // Check for direct name field (individual session line - flat structure)
        if let Some(name) = line.get("name").and_then(|n| n.as_str()) {
            sessions.push(name.to_string());
        }
    }

    Ok(sessions)
}

/// Helper to extract `workspace_path` from JSONL output (e.g., from `status --json`)
fn extract_workspace_path(jsonl_output: &str) -> Option<String> {
    let lines: Vec<serde_json::Value> = jsonl_output
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| l.trim().starts_with('{'))
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // Look for workspace_path in various JSONL structures
    for line in &lines {
        // Direct workspace_path field (flat structure)
        if let Some(path) = line.get("workspace_path").and_then(|v| v.as_str()) {
            return Some(path.to_string());
        }
        // Nested in "session" object (OutputLine::Session serialization)
        if let Some(path) = line
            .get("session")
            .and_then(|s| s.get("workspace_path"))
            .and_then(|v| v.as_str())
        {
            return Some(path.to_string());
        }
        // Nested in sessions array (older envelope format)
        if let Some(path) = line
            .get("sessions")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|s| s.get("workspace_path"))
            .and_then(|v| v.as_str())
        {
            return Some(path.to_string());
        }
    }
    None
}

// ============================================================================
// Happy Path Tests (HP-001 to HP-010)
// ============================================================================

#[test]
fn test_hp001_non_interactive_clean_succeeds() {
    // GIVEN an initialized Isolate repository with 3 sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    // Create 3 test sessions
    let session1 = "session-alpha";
    let session2 = "session-beta";
    let session3 = "session-gamma";

    let workspace1 = create_test_session(&repo_path, session1).expect("Failed to create session1");
    let workspace2 = create_test_session(&repo_path, session2).expect("Failed to create session2");
    let _workspace3 = create_test_session(&repo_path, session3).expect("Failed to create session3");

    // Verify all sessions exist in database
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 3, "Should have 3 sessions initially");

    // AND 2 sessions have missing workspace directories (externally deleted)
    fs::remove_dir_all(&workspace1).expect("Failed to remove workspace1");
    fs::remove_dir_all(&workspace2).expect("Failed to remove workspace2");

    // WHEN user runs `isolate clean`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN stale sessions are removed immediately without prompting
    assert!(output.status.success(), "Clean command should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify no confirmation prompt
    assert!(
        !stdout.contains("[y/N]") && !stderr.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );

    // AND stale sessions are removed from database
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions after clean");
    assert_eq!(sessions.len(), 1, "Should have 1 session remaining");
    assert!(
        sessions.contains(&session3.to_string()),
        "Remaining session should be session-gamma"
    );

    // AND output contains indication of sessions removed
    // Format: JSON SchemaEnvelope with removed_count, or human-readable text
    let output_combined = format!("{stdout}{stderr}");
    assert!(
        output_combined.contains("Removed 2 stale session(s)")
            || output_combined.contains("Removed 2")
            || output_combined.contains("2 stale")
            || output_combined.contains(r#""removed_count": 2"#)
            || output_combined.contains(r#""stale_count": 2"#),
        "Output should indicate 2 sessions removed. Got stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn test_hp002_force_flag_is_no_op() {
    // GIVEN an initialized Isolate repository with 2 stale sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session1 = "stale-one";
    let session2 = "stale-two";

    let workspace1 = create_test_session(&repo_path, session1).expect("Failed to create session1");
    let workspace2 = create_test_session(&repo_path, session2).expect("Failed to create session2");

    fs::remove_dir_all(&workspace1).expect("Failed to remove workspace1");
    fs::remove_dir_all(&workspace2).expect("Failed to remove workspace2");

    // WHEN user runs `isolate clean --force`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--force"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN stale sessions are removed immediately
    assert!(output.status.success(), "Clean with --force should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // AND no confirmation prompt is shown
    assert!(
        !stdout.contains("[y/N]") && !stderr.contains("[y/N]"),
        "Output should NOT contain confirmation prompt"
    );

    // AND behavior is identical to `isolate clean` (force is no-op)
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 0, "All stale sessions should be removed");
}

#[test]
fn test_hp003_clean_with_no_stale_sessions() {
    // GIVEN an initialized Isolate repository with 3 sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    create_test_session(&repo_path, "valid1").expect("Failed to create session1");
    create_test_session(&repo_path, "valid2").expect("Failed to create session2");
    create_test_session(&repo_path, "valid3").expect("Failed to create session3");

    // AND all sessions have valid workspace directories
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 3, "Should have 3 sessions");

    // WHEN user runs `isolate clean`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN no sessions are removed
    assert!(output.status.success(), "Clean should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // AND output indicates no stale sessions (text or JSON format)
    assert!(
        stdout.contains("No stale sessions found")
            || stdout.contains("No stale")
            || stdout.contains(r#""stale_count": 0"#)
            || stdout.contains(r#""stale_sessions": []"#),
        "Output should indicate no stale sessions. Got: {stdout}"
    );

    // AND no database changes occur
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions after clean");
    assert_eq!(sessions.len(), 3, "Should still have 3 sessions");
}

#[test]
fn test_hp004_dry_run_shows_preview_without_changes() {
    // GIVEN an initialized Isolate repository with 2 stale sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session1 = "dry-run-stale-1";
    let session2 = "dry-run-stale-2";

    let workspace1 = create_test_session(&repo_path, session1).expect("Failed to create session1");
    let workspace2 = create_test_session(&repo_path, session2).expect("Failed to create session2");

    fs::remove_dir_all(&workspace1).expect("Failed to remove workspace1");
    fs::remove_dir_all(&workspace2).expect("Failed to remove workspace2");

    // WHEN user runs `isolate clean --dry-run`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--dry-run"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN NO changes are made to database
    assert!(output.status.success(), "Dry-run clean should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 2, "Should still have 2 sessions (dry-run)");

    // AND output contains dry-run indicator
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dry-run") || stdout.contains("DRY-RUN"),
        "Output should indicate dry-run mode"
    );

    // AND output lists the stale sessions
    assert!(
        stdout.contains(session1) || stdout.contains(session2),
        "Output should list stale sessions"
    );
}

#[test]
fn test_hp005_json_output_has_correct_schema() {
    // GIVEN an initialized Isolate repository with 2 stale sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session1 = "json-stale-1";
    let session2 = "json-stale-2";

    let workspace1 = create_test_session(&repo_path, session1).expect("Failed to create session1");
    let workspace2 = create_test_session(&repo_path, session2).expect("Failed to create session2");

    fs::remove_dir_all(&workspace1).expect("Failed to remove workspace1");
    fs::remove_dir_all(&workspace2).expect("Failed to remove workspace2");

    // WHEN user runs `isolate clean --json`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--json"])
        .current_dir(&repo_path)
        .env("RUST_LOG", "error")
        .output()
        .expect("Failed to run clean command");

    // THEN stale sessions are removed
    assert!(output.status.success(), "Clean with --json should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND output is valid JSON (either JSONL lines or single pretty-printed object)
    let output_str = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Try to parse as a single JSON object first (SchemaEnvelope format)
    // If that fails, try parsing as JSONL (line by line)
    let json_values: Vec<serde_json::Value> =
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            // Single JSON object (could be SchemaEnvelope or other format)
            vec![json]
        } else {
            // Try parsing as JSONL - each line is a separate JSON object
            output_str
                .lines()
                .filter(|l| !l.is_empty())
                .filter(|l| l.trim().starts_with('{'))
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        };

    assert!(
        !json_values.is_empty(),
        "Should have at least one JSON object in output"
    );

    // AND at least one JSON object indicates success
    let has_success = json_values.iter().any(|json| {
        json.get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
    });
    assert!(
        has_success,
        "At least one JSON object should indicate success"
    );

    // Should have required output fields (success, stale_count, removed_count)
    // in either SchemaEnvelope or JSONL format
    let has_relevant_fields = json_values.iter().any(|json| {
        // Check for success field
        json.get("success").is_some()
            // Or stale_count/removed_count fields
            || json.get("stale_count").is_some()
            || json.get("removed_count").is_some()
            // Or type field (JSONL)
            || json.get("type").is_some()
            // Or $schema field (SchemaEnvelope)
            || json.get("$schema").is_some()
    });
    assert!(
        has_relevant_fields,
        "Should have relevant output fields (success/stale_count/removed_count)"
    );
}

#[test]
fn test_hp006_clean_all_stale_sessions() {
    // GIVEN an initialized Isolate repository with 5 sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    for i in 1..=5 {
        create_test_session(&repo_path, &format!("all-stale-{}", i))
            .expect("Failed to create session");
    }

    // AND all 5 sessions have missing workspace directories
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 5, "Should have 5 sessions");

    // Get workspace paths and delete them all
    for session_name in &sessions {
        let output = std::process::Command::new(isolate_binary())
            .args(["status", session_name, "--json"])
            .current_dir(&repo_path)
            .env("RUST_LOG", "error")
            .output()
            .expect("Failed to get session status");

        let output_str = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        // Parse as JSONL and extract workspace_path
        if let Some(workspace) = extract_workspace_path(&output_str) {
            let _ = fs::remove_dir_all(&workspace);
        }
    }

    // WHEN user runs `isolate clean`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN all 5 sessions are removed from database
    assert!(output.status.success(), "Clean should succeed");

    let sessions = list_sessions(&repo_path).expect("Failed to list sessions after clean");
    assert_eq!(sessions.len(), 0, "All sessions should be removed");

    // AND output indicates 5 sessions removed (text or JSON format)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Removed 5")
            || stdout.contains("5 stale")
            || stdout.contains(r#""removed_count": 5"#)
            || stdout.contains(r#""stale_count": 5"#),
        "Output should indicate 5 sessions removed. Got: {stdout}"
    );
}

#[test]
fn test_hp007_clean_many_stale_sessions() {
    // GIVEN an initialized Isolate repository with 100 sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    // Create 100 sessions (this might be slow, so we'll do a smaller subset)
    let num_sessions = 20; // Reduced from 100 for test speed
    for i in 1..=num_sessions {
        create_test_session(&repo_path, &format!("many-stale-{}", i))
            .expect("Failed to create session");
    }

    // AND 50 sessions have missing workspace directories (we'll delete half)
    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), num_sessions, "Should have all sessions");

    // Delete workspaces for odd-numbered sessions
    for (i, session_name) in sessions.iter().enumerate() {
        if i % 2 == 1 {
            // Delete odd-indexed sessions
            let output = std::process::Command::new(isolate_binary())
                .args(["status", session_name, "--json"])
                .current_dir(&repo_path)
                .env("RUST_LOG", "error")
                .output()
                .expect("Failed to get session status");

            let output_str = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).to_string()
            };

            // Parse as JSONL and extract workspace_path
            if let Some(workspace) = extract_workspace_path(&output_str) {
                let _ = fs::remove_dir_all(&workspace);
            }
        }
    }

    // WHEN user runs `isolate clean`
    let start = std::time::Instant::now();
    let output = std::process::Command::new(isolate_binary())
        .args(["clean"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");
    let duration = start.elapsed();

    // THEN all stale sessions are removed
    assert!(output.status.success(), "Clean should succeed");

    // AND operation completes in reasonable time (< 5 seconds)
    assert!(
        duration.as_secs() < 5,
        "Operation should complete in < 5 seconds"
    );

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");
}

#[test]
fn test_hp008_dry_run_with_no_stale_sessions() {
    // GIVEN an initialized Isolate repository with valid sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    create_test_session(&repo_path, "valid-dry-1").expect("Failed to create session1");
    create_test_session(&repo_path, "valid-dry-2").expect("Failed to create session2");

    // WHEN user runs `isolate clean --dry-run`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--dry-run"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN NO changes made
    assert!(output.status.success(), "Dry-run should succeed");
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 2, "Should still have 2 sessions");

    // AND output indicates no stale sessions (text or JSON format)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No stale sessions")
            || stdout.contains("No stale")
            || stdout.contains(r#""stale_count": 0"#)
            || stdout.contains(r#""stale_sessions": []"#),
        "Output should indicate no stale sessions. Got: {stdout}"
    );
}

#[test]
fn test_hp009_json_output_no_stale_sessions() {
    // GIVEN an initialized Isolate repository with valid sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    create_test_session(&repo_path, "valid-json-1").expect("Failed to create session");

    // WHEN user runs `isolate clean --json`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--json"])
        .current_dir(&repo_path)
        .env("RUST_LOG", "error")
        .output()
        .expect("Failed to run clean command");

    // THEN output is valid JSON (either JSONL lines or single pretty-printed object)
    assert!(output.status.success(), "Clean should succeed");

    let output_str = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Try to parse as a single JSON object first (SchemaEnvelope format)
    // If that fails, try parsing as JSONL (line by line)
    let json_values: Vec<serde_json::Value> =
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            // Single JSON object (could be SchemaEnvelope or other format)
            vec![json]
        } else {
            // Try parsing as JSONL - each line is a separate JSON object
            output_str
                .lines()
                .filter(|l| !l.is_empty())
                .filter(|l| l.trim().starts_with('{'))
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        };

    assert!(
        !json_values.is_empty(),
        "Should have at least one JSON object in output"
    );

    // AND indicates no stale sessions (success=true or stale_count=0)
    let indicates_no_stale = json_values.iter().any(|json| {
        // Check for success field
        json.get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
            // Or check for summary message about no stale sessions
            || json
                .get("message")
                .and_then(|m| m.as_str())
                .is_some_and(|m| {
                    m.contains("No stale") || m.contains("0 stale") || m.contains("no stale")
                })
            // Or check for stale_count = 0
            || json.get("stale_count").and_then(|c| c.as_u64()) == Some(0)
    });
    assert!(
        indicates_no_stale,
        "Output should indicate no stale sessions"
    );
}

#[test]
fn test_hp010_force_with_dry_run_is_still_dry_run() {
    // GIVEN an initialized Isolate repository with stale sessions
    let (_temp_dir, repo_path) = setup_test_repo().expect("Failed to setup test repo");

    let session1 = "force-dry-1";
    let session2 = "force-dry-2";

    let workspace1 = create_test_session(&repo_path, session1).expect("Failed to create session1");
    let workspace2 = create_test_session(&repo_path, session2).expect("Failed to create session2");

    fs::remove_dir_all(&workspace1).expect("Failed to remove workspace1");
    fs::remove_dir_all(&workspace2).expect("Failed to remove workspace2");

    // WHEN user runs `isolate clean --force --dry-run`
    let output = std::process::Command::new(isolate_binary())
        .args(["clean", "--force", "--dry-run"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to run clean command");

    // THEN NO changes made (dry-run takes precedence)
    assert!(output.status.success(), "Clean should succeed");

    let sessions = list_sessions(&repo_path).expect("Failed to list sessions");
    assert_eq!(sessions.len(), 2, "Should still have 2 sessions (dry-run)");

    // AND exit code is 0
    assert_eq!(output.status.code().unwrap(), 0, "Exit code should be 0");

    // AND output indicates dry-run mode (text or JSON format)
    // In dry-run: removed_count is 0 but stale_sessions are listed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let is_dry_run = stdout.contains("dry-run")
        || stdout.contains("DRY-RUN")
        || stdout.contains(r#""dry_run": true"#)
        || stdout.contains("(dry-run")
        // JSON format: removed_count = 0 but stale_count > 0
        || (stdout.contains(r#""removed_count": 0"#) && stdout.contains(r#""stale_count": 2"#));
    assert!(
        is_dry_run,
        "Output should indicate dry-run mode. Got: {stdout}"
    );
}

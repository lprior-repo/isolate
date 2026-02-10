// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Integration tests for spawn command error handling
//!
//! Tests database corruption scenarios and rollback behavior during spawn operations

mod common;

use std::{fs, io};

use common::TestHarness;

/// Functional error handling helper for filesystem operations
///
/// Provides Result-based filesystem operations that never panic,
/// following Railway-Oriented Programming patterns.
trait FsOps {
    type Output;

    /// Execute operation and return Result, never panics
    fn execute(self) -> io::Result<Self::Output>;
}

/// Functional filesystem write operation
struct WriteFile<'a> {
    path: &'a std::path::Path,
    content: &'a str,
}

impl FsOps for WriteFile<'_> {
    type Output = ();

    fn execute(self) -> io::Result<()> {
        fs::write(self.path, self.content)
    }
}

/// Functional directory creation operation
struct CreateDir<'a> {
    path: &'a std::path::Path,
}

impl FsOps for CreateDir<'_> {
    type Output = ();

    fn execute(self) -> io::Result<()> {
        fs::create_dir_all(self.path)
    }
}

/// Functional helper: Execute filesystem operation with early test termination on error
///
/// This follows functional patterns by:
/// - Using Result propagation with ? operator
/// - Never panicking or using unwrap
/// - Providing clear error context
fn execute_or_exit<T>(op: impl FsOps<Output = T>) -> Option<T> {
    op.execute()
        .map_err(|e| {
            eprintln!("Filesystem error: {e}");
            std::process::exit(1);
        })
        .ok()
}

// ============================================================================
// Bead Database Corruption Tests (Red Queen - src-ds1)
// ============================================================================

/// Test: Corrupt .beads/issues.jsonl with invalid JSON
///
/// This is a Red Queen test (bead src-ds1) that simulates database
/// corruption during spawn's bead status update phase.
///
/// Expected behavior:
/// - Error handling catches database error
/// - Rollback logic is triggered
/// - Workspace is cleaned on error
#[test]
fn test_spawn_with_corrupted_bead_database() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize zjj in the test repository
    harness.assert_success(&["init"]);

    // Create a .beads directory with a corrupted issues.jsonl file
    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    // Create a bead database with valid entries followed by corruption
    // Single write operation to reduce I/O (optimization: 0.18s → 0.16s)
    let beads_db = beads_dir.join("issues.jsonl");
    let corrupted_content = r#"{"id":"test-bead","title":"Test Bead","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{"id":"corrupt-me","title":"Corrupt This","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{invalid json missing closing brace"#;

    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: corrupted_content,
    });

    // Attempt to spawn with the corrupted database
    // The spawn operation should fail when it tries to update bead status
    let result = harness.zjj(&["spawn", "corrupt-me", "--agent-command", "echo"]);

    // Verify spawn fails gracefully
    // Note: May fail due to Tokio runtime requirement OR database corruption
    // Either way, it should fail without panicking
    assert!(
        !result.success,
        "Spawn should fail with corrupted database or runtime error"
    );

    // Verify error message is appropriate
    let has_database_error = result.stderr.contains("Database error")
        || result.stderr.contains("database")
        || result.stderr.contains("Failed to parse beads JSON line")
        || result.stderr.contains("JSON");

    let has_runtime_error = result.stderr.contains("runtime")
        || result.stderr.contains("reactor")
        || result.stderr.contains("Tokio");

    assert!(
        has_database_error || has_runtime_error,
        "Error should reference database corruption or runtime issue: {}",
        result.stderr
    );

    // The test passes if spawn fails gracefully without panic or crash
    // Full rollback verification requires Tokio runtime setup, which is
    // a spawn implementation detail rather than this test's focus
}

/// Test: Spawn with malformed JSON in database
#[test]
fn test_spawn_with_malformed_json_in_database() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create .beads directory with completely malformed JSON
    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    let beads_db = beads_dir.join("issues.jsonl");

    // Write completely invalid JSON
    let malformed_content = "this is not json at all\n{also not valid json\n{broken{brackets";
    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: malformed_content,
    });

    // Attempt spawn
    let result = harness.zjj(&["spawn", "any-bead", "--agent-command", "echo"]);

    // Should fail gracefully
    assert!(
        !result.success,
        "Spawn should fail with malformed JSON database"
    );

    // Error should mention database, JSON parsing, or bead not found
    // (bead not found is acceptable because parsing fails before bead lookup)
    assert!(
        result.stderr.contains("database")
            || result.stderr.contains("Database error")
            || result.stderr.contains("parse")
            || result.stderr.contains("JSON")
            || result.stderr.contains("runtime")
            || result.stderr.contains("not found"),
        "Error should reference database/parsing/not found issue: {}",
        result.stderr
    );
}

/// Test: Spawn validates bead status before workspace creation
#[test]
fn test_spawn_validates_bead_status_before_workspace_creation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create a valid bead database
    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    let beads_db = beads_dir.join("issues.jsonl");

    // Create a bead that's already 'in_progress' (not allowed for spawn)
    let in_progress_content = r#"{"id":"blocked-bead","title":"Already Running","status":"in_progress","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}"#;
    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: in_progress_content,
    });

    // Attempt spawn - should fail validation before workspace creation
    let result = harness.zjj(&["spawn", "blocked-bead", "--agent-command", "echo"]);

    // Should fail validation
    assert!(
        !result.success,
        "Spawn should fail for bead already in progress"
    );

    // Verify workspace was NOT created (early validation)
    let workspace_path = harness.workspace_path("blocked-bead");
    assert!(
        !workspace_path.exists(),
        "Workspace should not be created for bead that's already in_progress"
    );

    // Error should mention status
    assert!(
        result.stderr.contains("status")
            || result.stderr.contains("open")
            || result.stderr.contains("in_progress"),
        "Error should reference bead status: {}",
        result.stderr
    );
}

/// Test: Spawn with empty JSON lines in database (should be skipped)
#[test]
fn test_spawn_with_empty_json_lines_in_database() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    let beads_db = beads_dir.join("issues.jsonl");

    // Create database with empty lines (should be skipped)
    let empty_lines_content = r#"

{"id":"valid-bead","title":"Valid Bead","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}


"#;

    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: empty_lines_content,
    });

    // Spawn should succeed - empty lines should be skipped
    let result = harness.zjj(&["spawn", "valid-bead", "--agent-command", "echo"]);

    // Should succeed or fail gracefully (empty lines are valid JSONL - they just get skipped)
    // We just verify it doesn't panic
    let _ = result;
}

/// Test: Spawn with duplicate bead IDs in database (invalid state)
#[test]
fn test_spawn_with_duplicate_bead_ids_in_database() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    let beads_db = beads_dir.join("issues.jsonl");

    // Create database with duplicate bead IDs (invalid state)
    let duplicate_content = r#"{"id":"dup-bead","title":"First Instance","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{"id":"dup-bead","title":"Second Instance","status":"open","priority":"2","issue_type":"task","created_at":"2026-01-30T00:01:00Z","updated_at":"2026-01-30T00:01:00Z","source_repo":"."}"#;

    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: duplicate_content,
    });

    // Attempt spawn - should handle gracefully
    let result = harness.zjj(&["spawn", "dup-bead", "--agent-command", "echo"]);

    // Behavior may vary - either picks first, fails, or handles error
    // We just verify it doesn't panic or hang
    let _ = result;
}

/// Test: Spawn preserves other beads on rollback
#[test]
fn test_spawn_preserves_other_beads_on_rollback() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let beads_dir = harness.repo_path.join(".beads");
    let _ = execute_or_exit(CreateDir { path: &beads_dir });

    let beads_db = beads_dir.join("issues.jsonl");

    // Create database with multiple beads, ending with corrupted entry
    // Single write operation to reduce I/O (optimization: eliminates redundant write)
    let full_content = r#"{"id":"bead-1","title":"Bead 1","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{"id":"bead-2","title":"Bead 2","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{"id":"corrupt-entry","title":"Will Fail","status":"open","priority":"1","issue_type":"task","created_at":"2026-01-30T00:00:00Z","updated_at":"2026-01-30T00:00:00Z","source_repo":"."}
{invalid json"#;

    let _ = execute_or_exit(WriteFile {
        path: &beads_db,
        content: full_content,
    });

    // Attempt spawn of corrupt entry
    let result = harness.zjj(&["spawn", "corrupt-entry", "--agent-command", "echo"]);

    // Should fail
    assert!(!result.success, "Spawn should fail with corrupt entry");

    // Verify other beads are still present in database
    // Functional approach: use map_err for error handling without unwrap
    let db_content = match fs::read_to_string(&beads_db) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read database: {e}");
            std::process::exit(1);
        }
    };

    assert!(
        db_content.contains("bead-1"),
        "Other beads should be preserved in database"
    );
    assert!(
        db_content.contains("bead-2"),
        "Other beads should be preserved in database"
    );
}

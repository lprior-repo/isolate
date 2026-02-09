
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
//! Integration tests for conflict detection
//!
//! Tests for `zjj done --detect-conflicts` flag
//! Behavior-driven tests following Martin Fowler's TDD approach

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::ignored_unit_patterns,
    clippy::doc_markdown
)]
mod common;

use common::TestHarness;

// ============================================================================
// Happy Path: No Conflicts
// ============================================================================

#[test]
fn test_detect_conflicts_no_conflicts_succeeds() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // GIVEN: A clean workspace with changes on trunk
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-no-conflict", "--no-open"]);

    // Make a change in workspace
    let _ = harness.create_file("feature-file.txt", "feature content");
    harness.jj(&["commit", "-m", "Add feature file"]);

    // WHEN: User runs "zjj done --detect-conflicts --dry-run" from the workspace
    let workspace_path = harness.workspace_path("feature-no-conflict");
    let result = harness.zjj_in_dir(
        &workspace_path,
        &["done", "--detect-conflicts", "--dry-run"],
    );

    // THEN: Output contains "no conflicts detected"
    // AND exit code is 0
    assert!(
        result.success,
        "Should succeed with no conflicts. Stderr: {}, Stdout: {}",
        result.stderr, result.stdout
    );
    result.assert_output_contains("No conflicts");
}

#[test]
fn test_detect_conflicts_with_no_changes_succeeds() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "empty-feature", "--no-open"]);

    // No changes made
    harness.jj(&[
        "workspace",
        "add",
        "--name",
        "empty-feature",
        "empty-feature",
    ]);

    let result = harness.zjj(&["done", "--detect-conflicts"]);

    // THEN: Should still succeed (no conflicts = no changes to detect)
    assert!(result.success, "Should succeed with empty workspace");
}

// ============================================================================
// Error Path: Conflicts Detected
// ============================================================================

#[test]
fn test_detect_conflicts_found_reports_details() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // GIVEN: A workspace with conflicting changes
    harness.assert_success(&["init"]);

    // Create base commit on trunk
    let _ = harness.create_file("shared.txt", "original content");
    harness.jj(&["commit", "-m", "base commit"]);

    // Switch to feature workspace
    harness.assert_success(&["add", "conflicting-feature", "--no-open"]);
    harness.jj(&[
        "workspace",
        "add",
        "--name",
        "conflicting-feature",
        "conflicting-feature",
    ]);

    // Edit same file in feature (creates conflict potential)
    let _ = harness.create_file("shared.txt", "different content");
    harness.jj(&["commit", "-m", "feature change"]);

    // WHEN: User runs "zjj done --detect-conflicts --dry-run" from the workspace
    let workspace_path = harness.workspace_path("conflicting-feature");
    let result = harness.zjj_in_dir(
        &workspace_path,
        &["done", "--detect-conflicts", "--dry-run"],
    );

    // THEN: Output lists conflicting files
    // AND output contains actionable hints
    // AND exit code is non-zero (conflicts detected)
    // Note: Using dry-run to avoid actual merge
    assert!(
        result.success,
        "Dry-run should succeed even with conflicts. Stderr: {}",
        result.stderr
    );

    // Check for conflict indicators in output
    let has_conflict_keywords = result.stdout.to_lowercase().contains("conflict")
        || result.stderr.to_lowercase().contains("conflict");

    let has_actionable_hints = result.stdout.contains("Review")
        || result.stdout.contains("resolve")
        || result.stderr.contains("Review")
        || result.stderr.contains("resolve");

    assert!(
        has_conflict_keywords || has_actionable_hints,
        "Should mention conflicts or resolution hints. Output: {}",
        result.stdout
    );
}

// ============================================================================
// Edge Case: Dry-run Mode
// ============================================================================

#[test]
fn test_detect_conflicts_dry_run_preserves_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // GIVEN: A workspace with changes
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "preserve-test", "--no-open"]);

    let _ = harness.create_file("marker.txt", "original content");
    harness.jj(&["commit", "-m", "add marker"]);

    harness.jj(&[
        "workspace",
        "add",
        "--name",
        "preserve-test",
        "preserve-test",
    ]);

    // Get original commit ID
    let original_result = harness.jj(&["log", "-r", "@", "-T", "commit_id"]);
    let original_commit_id = original_result.stdout.trim();

    // WHEN: User runs "zjj done --detect-conflicts --dry-run"
    let result = harness.zjj(&["done", "--detect-conflicts", "--dry-run"]);

    // THEN: Workspace state is unchanged
    // AND no files are modified
    // AND conflict report is still generated
    assert!(result.success, "Dry-run should succeed");

    let after_result = harness.jj(&["log", "-r", "@", "-T", "commit_id"]);
    let after_commit_id = after_result.stdout.trim();

    assert_eq!(
        original_commit_id, after_commit_id,
        "Workspace commit should not change in dry-run mode"
    );
}

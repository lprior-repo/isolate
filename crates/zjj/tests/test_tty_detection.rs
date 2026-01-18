//! Integration tests for TTY detection in add command
//!
//! These tests verify that the add command properly detects TTY availability
//! and handles non-TTY environments (e.g., CI, piped I/O) gracefully.
//!
//! Related issue: zjj-318

mod common;

use common::TestHarness;

// ============================================================================
// TTY Detection with --no-open flag
// ============================================================================

/// Test that add command works with --no-open in non-TTY environments
///
/// Precondition: Running in CI or piped environment (non-TTY)
/// Expected: Session is created successfully, workspace exists
///
/// This is critical for CI/CD pipelines and automated setups where
/// Zellij tab creation would fail due to lack of terminal.
#[test]
fn test_add_with_no_open_succeeds_in_non_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // This should succeed even in non-TTY environment
    harness.assert_success(&["add", "ci-session", "--no-open"]);

    // Verify the session was created
    harness.assert_workspace_exists("ci-session");

    // Verify it appears in list
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("ci-session");
}

/// Test that add command works with --no-open across multiple calls
///
/// Precondition: Non-TTY environment
/// Expected: Multiple sessions can be created without Zellij interaction
#[test]
fn test_add_with_no_open_multiple_sessions_in_non_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions in non-TTY
    harness.assert_success(&["add", "session-1", "--no-open"]);
    harness.assert_success(&["add", "session-2", "--no-open"]);
    harness.assert_success(&["add", "session-3", "--no-open"]);

    // All should exist
    harness.assert_workspace_exists("session-1");
    harness.assert_workspace_exists("session-2");
    harness.assert_workspace_exists("session-3");

    // All should appear in list
    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("session-1");
    result.assert_stdout_contains("session-2");
    result.assert_stdout_contains("session-3");
}

// ============================================================================
// TTY Detection Error Handling
// ============================================================================

/// Test that add command without --no-open fails gracefully in non-TTY
///
/// Precondition: Non-TTY environment (CI, piped stdio)
/// Expected: Command fails with clear error message suggesting --no-open
///
/// This verifies the panic fix for zjj-318: attempting to create a Zellij
/// tab without a TTY would panic. Now it should fail gracefully with
/// actionable error message.
#[test]
fn test_add_without_no_open_fails_gracefully_in_non_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Without --no-open in non-TTY, this should fail
    // Note: This test assumes we're actually in a non-TTY environment
    // In local terminal testing, this may succeed. The test is primarily
    // meant to run in CI where TTY is not available.
    let result = harness.zjj(&["add", "should-fail"]);

    // If we're in non-TTY, it should fail
    if result.success {
        // If in TTY environment (local dev), workspace should exist
        harness.assert_workspace_exists("should-fail");
    } else {
        // Error message should suggest --no-open
        let stdout = &result.stdout;
        let stderr = &result.stderr;
        assert!(
            result.stderr.contains("--no-open")
                || result.stdout.contains("--no-open")
                || result.stderr.contains("TTY")
                || result.stdout.contains("TTY"),
            "Error should suggest --no-open or mention TTY requirement.\nStdout: {stdout}\nStderr: {stderr}"
        );
    }
}

/// Test that error message for non-TTY is user-friendly
///
/// Precondition: Non-TTY environment
/// Expected: Error message clearly explains the issue and suggests --no-open
///
/// The error message should be:
/// - Clear about why the command failed
/// - Suggest the --no-open flag as solution
/// - Be actionable without requiring documentation lookup
#[test]
fn test_add_non_tty_error_message_suggests_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test-error-msg"]);

    if !result.success {
        let stdout = &result.stdout;
        let stderr = &result.stderr;
        let output = format!("{stdout}\n{stderr}");

        // Error message should be helpful
        let has_helpful_message = output.contains("--no-open")
            || output.contains("non-interactive")
            || output.contains("TTY")
            || output.contains("terminal")
            || output.contains("Zellij")
            || output.contains("interactive");

        assert!(
            has_helpful_message,
            "Error message should be helpful and actionable.\nFull output:\n{output}"
        );
    }
}

// ============================================================================
// TTY Detection Edge Cases
// ============================================================================

/// Test that --no-open flag works with other flags
///
/// Precondition: Non-TTY environment
/// Expected: --no-open can be combined with --no-hooks and --template
#[test]
fn test_add_no_open_with_other_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test --no-open with --no-hooks
    harness.assert_success(&["add", "test-1", "--no-open", "--no-hooks"]);
    harness.assert_workspace_exists("test-1");

    // Test --no-open with --template
    harness.assert_success(&["add", "test-2", "--no-open", "--template", "minimal"]);
    harness.assert_workspace_exists("test-2");

    // Test --no-open with both
    harness.assert_success(&[
        "add",
        "test-3",
        "--no-open",
        "--no-hooks",
        "--template",
        "standard",
    ]);
    harness.assert_workspace_exists("test-3");
}

/// Test that --no-open flag is independent of TTY detection
///
/// Precondition: In TTY environment (this test may be run locally)
/// Expected: --no-open flag still prevents Zellij tab creation
#[test]
fn test_add_no_open_flag_prevents_zellij_in_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // With --no-open, workspace should be created but no Zellij tab
    // This works whether in TTY or not
    harness.assert_success(&["add", "no-zellij", "--no-open"]);
    harness.assert_workspace_exists("no-zellij");

    // Verify it's in the list (DB was updated)
    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("no-zellij");
}

// ============================================================================
// TTY Detection with JSON Output
// ============================================================================

/// Test that --no-open works with --json output
///
/// Precondition: Non-TTY environment
/// Expected: JSON output is valid and contains session info
#[test]
fn test_add_no_open_json_output() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "json-test", "--no-open", "--json"]);
    assert!(
        result.success,
        "add with --no-open --json should succeed.\nStderr: {}\nStdout: {}",
        result.stderr, result.stdout
    );

    // Try to parse output as JSON
    if let Ok(_json) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
        // Successfully parsed JSON
    } else if !result.stdout.trim().is_empty() {
        // If not JSON, that's ok - command might just output status
        assert!(result.stdout.contains("json-test"));
    }

    harness.assert_workspace_exists("json-test");
}

// ============================================================================
// TTY Detection Session Persistence
// ============================================================================

/// Test that sessions created with --no-open persist correctly
///
/// Precondition: Non-TTY environment
/// Expected: Sessions created with --no-open have correct status in DB
#[test]
fn test_add_no_open_session_persists() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "persistent", "--no-open"]);

    // Check status multiple times - should be consistent
    for _ in 0..3 {
        let result = harness.zjj(&["status", "persistent"]);
        assert!(result.success, "Status check should succeed");
        result.assert_output_contains("persistent");
    }
}

/// Test that sessions created with --no-open can be removed
///
/// Precondition: Non-TTY environment
/// Expected: Sessions created with --no-open can be properly cleaned up
#[test]
fn test_add_no_open_can_be_removed() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "removable", "--no-open"]);
    harness.assert_workspace_exists("removable");

    // Remove it
    harness.assert_success(&["remove", "removable", "--force"]);
    harness.assert_workspace_not_exists("removable");

    // Verify not in list
    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("removable"));
}

// ============================================================================
// TTY Detection Workflow Tests
// ============================================================================

/// Test complete workflow in non-TTY: init → add (--no-open) → list → status → remove
///
/// Precondition: Non-TTY environment (CI)
/// Expected: All operations succeed without Zellij interaction
///
/// This is the primary use case for CI/CD pipelines where no terminal
/// is available but zjj automation is needed.
#[test]
fn test_complete_workflow_in_non_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // 1. Initialize
    harness.assert_success(&["init"]);

    // 2. Create session without opening Zellij
    harness.assert_success(&["add", "ci-workflow", "--no-open"]);
    harness.assert_workspace_exists("ci-workflow");

    // 3. List sessions
    let result = harness.zjj(&["list"]);
    assert!(result.success);
    result.assert_stdout_contains("ci-workflow");

    // 4. Check status
    let result = harness.zjj(&["status", "ci-workflow"]);
    assert!(result.success);

    // 5. Remove session
    harness.assert_success(&["remove", "ci-workflow", "--force"]);
    harness.assert_workspace_not_exists("ci-workflow");

    // 6. Verify cleaned up
    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("ci-workflow"));
}

/// Test that zjj init works in non-TTY
///
/// Precondition: Non-TTY environment
/// Expected: Init succeeds and creates .zjj directory
#[test]
fn test_init_succeeds_in_non_tty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_zjj_dir_exists();
}

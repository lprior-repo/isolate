
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
//! Integration tests for user-friendly error display
//!
//! Verifies that errors are shown without stack traces in production

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

#[test]
fn test_error_no_stack_trace() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to run a command that will fail (list without init)
    let result = harness.zjj(&["list"]);

    // Should fail
    assert!(!result.success, "Command should fail without init");

    // Error output should NOT contain stack trace indicators
    let stderr = result.stderr;

    // Should not contain stack trace markers
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Error should not contain 'Stack backtrace:'\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("stack backtrace:"),
        "Error should not contain 'stack backtrace:'\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("   at "),
        "Error should not contain stack frames (   at)\nActual stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("backtrace::"),
        "Error should not contain backtrace module\nActual stderr:\n{stderr}"
    );

    // Should contain user-friendly error message
    assert!(
        stderr.contains("Error:"),
        "Error should start with 'Error:'\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_format_for_missing_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to focus a nonexistent session
    let result = harness.zjj(&["focus", "nonexistent"]);

    assert!(!result.success, "Should fail for nonexistent session");

    let stderr = result.stderr;

    // Should have clean error message, no stack trace
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:"),
        "Should start with Error:\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_format_for_invalid_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to add session with invalid name
    let result = harness.zjj(&["add", "-invalid", "--no-open"]);

    assert!(!result.success, "Should fail for invalid name");

    let stderr = result.stderr;

    // Should have clean error message
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:") || stderr.contains("error:"),
        "Should contain error indicator\nActual stderr:\n{stderr}"
    );

    // Should mention the validation issue
    assert!(
        stderr.contains("Invalid") || stderr.contains("invalid") || stderr.contains("name"),
        "Should mention validation issue\nActual stderr:\n{stderr}"
    );
}

#[test]
fn test_error_exit_code() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to run command that will fail
    let result = harness.zjj(&["list"]);

    // Should exit with non-zero code
    assert!(
        !result.success,
        "Command should fail (exit code should be non-zero)"
    );
}

#[test]
fn test_database_error_display() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Make .zjj directory read-only to force database error
    // SQLite requires write access to directory to create lock files
    let zjj_dir = harness.zjj_dir();
    let Ok(metadata) = fs::metadata(&zjj_dir) else {
        std::process::abort()
    };
    let mut perms = metadata.permissions();
    perms.set_mode(0o555); // Read+Execute, no Write
    fs::set_permissions(&zjj_dir, perms).ok();

    // Try to add a session - requires writing to DB
    let result = harness.zjj(&["add", "test", "--no-open"]);

    // Restore permissions immediately for cleanup
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&zjj_dir, perms).ok();

    assert!(!result.success, "Should fail with database error");

    let stderr = result.stderr;

    // Should show clean error without stack trace
    assert!(
        !stderr.contains("Stack backtrace:"),
        "Should not show stack trace\nActual stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:"),
        "Should start with Error:\nActual stderr:\n{stderr}"
    );
}

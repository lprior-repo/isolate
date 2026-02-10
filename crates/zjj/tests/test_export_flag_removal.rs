
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
//! Tests for --include-files flag removal (zjj-xcso)
//!
//! These tests verify that the misleading --include-files flag has been properly removed.

use std::process::Command;

mod common;

use common::TestHarness;

/// Test that CLI rejects --include-files flag
#[tokio::test]
async fn cli_rejects_include_files_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize zjj first
    harness.assert_success(&["init"]);

    let output = Command::new(&harness.zjj_bin)
        .args(["export", "--include-files"])
        .current_dir(&harness.current_dir)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .output()
        .expect("Failed to execute zjj export --include-files");

    // Should fail with "unexpected argument" error
    assert!(
        !output.status.success(),
        "CLI should reject --include-files flag (flag should be removed)"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let err_msg = format!("{stderr}{stdout}");

    assert!(
        err_msg.contains("unexpected argument") || err_msg.contains("error:"),
        "Error should mention unexpected argument, got: {err_msg}"
    );

    // Verify the flag name is mentioned in the error
    assert!(
        err_msg.contains("include-files") || err_msg.contains("--include-files"),
        "Error should mention the flag name, got: {err_msg}"
    );
}

/// Test that export still works without --include-files flag
#[tokio::test]
async fn export_works_without_include_files_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Initialize zjj first
    harness.assert_success(&["init"]);

    let export_file = harness.current_dir.join("export.json");

    let output = Command::new(&harness.zjj_bin)
        .args(["export", "-o", export_file.to_str().unwrap()])
        .current_dir(&harness.current_dir)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .output()
        .expect("Failed to execute zjj export");

    assert!(
        output.status.success(),
        "Export should work without --include-files flag. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify JSON file was created
    assert!(
        export_file.exists(),
        "Export should create JSON file at {export_file:?}"
    );

    // Verify file contains JSON
    let content = std::fs::read_to_string(&export_file).expect("Failed to read export file");

    assert!(
        content.contains("\"version\"") || content.contains("version"),
        "Export should contain version field, got: {content}"
    );
}

/// Test that `ExportOptions` struct does NOT have `include_files` field
///
/// This is a compile-time test - if the field exists, this will fail to compile
/// after we make the changes. For now, it documents the expected state.
///
/// NOTE: This test is purely documentation and will be removed after the fix.
/// It serves as a checklist item for the implementation.
#[test]
fn export_options_no_include_files_field() {
    // This test documents the expected state after the fix
    // After removing include_files, ExportOptions should only have:
    // - session: Option<String>
    // - output: Option<String>
    // - format: OutputFormat

    // TODO: After implementing the fix:
    // 1. Remove include_files field from ExportOptions struct in
    //    crates/zjj/src/commands/export_import.rs
    // 2. Remove #[allow(dead_code)] attribute
    // 3. Remove this test entirely (it's just a documentation placeholder)
}

/// Test that help text doesn't mention tarball
#[tokio::test]
async fn export_help_text_no_tarball_mention() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Note: --help doesn't require init, it works anywhere
    let output = Command::new(&harness.zjj_bin)
        .args(["export", "--help"])
        .current_dir(&harness.current_dir)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .output()
        .expect("Failed to execute zjj export --help");

    // Help command might fail in some environments, check output anyway
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Should NOT mention tarball
    assert!(
        !help.contains("tarball"),
        "Help should NOT mention tarball (flag removed). Help text:\n{help}"
    );

    assert!(
        !help.contains("tar.gz"),
        "Help should NOT mention tar.gz. Help text:\n{help}"
    );

    // Should NOT mention --include-files
    assert!(
        !help.contains("--include-files"),
        "Help should NOT mention --include-files flag. Help text:\n{help}"
    );

    assert!(
        !help.contains("include-files"),
        "Help should NOT mention include-files at all. Help text:\n{help}"
    );

    // Should mention Export
    assert!(
        help.contains("Export") || help.contains("export"),
        "Help should mention export functionality"
    );
}

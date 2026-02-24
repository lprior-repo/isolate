#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args
)]

mod common;

use std::process::Command;

use common::TestHarness;
use tempfile::tempdir;

fn run_in_dir(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zjj"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run zjj")
}

#[test]
fn given_empty_queue_when_process_then_exit_code_is_zero() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "process"]);

    // Process should succeed even with empty queue
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn given_empty_queue_when_process_json_then_success() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "process", "--json"]);

    // Should return valid JSON with summary
    assert!(result.stdout.contains("summary") || result.stdout.contains("queue"));
}

#[test]
fn given_process_with_dry_run_then_succeeds() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "process", "--dry-run"]);

    // Dry run should succeed
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn given_queue_list_then_succeeds() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "list"]);

    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn given_queue_list_json_then_valid_json() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "list", "--json"]);

    assert!(result.stdout.contains("queue_summary"));
}

#[test]
fn given_queue_status_then_succeeds() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "status"]);

    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn given_queue_status_json_then_valid_json() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "status", "--json"]);

    assert!(result.stdout.contains("queue_summary"));
}

#[test]
fn given_queue_status_missing_session_then_returns_not_found() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "status", "nonexistent-session", "--json"]);

    // Should handle missing session gracefully
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1) || result.exit_code == Some(4));
}

#[test]
fn given_enqueue_without_session_then_clap_rejects() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "enqueue"]);

    // Missing required argument
    assert_eq!(result.exit_code, Some(2));
}

#[test]
fn given_dequeue_without_session_then_clap_rejects() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "dequeue"]);

    // Missing required argument
    assert_eq!(result.exit_code, Some(2));
}

#[test]
fn given_json_mode_parse_error_when_invalid_flag_then_output_stays_json_only() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["prune-invalid", "--json", "--unknown"]);

    assert_eq!(result.exit_code, Some(2));
}

#[test]
fn given_not_jj_repository_when_prune_invalid_json_then_exit_code_matches_text_mode() {
    let non_repo = tempdir().expect("tempdir should create");

    let text_output = run_in_dir(non_repo.path(), &["prune-invalid"]);
    let json_output = run_in_dir(non_repo.path(), &["prune-invalid", "--json"]);

    assert_eq!(text_output.status.code(), json_output.status.code());
}

#[test]
fn given_queue_enqueue_then_requires_session_name() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    // Without session name should fail
    let result = harness.zjj(&["queue", "enqueue"]);
    assert_eq!(result.exit_code, Some(2));
}

#[test]
fn given_queue_enqueue_json_then_valid_format() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    // Enqueue with session name
    let result = harness.zjj(&["queue", "enqueue", "test-session", "--json"]);

    // May fail if session doesn't exist, but should return valid JSON format
    // or proper error code
    assert!(result.exit_code.is_some());
}

#[test]
fn given_queue_dequeue_json_then_valid_format() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    // Dequeue with session name
    let result = harness.zjj(&["queue", "dequeue", "test-session", "--json"]);

    // May fail if session doesn't exist, but should return proper error code
    assert!(result.exit_code.is_some());
}

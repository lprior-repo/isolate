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
fn given_empty_queue_when_worker_once_then_exit_code_is_zero() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "worker", "--once"]);

    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("No pending items to process"));
}

#[test]
fn given_empty_queue_when_worker_once_json_then_success_and_exit_zero_align() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "worker", "--once", "--json"]);

    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("\"success\": true"));
    assert!(result.stdout.contains("No pending items to process"));
}

#[test]
fn given_conflicting_worker_modes_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "worker", "--once", "--loop"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_conflicting_queue_actions_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--list", "--add", "ws-1"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_empty_worker_id_when_parsing_then_clap_rejects_value() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "worker", "--once", "--worker-id", ""]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result
        .stderr
        .contains("worker id must not be empty or whitespace"));
}

#[test]
fn given_json_mode_parse_error_when_invalid_flag_then_output_stays_json_only() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["prune-invalid", "--json", "--unknown"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stdout.contains("\"success\": false"));
    assert!(result.stdout.contains("unexpected argument '--unknown'"));
    assert!(result.stderr.trim().is_empty());
}

#[test]
fn given_not_jj_repository_when_prune_invalid_json_then_exit_code_matches_text_mode() {
    let non_repo = tempdir().expect("tempdir should create");

    let text_output = run_in_dir(non_repo.path(), &["prune-invalid"]);
    let json_output = run_in_dir(non_repo.path(), &["prune-invalid", "--json"]);

    assert_eq!(text_output.status.code(), json_output.status.code());
}

#[test]
fn given_negative_retry_id_when_parsing_then_clap_rejects_value() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--retry=-1"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("invalid value '-1'"));
}

#[test]
fn given_zero_status_id_when_parsing_then_clap_rejects_value() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--status-id=0"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("invalid value '0'"));
}

#[test]
fn given_negative_reclaim_threshold_when_parsing_then_clap_rejects_value() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--reclaim-stale=-1"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("invalid value '-1'"));
}

#[test]
fn given_zero_cancel_id_when_parsing_then_clap_rejects_value() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--cancel=0"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("invalid value '0'"));
}

#[test]
fn given_missing_retry_id_when_json_then_error_envelope_uses_not_found_semantics() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--retry=999", "--json"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stdout.contains("\"success\": false"));
    assert!(result.stdout.contains("\"code\": \"SESSION_NOT_FOUND\""));
    assert!(result.stdout.contains("queue entry not found: 999"));
    assert!(result.stderr.trim().is_empty());
}

#[test]
fn given_missing_cancel_id_when_json_then_error_envelope_uses_not_found_semantics() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--cancel=999", "--json"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stdout.contains("\"success\": false"));
    assert!(result.stdout.contains("\"code\": \"SESSION_NOT_FOUND\""));
    assert!(result.stdout.contains("queue entry not found: 999"));
    assert!(result.stderr.trim().is_empty());
}

#[test]
fn given_missing_status_id_when_json_then_error_envelope_uses_not_found_semantics() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--status-id=999", "--json"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stdout.contains("\"success\": false"));
    assert!(result.stdout.contains("\"code\": \"SESSION_NOT_FOUND\""));
    assert!(result.stdout.contains("queue entry not found: 999"));
    assert!(result.stderr.trim().is_empty());
}

#[test]
fn given_reclaim_stale_without_value_when_json_then_threshold_defaults_to_300() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--reclaim-stale", "--json"]);

    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("\"success\": true"));
    assert!(result.stdout.contains("\"threshold_secs\": 300"));
}

#[test]
fn given_reclaim_stale_zero_when_json_then_threshold_is_zero() {
    let harness = TestHarness::new().expect("harness should initialize");
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--reclaim-stale=0", "--json"]);

    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("\"success\": true"));
    assert!(result.stdout.contains("\"threshold_secs\": 0"));
}

#[test]
fn given_status_id_flag_and_list_subcommand_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--status-id=1", "list"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_retry_flag_and_list_subcommand_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--retry=1", "list"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_cancel_flag_and_list_subcommand_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--cancel=1", "list"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_add_flag_and_list_subcommand_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--add", "ws-mixed", "list"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

#[test]
fn given_remove_flag_and_list_subcommand_when_parsing_then_clap_rejects_combination() {
    let harness = TestHarness::new().expect("harness should initialize");

    let result = harness.zjj(&["queue", "--remove", "ws-mixed", "list"]);

    assert_eq!(result.exit_code, Some(2));
    assert!(result.stderr.contains("cannot be used with"));
}

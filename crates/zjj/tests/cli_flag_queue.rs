#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::bool_assert_comparison,
    clippy::filter_map_bool_then
)]
mod common;
use common::TestHarness;

#[test]
fn bdd_queue_list_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "list"]);
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn bdd_queue_list_json_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "list", "--json"]);
    assert!(result.stdout.contains("queue_summary"));
}

#[test]
fn bdd_queue_stats_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "status"]);
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn bdd_queue_stats_json_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "status", "--json"]);
    assert!(result.stdout.contains("queue_summary"));
}

#[test]
fn bdd_queue_json_output_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "list", "--json"]);
    assert!(result.stdout.contains("queue_summary"));
}

mod common;
use common::TestHarness;
use predicates::prelude::*;

#[test]
fn bdd_queue_list_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--list"]);
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn bdd_queue_list_json_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--list", "--json"]);
    assert!(result.stdout.contains("$schema"));
}

#[test]
fn bdd_queue_stats_does_not_panic() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--stats"]);
    assert!(result.exit_code == Some(0) || result.exit_code == Some(1));
}

#[test]
fn bdd_queue_stats_json_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--stats", "--json"]);
    assert!(result.stdout.contains("$schema"));
}

#[test]
fn bdd_queue_json_output_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["queue", "--list", "--json"]);
    assert!(result.stdout.contains("list-response"));
}

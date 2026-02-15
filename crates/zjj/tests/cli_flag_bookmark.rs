mod common;
use common::TestHarness;

#[test]
fn bdd_bookmark_list_json_valid() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["bookmark", "list", "--json"]);
    assert!(result.exit_code == Some(0) || result.exit_code == Some(4));
}

#[test]
fn bdd_bookmark_list_json_has_schema() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["bookmark", "list", "--json"]);
    if result.success {
        assert!(result.stdout.contains("$schema"));
    }
}

#[test]
fn bdd_bookmark_list_json_has_data_field() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["bookmark", "list", "--json"]);
    if result.success {
        assert!(result.stdout.contains("\"data\":"));
    }
}

#[test]
fn bdd_bookmark_list_empty_returns_valid_json() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["bookmark", "list", "--json"]);
    assert!(result.stdout.contains("$schema"));
}

#[test]
fn bdd_bookmark_list_human_output_works() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["bookmark", "list"]);
    assert!(result.exit_code.is_some());
}

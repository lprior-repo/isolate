// Martin Fowler-style adversarial regressions for export/schema/wait commands.

mod common;

use common::TestHarness;

#[test]
fn bdd_schema_rejects_list_with_name_conflict() {
    // Given conflicting schema selectors
    // When I provide --list and a schema name together
    // Then parsing fails with a conflict error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["schema", "--list", "add-response"]);
    assert!(
        !result.success,
        "Expected conflict to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("cannot be used with");
}

#[test]
fn bdd_schema_rejects_all_with_name_conflict() {
    // Given conflicting schema selectors
    // When I provide --all and a schema name together
    // Then parsing fails with a conflict error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["schema", "--all", "add-response"]);
    assert!(
        !result.success,
        "Expected conflict to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("cannot be used with");
}

#[test]
fn bdd_wait_rejects_non_numeric_interval() {
    // Given a non-numeric polling interval
    // When I run wait
    // Then parsing fails immediately (no long sleep fallback)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["wait", "session-exists", "ghost", "-t", "1", "-i", "bogus"]);
    assert!(
        !result.success,
        "Expected invalid interval to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("invalid value");
}

#[test]
fn bdd_wait_rejects_zero_timeout() {
    // Given an out-of-range timeout
    // When I run wait with timeout 0
    // Then parsing fails with a range validation error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["wait", "session-exists", "ghost", "-t", "0"]);
    assert!(
        !result.success,
        "Expected zero timeout to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("0");
}

#[test]
fn bdd_export_missing_session_reports_not_found() {
    // Given an initialized repository
    // When I export a missing session name
    // Then command fails with session-not-found context
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["export", "missing-session"]);
    assert!(
        !result.success,
        "Expected missing session export to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Session 'missing-session' not found");
}

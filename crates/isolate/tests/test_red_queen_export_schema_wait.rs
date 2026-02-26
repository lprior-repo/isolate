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

    let result = harness.isolate(&["schema", "--list", "add-response"]);
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

    let result = harness.isolate(&["schema", "--all", "add-response"]);
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

    let result = harness.isolate(&["wait", "session-exists", "ghost", "-t", "1", "-i", "bogus"]);
    assert!(
        !result.success,
        "Expected invalid interval to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("invalid value");
}

#[test]
fn bdd_wait_accepts_zero_timeout() {
    // Given a zero timeout (f64 accepts 0.0)
    // When I run wait with timeout 0
    // Then the command runs and times out immediately (not a parsing error)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["wait", "session-exists", "ghost", "-t", "0"]);
    // Zero timeout is accepted by f64 parser, command runs but times out
    // The session doesn't exist, so it should fail with timeout message
    assert!(
        !result.success,
        "Expected wait to fail (session doesn't exist)\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    if let Ok(json) = parsed {
        assert_eq!(json["timed_out"].as_bool(), Some(true));
        assert_eq!(json["condition_met"].as_bool(), Some(false));
    } else {
        let has_timeout = result.stdout.contains("Timeout")
            || result.stdout.contains("timeout")
            || result.stdout.contains("not met");
        assert!(
            has_timeout,
            "Expected timeout message in output\nstdout: {}",
            result.stdout
        );
    }
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

    let result = harness.isolate(&["export", "missing-session"]);
    assert!(
        !result.success,
        "Expected missing session export to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Session 'missing-session' not found");
}

//! Tests for query command error handling
//!
//! Ensures query commands return graceful JSON errors instead of crashing
//! when prerequisites are not met (zjj-audit-006)

mod common;

use common::TestHarness;

/// Test that session-exists returns JSON error when zjj not initialized
#[test]
fn test_session_exists_not_initialized() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Run query without initializing zjj
    let result = harness.zjj(&["query", "session-exists", "test"]);

    // Should output valid JSON (not crash)
    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON: {}",
        result.stdout
    );

    // Check the JSON structure
    if let Ok(json) = parsed {
        // exists should be null (not true/false) when there's an error
        assert!(
            json.get("exists")
                .and_then(serde_json::Value::as_bool)
                .is_none(),
            "exists field should be null on error"
        );

        // Should have error object
        let error = json.get("error");
        assert!(error.is_some(), "JSON should contain error field");

        if let Some(err) = error {
            // Check error has code and message
            assert!(err.get("code").is_some(), "Error should have code field");
            assert!(
                err.get("message").is_some(),
                "Error should have message field"
            );

            // Error code should indicate not initialized
            let code = err.get("code").and_then(serde_json::Value::as_str);
            assert!(
                code == Some("NOT_INITIALIZED") || code == Some("DATABASE_ERROR"),
                "Error code should indicate initialization issue"
            );
        }
    }
}

/// Test that session-count returns JSON error when zjj not initialized
#[test]
fn test_session_count_not_initialized() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Run query without initializing zjj
    let result = harness.zjj(&["query", "session-count"]);

    // Should output valid JSON (not crash)
    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON: {}",
        result.stdout
    );

    // Check the JSON structure
    if let Ok(json) = parsed {
        // count should be null when there's an error
        assert!(
            json.get("count")
                .and_then(serde_json::Value::as_u64)
                .is_none(),
            "count field should be null on error"
        );

        // Should have error object
        assert!(
            json.get("error").is_some(),
            "JSON should contain error field"
        );
    }
}

/// Test that session-exists works normally when initialized
#[test]
fn test_session_exists_success() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Query for existing session
    let result = harness.zjj(&["query", "session-exists", "test"]);

    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");

    if let Ok(json) = parsed {
        // exists should be true
        assert_eq!(
            json.get("exists").and_then(serde_json::Value::as_bool),
            Some(true),
            "Session should exist"
        );

        // Should have session object
        assert!(json.get("session").is_some(), "Should have session field");

        // Should NOT have error
        assert!(json.get("error").is_none(), "Should not have error field");
    }
}

/// Test that session-exists returns false for non-existent session
#[test]
fn test_session_exists_not_found() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Query for non-existent session
    let result = harness.zjj(&["query", "session-exists", "nonexistent"]);

    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");

    if let Ok(json) = parsed {
        // exists should be false
        assert_eq!(
            json.get("exists").and_then(serde_json::Value::as_bool),
            Some(false),
            "Session should not exist"
        );

        // Should NOT have error
        assert!(json.get("error").is_none(), "Should not have error field");
    }
}

/// Test that session-count works normally when initialized
#[test]
fn test_session_count_success() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test1", "--no-open"]);
    harness.assert_success(&["add", "test2", "--no-open"]);

    // Query count
    let result = harness.zjj(&["query", "session-count"]);

    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");

    if let Ok(json) = parsed {
        // count should be 2
        assert_eq!(
            json.get("count").and_then(serde_json::Value::as_u64),
            Some(2),
            "Should have 2 sessions"
        );

        // Should NOT have error
        assert!(json.get("error").is_none(), "Should not have error field");
    }
}

/// Test that suggest-name works even without database access
#[test]
fn test_suggest_name_without_db() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Run suggest-name without initializing (should still work with empty list)
    let result = harness.zjj(&["query", "suggest-name", "feature-{n}"]);

    let parsed = serde_json::from_str::<serde_json::Value>(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON: {}",
        result.stdout
    );

    if let Ok(json) = parsed {
        // Should suggest feature-1 (first in sequence)
        assert_eq!(
            json.get("suggested").and_then(serde_json::Value::as_str),
            Some("feature-1"),
            "Should suggest first in sequence"
        );
    }
}

//! Integration tests for JSON error output
//!
//! Tests that --json flag outputs proper JSON on error conditions

mod common;

use common::TestHarness;
use serde::de::Error as _;

// ============================================================================
// JSON Error Output Tests
// ============================================================================

#[test]
fn test_init_json_error_when_already_initialized() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Initialize once
    harness.assert_success(&["init"]);

    // Try to initialize again with --json flag
    let result = harness.zjj(&["init", "--json"]);

    // Should still succeed (init is idempotent), but if it errors, should be JSON
    if !result.success {
        // Check that output is valid JSON
        let output = result.stdout.trim();
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
        assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

        // Check for required error fields
        if let Ok(json) = parsed {
            assert!(json.get("$schema").is_some(), "Should have '$schema' field");
            assert!(
                json.get("schema_type").is_some(),
                "Should have 'schema_type' field"
            );
            assert!(json.get("success").is_some(), "Should have 'success' field");
            assert!(json.get("error").is_some(), "Should have 'error' field");

            let error = &json["error"];
            assert!(
                error.get("code").is_some(),
                "Error should have 'code' field"
            );
            assert!(
                error.get("message").is_some(),
                "Error should have 'message' field"
            );
        }
    }
}

#[test]
fn test_list_json_error_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to list without init, with --json flag
    let result = harness.zjj(&["list", "--json"]);

    // Should fail
    assert!(!result.success, "list should fail without init");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");

        let error = &json["error"];
        assert!(
            error.get("code").is_some(),
            "Error should have 'code' field"
        );
        assert!(
            error.get("message").is_some(),
            "Error should have 'message' field"
        );
    }
}

#[test]
fn test_focus_json_error_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to focus nonexistent session with --json flag
    let result = harness.zjj(&["focus", "nonexistent", "--json"]);

    // Should fail
    assert!(!result.success, "focus should fail for nonexistent session");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");

        let error = &json["error"];
        assert!(
            error.get("code").is_some(),
            "Error should have 'code' field"
        );
        assert!(
            error.get("message").is_some(),
            "Error should have 'message' field"
        );

        // Should suggest using 'zjj list'
        if let Some(suggestion) = error.get("suggestion") {
            let sugg_str = suggestion.as_str().unwrap_or("");
            assert!(
                sugg_str.contains("list"),
                "Should suggest using list command"
            );
        }
    }
}

#[test]
fn test_remove_json_error_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to remove nonexistent session with --json flag
    let result = harness.zjj(&["remove", "nonexistent", "--force", "--json"]);

    // Should fail
    assert!(
        !result.success,
        "remove should fail for nonexistent session"
    );

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");
    }
}

#[test]
fn test_add_json_error_invalid_name() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to add session with invalid name (starts with dash)
    let result = harness.zjj(&["add", "-invalid", "--no-open", "--json"]);

    // Should fail
    assert!(!result.success, "add should fail with invalid name");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");

        let error = &json["error"];
        let code = error.get("code").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            code.contains("INVALID") || code.contains("VALIDATION"),
            "Error code should indicate validation error: {code}"
        );
    }
}

#[test]
fn test_add_json_error_duplicate_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Add a session
    harness.assert_success(&["add", "test", "--no-open"]);

    // Try to add same session again with --json flag
    let result = harness.zjj(&["add", "test", "--no-open", "--json"]);

    // Should fail
    assert!(!result.success, "add should fail for duplicate session");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");

        let error = &json["error"];
        let message = error.get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            message.contains("already exists") || message.contains("duplicate"),
            "Error message should indicate duplicate: {message}"
        );
    }
}

#[test]
fn test_status_json_error_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to get status without init, with --json flag
    let result = harness.zjj(&["status", "--json"]);

    // Should fail
    assert!(!result.success, "status should fail without init");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");
    }
}

#[test]
fn test_sync_json_error_without_init() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try to sync without init, with --json flag
    let result = harness.zjj(&["sync", "--json"]);

    // Should fail
    assert!(!result.success, "sync should fail without init");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");
    }
}

#[test]
fn test_doctor_json_error_format() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try doctor with --json (may or may not error, but should output JSON)
    let result = harness.zjj(&["doctor", "--json"]);

    // If it outputs anything, it should be valid JSON
    let output = result.stdout.trim();
    if !output.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
        assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

        if let Ok(json) = parsed {
            // Doctor output uses SchemaEnvelope wrapper
            assert!(json.get("$schema").is_some(), "Should have '$schema' field");
            assert!(json.get("success").is_some(), "Should have 'success' field");
            assert!(json.get("checks").is_some(), "Should have 'checks' field");
            assert!(json.get("summary").is_some(), "Should have 'summary' field");
        }
    }
}

#[test]
fn test_introspect_json_error_format() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    // Try introspect with --json (should always output JSON)
    let result = harness.zjj(&["introspect", "--json"]);

    // Should output valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");
}

#[test]
fn test_diff_json_error_nonexistent_session() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };
    harness.assert_success(&["init"]);

    // Try to diff nonexistent session with --json flag
    let result = harness.zjj(&["diff", "nonexistent", "--json"]);

    // Should fail
    assert!(!result.success, "diff should fail for nonexistent session");

    // Check that output is valid JSON
    let output = result.stdout.trim();
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(parsed.is_ok(), "Output should be valid JSON: {output}");

    // Check for required error fields
    if let Ok(json) = parsed {
        assert_eq!(
            json.get("success").and_then(serde_json::Value::as_bool),
            Some(false),
            "success should be false"
        );
        assert!(json.get("$schema").is_some(), "Should have '$schema' field");
        assert_eq!(
            json.get("schema_type").and_then(serde_json::Value::as_str),
            Some("single"),
            "schema_type should be 'single'"
        );
        assert!(json.get("error").is_some(), "Should have 'error' field");
    }
}

#[test]
fn test_error_response_schema_contract() -> Result<(), serde_json::Error> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "", "--json"]);
    assert!(!result.success, "add should fail for empty name");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

    assert_eq!(
        parsed.get("$schema").and_then(|v| v.as_str()),
        Some("zjj://error-response/v1"),
        "$schema must point to error-response"
    );
    assert_eq!(
        parsed.get("_schema_version").and_then(|v| v.as_str()),
        Some("1.0"),
        "Schema version should be 1.0"
    );
    assert_eq!(
        parsed.get("schema_type").and_then(|v| v.as_str()),
        Some("single"),
        "schema_type should be single"
    );
    assert_eq!(
        parsed.get("success").and_then(serde_json::Value::as_bool),
        Some(false),
        "success should be false"
    );

    let error_obj = parsed
        .get("error")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| serde_json::Error::custom("Error field should be object"))?;

    assert!(error_obj.get("code").is_some(), "Error should expose code");
    assert_eq!(
        error_obj
            .get("exit_code")
            .and_then(serde_json::Value::as_i64),
        Some(1),
        "Validation errors must expose exit_code 1"
    );
    assert!(
        error_obj.get("message").is_some(),
        "Error should expose message"
    );
    Ok(())
}

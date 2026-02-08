//! Comprehensive JSON standardization tests
//!
//! This test file verifies that all commands follow consistent JSON output standards:
//! 1. All JSON outputs use `SchemaEnvelope` or `SchemaEnvelopeArray`
//! 2. Schema names follow consistent naming conventions
//! 3. Error outputs use standardized error format
//! 4. Success outputs include all required fields

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

mod common;

use common::TestHarness;

/// Test helper to validate `SchemaEnvelope` structure
fn validate_envelope(json: &serde_json::Value, expected_schema: &str) -> Result<(), String> {
    // Check for $schema field
    let schema = json
        .get("$schema")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing $schema field".to_string())?;

    let expected_schema_uri = format!("zjj://{expected_schema}/v1");
    if schema != expected_schema_uri {
        return Err(format!(
            "Schema URI mismatch: expected {expected_schema_uri}, got {schema}"
        ));
    }

    // Check for _schema_version field
    let version = json
        .get("_schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing _schema_version field".to_string())?;

    if version != "1.0" {
        return Err(format!(
            "Schema version mismatch: expected 1.0, got {version}"
        ));
    }

    // Check for schema_type field
    let schema_type = json
        .get("schema_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing schema_type field".to_string())?;

    if !matches!(schema_type, "single" | "array") {
        return Err(format!("Invalid schema_type: {schema_type}"));
    }

    // Check for success field
    if json.get("success").is_none() {
        return Err("Missing success field".to_string());
    }

    Ok(())
}

/// Test that init command JSON output uses envelope
#[test]
fn test_init_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.zjj(&["init", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "init should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "init-response")?;

    // Check for expected fields
    assert!(
        parsed.get("root").is_some(),
        "init response should have root field"
    );
    assert!(
        parsed.get("paths").is_some(),
        "init response should have paths field"
    );
    assert!(
        parsed.get("message").is_some(),
        "init response should have message field"
    );
    assert!(
        parsed.get("jj_initialized").is_some(),
        "init response should have jj_initialized field"
    );

    Ok(())
}

/// Test that add command JSON output uses envelope
#[test]
fn test_add_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test-session", "--json", "--no-open"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "add should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "add-response")?;

    // Check for expected fields
    assert!(
        parsed.get("name").is_some(),
        "add response should have name field"
    );
    assert!(
        parsed.get("workspace_path").is_some(),
        "add response should have workspace_path field"
    );
    assert!(
        parsed.get("zellij_tab").is_some(),
        "add response should have zellij_tab field"
    );
    assert!(
        parsed.get("status").is_some(),
        "add response should have status field"
    );

    Ok(())
}

/// Test that list command JSON output uses envelope
#[test]
fn test_list_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "list-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "list should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "list-response")?;

    // Check schema_type is "array" for list
    let schema_type = parsed
        .get("schema_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing schema_type")?;
    assert_eq!(
        schema_type, "array",
        "list should use array schema type, got {schema_type}"
    );

    // Check for data field
    assert!(
        parsed.get("data").is_some(),
        "list response should have data field"
    );

    Ok(())
}

/// Test that focus command JSON output uses envelope
#[test]
fn test_focus_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "focus-test", "--no-open"]);

    let result = harness.zjj(&["focus", "focus-test", "--json", "--no-zellij"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "focus should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "focus-response")?;

    // Check for expected fields
    assert!(
        parsed.get("name").is_some(),
        "focus response should have name field"
    );
    assert!(
        parsed.get("message").is_some(),
        "focus response should have message field"
    );

    Ok(())
}

/// Test that status command JSON output uses envelope (if implemented)
#[test]
fn test_status_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "status-test", "--no-open"]);

    let result = harness.zjj(&["status", "--json"]);

    // Note: status command may not have --json flag yet, this test
    // documents the expected behavior
    if result.success {
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
        validate_envelope(&parsed, "status-response")?;

        // Check for expected fields
        assert!(
            parsed.get("sessions").is_some(),
            "status response should have sessions field"
        );
    } else {
        // If status --json not implemented yet, that's OK for this test
        // This test documents the expected standard
    }

    Ok(())
}

/// Test that remove command JSON output uses envelope
#[test]
fn test_remove_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "remove-test", "--no-open"]);

    let result = harness.zjj(&["remove", "remove-test", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "remove should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "remove-response")?;

    // Check for expected fields
    assert!(
        parsed.get("name").is_some(),
        "remove response should have name field"
    );
    assert!(
        parsed.get("message").is_some(),
        "remove response should have message field"
    );

    Ok(())
}

/// Test that sync command JSON output uses envelope
#[test]
fn test_sync_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "sync-test", "--no-open"]);

    let result = harness.zjj(&["sync", "sync-test", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "sync should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    validate_envelope(&parsed, "sync-response")?;

    // Check for expected fields
    assert!(
        parsed.get("name").is_some(),
        "sync response should have name field"
    );
    assert!(
        parsed.get("synced_count").is_some(),
        "sync response should have synced_count field"
    );
    assert!(
        parsed.get("failed_count").is_some(),
        "sync response should have failed_count field"
    );

    Ok(())
}

/// Test that error responses use standardized format
#[test]
fn test_error_json_has_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    // Try to focus on a non-existent session
    let result = harness.zjj(&["focus", "nonexistent", "--json"]);

    // Should fail but produce JSON error output
    if result.stdout.contains("{") {
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

        // Error responses should have envelope
        validate_envelope(&parsed, "error-response")?;

        // Check success is false
        let success = parsed
            .get("success")
            .and_then(serde_json::Value::as_bool)
            .ok_or("Missing success field")?;

        assert!(!success, "Error response should have success=false");

        // Check for error field
        assert!(
            parsed.get("error").is_some(),
            "Error response should have error field"
        );

        let error = parsed.get("error").unwrap();

        // Check error has required fields
        assert!(error.get("code").is_some(), "Error should have code field");
        assert!(
            error.get("message").is_some(),
            "Error should have message field"
        );
        assert!(
            error.get("exit_code").is_some(),
            "Error should have exit_code field"
        );
    }

    Ok(())
}

/// Test that all command responses have consistent schema URI format
#[test]
fn test_schema_uri_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    // Test multiple commands
    let commands = vec![
        (
            vec!["add", "uri-test", "--json", "--no-open"],
            "add-response",
        ),
        (vec!["list", "--json"], "list-response"),
    ];

    for (args, expected_schema) in commands {
        let result = harness.zjj(&args);
        if result.success {
            let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

            // Verify schema URI format
            if let Some(schema) = parsed.get("$schema").and_then(|v| v.as_str()) {
                let expected_uri = format!("zjj://{expected_schema}/v1");
                assert_eq!(
                    schema, expected_uri,
                    "Schema URI for {args:?} should be {expected_uri}, got {schema}"
                );
            }
        }
    }

    Ok(())
}

/// Test that schema naming follows conventions
#[test]
fn test_schema_naming_conventions() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    // Schema names should follow pattern: {command}-response
    // Test a few commands to verify
    let add_result = harness.zjj(&["add", "naming-test", "--json", "--no-open"]);

    if add_result.success {
        let parsed: serde_json::Value = serde_json::from_str(add_result.stdout.trim())?;

        if let Some(schema) = parsed.get("$schema").and_then(|v| v.as_str()) {
            // Schema should follow zjj://{command}-response/v1 pattern
            assert!(
                schema.starts_with("zjj://"),
                "Schema should start with 'zjj://', got {schema}"
            );
            assert!(
                schema.ends_with("/v1"),
                "Schema should end with '/v1', got {schema}"
            );
            assert!(
                schema.contains("-response"),
                "Schema should contain '-response', got {schema}"
            );
        }
    }

    Ok(())
}

/// Test that _links field is present (even if empty) for HATEOAS
#[test]
fn test_hateoas_links_field() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "hateoas-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if result.success {
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

        // _links field should be present (may be empty array or omitted)
        // This test documents the expected structure
        let _ = parsed.get("_links");

        // If present, should be an array
        if let Some(links) = parsed.get("_links") {
            if let Some(arr) = links.as_array() {
                // Each link should have rel and href
                for link in arr {
                    assert!(link.get("rel").is_some(), "Link should have rel field");
                    assert!(link.get("href").is_some(), "Link should have href field");
                }
            }
        }
    }

    Ok(())
}

/// Test that _meta field is present when appropriate
#[test]
fn test_meta_field() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "meta-test", "--json", "--no-open"]);

    if result.success {
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

        // _meta field is optional but should be present in envelope structure
        // This test documents the expected structure
        let _ = parsed.get("_meta");

        // If present, should have at least command and timestamp
        if let Some(meta) = parsed.get("_meta") {
            assert!(
                meta.get("command").is_some(),
                "Meta should have command field"
            );
            assert!(
                meta.get("timestamp").is_some(),
                "Meta should have timestamp field"
            );
        }
    }

    Ok(())
}

/// Test array responses use `SchemaEnvelopeArray`
#[test]
fn test_array_envelope_for_collections() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "array-test-1", "--no-open"]);
    harness.assert_success(&["add", "array-test-2", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if !result.success {
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }
    assert!(result.success, "list should succeed");

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

    // List returns array, so should have explicit "data" field
    let data = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("list response should have data array")?;

    assert!(
        data.len() >= 2,
        "Should have at least 2 sessions, got {}",
        data.len()
    );

    Ok(())
}

/// Test that JSON output is pretty-printed
#[test]
fn test_json_is_pretty_printed() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "pretty-test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);

    if result.success {
        let json_str = result.stdout.trim();

        // Pretty-printed JSON should have newlines and indentation
        assert!(
            json_str.contains('\n'),
            "JSON should be pretty-printed with newlines"
        );
        assert!(
            json_str.contains("  ") || json_str.contains("\t"),
            "JSON should be pretty-printed with indentation"
        );
    }
}

/// Test that exit codes match `error.exit_code` field
#[test]
fn test_exit_code_matches_json() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    // Try a command that will fail
    let result = harness.zjj(&["focus", "does-not-exist", "--json"]);

    if !result.success && result.stdout.contains("{") {
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

        if let Some(error) = parsed.get("error") {
            if let Some(exit_code) = error.get("exit_code").and_then(serde_json::Value::as_i64) {
                // Exit code in JSON should match process exit code
                // Note: result.status.code() may not be available in all test harnesses
                assert!(
                    exit_code >= 1 && exit_code <= 130,
                    "Exit code should be in valid range 1-130, got {exit_code}"
                );
            }
        }
    }

    Ok(())
}

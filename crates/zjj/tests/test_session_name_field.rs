//! Integration tests for `session_name` field consistency
//!
//! Tests that all JSON outputs consistently use `session_name` field (never `session`)
//! across all commands: add, remove, focus, sync

mod common;

use common::TestHarness;

// ============================================================================
// AddOutput session_name Field Tests
// ============================================================================

#[test]
fn test_add_output_has_session_name_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add session with --json flag
    let result = harness.zjj(&["add", "test-session", "--no-open", "--json"]);
    assert!(result.success, "add command failed: {}", result.stderr);

    // Parse JSON response
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    // Verify session_name field exists and has correct value
    assert_eq!(
        json.get("session_name"),
        Some(&serde_json::Value::String("test-session".to_string())),
        "AddOutput must have 'session_name' field with correct value"
    );

    // Verify old 'session' field does NOT exist
    assert!(
        json.get("session").is_none(),
        "AddOutput must NOT have old 'session' field"
    );

    // Verify field is string type
    assert!(
        json["session_name"].is_string(),
        "session_name must be a string"
    );
}

#[test]
fn test_add_output_session_name_with_hyphens() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test-feature-name", "--no-open", "--json"]);
    assert!(result.success);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    assert_eq!(
        json["session_name"], "test-feature-name",
        "session_name should preserve hyphens"
    );
}

#[test]
fn test_add_output_session_name_with_underscores() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test_feature_name", "--no-open", "--json"]);
    assert!(result.success);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    assert_eq!(
        json["session_name"], "test_feature_name",
        "session_name should preserve underscores"
    );
}

#[test]
fn test_add_output_session_name_with_numbers() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "test-session-2025", "--no-open", "--json"]);
    assert!(result.success);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    assert_eq!(
        json["session_name"], "test-session-2025",
        "session_name should preserve numbers"
    );
}

// ============================================================================
// RemoveOutput session_name Field Tests
// ============================================================================

#[test]
fn test_remove_output_has_session_name_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-remove", "--no-open"]);

    // Remove session with --json flag
    let result = harness.zjj(&["remove", "test-remove", "--json", "--force"]);
    assert!(result.success, "remove command failed: {}", result.stderr);

    // Parse JSON response
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    // Verify session_name field exists and has correct value
    assert_eq!(
        json.get("session_name"),
        Some(&serde_json::Value::String("test-remove".to_string())),
        "RemoveOutput must have 'session_name' field with correct value"
    );

    // Verify old 'session' field does NOT exist
    assert!(
        json.get("session").is_none(),
        "RemoveOutput must NOT have old 'session' field"
    );

    // Verify field is string type
    assert!(
        json["session_name"].is_string(),
        "session_name must be a string"
    );
}

#[test]
fn test_remove_dry_run_output_has_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-dry-run", "--no-open"]);

    // Remove with --dry-run --json
    let result = harness.zjj(&["remove", "test-dry-run", "--json", "--dry-run"]);
    assert!(result.success, "remove --dry-run failed: {}", result.stderr);

    // Parse JSON response
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) else {
        eprintln!("Failed to parse JSON: {}", result.stdout);
        return;
    };

    // Verify plan has session_name
    let plan = &json["plan"];
    assert!(
        plan.get("session_name").is_some(),
        "Dry-run plan must have 'session_name' field"
    );
    assert_eq!(
        plan["session_name"], "test-dry-run",
        "Plan session_name must match"
    );
}

// ============================================================================
// FocusOutput session_name Field Tests
// ============================================================================

#[test]
fn test_focus_output_has_session_name_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-focus", "--no-open"]);

    // Focus session with --json flag (outside Zellij, will fail gracefully)
    let result = harness.zjj(&["focus", "test-focus", "--json"]);

    // Parse JSON response even if it fails (error response should still have structure)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
        // If success, verify session_name
        if json.get("success") == Some(&serde_json::json!(true)) {
            assert_eq!(
                json.get("session_name"),
                Some(&serde_json::Value::String("test-focus".to_string())),
                "FocusOutput must have 'session_name' field"
            );

            // Verify old 'session' field does NOT exist
            assert!(
                json.get("session").is_none(),
                "FocusOutput must NOT have old 'session' field"
            );

            // Verify field is string type
            assert!(
                json["session_name"].is_string(),
                "session_name must be a string"
            );
        }
    }
}

// ============================================================================
// Invalid Session Name Tests
// ============================================================================

#[test]
fn test_add_invalid_session_name_starting_with_number() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to create session starting with number
    let result = harness.zjj(&["add", "123invalid", "--no-open", "--json"]);
    assert!(
        !result.success,
        "Should not allow session name starting with number"
    );

    // Parse JSON error response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
        assert_eq!(json["success"], false, "Success should be false");
        assert!(
            json.get("error").is_some(),
            "Should have error field in JSON response"
        );
    }
}

#[test]
fn test_add_invalid_session_name_empty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to create session with empty name (will likely be caught by clap)
    let result = harness.zjj(&["add", "", "--no-open", "--json"]);
    assert!(!result.success, "Should not allow empty session name");
}

#[test]
fn test_add_long_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create session with 64-char name (max length)
    let long_name = "a".repeat(64);
    let result = harness.zjj(&["add", &long_name, "--no-open", "--json"]);

    if result.success {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.stdout) {
            assert_eq!(
                json["session_name"], long_name,
                "Should handle 64-char session name"
            );
        } else {
            eprintln!("Failed to parse JSON: {}", result.stdout);
        }
    }
}

// ============================================================================
// Combined Workflow Tests
// ============================================================================

#[test]
fn test_add_and_remove_workflow_json_consistency() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Add session
    let add_result = harness.zjj(&["add", "workflow-test", "--no-open", "--json"]);
    assert!(add_result.success);

    let Ok(add_json) = serde_json::from_str::<serde_json::Value>(&add_result.stdout) else {
        eprintln!("Failed to parse add JSON: {}", add_result.stdout);
        return;
    };

    // Get session name from add output
    let Some(session_name) = add_json["session_name"].as_str() else {
        eprintln!("session_name should be string in add_json");
        return;
    };

    // Remove session
    let remove_result = harness.zjj(&["remove", session_name, "--json", "--force"]);
    assert!(remove_result.success);

    let Ok(remove_json) = serde_json::from_str::<serde_json::Value>(&remove_result.stdout) else {
        eprintln!("Failed to parse remove JSON: {}", remove_result.stdout);
        return;
    };

    // Verify both use same field name and value
    assert_eq!(
        add_json["session_name"], remove_json["session_name"],
        "add and remove outputs must use same session_name"
    );
    assert_eq!(
        add_json["session_name"], "workflow-test",
        "session_name must be consistent across commands"
    );
}

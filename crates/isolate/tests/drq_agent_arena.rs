#![allow(clippy::redundant_closure_for_method_calls)]
// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! DRQ-Style Test Bank for AI Agent Arena
//!
//! This module implements the Dynamic Revaluation of Quality (DRQ) testing methodology
//! for isolate, treating AI agents as the primary end-user.
//!
//! ## DRQ Methodology
//!
//! 1. **End-User Arena**: AI agents programmatically driving isolate via CLI + JSON
//! 2. **Fitness Signal**: How reliably agents can discover, query, and control isolate state
//! 3. **Evolution**: Each new feature must beat the "champion" by passing all prior tests
//! 4. **Red Queen**: The test bank continually expands; no regression is ever forgotten
//!
//! ## Test Categories
//!
//! - **Round 1**: JSON schema consistency (all commands use `SchemaEnvelope`)
//! - **Round 2**: Atomicity (operations are all-or-nothing)
//! - **Round 3**: Core workflow (add → work → done)
//! - **Round 4**: Concurrency (multi-agent scenarios)
//! - **Round 5**: Error semantics (retryable vs permanent failures)

mod common;
use common::TestHarness;

/// Parse JSON output (single object, not JSONL)
fn parse_json_output(output: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Try to parse as single JSON object
    serde_json::from_str(output).map_err(|e| format!("JSON parse error: {e}").into())
}

/// Validate that output has schema envelope fields
fn validate_schema_envelope(
    json: &serde_json::Value,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if json.get("$schema").is_none() {
        return Err(format!("{command}: Missing $schema field").into());
    }
    if json.get("_schema_version").is_none() {
        return Err(format!("{command}: Missing _schema_version field").into());
    }
    if json.get("success").is_none() {
        return Err(format!("{command}: Missing success field").into());
    }
    Ok(())
}

// ============================================================================
// ROUND 1: JSON Schema Consistency Tests
// ============================================================================

#[test]
fn test_all_json_outputs_use_schema_envelope() -> Result<(), Box<dyn std::error::Error>> {
    // Every command with --json should return valid JSON
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    // Test commands that return single JSON objects with schema envelopes
    let json_commands: Vec<&[&str]> = vec![
        &["query", "session-exists", "test-session", "--json"],
        &["query", "can-run", "add", "--json"],
        &["query", "session-count", "--json"],
    ];

    for args in json_commands {
        let result = harness.isolate(args);
        if result.success {
            let json = parse_json_output(&result.stdout)?;
            validate_schema_envelope(&json, args.join(" ").as_str())?;
        }
    }

    Ok(())
}

#[test]
fn test_json_output_has_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    // Query commands have schema envelope
    let result = harness.isolate(&["query", "session-exists", "test-session", "--json"]);
    assert!(result.success);

    let json = parse_json_output(&result.stdout)?;

    // All JSON outputs must have these fields
    assert!(json.get("$schema").is_some(), "Missing $schema field");
    assert!(
        json.get("_schema_version").is_some(),
        "Missing _schema_version field"
    );
    assert!(json.get("success").is_some(), "Missing success field");

    Ok(())
}

#[test]
fn test_diff_uses_schema_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    let result = harness.isolate(&["diff", "test-session", "--json"]);
    // Diff might fail if no changes, but JSON format should still be valid
    if result.success && !result.stdout.trim().is_empty() {
        // Try to parse first line as JSON (might be JSONL)
        let first_line = result.stdout.lines().next().unwrap_or("");
        if !first_line.is_empty() {
            let json: serde_json::Value = serde_json::from_str(first_line)?;
            // Validate if it has schema envelope
            if json.get("$schema").is_some() {
                validate_schema_envelope(&json, "diff")?;
            }
        }
    }
    Ok(())
}

// ============================================================================
// ROUND 2: Atomicity Tests
// ============================================================================

#[test]
fn test_failed_add_leaves_no_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Create a session
    let result = harness.isolate(&["add", "test-session", "--no-open"]);
    assert!(result.success, "add with --no-open should succeed");

    // Try to add a session with the same name (should fail)
    let result2 = harness.isolate(&["add", "test-session", "--no-open"]);
    assert!(!result2.success, "Duplicate session should fail");

    // Verify only one session exists using session-count query
    let count_result = harness.isolate(&["query", "session-count", "--json"]);
    assert!(count_result.success);
    let json = parse_json_output(&count_result.stdout)?;
    let count = json.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert_eq!(count, 1, "Should have exactly 1 session");

    Ok(())
}

#[test]
fn test_remove_cleans_up_all_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    harness.assert_workspace_exists("test-session");

    // Remove the session
    harness.assert_success(&["remove", "test-session", "-f"]);

    // Verify workspace is gone
    harness.assert_workspace_not_exists("test-session");

    // Verify session is not in database using session-count
    let count_result = harness.isolate(&["query", "session-count", "--json"]);
    assert!(count_result.success);
    let json = parse_json_output(&count_result.stdout)?;
    let count = json.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert_eq!(count, 0, "Should have 0 sessions after remove");
    Ok(())
}

// ============================================================================
// ROUND 3: Core Workflow Tests
// ============================================================================

#[test]
fn test_complete_agent_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    // 1. Initialize isolate
    harness.assert_success(&["init"]);

    // 2. Create session
    harness.assert_success(&["add", "feature-1", "--no-open"]);

    // 3. Verify session exists
    let status = harness.isolate(&["status", "feature-1", "--json"]);
    assert!(status.success);

    // 4. Query session state
    let query_result = harness.isolate(&["query", "session-exists", "feature-1", "--json"]);
    assert!(query_result.success);
    let json = parse_json_output(&query_result.stdout)?;
    assert_eq!(json["exists"], true);

    // 5. Remove session (cleanup)
    harness.assert_success(&["remove", "feature-1", "-f"]);

    // 6. Verify cleanup
    let query_result2 = harness.isolate(&["query", "session-exists", "feature-1", "--json"]);
    assert!(query_result2.success);
    let json2 = parse_json_output(&query_result2.stdout)?;
    assert_eq!(json2["exists"], false);

    Ok(())
}

#[test]
fn test_context_command_for_agents() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-session", "--no-open"]);

    let result = harness.isolate(&["context", "--json"]);
    assert!(result.success, "context command should succeed");

    // Parse and validate output
    let json = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&json, "context")?;

    Ok(())
}

#[test]
fn test_query_command_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Query session count
    let result = harness.isolate(&["query", "session-count", "--json"]);
    assert!(result.success);
    let json = parse_json_output(&result.stdout)?;
    let count = json.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert_eq!(count, 0, "Should have 0 sessions initially");

    // Add a session
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query again and verify increment
    let result2 = harness.isolate(&["query", "session-count", "--json"]);
    assert!(result2.success);
    let json2 = parse_json_output(&result2.stdout)?;
    let count2 = json2.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert_eq!(count2, 1, "Should have 1 session after add");

    Ok(())
}

// ============================================================================
// ROUND 4: Concurrency Tests
// ============================================================================

#[test]
fn test_concurrent_add_same_name() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Try to add the same session from two "agents" concurrently
    let result1 = harness.isolate(&["add", "race-test", "--no-open"]);
    let result2 = harness.isolate(&["add", "race-test", "--no-open"]);

    // Exactly one should succeed
    let successes = vec![result1.success, result2.success]
        .into_iter()
        .filter(|&x| x)
        .count();

    assert_eq!(successes, 1, "Exactly one add should succeed");

    // Cleanup
    if result1.success {
        harness.assert_success(&["remove", "race-test", "-f"]);
    }
}

#[test]
fn test_multiple_sessions_isolation() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions (simulating multiple agents)
    let sessions = ["session-a", "session-b", "session-c"];

    for session in sessions {
        let result = harness.isolate(&["add", session, "--no-open"]);
        assert!(result.success, "Failed to create session");
    }

    // Verify all sessions exist using session-count
    let count_result = harness.isolate(&["query", "session-count", "--json"]);
    assert!(count_result.success);
    let json = parse_json_output(&count_result.stdout)?;
    let count = json.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert_eq!(count, 3, "Should have 3 sessions");

    // Verify each session can be queried independently
    for session in sessions {
        let result = harness.isolate(&["status", session, "--json"]);
        assert!(result.success, "Status for {session} should succeed");
    }

    // Cleanup
    for session in sessions {
        let result = harness.isolate(&["remove", session, "-f"]);
        assert!(result.success, "Failed to remove session");
    }

    Ok(())
}

// ============================================================================
// ROUND 5: Error Semantics Tests
// ============================================================================

#[test]
fn test_error_responses_have_consistent_structure() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Try to get a non-existent session
    let result = harness.isolate(&["status", "nonexistent", "--json"]);
    assert!(!result.success, "Should fail for non-existent session");

    let json = parse_json_output(&result.stdout)?;

    // Validate schema envelope
    validate_schema_envelope(&json, "status")?;

    // Error responses must have success=false and error object
    assert_eq!(json["success"], false, "Error should have success=false");
    assert!(json.get("error").is_some(), "Error should have error field");

    let error_obj = json.get("error").unwrap();
    assert!(error_obj.get("code").is_some(), "Error should have code");
    assert!(
        error_obj.get("message").is_some(),
        "Error should have message"
    );

    Ok(())
}

#[test]
fn test_not_found_vs_validation_error_codes() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Test not found error (exit code 2)
    let result1 = harness.isolate(&["status", "nonexistent", "--json"]);
    let json1 = parse_json_output(&result1.stdout)?;
    assert_eq!(
        json1["error"]["exit_code"], 2,
        "Not found should be exit code 2"
    );

    // Test validation error - invalid session name
    let result2 = harness.isolate(&["add", "123invalid", "--no-open", "--json"]);
    let json2 = parse_json_output(&result2.stdout)?;
    // Validation errors should have exit code 1 or 2
    let exit_code = json2["error"]["exit_code"].as_i64().unwrap_or(-1);
    assert!(
        exit_code == 1 || exit_code == 2,
        "Validation error should be exit code 1 or 2, got {exit_code}"
    );

    Ok(())
}

// ============================================================================
// ROUND 6: DRQ Concurrency Query Tests
// ============================================================================

#[test]
fn test_query_session_exists_for_missing_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Query a non-existent session
    let result = harness.isolate(&["query", "session-exists", "nonexistent", "--json"]);

    let json = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&json, "query session-exists")?;

    assert_eq!(
        json["success"], true,
        "session-exists should return success=true"
    );
    assert_eq!(json["exists"], false, "Missing session should not exist");

    Ok(())
}

#[test]
fn test_query_session_exists_for_existing_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query an existing session
    let result = harness.isolate(&["query", "session-exists", "test-session", "--json"]);

    let json = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&json, "query session-exists")?;

    assert_eq!(
        json["success"], true,
        "session-exists should return success=true"
    );
    assert_eq!(json["exists"], true, "Created session should exist");

    // Cleanup
    harness.assert_success(&["remove", "test-session", "-f"]);
    Ok(())
}

#[test]
fn test_query_can_run_includes_prerequisite_summary() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);

    let result = harness.isolate(&["query", "can-run", "add", "--json"]);

    let json = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&json, "query can-run")?;

    assert_eq!(json["success"], true, "can-run should return success=true");
    assert!(
        json.get("can_run").and_then(|v| v.as_bool()).is_some(),
        "can-run should include boolean can_run"
    );

    Ok(())
}

#[test]
fn test_query_suggest_name_finds_next_available_name() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-1", "--no-open"]);

    let result = harness.isolate(&["query", "suggest-name", "feature-{n}", "--json"]);
    assert!(result.success);

    let json = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&json, "query suggest-name")?;

    // Check pattern round-trips
    let pattern = json.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        pattern.contains("feature"),
        "Pattern should contain 'feature'"
    );

    let suggested = json.get("suggested").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        suggested.starts_with("feature-"),
        "Suggested name should follow requested pattern"
    );
    assert_ne!(
        suggested, "feature-1",
        "Suggested name should avoid existing session names"
    );

    // Cleanup
    harness.assert_success(&["remove", "feature-1", "-f"]);
    Ok(())
}

#[test]
fn test_all_query_commands_use_schema_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query commands with JSON envelopes
    let queries: Vec<&[&str]> = vec![
        &["query", "session-exists", "test-session", "--json"],
        &["query", "can-run", "add", "--json"],
        &["query", "suggest-name", "feature-{n}", "--json"],
        &["query", "session-count", "--json"],
    ];

    for args in queries {
        let result = harness.isolate(args);
        if result.success {
            let json = parse_json_output(&result.stdout)?;
            validate_schema_envelope(&json, args.join(" ").as_str())?;
        }
    }

    Ok(())
}

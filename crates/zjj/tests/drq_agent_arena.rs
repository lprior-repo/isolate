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
//! for zjj, treating AI agents as the primary end-user.
//!
//! ## DRQ Methodology
//!
//! 1. **End-User Arena**: AI agents programmatically driving zjj via CLI + JSON
//! 2. **Fitness Signal**: How reliably agents can discover, query, and control zjj state
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
use common::{parse_json_output, payload, session_entries, validate_schema_envelope, TestHarness};
use serde_json::Value as JsonValue;

// ============================================================================
// ROUND 1: JSON Schema Consistency Tests
// ============================================================================

#[test]
fn test_all_json_outputs_use_schema_envelope() -> Result<(), Box<dyn std::error::Error>> {
    // Every command with --json should return SchemaEnvelope
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // Initialize zjj first
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    let json_commands = [
        vec!["list", "--json"],
        vec!["status", "test-session", "--json"],
        vec!["sync", "test-session", "--json"],
        vec!["diff", "test-session", "--json"],
        vec!["query", "session-exists", "test-session", "--json"],
        vec!["query", "can-run", "add", "--json"],
        vec!["context", "--json"],
    ];

    // Use functional traversal and validate each output shape directly
    json_commands
        .iter()
        .map(|args| (args, harness.zjj(args)))
        .try_for_each(|(args, result)| {
            validate_schema_envelope(&result.stdout, args[0]).map_err(
                |e| -> Box<dyn std::error::Error> {
                    format!(
                        "{} schema validation failed\nStdout: {}\nStderr: {}\nError: {}",
                        args[0], result.stdout, result.stderr, e
                    )
                    .into()
                },
            )
        })?;

    Ok(())
}

#[test]
fn test_json_output_has_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    let result = harness.zjj(&["status", "test-session", "--json"]);
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
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    let result = harness.zjj(&["diff", "test-session", "--json"]);
    // Diff might fail if no changes, but JSON format should still be consistent
    if result.success {
        validate_schema_envelope(&result.stdout, "diff")?;
    }
    Ok(())
}

// ============================================================================
// ROUND 2: Atomicity Tests
// ============================================================================

#[test]
fn test_failed_add_leaves_no_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Create a session that should fail during Zellij creation
    // We can't easily mock Zellij failure, but we can test the partial state
    let result = harness.zjj(&["add", "test-session", "--no-open"]);
    assert!(result.success, "add with --no-open should succeed");

    // Now test that a failed add doesn't leave artifacts
    // Try to add a session with the same name (should fail)
    let result2 = harness.zjj(&["add", "test-session", "--no-open"]);
    assert!(!result2.success, "Duplicate session should fail");

    // Verify only one session exists
    let list_result = harness.zjj(&["list", "--json"]);
    assert!(list_result.success);

    let json = parse_json_output(&list_result.stdout)?;
    let count = session_entries(&json).map(Vec::len).unwrap_or(0);
    assert_eq!(count, 1, "Should have exactly 1 session");

    Ok(())
}

#[test]
fn test_remove_cleans_up_all_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    harness.assert_workspace_exists("test-session");

    // Remove the session
    harness.assert_success(&["remove", "test-session", "-f"]);

    // Verify workspace is gone
    harness.assert_workspace_not_exists("test-session");

    // Verify session is not in database
    let list_result = harness.zjj(&["list", "--json"]);
    let json: JsonValue = parse_json_output(&list_result.stdout)?;
    let count = session_entries(&json).map(Vec::len).unwrap_or(0);
    assert_eq!(count, 0, "Should have 0 sessions after remove");
    Ok(())
}

// ============================================================================
// ROUND 3: Core Workflow Tests
// ============================================================================

#[test]
fn test_complete_agent_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    // 1. Initialize zjj
    harness.assert_success(&["init"]);

    // 2. Create session
    harness.assert_success(&["add", "feature-1", "--no-open"]);

    // 3. Verify session exists
    let status = harness.zjj(&["status", "feature-1", "--json"]);
    assert!(status.success);

    // 4. Query session state
    let query_result = harness.zjj(&["query", "session-exists", "feature-1", "--json"]);
    assert!(query_result.success);
    let json: JsonValue = parse_json_output(&query_result.stdout)?;
    assert_eq!(payload(&json)["exists"], true);

    // 5. Remove session (cleanup)
    harness.assert_success(&["remove", "feature-1", "-f"]);

    // 6. Verify cleanup
    let query_result2 = harness.zjj(&["query", "session-exists", "feature-1", "--json"]);
    let json2: JsonValue = parse_json_output(&query_result2.stdout)?;
    assert_eq!(
        json2["success"], true,
        "session-exists should return success=true envelope\nStdout: {}\nStderr: {}",
        query_result2.stdout, query_result2.stderr
    );
    assert_eq!(payload(&json2)["exists"], false);
    Ok(())
}

#[test]
fn test_context_command_for_agents() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-session", "--no-open"]);

    let result = harness.zjj(&["context", "--json"]);
    assert!(result.success, "context command should succeed");

    let json: JsonValue = parse_json_output(&result.stdout)?;

    // Context should provide actionable environment and repository state
    validate_schema_envelope(&result.stdout, "context")?;
    assert!(
        json.get("location").is_some(),
        "Context should include location"
    );
    assert!(
        json.get("repository").is_some(),
        "Context should include repository details"
    );
    assert!(
        json.get("suggestions").is_some(),
        "Context should include suggestions"
    );
    Ok(())
}

#[test]
fn test_query_command_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // session-count returns a JSON envelope in --json mode
    let initial_count = || -> Result<usize, Box<dyn std::error::Error>> {
        let result = harness.zjj(&["query", "session-count", "--json"]);
        assert!(result.success);
        let json = parse_json_output(&result.stdout)
            .map_err(|e| format!("Invalid session-count JSON '{}': {e}", result.stdout))?;
        payload(&json)
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or_else(|| {
                format!(
                    "Invalid session-count payload '{}': missing numeric count",
                    result.stdout
                )
                .into()
            })
    };

    assert_eq!(initial_count()?, 0, "Should have 0 sessions initially");

    // Add a session
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query again and verify increment
    let final_count = initial_count()?;
    assert_eq!(final_count, 1, "Should have 1 session after add");
    Ok(())
}

// ============================================================================
// ROUND 4: Concurrency Tests
// ============================================================================

#[test]
fn test_concurrent_add_same_name() {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return;
    };

    harness.assert_success(&["init"]);

    // Try to add the same session from two "agents" concurrently
    // This is a simplified test - real concurrency would need multiple processes
    let result1 = harness.zjj(&["add", "race-test", "--no-open"]);
    let result2 = harness.zjj(&["add", "race-test", "--no-open"]);

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
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions (simulating multiple agents)
    let sessions = ["session-a", "session-b", "session-c"];

    // Functional chain: create all sessions, collect results
    sessions
        .iter()
        .map(|session| harness.zjj(&["add", session, "--no-open"]))
        .for_each(|result| {
            assert!(result.success, "Failed to create session");
        });

    // Verify all sessions exist independently
    let list_result = harness.zjj(&["list", "--json"]);
    let json: JsonValue = parse_json_output(&list_result.stdout)?;
    let count = session_entries(&json).map(Vec::len).unwrap_or(0);
    assert_eq!(count, 3, "Should have 3 sessions");

    // Verify each session can be queried independently using functional traversal
    sessions
        .iter()
        .map(|session| (session, harness.zjj(&["status", session, "--json"])))
        .for_each(|(session, result)| {
            assert!(result.success, "Status for {session} should succeed");
        });

    // Cleanup using functional chain
    sessions
        .iter()
        .map(|session| harness.zjj(&["remove", session, "-f"]))
        .for_each(|result| {
            assert!(result.success, "Failed to remove session");
        });

    Ok(())
}

// ============================================================================
// ROUND 5: Error Semantics Tests
// ============================================================================

#[test]
fn test_error_responses_have_consistent_structure() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Try to get a non-existent session
    let result = harness.zjj(&["status", "nonexistent", "--json"]);
    assert!(!result.success, "Should fail for non-existent session");

    let json: JsonValue = parse_json_output(&result.stdout)?;

    // Define required error fields
    let error_fields: &[&str] = &["code", "message", "exit_code"];

    // Error responses must have success=false and error object
    json.get("success")
        .and_then(JsonValue::as_bool)
        .filter(|&v| !v)
        .ok_or_else(|| "Error should have success=false".to_string())?;

    json.get("error")
        .ok_or_else(|| "Error should have error field".to_string())
        .and_then(|error_obj| {
            // Validate all required error fields exist using functional traversal
            error_fields.iter().try_for_each(|&field| {
                error_obj
                    .get(field)
                    .ok_or_else(|| format!("Error should have {field}"))
                    .map(|_| ())
            })
        })?;

    Ok(())
}

#[test]
fn test_not_found_vs_validation_error_codes() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Test not found error (exit code 2)
    let result1 = harness.zjj(&["status", "nonexistent", "--json"]);
    let json1: JsonValue = parse_json_output(&result1.stdout)?;
    assert_eq!(
        json1["error"]["exit_code"], 2,
        "Not found should be exit code 2"
    );

    // Test validation error (exit code 1)
    let result2 = harness.zjj(&["add", "123invalid", "--no-open", "--json"]);
    let json2: JsonValue = parse_json_output(&result2.stdout)?;
    assert_eq!(
        json2["error"]["exit_code"], 1,
        "Validation error should be exit code 1"
    );
    Ok(())
}

// ============================================================================
// ROUND 6: DRQ Concurrency Query Tests (NEW)
// ============================================================================

#[test]
fn test_query_session_exists_for_missing_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Query a non-existent session
    let result = harness.zjj(&["query", "session-exists", "nonexistent", "--json"]);

    let json: JsonValue = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&result.stdout, "query session-exists")?;
    assert_eq!(
        json["success"], true,
        "session-exists should return success=true envelope\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert_eq!(
        payload(&json)["exists"],
        false,
        "Missing session should not exist"
    );
    assert!(
        payload(&json).get("session").is_none(),
        "Missing session should not include session details"
    );
    Ok(())
}

#[test]
fn test_query_session_exists_for_existing_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query an existing session
    let result = harness.zjj(&["query", "session-exists", "test-session", "--json"]);

    let json: JsonValue = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&result.stdout, "query session-exists")?;
    assert_eq!(
        json["success"], true,
        "session-exists should return success=true envelope\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert_eq!(
        payload(&json)["exists"],
        true,
        "Created session should exist"
    );
    assert_eq!(
        payload(&json)["session"]["name"],
        "test-session",
        "Session payload should include session metadata"
    );

    // Cleanup
    harness.assert_success(&["remove", "test-session", "-f"]);
    Ok(())
}

#[test]
fn test_query_can_run_includes_prerequisite_summary() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "can-run", "add", "--json"]);
    let json: JsonValue = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&result.stdout, "query can-run")?;
    assert_eq!(
        json["success"], true,
        "can-run should return success=true envelope\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        payload(&json)
            .get("can_run")
            .and_then(JsonValue::as_bool)
            .is_some(),
        "can-run should include boolean can_run"
    );
    assert!(
        payload(&json)
            .get("prerequisites_total")
            .and_then(JsonValue::as_u64)
            .is_some(),
        "can-run should include prerequisite totals"
    );
    assert!(
        payload(&json)
            .get("prerequisites_met")
            .and_then(JsonValue::as_u64)
            .is_some(),
        "can-run should include met prerequisite count"
    );
    Ok(())
}

#[test]
fn test_query_suggest_name_finds_next_available_name() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "feature-1", "--no-open"]);

    let result = harness.zjj(&["query", "suggest-name", "feature-{n}", "--json"]);
    assert!(result.success);

    let json: JsonValue = parse_json_output(&result.stdout)?;
    validate_schema_envelope(&result.stdout, "query suggest-name")?;
    assert_eq!(
        payload(&json)["pattern"],
        "feature-{n}",
        "Pattern should round-trip in response"
    );
    let suggested = payload(&json)["suggested"].as_str().unwrap_or("");
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
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query commands with JSON envelopes (session-count is intentionally scalar text)
    let queries = [
        vec!["session-exists", "test-session"],
        vec!["can-run", "add"],
        vec!["suggest-name", "feature-{n}"],
    ];

    // Use functional chain: execute queries and validate each envelope
    queries
        .iter()
        .map(|query_args| {
            let mut args = vec!["query"];
            args.extend(query_args.iter().copied());
            args.push("--json");
            let result = harness.zjj(&args);
            (query_args[0], result)
        })
        .try_for_each(|(query_name, result)| {
            validate_schema_envelope(&result.stdout, query_name).map_err(
                |e| -> Box<dyn std::error::Error> {
                    format!(
                        "{} schema validation failed\nStdout: {}\nStderr: {}\nError: {}",
                        query_name, result.stdout, result.stderr, e
                    )
                    .into()
                },
            )
        })?;

    Ok(())
}

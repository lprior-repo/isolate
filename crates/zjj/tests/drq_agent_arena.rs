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
//! - **Round 1**: JSON schema consistency (all commands use SchemaEnvelope)
//! - **Round 2**: Atomicity (operations are all-or-nothing)
//! - **Round 3**: Core workflow (add → work → done)
//! - **Round 4**: Concurrency (multi-agent scenarios)
//! - **Round 5**: Error semantics (retryable vs permanent failures)

// Import from the test common module
mod common;
use common::TestHarness;

use serde_json::Value as JsonValue;

// ============================================================================
// ROUND 1: JSON Schema Consistency Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_all_json_outputs_use_schema_envelope() {
    // Every command with --json should return SchemaEnvelope
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Initialize zjj first
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    let json_commands = [
        vec!["list", "--json"],
        vec!["status", "test-session", "--json"],
        vec!["sync", "test-session", "--json"],
        vec!["diff", "test-session", "--json"],
        vec!["query", "session-count", "--json"],
        vec!["context", "--json"],
    ];

    for args in json_commands {
        let result = harness.zjj(&args);
        if result.success {
            validate_schema_envelope(&result.stdout, &args[0]);
        }
    }
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_json_output_has_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    let result = harness.zjj(&["status", "test-session", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout)
        .expect("JSON output should be valid");

    // All JSON outputs must have these fields
    assert!(json.get("$schema").is_some(), "Missing $schema field");
    assert!(json.get("_schema_version").is_some(), "Missing _schema_version field");
    assert!(json.get("success").is_some(), "Missing success field");
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_diff_uses_schema_envelope() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    let result = harness.zjj(&["diff", "test-session", "--json"]);
    // Diff might fail if no changes, but JSON format should still be consistent
    if result.success {
        validate_schema_envelope(&result.stdout, "diff");
    }
}

// ============================================================================
// ROUND 2: Atomicity Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_failed_add_leaves_no_artifacts() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
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

    let json: JsonValue = serde_json::from_str(&list_result.stdout).unwrap();
    let count = json["sessions"].as_array().map(|v| v.len()).unwrap_or(0);
    assert_eq!(count, 1, "Should have exactly 1 session");
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_remove_cleans_up_all_artifacts() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
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
    let json: JsonValue = serde_json::from_str(&list_result.stdout).unwrap();
    let count = json["sessions"].as_array().map(|v| v.len()).unwrap_or(0);
    assert_eq!(count, 0, "Should have 0 sessions after remove");
}

// ============================================================================
// ROUND 3: Core Workflow Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_complete_agent_workflow() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
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
    let json: JsonValue = serde_json::from_str(&query_result.stdout).unwrap();
    assert_eq!(json["exists"], true);

    // 5. Remove session (cleanup)
    harness.assert_success(&["remove", "feature-1", "-f"]);

    // 6. Verify cleanup
    let query_result2 = harness.zjj(&["query", "session-exists", "feature-1", "--json"]);
    assert!(query_result2.success);
    let json2: JsonValue = serde_json::from_str(&query_result2.stdout).unwrap();
    assert_eq!(json2["exists"], false);
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_context_command_for_agents() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-session", "--no-open"]);

    let result = harness.zjj(&["context", "--json"]);
    assert!(result.success, "context command should succeed");

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();

    // Context should provide all the information an AI agent needs
    assert!(json.get("zjj").is_some(), "Context should have zjj section");
    assert!(json.get("jj").is_some(), "Context should have jj section");
    assert!(json.get("zellij").is_some(), "Context should have zellij section");
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_command_discovery() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Test session-count query
    let result = harness.zjj(&["query", "session-count", "--json"]);
    assert!(result.success);
    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(json["count"], 0, "Should have 0 sessions initially");

    // Add a session
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query again
    let result2 = harness.zjj(&["query", "session-count", "--json"]);
    assert!(result2.success);
    let json2: JsonValue = serde_json::from_str(&result2.stdout).unwrap();
    assert_eq!(json2["count"], 1, "Should have 1 session after add");
}

// ============================================================================
// ROUND 4: Concurrency Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_concurrent_add_same_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_multiple_sessions_isolation() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Create multiple sessions (simulating multiple agents)
    harness.assert_success(&["add", "session-a", "--no-open"]);
    harness.assert_success(&["add", "session-b", "--no-open"]);
    harness.assert_success(&["add", "session-c", "--no-open"]);

    // Verify all sessions exist independently
    let list_result = harness.zjj(&["list", "--json"]);
    let json: JsonValue = serde_json::from_str(&list_result.stdout).unwrap();
    let count = json["sessions"].as_array().map(|v| v.len()).unwrap_or(0);
    assert_eq!(count, 3, "Should have 3 sessions");

    // Verify each session can be queried independently
    for session in ["session-a", "session-b", "session-c"] {
        let result = harness.zjj(&["status", session, "--json"]);
        assert!(result.success, "Status for {session} should succeed");
    }

    // Cleanup
    harness.assert_success(&["remove", "session-a", "-f"]);
    harness.assert_success(&["remove", "session-b", "-f"]);
    harness.assert_success(&["remove", "session-c", "-f"]);
}

// ============================================================================
// ROUND 5: Error Semantics Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_error_responses_have_consistent_structure() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Try to get a non-existent session
    let result = harness.zjj(&["status", "nonexistent", "--json"]);
    assert!(!result.success, "Should fail for non-existent session");

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();

    // Error responses must have these fields
    assert!(json.get("success").is_some(), "Error should have success field");
    assert_eq!(json["success"], false, "success should be false");
    assert!(json.get("error").is_some(), "Error should have error field");
    assert!(json["error"].get("code").is_some(), "Error should have code");
    assert!(json["error"].get("message").is_some(), "Error should have message");
    assert!(json["error"].get("exit_code").is_some(), "Error should have exit_code");
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_not_found_vs_validation_error_codes() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Test not found error (exit code 2)
    let result1 = harness.zjj(&["status", "nonexistent", "--json"]);
    let json1: JsonValue = serde_json::from_str(&result1.stdout).unwrap();
    assert_eq!(json1["error"]["exit_code"], 2, "Not found should be exit code 2");

    // Test validation error (exit code 1)
    let result2 = harness.zjj(&["add", "123invalid", "--no-open", "--json"]);
    let json2: JsonValue = serde_json::from_str(&result2.stdout).unwrap();
    assert_eq!(json2["error"]["exit_code"], 1, "Validation error should be exit code 1");
}

// ============================================================================
// ROUND 6: DRQ Concurrency Query Tests (NEW)
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_session_locked_no_locks() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Query a non-existent session - should not be locked
    let result = harness.zjj(&["query", "session-locked", "nonexistent", "--json"]);
    assert!(result.success, "Query should succeed even for non-existent session");

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(json["locked"], false, "Non-existent session should not be locked");
    assert!(json["lock_info"].is_null(), "lock_info should be null when not locked");
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_session_locked_with_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query an existing session - should not be locked (no operations)
    let result = harness.zjj(&["query", "session-locked", "test-session", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(json["locked"], false, "Session should not be locked when idle");

    // Cleanup
    harness.assert_success(&["remove", "test-session", "-f"]);
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_operations_in_progress_empty() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // No active operations initially
    let result = harness.zjj(&["query", "operations-in-progress", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(json["count"], 0, "Should have 0 active operations initially");
    assert!(json["operations"].as_array().map(|v| v.is_empty()).unwrap_or(true));
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_operations_in_progress_with_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "active-session", "--no-open"]);

    // Still no active operations just from creating a session
    let result = harness.zjj(&["query", "operations-in-progress", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(json["count"], 0, "Session creation doesn't create a persistent lock");

    // Cleanup
    harness.assert_success(&["remove", "active-session", "-f"]);
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_all_query_commands_use_schema_envelope() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // All query commands should output with SchemaEnvelope
    let queries = [
        "session-count",
        "session-exists test",
        "operations-in-progress",
        "session-locked test-session",
    ];

    for query in queries {
        let result = harness.zjj(&["query", query, "--json"]);
        if result.success {
            validate_schema_envelope(&result.stdout, &query.split_whitespace().next().unwrap_or(query));
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that JSON output uses SchemaEnvelope structure
fn validate_schema_envelope(json_str: &str, command_name: &str) {
    let json: JsonValue = serde_json::from_str(json_str)
        .unwrap_or_else(|e| panic!("{}: Invalid JSON: {}", command_name, e));

    assert!(
        json.get("$schema").is_some(),
        "{}: Missing $schema field in output: {}",
        command_name,
        json_str
    );
    assert!(
        json.get("_schema_version").is_some(),
        "{}: Missing _schema_version field",
        command_name
    );
    assert!(
        json.get("success").is_some(),
        "{}: Missing success field",
        command_name
    );

    // Validate schema format
    let schema = json["$schema"].as_str().unwrap();
    assert!(
        schema.starts_with("zjj://"),
        "{}: $schema should start with 'zjj://', got: {}",
        command_name,
        schema
    );
}

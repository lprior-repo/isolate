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

// Import from the test common module
mod common;
use common::TestHarness;
use serde_json::Value as JsonValue;

/// Parse JSON from string, returning Result for error propagation
fn parse_json(s: &str) -> Result<JsonValue, serde_json::Error> {
    serde_json::from_str(s)
}

/// Validation error type for better error reporting
#[derive(Debug)]
enum ValidationError {
    MissingField(&'static str, String),
    InvalidFormat(String),
    SchemaMismatch(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field, ctx) => {
                write!(f, "Missing required field '{field}' in {ctx}")
            }
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
            Self::SchemaMismatch(msg) => write!(f, "Schema mismatch: {msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// ROUND 1: JSON Schema Consistency Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
        vec!["query", "session-count", "--json"],
        vec!["context", "--json"],
    ];

    // Use functional traversal: map results, filter successful ones, validate each
    json_commands
        .iter()
        .map(|args| (args, harness.zjj(args)))
        .filter(|(_, result)| result.success)
        .try_for_each(|(args, result)| validate_schema_envelope(&result.stdout, args[0]))?;

    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_json_output_has_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open", "--json"]);

    let result = harness.zjj(&["status", "test-session", "--json"]);
    assert!(result.success);

    let json = parse_json(&result.stdout)?;

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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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

    let json = parse_json(&list_result.stdout)?;
    let count = json["sessions"].as_array().map(Vec::len).unwrap_or(0);
    assert_eq!(count, 1, "Should have exactly 1 session");

    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
    let json: JsonValue = parse_json(&list_result.stdout)?;
    let count = json["sessions"].as_array().map(Vec::len).unwrap_or(0);
    assert_eq!(count, 0, "Should have 0 sessions after remove");
    Ok(())
}

// ============================================================================
// ROUND 3: Core Workflow Tests
// ============================================================================

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
    let json: JsonValue = parse_json(&query_result.stdout)?;
    assert_eq!(json["exists"], true);

    // 5. Remove session (cleanup)
    harness.assert_success(&["remove", "feature-1", "-f"]);

    // 6. Verify cleanup
    let query_result2 = harness.zjj(&["query", "session-exists", "feature-1", "--json"]);
    assert!(query_result2.success);
    let json2: JsonValue = parse_json(&query_result2.stdout)?;
    assert_eq!(json2["exists"], false);
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_context_command_for_agents() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "agent-session", "--no-open"]);

    let result = harness.zjj(&["context", "--json"]);
    assert!(result.success, "context command should succeed");

    let json: JsonValue = parse_json(&result.stdout)?;

    // Context should provide all the information an AI agent needs
    assert!(json.get("zjj").is_some(), "Context should have zjj section");
    assert!(json.get("jj").is_some(), "Context should have jj section");
    assert!(
        json.get("zellij").is_some(),
        "Context should have zellij section"
    );
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_command_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Test session-count query using functional composition
    let initial_count = || -> Result<usize, Box<dyn std::error::Error>> {
        let result = harness.zjj(&["query", "session-count", "--json"]);
        assert!(result.success);
        let json: JsonValue = parse_json(&result.stdout)?;
        json["count"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| "Invalid count value or overflow".into())
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
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
    let json: JsonValue = parse_json(&list_result.stdout)?;
    let count = json["sessions"].as_array().map(Vec::len).unwrap_or(0);
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_error_responses_have_consistent_structure() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Try to get a non-existent session
    let result = harness.zjj(&["status", "nonexistent", "--json"]);
    assert!(!result.success, "Should fail for non-existent session");

    let json: JsonValue = parse_json(&result.stdout)?;

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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_not_found_vs_validation_error_codes() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Test not found error (exit code 2)
    let result1 = harness.zjj(&["status", "nonexistent", "--json"]);
    let json1: JsonValue = parse_json(&result1.stdout)?;
    assert_eq!(
        json1["error"]["exit_code"], 2,
        "Not found should be exit code 2"
    );

    // Test validation error (exit code 1)
    let result2 = harness.zjj(&["add", "123invalid", "--no-open", "--json"]);
    let json2: JsonValue = parse_json(&result2.stdout)?;
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
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_session_locked_no_locks() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // Query a non-existent session - should not be locked
    let result = harness.zjj(&["query", "session-locked", "nonexistent", "--json"]);
    assert!(
        result.success,
        "Query should succeed even for non-existent session"
    );

    let json: JsonValue = parse_json(&result.stdout)?;
    assert_eq!(
        json["locked"], false,
        "Non-existent session should not be locked"
    );
    assert!(
        json["lock_info"].is_null(),
        "lock_info should be null when not locked"
    );
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_session_locked_with_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);

    // Query an existing session - should not be locked (no operations)
    let result = harness.zjj(&["query", "session-locked", "test-session", "--json"]);
    assert!(result.success);

    let json: JsonValue = parse_json(&result.stdout)?;
    assert_eq!(
        json["locked"], false,
        "Session should not be locked when idle"
    );

    // Cleanup
    harness.assert_success(&["remove", "test-session", "-f"]);
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_operations_in_progress_empty() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // No active operations initially
    let result = harness.zjj(&["query", "operations-in-progress", "--json"]);
    assert!(result.success);

    let json: JsonValue = parse_json(&result.stdout)?;
    assert_eq!(
        json["count"], 0,
        "Should have 0 active operations initially"
    );
    assert!(json["operations"]
        .as_array()
        .map(Vec::is_empty)
        .unwrap_or(true));
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_query_operations_in_progress_with_session() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "active-session", "--no-open"]);

    // Still no active operations just from creating a session
    let result = harness.zjj(&["query", "operations-in-progress", "--json"]);
    assert!(result.success);

    let json: JsonValue = parse_json(&result.stdout)?;
    assert_eq!(
        json["count"], 0,
        "Session creation doesn't create a persistent lock"
    );

    // Cleanup
    harness.assert_success(&["remove", "active-session", "-f"]);
    Ok(())
}

#[test]
#[ignore = "DRQ test bank - run with: cargo test --test drq_agent_arena -- --ignored"]
fn test_all_query_commands_use_schema_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        // Test framework will handle skipping - no output needed
        return Ok(());
    };

    harness.assert_success(&["init"]);

    // All query commands should output with SchemaEnvelope
    let queries = [
        "session-count",
        "session-exists test",
        "operations-in-progress",
        "session-locked test-session",
    ];

    // Use functional chain: execute queries, filter successful, validate each
    queries
        .iter()
        .map(|query| {
            let result = harness.zjj(&["query", query, "--json"]);
            let command_name = query.split_whitespace().next().unwrap_or(query);
            (query, result, command_name)
        })
        .filter(|(_, result, _)| result.success)
        .try_for_each(|(_, result, command_name)| {
            validate_schema_envelope(&result.stdout, command_name)
        })?;

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that JSON output uses `SchemaEnvelope` structure
///
/// Uses functional composition for better error handling and clarity.
/// Returns detailed validation errors instead of using assert! macros.
fn validate_schema_envelope(
    json_str: &str,
    command_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse JSON with context
    let json = parse_json(json_str).map_err(|e| format!("{command_name}: Invalid JSON: {e}"))?;

    // Define required fields with their error messages
    let required_fields: &[&str] = &["$schema", "_schema_version", "success"];

    // Validate all required fields exist using functional traversal
    required_fields.iter().try_for_each(|&field| {
        json.get(field)
            .ok_or_else(|| {
                ValidationError::MissingField(field, format!("{command_name}: output: {json_str}"))
            })
            .map(|_| ())
    })?;

    // Validate schema format using and_then for composition
    let schema = json["$schema"].as_str().ok_or_else(|| {
        ValidationError::InvalidFormat(format!("{command_name}: $schema field is not a string"))
    })?;

    // Validate schema URI pattern
    if !schema.starts_with("zjj://") {
        return Err(ValidationError::SchemaMismatch(format!(
            "{command_name}: $schema should start with 'zjj://', got: {schema}"
        ))
        .into());
    }

    Ok(())
}

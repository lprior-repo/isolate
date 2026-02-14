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
    clippy::collapsible_if,
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
//! Schema Conformance Tests (bd-2nv: cli-contracts)
//!
//! These tests verify that:
//! 1. Contract documentation schemas match the schema registry
//! 2. Runtime JSON outputs use the correct schema IDs
//! 3. All schema names follow consistent conventions
//!
//! This prevents drift between documentation and runtime behavior.

mod common;

use common::TestHarness;
use zjj_core::json::schemas;

/// Helper to extract schema from JSON output
fn extract_schema(json_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed: serde_json::Value = serde_json::from_str(json_str.trim())?;
    parsed
        .get("$schema")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "Missing $schema field".into())
}

/// Test that add command output schema matches contract
#[test]
fn test_add_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "schema-test-add", "--json", "--no-open"]);
    assert!(result.success, "add should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::ADD_RESPONSE);
    assert_eq!(
        schema, expected,
        "add response schema should match contract"
    );

    Ok(())
}

/// Test that done command output schema matches contract
#[test]
fn test_done_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-done", "--no-open"]);

    let result = harness.zjj(&["done", "--workspace", "schema-test-done", "--json"]);
    assert!(result.success, "done should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::DONE_RESPONSE);
    assert_eq!(
        schema, expected,
        "done response schema should match contract"
    );

    Ok(())
}

/// Test that context command output schema matches contract
#[test]
fn test_context_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["context", "--json"]);
    assert!(result.success, "context should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::CONTEXT_RESPONSE);
    assert_eq!(
        schema, expected,
        "context response schema should match contract"
    );

    Ok(())
}

/// Test that diff command output schema matches contract
#[test]
fn test_diff_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-diff", "--no-open"]);

    let result = harness.zjj(&["diff", "schema-test-diff", "--json"]);
    // diff may succeed or fail depending on state, but should output valid JSON
    if result.success || result.stdout.contains("zjj://") {
        if result.stdout.trim().starts_with('{') {
            let schema = extract_schema(result.stdout.trim())?;
            let expected = schemas::uri(schemas::DIFF_RESPONSE);
            assert_eq!(
                schema, expected,
                "diff response schema should match contract"
            );
        }
    }

    Ok(())
}

/// Test that diff --stat command output schema matches contract
#[test]
fn test_diff_stat_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-diff-stat", "--no-open"]);

    let result = harness.zjj(&["diff", "schema-test-diff-stat", "--stat", "--json"]);
    // diff may succeed or fail depending on state, but should output valid JSON
    if result.success || result.stdout.contains("zjj://") {
        if result.stdout.trim().starts_with('{') {
            let schema = extract_schema(result.stdout.trim())?;
            let expected = schemas::uri(schemas::DIFF_STAT_RESPONSE);
            assert_eq!(
                schema, expected,
                "diff --stat response schema should match contract"
            );
        }
    }

    Ok(())
}

/// Test that query session-exists output schema matches contract
#[test]
fn test_query_session_exists_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "session-exists", "nonexistent"]);
    assert!(result.success || result.stdout.contains("zjj://"));

    if result.stdout.trim().starts_with('{') {
        let schema = extract_schema(result.stdout.trim())?;
        let expected = schemas::uri(schemas::QUERY_SESSION_EXISTS);
        assert_eq!(
            schema, expected,
            "query session-exists response schema should match contract"
        );
    }

    Ok(())
}

/// Test that query can-run output schema matches contract
#[test]
fn test_query_can_run_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.zjj(&["query", "can-run", "add"]);
    assert!(result.success || result.stdout.contains("zjj://"));

    if result.stdout.trim().starts_with('{') {
        let schema = extract_schema(result.stdout.trim())?;
        let expected = schemas::uri(schemas::QUERY_CAN_RUN);
        assert_eq!(
            schema, expected,
            "query can-run response schema should match contract"
        );
    }

    Ok(())
}

/// Test that query location output schema matches contract
#[test]
fn test_query_location_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.zjj(&["query", "location"]);
    assert!(result.success || result.stdout.contains("zjj://"));

    if result.stdout.trim().starts_with('{') {
        let schema = extract_schema(result.stdout.trim())?;
        let expected = schemas::uri(schemas::QUERY_LOCATION);
        assert_eq!(
            schema, expected,
            "query location response schema should match contract"
        );
    }

    Ok(())
}

/// Test that list command output schema matches contract
#[test]
fn test_list_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success, "list should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::LIST_RESPONSE);
    assert_eq!(
        schema, expected,
        "list response schema should match contract"
    );

    Ok(())
}

/// Test that init command output schema matches contract
#[test]
fn test_init_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.zjj(&["init", "--json"]);
    assert!(result.success, "init should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::INIT_RESPONSE);
    assert_eq!(
        schema, expected,
        "init response schema should match contract"
    );

    Ok(())
}

/// Test that remove command output schema matches contract
#[test]
fn test_remove_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-remove", "--no-open"]);

    let result = harness.zjj(&["remove", "schema-test-remove", "--json", "--force"]);
    assert!(result.success, "remove should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::REMOVE_RESPONSE);
    assert_eq!(
        schema, expected,
        "remove response schema should match contract"
    );

    Ok(())
}

/// Test that contract command output schema matches contract
#[test]
fn test_contract_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };

    let result = harness.zjj(&["contract", "add", "--json"]);
    assert!(result.success, "contract should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::CONTRACT_RESPONSE);
    assert_eq!(
        schema, expected,
        "contract response schema should match contract"
    );

    Ok(())
}

/// Test that sync command output schema matches contract
#[test]
fn test_sync_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-sync", "--no-open"]);

    let result = harness.zjj(&["sync", "schema-test-sync", "--json"]);
    // sync may succeed or fail depending on state
    if result.success || result.stdout.contains("zjj://") {
        if result.stdout.trim().starts_with('{') {
            let schema = extract_schema(result.stdout.trim())?;
            let expected = schemas::uri(schemas::SYNC_RESPONSE);
            assert_eq!(
                schema, expected,
                "sync response schema should match contract"
            );
        }
    }

    Ok(())
}

/// Test that focus command output schema matches contract
#[test]
fn test_focus_schema_matches_contract() -> Result<(), Box<dyn std::error::Error>> {
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "schema-test-focus", "--no-open"]);

    let result = harness.zjj(&["focus", "schema-test-focus", "--json", "--no-zellij"]);
    assert!(result.success, "focus should succeed: {}", result.stderr);

    let schema = extract_schema(result.stdout.trim())?;
    let expected = schemas::uri(schemas::FOCUS_RESPONSE);
    assert_eq!(
        schema, expected,
        "focus response schema should match contract"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA REGISTRY COMPLETENESS TESTS
// ═══════════════════════════════════════════════════════════════════════════

/// Test that all schema registry constants are unique
#[test]
fn test_schema_registry_constants_are_unique() {
    let schemas_list = schemas::all_valid_schemas();
    let mut seen = std::collections::HashSet::new();
    for schema in schemas_list {
        assert!(
            seen.insert(schema),
            "Duplicate schema name in registry: {schema}"
        );
    }
}

/// Test that all schema names follow naming conventions
#[test]
fn test_schema_naming_conventions() {
    for schema in schemas::all_valid_schemas() {
        // Schema names should be lowercase
        assert!(
            schema == schema.to_lowercase(),
            "Schema '{schema}' should be lowercase"
        );
        // Schema names should use kebab-case (not underscores or spaces)
        assert!(
            !schema.contains(' '),
            "Schema '{schema}' should not contain spaces"
        );
        // Schema names should end with '-response' or start with 'query-'
        let is_valid = schema.ends_with("-response") || schema.starts_with("query-");
        assert!(
            is_valid,
            "Schema '{schema}' should end with '-response' or start with 'query-'"
        );
    }
}

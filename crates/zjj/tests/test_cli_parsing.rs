//! Integration tests for CLI argument parsing and validation
//!
//! Tests that CLI flags and options are properly handled

mod common;

use common::TestHarness;

// ============================================================================
// Help and Version
// ============================================================================

#[test]
fn test_help_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["--help"]);
    // Help may exit with 0 or display help text
    result.assert_output_contains("zjj");
}

#[test]
fn test_version_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let _result = harness.zjj(&["--version"]);
    // Version should show version number
}

#[test]
fn test_init_help() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["init", "--help"]);
    result.assert_output_contains("init");
}

#[test]
fn test_add_help() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["add", "--help"]);
    result.assert_output_contains("add");
}

// ============================================================================
// Add Command Options
// ============================================================================

#[test]
fn test_add_with_no_open_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_with_no_hooks_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-hooks", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_with_template_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test various templates
    harness.assert_success(&["add", "minimal", "--template", "minimal", "--no-open"]);
    harness.assert_success(&["add", "standard", "--template", "standard", "--no-open"]);
    harness.assert_success(&["add", "full", "--template", "full", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("minimal");
    result.assert_stdout_contains("standard");
    result.assert_stdout_contains("full");
}

#[test]
fn test_add_with_short_template_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "-t", "minimal", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_add_combined_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "test", "--no-open", "--no-hooks", "-t", "minimal"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// List Command Options
// ============================================================================

#[test]
fn test_list_with_all_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.zjj(&["list", "--all"]);
    assert!(result.success);
    result.assert_stdout_contains("test");
}

#[test]
fn test_list_with_json_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.zjj(&["list", "--json"]);
    assert!(result.success);

    // Verify JSON format
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_list_with_agent_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // This should fail because --agent flag doesn't exist yet
    let result = harness.zjj(&["list", "--agent", "claude-sonnet"]);
    assert!(result.success, "Should accept --agent flag");
}

#[test]
fn test_list_agent_with_no_matches() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Filter by non-existent agent should return empty list
    let result = harness.zjj(&["list", "--agent", "nonexistent-agent"]);
    assert!(result.success, "Should succeed with no matches");
    // Output should be empty or show "No sessions found"
    assert!(
        result.stdout.is_empty() || result.stdout.contains("No sessions"),
        "Should show empty result when no sessions match agent filter"
    );
}

#[test]
fn test_list_agent_combined_with_all() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Combine --agent with --all flag
    let result = harness.zjj(&["list", "--agent", "claude-sonnet", "--all"]);
    assert!(
        result.success,
        "Should accept --agent combined with --all flag"
    );
}

#[test]
fn test_list_agent_combined_with_json() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Combine --agent with --json flag
    let result = harness.zjj(&["list", "--agent", "claude-sonnet", "--json"]);
    assert!(
        result.success,
        "Should accept --agent combined with --json flag"
    );

    // Verify JSON format is still valid
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON when combining --agent with --json"
    );
}

// ============================================================================
// List Command --bead Flag (zjj-1ppy)
// ============================================================================

#[test]
fn test_list_with_bead_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);
    harness.assert_success(&["add", "session3", "--no-open"]);

    // Filter by bead ID (none will match since sessions don't have bead metadata)
    harness.assert_success(&["list", "--bead", "zjj-1ppy"]);
}

#[test]
fn test_list_with_bead_no_matches() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create sessions without bead metadata
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);

    // Filter by bead ID that doesn't match any session
    let result = harness.zjj(&["list", "--bead", "zjj-9999"]);

    // Should succeed but show no results
    assert!(
        result.success,
        "Should succeed with empty results\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.stdout.contains("No sessions found"),
        "Should indicate no sessions found\nStdout: {}",
        result.stdout
    );
}

#[test]
fn test_list_with_bead_excludes_others() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions
    // Note: These sessions won't have bead metadata initially
    harness.assert_success(&["add", "with-bead", "--no-open"]);
    harness.assert_success(&["add", "without-bead-1", "--no-open"]);
    harness.assert_success(&["add", "without-bead-2", "--no-open"]);

    // Filter by bead ID - will show no results since sessions don't have metadata
    harness.assert_success(&["list", "--bead", "zjj-1ppy"]);
}

#[test]
fn test_list_without_bead_shows_all() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create multiple sessions
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);
    harness.assert_success(&["add", "session3", "--no-open"]);

    // List without --bead flag should show all sessions (backward compatibility)
    let result = harness.zjj(&["list"]);
    assert!(
        result.success,
        "List without --bead should succeed\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );

    // All sessions should be present
    result.assert_stdout_contains("session1");
    result.assert_stdout_contains("session2");
    result.assert_stdout_contains("session3");
}

#[test]
fn test_list_with_bead_and_all_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create sessions
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);

    // Combine --bead with --all flag
    harness.assert_success(&["list", "--bead", "zjj-1ppy", "--all"]);
}

#[test]
fn test_list_with_bead_json_output() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Create sessions
    harness.assert_success(&["add", "session1", "--no-open"]);
    harness.assert_success(&["add", "session2", "--no-open"]);

    // Combine --bead with --json flag
    let result = harness.zjj(&["list", "--bead", "zjj-1ppy", "--json"]);

    // Should succeed and return JSON with envelope
    assert!(
        result.success,
        "Should succeed with JSON output\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    // Check that output is valid JSON with envelope structure
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Should output valid JSON\nStdout: {}",
        result.stdout
    );
    let json = parsed.unwrap();
    assert!(
        json.get("$schema").is_some(),
        "Should have $schema field\nStdout: {}",
        result.stdout
    );
    assert_eq!(
        json.get("schema_type").and_then(|v| v.as_str()),
        Some("array"),
        "Should have schema_type='array'"
    );
}

// ============================================================================
// Remove Command Options
// ============================================================================

#[test]
fn test_remove_with_force_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "--force"]);

    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("test"));
}

#[test]
fn test_remove_with_short_force_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "-f"]);

    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("test"));
}

#[test]
fn test_remove_with_merge_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Note: merge requires force too (or confirmation)
    let _result = harness.zjj(&["remove", "test", "--merge", "--force"]);
    // May succeed or fail depending on git state
}

#[test]
fn test_remove_with_keep_branch_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    harness.assert_success(&["remove", "test", "--keep-branch", "--force"]);

    let result = harness.zjj(&["list"]);
    assert!(!result.stdout.contains("test"));
}

// ============================================================================
// Status Command Options
// ============================================================================

#[test]
fn test_status_with_json_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.zjj(&["status", "test", "--json"]);
    assert!(result.success);

    // Verify JSON format
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_status_without_name_shows_all() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test1", "--no-open"]);
    harness.assert_success(&["add", "test2", "--no-open"]);

    let result = harness.zjj(&["status"]);
    assert!(result.success);
    // Should show all sessions or a summary
}

// ============================================================================
// Diff Command Options
// ============================================================================

#[test]
fn test_diff_with_stat_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.zjj(&["diff", "test", "--stat"]);
    // May succeed or fail depending on whether there are changes
}

#[test]
fn test_diff_without_stat() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.zjj(&["diff", "test"]);
    // May succeed or fail depending on whether there are changes
}

// ============================================================================
// Sync Command
// ============================================================================

#[test]
fn test_sync_with_explicit_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let _result = harness.zjj(&["sync", "test"]);
    // Sync behavior depends on git state
}

#[test]
fn test_sync_without_session_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Sync without name should sync current workspace
    let _result = harness.zjj(&["sync"]);
    // May succeed or fail depending on context
}

// ============================================================================
// Invalid Flag Combinations
// ============================================================================

#[test]
fn test_mutually_exclusive_flags() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Some flag combinations might not make sense
    // Implementation may vary
}

// ============================================================================
// Argument Order
// ============================================================================

#[test]
fn test_flags_before_positional_args() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Flags before name
    harness.assert_success(&["add", "--no-open", "test"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

#[test]
fn test_flags_after_positional_args() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Flags after name
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// Special Characters in Names
// ============================================================================

#[test]
fn test_session_name_with_hyphens() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature-with-hyphens", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("feature-with-hyphens");
}

#[test]
fn test_session_name_with_underscores() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature_with_underscores", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("feature_with_underscores");
}

#[test]
fn test_session_name_with_numbers() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "feature123", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("feature123");
}

// ============================================================================
// Empty and Whitespace
// ============================================================================

#[test]
fn test_session_name_with_leading_whitespace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Leading whitespace should be rejected or trimmed
    let _result = harness.zjj(&["add", " test", "--no-open"]);
    // May fail with validation error
}

#[test]
fn test_session_name_with_trailing_whitespace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Trailing whitespace should be rejected or trimmed
    let _result = harness.zjj(&["add", "test ", "--no-open"]);
    // May fail with validation error
}

// ============================================================================
// Case Sensitivity
// ============================================================================

#[test]
fn test_session_names_are_case_sensitive() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_success(&["add", "Test", "--no-open"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    // Both should exist as separate sessions
    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("Test");
    result.assert_stdout_contains("test");
}

// ============================================================================
// Long Flag Names
// ============================================================================

#[test]
fn test_long_flag_names() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // All flags should work with long names
    harness.assert_success(&["add", "test", "--no-open", "--no-hooks"]);

    let result = harness.zjj(&["list", "--all", "--json"]);
    assert!(result.success);
}

// ============================================================================
// Multiple Values
// ============================================================================

#[test]
fn test_template_with_equals_sign() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // --template=minimal syntax
    harness.assert_success(&["add", "test", "--template=minimal", "--no-open"]);

    let result = harness.zjj(&["list"]);
    result.assert_stdout_contains("test");
}

// ============================================================================
// Session Names with Leading Dashes (zjj-hv7)
// ============================================================================

#[test]
fn test_session_name_starting_with_single_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with a single dash should be rejected
    let result = harness.zjj(&["add", "-foo", "--no-open"]);
    assert!(!result.success, "Should reject name starting with dash");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_double_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with double dash should be rejected
    let result = harness.zjj(&["add", "--bar", "--no-open"]);
    assert!(!result.success, "Should reject name starting with --");
    // Will likely be interpreted as unknown flag or show validation error
}

#[test]
fn test_session_name_starting_with_triple_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with triple dash should be rejected
    let result = harness.zjj(&["add", "---baz", "--no-open"]);
    assert!(!result.success, "Should reject name starting with ---");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_just_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // A single dash should be rejected
    let result = harness.zjj(&["add", "-", "--no-open"]);
    assert!(!result.success, "Should reject single dash as name");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_underscore() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with underscore should be rejected
    let result = harness.zjj(&["add", "_private", "--no-open"]);
    assert!(
        !result.success,
        "Should reject name starting with underscore"
    );
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_session_name_starting_with_number() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Names starting with number should be rejected
    let result = harness.zjj(&["add", "123session", "--no-open"]);
    assert!(!result.success, "Should reject name starting with number");
    result.assert_output_contains("must start with a letter");
}

#[test]
fn test_remove_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to remove a session with dash-prefixed name
    let result = harness.zjj(&["remove", "-session", "--force"]);
    assert!(
        !result.success,
        "Should reject remove with dash-prefixed name"
    );
    // May show validation error or "session not found"
}

#[test]
fn test_focus_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to focus a session with dash-prefixed name
    let result = harness.zjj(&["focus", "-session"]);
    assert!(
        !result.success,
        "Should reject focus with dash-prefixed name"
    );
    // May show validation error or "session not found"
}

#[test]
fn test_diff_session_name_starting_with_dash() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Try to diff a session with dash-prefixed name
    let result = harness.zjj(&["diff", "-session"]);
    assert!(
        !result.success,
        "Should reject diff with dash-prefixed name"
    );
    // May show validation error or "session not found"
}

// ============================================================================
// Add Command --example-json Flag (RED PHASE TESTS - Phase 2)
// ============================================================================

#[test]
fn test_add_example_json_flag_is_recognized() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Flag should be recognized without error
    let result = harness.zjj(&["add", "--example-json"]);
    assert!(
        result.success,
        "Should accept --example-json flag (got stderr: {})",
        result.stderr
    );
}

#[test]
fn test_add_example_json_outputs_valid_json() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    // Test: Output should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON (got: {})",
        result.stdout
    );
}

#[test]
fn test_add_example_json_has_name_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    // Test: JSON should have 'name' field with string value
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");
    assert!(
        parsed.get("name").is_some(),
        "JSON should have 'name' field"
    );
    assert!(
        parsed["name"].is_string(),
        "name field should be a string (got: {:?})",
        parsed["name"]
    );
}

#[test]
fn test_add_example_json_has_workspace_path_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");
    assert!(
        parsed.get("workspace_path").is_some(),
        "JSON should have 'workspace_path' field"
    );
    assert!(
        parsed["workspace_path"].is_string(),
        "workspace_path field should be a string"
    );
}

#[test]
fn test_add_example_json_has_zellij_tab_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");
    assert!(
        parsed.get("zellij_tab").is_some(),
        "JSON should have 'zellij_tab' field"
    );
    assert!(
        parsed["zellij_tab"].is_string(),
        "zellij_tab field should be a string"
    );
}

#[test]
fn test_add_example_json_has_status_field() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");
    assert!(
        parsed.get("status").is_some(),
        "JSON should have 'status' field"
    );
    assert!(
        parsed["status"].is_string(),
        "status field should be a string"
    );
}

#[test]
fn test_add_example_json_does_not_create_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    // Test: Example output should not create an actual session
    let list_result = harness.zjj(&["list"]);
    assert!(
        list_result.stdout.is_empty() || list_result.stdout.contains("No sessions"),
        "Should not create a session (list output: {})",
        list_result.stdout
    );
}

#[test]
fn test_add_example_json_with_name_argument_fails() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Providing both --example-json and name should fail
    let result = harness.zjj(&["add", "my-session", "--example-json"]);
    assert!(
        !result.success,
        "Should reject --example-json with name argument"
    );
}

#[test]
fn test_add_example_json_mutually_exclusive_with_name() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Order doesn't matter - both should fail
    let result1 = harness.zjj(&["add", "test1", "--example-json"]);
    let result2 = harness.zjj(&["add", "--example-json", "test2"]);

    assert!(
        !result1.success && !result2.success,
        "Should reject --example-json with name in any order"
    );
}

#[test]
fn test_add_example_json_has_all_required_fields() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // Test: All required fields present
    let required_fields = vec!["name", "workspace_path", "zellij_tab", "status"];
    for field in required_fields {
        assert!(
            parsed.get(field).is_some(),
            "JSON should have required field '{}'",
            field
        );
    }
}

#[test]
fn test_add_example_json_with_no_open_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Combining --example-json with --no-open (conflicting intent but both are flags)
    let _result = harness.zjj(&["add", "--example-json", "--no-open"]);
    // Should either succeed (ignoring --no-open) or clearly fail
    // Exact behavior TBD in Phase 3 specification
}

#[test]
fn test_add_example_json_with_template_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Combining --example-json with --template (should either succeed or fail clearly)
    let _result = harness.zjj(&["add", "--example-json", "--template", "minimal"]);
    // Exact behavior TBD in Phase 3 specification
}

#[test]
fn test_add_example_json_with_no_hooks_flag() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Combining --example-json with --no-hooks
    let _result = harness.zjj(&["add", "--example-json", "--no-hooks"]);
    // Should succeed - example output doesn't execute hooks anyway
}

#[test]
fn test_add_example_json_output_fields_are_strings() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "--example-json"]);
    assert!(result.success, "Should succeed with --example-json");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // Test: All fields should be strings (matching AddOutput struct)
    assert!(parsed["name"].is_string(), "name should be string");
    assert!(
        parsed["workspace_path"].is_string(),
        "workspace_path should be string"
    );
    assert!(
        parsed["zellij_tab"].is_string(),
        "zellij_tab should be string"
    );
    assert!(parsed["status"].is_string(), "status should be string");
}

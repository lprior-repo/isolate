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
//! Integration tests for CLI flag consistency across all commands
//!
//! Tests that newly added flags (--json, --dry-run, --contract, --ai-hints, etc.)
//! are properly handled by all commands that should support them.
//!
//! # Test Coverage
//!
//! Phase 1: Quick wins (--json, --idempotent)
//! Phase 2: Mutating commands (--dry-run)
//! Phase 3: AI integration (--contract, --ai-hints)

mod common;

use common::TestHarness;

// ============================================================================
// Phase 1: Spawn --idempotent flag
// ============================================================================

#[test]
fn test_spawn_accepts_idempotent_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: spawn should accept --idempotent flag
    // Will fail because we don't have beads set up, but flag should be recognized
    let result = harness.isolate(&[
        "spawn",
        "isolate-test123",
        "--idempotent",
        "--no-auto-merge",
    ]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --idempotent flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_spawn_help_shows_idempotent_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["spawn", "--help"]);
    result.assert_output_contains("--idempotent");
}

// ============================================================================
// Phase 2: --dry-run on remove command
// ============================================================================

#[test]
fn test_remove_accepts_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: remove should accept --dry-run flag
    let result = harness.isolate(&["remove", "test", "--dry-run"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --dry-run flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_remove_dry_run_does_not_delete_session() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: --dry-run should not actually remove the session
    let _result = harness.isolate(&["remove", "test", "--dry-run"]);

    // Session should still exist
    let list_result = harness.isolate(&["list"]);
    list_result.assert_stdout_contains("test");
}

#[test]
fn test_remove_dry_run_json_preview() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    let result = harness.isolate(&["remove", "test", "--dry-run", "--json"]);
    assert!(
        !result.stdout.is_empty(),
        "Expected JSON output, stdout was empty"
    );
    assert!(
        result.stdout.contains("DRY-RUN"),
        "Expected DRY-RUN preview in JSON output: {}",
        result.stdout
    );
    // JSONL format uses variant names like "result" and "action" as top-level keys
    assert!(
        result.stdout.contains("\"result\":") || result.stdout.contains("\"action\":"),
        "Expected 'result' or 'action' variant in JSONL output: {}",
        result.stdout
    );
}

#[test]
fn test_remove_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["remove", "--help"]);
    result.assert_output_contains("--dry-run");
}

// ============================================================================
// Phase 2: --dry-run on spawn command
// ============================================================================

#[test]
fn test_spawn_accepts_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: spawn should accept --dry-run flag
    let result = harness.isolate(&["spawn", "isolate-test123", "--dry-run"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --dry-run flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_spawn_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["spawn", "--help"]);
    result.assert_output_contains("--dry-run");
}

// ============================================================================
// Phase 2: --dry-run on sync command
// ============================================================================

#[test]
fn test_sync_accepts_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: sync should accept --dry-run flag
    let result = harness.isolate(&["sync", "test", "--dry-run"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --dry-run flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_sync_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["sync", "--help"]);
    result.assert_output_contains("--dry-run");
}

// ============================================================================
// Phase 2: --dry-run on batch command
// ============================================================================

#[test]
fn test_batch_accepts_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: batch should accept --dry-run flag
    let result = harness.isolate(&["batch", "--dry-run"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --dry-run flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_batch_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["batch", "--help"]);
    result.assert_output_contains("--dry-run");
}

// ============================================================================
// Phase 2: --dry-run on init command
// ============================================================================

#[test]
fn test_init_accepts_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Test: init should accept --dry-run flag
    let result = harness.isolate(&["init", "--dry-run"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --dry-run flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_init_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["init", "--help"]);
    result.assert_output_contains("--dry-run");
}

// ============================================================================
// Phase 3: whereami --contract and --ai-hints flags
// ============================================================================

#[test]
fn test_whereami_accepts_contract_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: whereami should accept --contract flag
    let result = harness.isolate(&["whereami", "--contract"]);
    assert!(
        result.success,
        "Should accept --contract flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_whereami_accepts_ai_hints_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: whereami should accept --ai-hints flag
    let result = harness.isolate(&["whereami", "--ai-hints"]);
    assert!(
        result.success,
        "Should accept --ai-hints flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_whereami_help_shows_contract_and_ai_hints() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["whereami", "--help"]);
    result.assert_output_contains("--contract");
    result.assert_output_contains("--ai-hints");
}

// ============================================================================
// Phase 3: whoami --contract and --ai-hints flags
// ============================================================================

#[test]
fn test_whoami_accepts_contract_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: whoami should accept --contract flag
    let result = harness.isolate(&["whoami", "--contract"]);
    assert!(
        result.success,
        "Should accept --contract flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_whoami_accepts_ai_hints_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: whoami should accept --ai-hints flag
    let result = harness.isolate(&["whoami", "--ai-hints"]);
    assert!(
        result.success,
        "Should accept --ai-hints flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_whoami_help_shows_contract_and_ai_hints() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["whoami", "--help"]);
    result.assert_output_contains("--contract");
    result.assert_output_contains("--ai-hints");
}

// ============================================================================
// Phase 3: list --contract and --ai-hints flags
// ============================================================================

#[test]
fn test_list_accepts_contract_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: list should accept --contract flag
    let result = harness.isolate(&["list", "--contract"]);
    assert!(
        result.success,
        "Should accept --contract flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_list_accepts_ai_hints_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: list should accept --ai-hints flag
    let result = harness.isolate(&["list", "--ai-hints"]);
    assert!(
        result.success,
        "Should accept --ai-hints flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_list_help_shows_contract_and_ai_hints() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["list", "--help"]);
    result.assert_output_contains("--contract");
    result.assert_output_contains("--ai-hints");
}

// ============================================================================
// Combined flag tests - Martin Fowler style: test combinations
// ============================================================================

#[test]
fn test_list_contract_and_json_combined() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: Should be able to combine --contract and --json
    let result = harness.isolate(&["list", "--contract", "--json"]);
    assert!(
        result.success,
        "Should accept both --contract and --json (stderr: {})",
        result.stderr
    );

    // When --contract is used, output is a single JSON object (pretty-printed),
    // not JSONL. Parse the entire output as a single JSON object.
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON (contract format): {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // Verify it's a contract object with expected fields
    let json = parsed.expect("already checked is_ok");
    assert!(
        json.get("command").is_some() || json.get("intent").is_some(),
        "Contract should have 'command' or 'intent' field: {}",
        result.stdout
    );
}

#[test]
fn test_list_json_alone_is_jsonl() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: --json alone should produce JSONL output
    let result = harness.isolate(&["list", "--json"]);
    assert!(
        result.success,
        "Should accept --json (stderr: {})",
        result.stderr
    );

    // Verify output is valid JSONL (each line is a valid JSON object)
    let lines: Vec<&str> = result
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(
        !lines.is_empty(),
        "Output should contain at least one JSONL line: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );

    // Verify each line is valid JSON
    for (idx, line) in lines.iter().enumerate() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "Line {} should be valid JSON: {}\nFull output: {}\nstderr: {}",
            idx,
            line,
            result.stdout,
            result.stderr
        );
    }
}

#[test]
fn test_remove_dry_run_and_force_combined() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-hooks"]);

    // Test: Should be able to combine --dry-run and --force
    let result = harness.isolate(&["remove", "test", "--dry-run", "--force"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should accept both --dry-run and --force (stderr: {})",
        result.stderr
    );

    // Session should still exist (dry-run)
    let list_result = harness.isolate(&["list"]);
    list_result.assert_stdout_contains("test");
}

#[test]
fn test_whereami_all_flags_combined() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Should accept --json, --contract, and --ai-hints together
    let result = harness.isolate(&["whereami", "--json", "--contract", "--ai-hints"]);
    assert!(
        result.success,
        "Should accept all flags combined (stderr: {})",
        result.stderr
    );
}

// ============================================================================
// Regression tests: ensure existing flags still work
// ============================================================================

#[test]
fn test_existing_flags_not_broken_by_new_flags() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Existing functionality should still work
    harness.assert_success(&["add", "test1", "--no-hooks"]);
    harness.assert_success(&["add", "test2", "--no-open"]);
    harness.assert_success(&["list", "--all", "--json"]);
    harness.assert_success(&["remove", "test1", "--force"]);
}

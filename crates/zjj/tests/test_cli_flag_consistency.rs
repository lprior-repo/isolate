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
//! Phase 1: Quick wins (--json, --idempotent, --no-zellij)
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
    let result = harness.zjj(&["spawn", "zjj-test123", "--idempotent", "--no-auto-merge"]);
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

    let result = harness.zjj(&["spawn", "--help"]);
    result.assert_output_contains("--idempotent");
}

// ============================================================================
// Phase 1: Switch --no-zellij flag
// ============================================================================

#[test]
fn test_switch_accepts_no_zellij_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-zellij", "--no-hooks"]);

    // Test: switch should accept --no-zellij flag
    let result = harness.zjj(&["switch", "test-session", "--no-zellij"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should recognize --no-zellij flag (stderr: {})",
        result.stderr
    );
}

#[test]
fn test_switch_help_shows_no_zellij_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["switch", "--help"]);
    result.assert_output_contains("--no-zellij");
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
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    // Test: remove should accept --dry-run flag
    let result = harness.zjj(&["remove", "test", "--dry-run"]);
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
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    // Test: --dry-run should not actually remove the session
    let _result = harness.zjj(&["remove", "test", "--dry-run"]);

    // Session should still exist
    let list_result = harness.zjj(&["list"]);
    list_result.assert_stdout_contains("test");
}

#[test]
fn test_remove_dry_run_json_preview() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&["remove", "test", "--dry-run", "--json"]);
    assert!(
        !result.stdout.is_empty(),
        "Expected JSON output, stdout was empty"
    );
    assert!(
        result.stdout.contains("DRY-RUN"),
        "Expected DRY-RUN preview in JSON output: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("remove-response"),
        "Expected remove-response schema in JSON output: {}",
        result.stdout
    );
}

#[test]
fn test_remove_help_shows_dry_run_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["remove", "--help"]);
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
    let result = harness.zjj(&["spawn", "zjj-test123", "--dry-run"]);
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

    let result = harness.zjj(&["spawn", "--help"]);
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
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    // Test: sync should accept --dry-run flag
    let result = harness.zjj(&["sync", "test", "--dry-run"]);
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

    let result = harness.zjj(&["sync", "--help"]);
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
    let result = harness.zjj(&["batch", "--dry-run"]);
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

    let result = harness.zjj(&["batch", "--help"]);
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
    let result = harness.zjj(&["init", "--dry-run"]);
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

    let result = harness.zjj(&["init", "--help"]);
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
    let result = harness.zjj(&["whereami", "--contract"]);
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
    let result = harness.zjj(&["whereami", "--ai-hints"]);
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

    let result = harness.zjj(&["whereami", "--help"]);
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
    let result = harness.zjj(&["whoami", "--contract"]);
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
    let result = harness.zjj(&["whoami", "--ai-hints"]);
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

    let result = harness.zjj(&["whoami", "--help"]);
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
    let result = harness.zjj(&["list", "--contract"]);
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
    let result = harness.zjj(&["list", "--ai-hints"]);
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

    let result = harness.zjj(&["list", "--help"]);
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
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    // Test: Should be able to combine --contract and --json
    let result = harness.zjj(&["list", "--contract", "--json"]);
    assert!(
        result.success,
        "Should accept both --contract and --json (stderr: {})",
        result.stderr
    );

    // Verify output is valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON: {}\nstderr: {}",
        result.stdout,
        result.stderr
    );
}

#[test]
fn test_remove_dry_run_and_force_combined() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-zellij", "--no-hooks"]);

    // Test: Should be able to combine --dry-run and --force
    let result = harness.zjj(&["remove", "test", "--dry-run", "--force"]);
    assert!(
        !result.stderr.contains("unexpected argument"),
        "Should accept both --dry-run and --force (stderr: {})",
        result.stderr
    );

    // Session should still exist (dry-run)
    let list_result = harness.zjj(&["list"]);
    list_result.assert_stdout_contains("test");
}

#[test]
fn test_whereami_all_flags_combined() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Test: Should accept --json, --contract, and --ai-hints together
    let result = harness.zjj(&["whereami", "--json", "--contract", "--ai-hints"]);
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
    harness.assert_success(&["add", "test1", "--no-zellij", "--no-hooks"]);
    harness.assert_success(&["add", "test2", "--no-open"]);
    harness.assert_success(&["list", "--all", "--json"]);
    harness.assert_success(&["remove", "test1", "--force"]);
}

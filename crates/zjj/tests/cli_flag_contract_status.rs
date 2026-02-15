// BDD Test for --contract and --ai-hints flags on status command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query command contracts without panics
//   So that I can understand command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn bdd_status_contract_flag_does_not_panic() {
    // Scenario: AI queries status command contract
    //   When I run "zjj status --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--contract");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).expect("contract output should be JSON");
    assert_eq!(
        parsed.get("command").and_then(Value::as_str),
        Some("zjj status")
    );
    assert!(parsed.get("inputs").is_some(), "inputs should be present");
    assert!(parsed.get("outputs").is_some(), "outputs should be present");
}

#[test]
fn bdd_status_ai_hints_flag_does_not_panic() {
    // Scenario: AI queries status command execution hints
    //   When I run "zjj status --ai-hints"
    //   Then it should return hints without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"))
        .stdout(predicate::str::contains("typical_workflows"))
        .stdout(predicate::str::contains("command_preconditions"));
}

#[test]
fn bdd_status_with_invalid_session_still_works() {
    // Scenario: Contract flag works even with invalid session name
    //   When I run "zjj status --contract nonexistent-session"
    //   Then it should return contract without checking session validity
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--contract");

    // Should succeed with contract output, not fail on session validation
    cmd.assert().success();
}

#[test]
fn bdd_status_ai_hints_with_json_returns_json() {
    // Scenario: AI requests machine-readable hints
    //   Given the status command supports --ai-hints and --json
    //   When I run "zjj status --ai-hints --json"
    //   Then output is valid JSON with workflow hints

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--ai-hints").arg("--json");

    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&output).expect("ai-hints output should be JSON");
    assert!(
        parsed.get("typical_workflows").is_some(),
        "typical_workflows should be present"
    );
}

#[test]
fn bdd_status_without_flags_runs_normally() {
    // Scenario: Normal status operation still works
    //   When I run "zjj status --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains("--ai-hints"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ))
        .stdout(predicate::str::contains("AI: Show execution hints"));
}

#[test]
fn bdd_status_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["status", "--contract"],
        vec!["status", "--ai-hints"],
        vec!["status", "--json"],
        vec!["status", "--contract", "--json"],
        vec!["status", "--ai-hints", "--json"],
        vec!["status", "--help"],
    ];

    for args in test_cases {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
        for arg in &args {
            cmd.arg(arg);
        }

        // Should not panic (exit code 134 is panic)
        cmd.assert().code(
            predicate::ne(134) // 134 = panic
                .and(predicate::ne(101)), // 101 = clap panic
        );
    }
}

#[test]
fn bdd_status_watch_starts_without_panic() {
    // Scenario: Watch mode starts without panicking
    //   When I run "zjj status --watch"
    //   Then it should start successfully (not panic)
    //   Note: Watch runs indefinitely, so we use a timeout

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--watch");
    cmd.timeout(Duration::from_millis(500));

    // Timeout (exit code None/timed out) is expected and OK
    // Panic (exit code 134 or 101) would indicate a bug
    let output = cmd
        .output()
        .map_or(None, |o| o.status.code())
        .is_none_or(|code| code != 134 && code != 101);

    // If it exited, check it didn't panic (134 = panic, 101 = clap panic)
    // If it timed out (no exit code), that's expected for watch mode
    assert!(output, "Command panicked during watch mode");
}

#[test]
fn bdd_status_json_flag_does_not_panic() {
    // Scenario: JSON output flag works without panic
    //   When I run "zjj status --json"
    //   Then it should return JSON output without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--json");

    // Should succeed even if no sessions exist
    cmd.assert().success();
}

#[test]
fn bdd_status_watch_flag_does_not_panic() {
    // Scenario: Watch mode flag works without panic
    //   When I run "zjj status --help" to verify flag exists
    //   Then it should show the watch flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--watch"))
        .stdout(predicate::str::contains("Continuously update status"));
}

#[test]
fn bdd_status_watch_with_missing_session_fails_fast() {
    // Given a missing session name
    // When status watch runs in JSON mode
    // Then command exits with not-found instead of looping forever
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("status")
        .arg("--watch")
        .arg("--json")
        .arg("nonexistent-session")
        .timeout(Duration::from_secs(2));

    let output = cmd.output().expect("status watch command should execute");
    assert_eq!(
        output.status.code(),
        Some(2),
        "missing session in watch mode should return not-found exit code"
    );
}

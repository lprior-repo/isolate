// BDD Test for --contract and --ai-hints flags on ai command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query command contracts without panics
//   So that I can understand command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_ai_contract_flag_does_not_panic() {
    // Scenario: AI queries ai command contract
    //   When I run "zjj ai --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj ai"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_ai_ai_hints_flag_does_not_panic() {
    // Scenario: AI queries ai command execution hints
    //   When I run "zjj ai --ai-hints"
    //   Then it should return hints without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"))
        .stdout(predicate::str::contains("typical_workflows"))
        .stdout(predicate::str::contains("command_preconditions"));
}

#[test]
fn bdd_ai_without_flags_runs_normally() {
    // Scenario: Normal ai operation still works
    //   When I run "zjj ai --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("--help");

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
fn bdd_ai_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["ai", "--contract"],
        vec!["ai", "--ai-hints"],
        vec!["ai", "--json"],
        vec!["ai", "--contract", "--json"],
        vec!["ai", "--ai-hints", "--json"],
        vec!["ai", "--help"],
        vec!["ai", "status", "--contract"],
        vec!["ai", "status", "--ai-hints"],
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
fn bdd_ai_contract_works_with_subcommands() {
    // Scenario: Contract flag works with ai subcommands
    //   When I run "zjj ai status --contract"
    //   Then it should return contract without checking session validity
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("status").arg("--contract");

    // Should succeed with contract output, not fail on session validation
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT"));
}

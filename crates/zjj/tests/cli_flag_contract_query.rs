// BDD Test for --contract and --ai-hints flags on query command
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
fn bdd_query_contract_flag_does_not_panic() {
    // Scenario: AI queries query command contract
    //   When I run "zjj query --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("query").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj query"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_query_ai_hints_flag_does_not_panic() {
    // Scenario: AI queries query command execution hints
    //   When I run "zjj query --ai-hints"
    //   Then it should return hints without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("query").arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"))
        .stdout(predicate::str::contains("typical_workflows"))
        .stdout(predicate::str::contains("command_preconditions"));
}

#[test]
fn bdd_query_without_flags_runs_normally() {
    // Scenario: Normal query operation still works
    //   When I run "zjj query --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("query").arg("--help");

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
fn bdd_query_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["query", "--contract"],
        vec!["query", "--ai-hints"],
        vec!["query", "--json"],
        vec!["query", "--contract", "--json"],
        vec!["query", "--ai-hints", "--json"],
        vec!["query", "--help"],
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

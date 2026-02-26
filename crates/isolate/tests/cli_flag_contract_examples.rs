// BDD Test for --contract flag on examples command
//
// Feature: Examples Contract Integration
//   As an AI agent
//   I want to query the examples command contract without panics
//   So that I can understand the command schema autonomously
//
// Background:
//   Given isolate is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_examples_contract_flag_does_not_panic() {
    // Scenario: AI queries examples command contract
    //   When I run "isolate examples --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("examples").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for isolate examples"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_examples_without_flags_runs_normally() {
    // Scenario: Normal examples operation still works
    //   When I run "isolate examples --help"
    //   Then it should show help including new --contract flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("examples").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_examples_contract_returns_valid_structure() {
    // Scenario: Contract output contains expected structure
    //   When I run "isolate examples --contract"
    //   Then output should contain examples-specific fields
    //   And output should document the command/use-case filters

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("examples").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("isolate examples"))
        .stdout(predicate::str::contains("use_case"))
        .stdout(predicate::str::contains("examples"));
}

#[test]
fn bdd_examples_no_panic_with_any_flag_combination() {
    // Scenario: Multiple flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["examples", "--contract"],
        vec!["examples", "--json"],
        vec!["examples", "--contract", "--json"],
        vec!["examples", "--help"],
        vec!["examples", "add", "--contract"],
        vec!["examples", "--use-case", "workflow", "--contract"],
    ];

    for args in test_cases {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
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
fn bdd_examples_contract_shows_examples_field() {
    // Scenario: Contract documents available use cases
    //   When I run "isolate examples --contract"
    //   Then output should document use_case options

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("examples").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("use_case"));
}

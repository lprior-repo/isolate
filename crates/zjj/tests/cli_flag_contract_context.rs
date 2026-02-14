// BDD Test for --contract flag on context command
//
// Feature: Context Contract Integration
//   As an AI agent
//   I want to query context command contracts without panics
//   So that I can understand context command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_context_contract_flag_does_not_panic() {
    // Scenario: AI queries context command contract
    //   When I run "zjj context --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("context").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj context"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_context_without_flags_runs_normally() {
    // Scenario: Normal context operation still works
    //   When I run "zjj context --help"
    //   Then it should show help including new flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("context").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains("Show machine-readable contract"));
}

#[test]
fn bdd_context_no_panic_with_any_flag_combination() {
    // Scenario: Multiple context flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["context", "--contract"],
        vec!["context", "--json"],
        vec!["context", "--contract", "--json"],
        vec!["context", "--no-beads"],
        vec!["context", "--no-health"],
        vec!["context", "--help"],
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
fn bdd_context_contract_returns_valid_json_structure() {
    // Scenario: Contract output has valid structure
    //   When I run "zjj context --contract"
    //   Then the output should contain expected sections

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("context").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("location"))
        .stdout(predicate::str::contains("session"))
        .stdout(predicate::str::contains("repository"))
        .stdout(predicate::str::contains("beads"))
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("suggestions"));
}

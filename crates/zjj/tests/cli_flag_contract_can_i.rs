// BDD Test for --contract flag on can-i command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query the can-i command contract without panics
//   So that I can understand command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_can_i_contract_flag_does_not_panic() {
    // Scenario: AI queries can-i command contract
    //   When I run "zjj can-i --contract add"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("can-i").arg("--contract").arg("add");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj can-i"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_can_i_contract_with_different_actions() {
    // Scenario: Contract flag works for different actions
    //   When I run "zjj can-i --contract done"
    //   Then it should return contract without checking action validity
    //   And exit code should be 0

    let actions = vec!["add", "remove", "done", "undo", "sync", "spawn", "claim", "merge"];

    for action in actions {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
        cmd.arg("can-i").arg("--contract").arg(action);

        // Should succeed with contract output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("AI CONTRACT"));
    }
}

#[test]
fn bdd_can_i_without_contract_runs_normally() {
    // Scenario: Normal can-i operation still works
    //   When I run "zjj can-i --help"
    //   Then it should show help including new flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("can-i").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_no_panic_with_any_flag_combination() {
    // Scenario: Multiple flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["can-i", "--contract", "add"],
        vec!["can-i", "--contract", "done"],
        vec!["can-i", "--json", "add"],
        vec!["can-i", "--contract", "--json", "add"],
        vec!["can-i", "--help"],
    ];

    for args in test_cases {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
        for arg in args {
            cmd.arg(arg);
        }

        // Should not panic (exit code 134 is panic)
        cmd.assert().code(
            predicate::ne(134) // 134 = panic
                .and(predicate::ne(101)), // 101 = clap panic
        );
    }
}

// BDD Test for --contract flag on remove command
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
fn bdd_remove_contract_flag_does_not_panic() {
    // Scenario: AI queries remove command contract
    //   When I run "zjj remove --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("remove").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj remove"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_remove_contract_flag_works_without_session_name() {
    // Scenario: --contract flag allows remove without requiring session name
    //   When I run "zjj remove --contract" (no session name)
    //   Then it should return contract without validation error
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("remove").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT"));
}

#[test]
fn bdd_remove_without_flags_requires_session_name() {
    // Scenario: Normal remove operation still requires session name
    //   When I run "zjj remove" (no session name, no --contract)
    //   Then it should fail with validation error
    //   Because session name is required

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("remove");

    cmd.assert().failure();
}

#[test]
fn bdd_remove_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["remove", "--contract"],
        vec!["remove", "--contract", "--json"],
        vec!["remove", "--help"],
        vec!["remove", "--contract", "test-session"], // contract ignores name
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

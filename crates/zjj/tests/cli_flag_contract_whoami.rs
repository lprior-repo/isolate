// BDD Test for --contract flag on whoami command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query the whoami command contract without panics
//   So that I can understand command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_whoami_contract_flag_does_not_panic() {
    // Scenario: AI queries whoami command contract
    //   When I run "zjj whoami --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whoami").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj whoami"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_whoami_contract_flag_returns_valid_json_structure() {
    // Scenario: Contract flag returns structured JSON contract
    //   When I run "zjj whoami --contract"
    //   Then it should return contract with expected structure
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whoami").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"zjj whoami\""))
        .stdout(predicate::str::contains("\"intent\":"))
        .stdout(predicate::str::contains("\"prerequisites\":"))
        .stdout(predicate::str::contains("\"side_effects\":"))
        .stdout(predicate::str::contains("\"inputs\":"))
        .stdout(predicate::str::contains("\"outputs\":"))
        .stdout(predicate::str::contains("\"examples\":"));
}

#[test]
fn bdd_whoami_without_contract_runs_normally() {
    // Scenario: Normal whoami operation still works
    //   When I run "zjj whoami --help"
    //   Then it should show help including new flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whoami").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_whoami_no_panic_with_any_flag_combination() {
    // Scenario: Multiple flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["whoami", "--contract"],
        vec!["whoami", "--json"],
        vec!["whoami", "--contract", "--json"],
        vec!["whoami", "--help"],
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

// BDD Test for --contract and --ai-hints flags on whatif command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query command contracts without panics
//   So that I can understand command schemas autonomously
//
// Background:
//   Given isolate is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_whatif_contract_flag_does_not_panic() {
    // Scenario: AI queries whatif command contract
    //   When I run "isolate whatif --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("whatif").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for isolate whatif"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_whatif_contract_flag_shows_json_structure() {
    // Scenario: Contract output contains expected JSON structure
    //   When I run "isolate whatif --contract"
    //   Then it should return JSON with expected fields

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("whatif").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"isolate whatif\""))
        .stdout(predicate::str::contains("side_effects"))
        .stdout(predicate::str::contains("examples"));
}

#[test]
fn bdd_whatif_without_flags_runs_normally() {
    // Scenario: Normal whatif operation still works
    //   When I run "isolate whatif --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("whatif").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_whatif_no_panic_with_flag_combinations() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases: Vec<Vec<&str>> = vec![
        vec!["whatif", "--contract"],
        vec!["whatif", "--json"],
        vec!["whatif", "--help"],
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

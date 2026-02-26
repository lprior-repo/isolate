// BDD Test for --contract flag on introspect command
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
fn bdd_introspect_contract_flag_does_not_panic() {
    // Scenario: AI queries introspect command contract
    //   When I run "isolate introspect --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("introspect").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for isolate introspect"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_introspect_contract_includes_flags() {
    // Scenario: Contract documents available flags
    //   When I run "isolate introspect --contract"
    //   Then it should include documentation for all flags
    //   And document the --env-vars flag
    //   And document the --workflows flag
    //   And document the --session-states flag

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("introspect").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("env-vars"))
        .stdout(predicate::str::contains("workflows"))
        .stdout(predicate::str::contains("session-states"));
}

#[test]
fn bdd_introspect_without_flags_runs_normally() {
    // Scenario: Normal introspect operation still works
    //   When I run "isolate introspect --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_isolate"));
    cmd.arg("introspect").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_introspect_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["introspect", "--contract"],
        vec!["introspect", "--json"],
        vec!["introspect", "--contract", "--json"],
        vec!["introspect", "--help"],
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

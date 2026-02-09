// BDD Test for --contract and --ai-hints flags on spawn command
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
fn bdd_spawn_contract_flag_does_not_panic() {
    // Scenario: AI queries spawn command contract
    //   When I run "zjj spawn --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("spawn")
        .arg("--contract")
        .arg("zjj-test");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj spawn"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("prerequisites"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_spawn_ai_hints_flag_does_not_panic() {
    // Scenario: AI queries spawn command execution hints
    //   When I run "zjj spawn --ai-hints"
    //   Then it should return hints without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("spawn")
        .arg("--ai-hints")
        .arg("zjj-test");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI HINTS for zjj spawn"))
        .stdout(predicate::str::contains("workflow"))
        .stdout(predicate::str::contains("patterns"));
}

#[test]
fn bdd_spawn_with_invalid_bead_still_works() {
    // Scenario: Contract flag works even with invalid bead
    //   When I run "zjj spawn --contract nonexistent-bead"
    //   Then it should return contract without checking bead validity
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("spawn")
        .arg("--contract")
        .arg("zjj-nonexistent");

    // Should succeed with contract output, not fail on bead validation
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT"));
}

#[test]
fn bdd_spawn_without_flags_runs_normally() {
    // Scenario: Normal spawn operation still works
    //   When I run "zjj spawn --help"
    //   Then it should show help including new flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("spawn")
        .arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains("--ai-hints"))
        .stdout(predicate::str::contains("AI: Show machine-readable contract"))
        .stdout(predicate::str::contains("AI: Show execution hints"));
}

#[test]
fn bdd_no_panic_with_any_flag_combination() {
    // Scenario: Multiple AI flags don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic with "ArgAction should be SetTrue or SetFalse"

    let test_cases = vec![
        vec!["spawn", "--contract", "zjj-test"],
        vec!["spawn", "--ai-hints", "zjj-test"],
        vec!["spawn", "--json", "zjj-test"],
        vec!["spawn", "--contract", "--json", "zjj-test"],
        vec!["spawn", "--ai-hints", "--json", "zjj-test"],
        vec!["spawn", "--help"],
    ];

    for args in test_cases {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
        for arg in args {
            cmd.arg(arg);
        }

        // Should not panic (exit code 134 is panic)
        cmd.assert()
            .code(
                predicate::ne(134) // 134 = panic
                    .and(predicate::ne(101)) // 101 = clap panic
            );
    }
}

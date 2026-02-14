// BDD Test for --contract flag on contract command
//
// Feature: AI Contract Integration
//   As an AI agent
//   I want to query the contract command's own contract without panics
//   So that I can understand the contract schema autonomously
//
// Background:
//   Given zjj is initialized

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_contract_contract_flag_does_not_panic() {
    // Scenario: AI queries contract command contract
    //   When I run "zjj contract --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("contract").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj contract"))
        .stdout(predicate::str::contains("command"))
        .stdout(predicate::str::contains("intent"))
        .stdout(predicate::str::contains("inputs"))
        .stdout(predicate::str::contains("outputs"));
}

#[test]
fn bdd_contract_contract_flag_with_command_arg() {
    // Scenario: AI queries contract command contract with command arg
    //   When I run "zjj contract add --contract"
    //   Then it should return contract command's own contract, not add's
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("contract").arg("add").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj contract"));
}

#[test]
fn bdd_contract_help_shows_contract_flag() {
    // Scenario: Help shows --contract flag
    //   When I run "zjj contract --help"
    //   Then it should show --contract flag in help

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("contract").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains(
            "AI: Show machine-readable contract",
        ));
}

#[test]
fn bdd_contract_normal_operation_still_works() {
    // Scenario: Normal contract operation still works
    //   When I run "zjj contract add"
    //   Then it should show contract for add command

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("contract").arg("add");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Command: add"));
}

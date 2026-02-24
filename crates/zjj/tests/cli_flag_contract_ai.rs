// BDD Test for AI command introspection
//
// Feature: AI Introspection Integration
//   As an AI agent
//   I want to query command contracts via introspect
//   So that I can understand command schemas autonomously
//
// Background:
//   Given zjj is initialized
//   And the beads database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_ai_introspect_contract_works() {
    // Scenario: AI queries introspect command contract
    //   When I run "zjj introspect --contract"
    //   Then it should return JSON contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("introspect").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("contract"));
}

#[test]
fn bdd_ai_introspect_init_works() {
    // Scenario: AI queries specific command introspection
    //   When I run "zjj introspect init"
    //   Then it should return init command info without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("introspect").arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("init"));
}

#[test]
fn bdd_ai_without_flags_runs_normally() {
    // Scenario: Normal ai operation still works
    //   When I run "zjj ai --help"
    //   Then it should show help
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI-first commands"));
}

#[test]
fn bdd_ai_no_panic_with_any_flag_combination() {
    // Scenario: Various AI commands don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic

    let test_cases: Vec<Vec<&str>> = vec![
        vec!["ai", "--help"],
        vec!["ai", "work", "--help"],
        vec!["introspect", "--contract"],
        vec!["introspect", "init"],
        vec!["introspect", "--json"],
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
fn bdd_ai_work_subcommand_help_works() {
    // Scenario: AI work subcommand help works
    //   When I run "zjj ai work --help"
    //   Then it should show help for work subcommand
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("ai").arg("work").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("work"));
}

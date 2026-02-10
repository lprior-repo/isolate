// BDD Test for pane focus command - fix clap panic
//
// Feature: Pane Focus AI Integration
//   As an AI agent
//   I want to query pane focus contracts without panics
//   So that I can understand pane management commands
//
// Background:
//   Given zjj is initialized

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_pane_focus_contract_flag_does_not_panic() {
    // Scenario: AI queries pane focus contract
    //   When I run "zjj pane focus test-session --contract"
    //   Then it should return contract without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pane")
        .arg("focus")
        .arg("test-session")
        .arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn bdd_pane_focus_ai_hints_flag_does_not_panic() {
    // Scenario: AI queries pane focus hints
    //   When I run "zjj pane focus test-session --ai-hints"
    //   Then it should return hints without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pane")
        .arg("focus")
        .arg("test-session")
        .arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn bdd_pane_focus_help_shows_new_flags() {
    // Scenario: Help text includes new AI flags
    //   When I run "zjj pane focus --help"
    //   Then it should show --contract and --ai-hints flags
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pane")
        .arg("focus")
        .arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--contract"))
        .stdout(predicate::str::contains("--ai-hints"))
        .stdout(predicate::str::contains("AI: Show machine-readable contract"))
        .stdout(predicate::str::contains("AI: Show execution hints"));
}

#[test]
fn bdd_pane_focus_no_panic_with_any_flag_combination() {
    // Scenario: Multiple flag combinations don't cause clap panic
    //   When I run various flag combinations
    //   Then none should panic

    let test_cases = vec![
        vec!["pane", "focus", "test-session"],
        vec!["pane", "focus", "test-session", "--contract"],
        vec!["pane", "focus", "test-session", "--ai-hints"],
        vec!["pane", "focus", "test-session", "--json"],
        vec!["pane", "focus", "test-session", "--direction", "left"],
        vec!["pane", "focus", "--help"],
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

#[test]
fn bdd_pane_focus_with_direction_flag() {
    // Scenario: Direction flag works without panic
    //   When I run "zjj pane focus test-session --direction left"
    //   Then it should not panic
    //   And exit code should be non-zero (session doesn't exist but no panic)

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pane")
        .arg("focus")
        .arg("test-session")
        .arg("--direction")
        .arg("left");

    // Should not panic even if session doesn't exist
    cmd.assert()
        .code(predicate::ne(134).and(predicate::ne(101)));
}

// BDD Test for pause/resume command argument contracts
//
// Feature: Required Argument Validation
//   As a CLI user
//   I want clear error messages when I forget required arguments
//   So that I understand what to fix immediately
//
// Background:
//   Given zjj is initialized
//   And required arguments must not silently default

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_pause_without_name_returns_error() {
    // Scenario: User runs pause without session name
    //   When I run "zjj pause" without --name flag
    //   Then it should return an error (not success)
    //   And the error message should mention that session name is required
    //   And exit code should be non-zero

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pause");

    cmd.assert()
        .failure() // Must fail, not succeed with silent default
        .stderr(predicate::str::contains("required"))
        .stderr(predicate::str::contains("name").or(predicate::str::contains("session")));
}

#[test]
fn bdd_resume_without_name_returns_error() {
    // Scenario: User runs resume without session name
    //   When I run "zjj resume" without --name flag
    //   Then it should return an error (not success)
    //   And the error message should mention that session name is required
    //   And exit code should be non-zero

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("resume");

    cmd.assert()
        .failure() // Must fail, not succeed with silent default
        .stderr(predicate::str::contains("required"))
        .stderr(predicate::str::contains("name").or(predicate::str::contains("session")));
}

#[test]
fn bdd_pause_with_explicit_name_works() {
    // Scenario: User runs pause with explicit session name
    //   When I run "zjj pause --name test-session"
    //   Then it should attempt to pause that session
    //   And not fail due to missing argument
    //
    // Note: The operation may fail if session doesn't exist,
    // but it should NOT fail with "name is required" error

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pause").arg("--name").arg("test-session");

    // Should not fail with "required" error
    // (May fail with "session not found" which is acceptable)
    let output = cmd.output().expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain argument validation errors
    assert!(
        !stderr.contains("required") || stderr.contains("not found"),
        "Expected either success or 'not found' error, got: {}",
        stderr
    );
}

#[test]
fn bdd_resume_with_explicit_name_works() {
    // Scenario: User runs resume with explicit session name
    //   When I run "zjj resume --name test-session"
    //   Then it should attempt to resume that session
    //   And not fail due to missing argument
    //
    // Note: The operation may fail if session doesn't exist,
    // but it should NOT fail with "name is required" error

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("resume").arg("--name").arg("test-session");

    // Should not fail with "required" error
    // (May fail with "session not found" which is acceptable)
    let output = cmd.output().expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain argument validation errors
    assert!(
        !stderr.contains("required") || stderr.contains("not found"),
        "Expected either success or 'not found' error, got: {}",
        stderr
    );
}

#[test]
fn bdd_pause_error_message_is_actionable() {
    // Scenario: Error message tells user how to fix the problem
    //   When I run "zjj pause" without --name
    //   Then the error message should be actionable
    //   And should mention how to provide the session name

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pause");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("--name").or(predicate::str::contains("session")));
}

#[test]
fn bdd_resume_error_message_is_actionable() {
    // Scenario: Error message tells user how to fix the problem
    //   When I run "zjj resume" without --name
    //   Then the error message should be actionable
    //   And should mention how to provide the session name

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("resume");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("--name").or(predicate::str::contains("session")));
}

#[test]
fn bdd_pause_help_shows_name_parameter() {
    // Scenario: Help text documents the name parameter
    //   When I run "zjj pause --help"
    //   Then it should show the name parameter (positional arg)
    //   And explain what it does

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("pause").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[name]").or(predicate::str::contains("name")))
        .stdout(predicate::str::contains("session").or(predicate::str::contains("Session")));
}

#[test]
fn bdd_resume_help_shows_name_parameter() {
    // Scenario: Help text documents the name parameter
    //   When I run "zjj resume --help"
    //   Then it should show the name parameter (positional arg)
    //   And explain what it does

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("resume").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[name]").or(predicate::str::contains("name")))
        .stdout(predicate::str::contains("session").or(predicate::str::contains("Session")));
}

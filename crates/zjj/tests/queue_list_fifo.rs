#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! BDD Test for queue list subcommand with FIFO ordering
//!
//! Feature: Queue List Subcommand
//!   As a user
//!   I want to run "zjj queue list" (not "zjj queue --list")
//!   So that I can see queue entries in FIFO order
//!
//! Background:
//!   Given zjj is initialized
//!   And the queue database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_queue_list_subcommand_exists() {
    // Scenario: Queue list subcommand is available
    //   Given I have zjj installed
    //   When I run "zjj queue list"
    //   Then it should execute successfully
    //   And not show "unrecognized subcommand" error

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue").arg("list");

    // Should not show "unrecognized subcommand" error
    cmd.assert()
        .stdout(predicate::str::contains("unrecognized subcommand").not())
        .stderr(predicate::str::contains("unrecognized subcommand").not());
}

#[test]
fn test_queue_list_shows_empty_queue() {
    // Scenario: List empty queue
    //   Given the queue is empty
    //   When I run "zjj queue list"
    //   Then it should show "Queue is empty"
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Queue is empty"));
}

#[test]
fn test_queue_list_json_format() {
    // Scenario: List queue as JSON
    //   Given the queue may have entries
    //   When I run "zjj queue list --json"
    //   Then it should return valid JSON with schema envelope
    //   And include state counts
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"total\""))
        .stdout(predicate::str::contains("\"pending\""))
        .stdout(predicate::str::contains("\"processing\""))
        .stdout(predicate::str::contains("\"completed\""))
        .stdout(predicate::str::contains("\"failed\""));
}

#[test]
fn test_queue_list_shows_state_counts() {
    // Scenario: List queue shows state summary
    //   Given the queue may have entries in various states
    //   When I run "zjj queue list"
    //   Then it should show state counts
    //   And display total, pending, processing, completed, failed

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("total"))
        .stdout(predicate::str::contains("pending"))
        .stdout(predicate::str::contains("processing"))
        .stdout(predicate::str::contains("completed"))
        .stdout(predicate::str::contains("failed"));
}

#[test]
fn test_queue_list_help() {
    // Scenario: Queue list help shows usage
    //   When I run "zjj queue list --help"
    //   Then it should show help for the list subcommand
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue").arg("list").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list").or(predicate::str::contains("List")));
}

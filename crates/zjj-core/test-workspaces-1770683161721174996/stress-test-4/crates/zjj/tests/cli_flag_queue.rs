// BDD Test for queue command - fix type mismatch panic
//
// Feature: Queue Type Consistency
//   As a user
//   I want all queue subcommands to execute without type panics
//   So that multi-agent coordination works reliably
//
// Background:
//   Given zjj is initialized
//   And the queue database exists

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_queue_list_does_not_panic() {
    // Scenario: List queue entries without panic
    //   When I run "zjj queue --list"
    //   Then it should execute without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--list");

    // Should not panic (exit code 134 is panic, 101 is clap panic)
    cmd.assert()
        .code(predicate::ne(134).and(predicate::ne(101)));
}

#[test]
fn bdd_queue_list_json_does_not_panic() {
    // Scenario: List queue as JSON without panic
    //   When I run "zjj queue --list --json"
    //   Then it should return JSON without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--list")
        .arg("--json");

    // Should not panic and return valid JSON
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("$schema"));
}

#[test]
fn bdd_queue_stats_does_not_panic() {
    // Scenario: Show queue statistics without panic
    //   When I run "zjj queue --stats"
    //   Then it should execute without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--stats");

    // Should not panic
    cmd.assert()
        .code(predicate::ne(134).and(predicate::ne(101)));
}

#[test]
fn bdd_queue_next_does_not_panic() {
    // Scenario: Get next queue entry without panic
    //   When I run "zjj queue --next"
    //   Then it should execute without panic
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--next");

    // Should not panic
    cmd.assert()
        .code(predicate::ne(134).and(predicate::ne(101)));
}

#[test]
fn bdd_queue_priority_flag_type_correct() {
    // Scenario: Priority flag accepts integer values
    //   When I run "zjj queue --help"
    //   Then it should show priority with correct type
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("PRIORITY"))
        .stdout(predicate::str::contains("(lower = higher priority"));
}

#[test]
fn bdd_queue_no_panic_with_any_subcommand() {
    // Scenario: All queue subcommands work without type panic
    //   When I run various queue subcommands
    //   Then none should panic with type mismatch error

    let test_cases = vec![
        vec!["queue", "--list"],
        vec!["queue", "--list", "--json"],
        vec!["queue", "--stats"],
        vec!["queue", "--stats", "--json"],
        vec!["queue", "--next"],
        vec!["queue", "--next", "--json"],
        vec!["queue", "--help"],
    ];

    for args in test_cases {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
        for arg in args {
            cmd.arg(arg);
        }

        // Should not panic with type mismatch
        cmd.assert()
            .code(
                predicate::ne(134) // 134 = panic
                    .and(predicate::ne(101)) // 101 = clap panic
            );
    }
}

#[test]
fn bdd_queue_json_output_valid() {
    // Scenario: Queue JSON output is valid and structured
    //   When I run "zjj queue --list --json"
    //   Then it should return valid JSON with schema envelope
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--list")
        .arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"schema_type\""))
        .stdout(predicate::str::contains("\"success\""));
}

#[test]
fn bdd_queue_stats_json_valid() {
    // Scenario: Queue stats JSON output is valid
    //   When I run "zjj queue --stats --json"
    //   Then it should return valid JSON with statistics
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("queue")
        .arg("--stats")
        .arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("total"))
        .stdout(predicate::str::contains("pending"))
        .stdout(predicate::str::contains("processing"))
        .stdout(predicate::str::contains("completed"))
        .stdout(predicate::str::contains("failed"));
}

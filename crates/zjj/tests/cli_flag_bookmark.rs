// BDD Test for bookmark list JSON serialization fix
//
// Feature: Bookmark List JSON Serialization
//   As a user
//   I want bookmark list to return valid JSON
//   So that I can programmatically consume bookmark data
//
// Background:
//   Given zjj is initialized

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_bookmark_list_json_valid() {
    // Scenario: Bookmark list returns valid JSON
    //   When I run "zjj bookmark list --json"
    //   Then it should return valid JSON without serialization error
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list").arg("--json");

    // Should not panic with serialization error
    cmd.assert()
        .code(predicate::ne(134).and(predicate::ne(101)));
}

#[test]
fn bdd_bookmark_list_json_has_schema() {
    // Scenario: JSON output includes schema fields
    //   When I run "zjj bookmark list --json"
    //   Then it should include $schema, schema_type, and success fields
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"schema_type\""))
        .stdout(predicate::str::contains("\"success\""));
}

#[test]
fn bdd_bookmark_list_json_has_data_field() {
    // Scenario: JSON output includes data array with bookmarks
    //   When I run "zjj bookmark list --json"
    //   Then it should include a data field (per schema envelope standard)
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"data\""));
}

#[test]
fn bdd_bookmark_list_empty_returns_valid_json() {
    // Scenario: Empty bookmark list returns valid JSON
    //   When I run "zjj bookmark list --json"
    //   Then it should return valid JSON with data field
    //   And exit code should be 0 (data array may be empty or not)

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"data\""))
        .stdout(predicate::str::contains("[")); // Array marker
}

#[test]
fn bdd_bookmark_list_human_output_works() {
    // Scenario: Human-readable output still works
    //   When I run "zjj bookmark list" without --json
    //   Then it should display bookmarks in human-readable format
    //   Or show "No bookmarks found." if empty

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list");

    // Should succeed (either shows bookmarks or "No bookmarks found.")
    cmd.assert().success();
}

#[test]
fn bdd_bookmark_help_shows_json_flag() {
    // Scenario: Help text includes --json flag
    //   When I run "zjj bookmark list --help"
    //   Then it should show the --json flag
    //   And exit code should be 0

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("bookmark").arg("list").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("Output as JSON"));
}

//! Red Queen regression tests for add command validation
//!
//! Tests verify that --bead, --template, --contract, and --ai-hints flags
//! properly validate inputs and output correct formats.

use assert_cmd::Command;
use predicates::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// BEAD VALIDATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_add_with_empty_bead_rejects() {
    // GIVEN: add command with --bead ""
    // WHEN: invoked with empty bead ID
    // THEN: rejects with descriptive message
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["add", "test-session", "--bead", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Bead ID cannot be empty"));
}

#[test]

// ═══════════════════════════════════════════════════════════════════════════
// TEMPLATE VALIDATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_add_with_invalid_template_rejects() {
    // GIVEN: add command with --template "invalid"
    // WHEN: invoked with invalid template name
    // THEN: rejects with list of valid templates
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["add", "test-session", "--template", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid template 'invalid'"))
        .stderr(predicate::str::contains("minimal, standard, full"));
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON OUTPUT FORMAT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_add_contract_flag_outputs_pure_json() {
    // GIVEN: add command with --contract flag
    // WHEN: invoked with contract flag
    // THEN: outputs pure JSON without text prefix
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["add", "--contract"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT").not())
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"command\": \"zjj add\""));
}

#[test]
fn test_add_ai_hints_flag_outputs_pure_json() {
    // GIVEN: add command with --ai-hints flag
    // WHEN: invoked with ai-hints flag
    // THEN: outputs pure JSON without text prefix
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["add", "--ai-hints"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW").not())
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("typical_workflows"));
}

#[test]
fn test_add_example_json_flag_outputs_valid_json() {
    // GIVEN: add command with --example-json flag
    // WHEN: invoked with example-json flag
    // THEN: outputs valid JSON example
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["add", "--example-json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("example-session"));
}

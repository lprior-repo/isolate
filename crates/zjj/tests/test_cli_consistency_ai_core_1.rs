use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_whereami_contract() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("whereami")
        .arg("--contract")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj whereami"));
}

#[test]
fn test_whereami_ai_hints() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("whereami")
        .arg("--ai-hints")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn test_whoami_contract() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("whoami")
        .arg("--contract")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj whoami"));
}

#[test]
fn test_whoami_ai_hints() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("whoami")
        .arg("--ai-hints")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn test_list_contract() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("list")
        .arg("--contract")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj list"));
}

#[test]
fn test_list_ai_hints() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("list")
        .arg("--ai-hints")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn test_focus_contract() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("focus")
        .arg("--contract")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj focus"));
}

#[test]
fn test_focus_ai_hints() {
    let mut cmd = Command::cargo_bin("zjj").unwrap();
    cmd.arg("focus")
        .arg("--ai-hints")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

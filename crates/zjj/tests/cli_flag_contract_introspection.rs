use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bdd_context_contract_flag_does_not_panic() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("context").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj context"));
}

#[test]
fn bdd_context_ai_hints_flag_does_not_panic() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("context").arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn bdd_whatif_contract_flag_allows_missing_command() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whatif").arg("--contract");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI CONTRACT for zjj whatif"));
}

#[test]
fn bdd_whatif_ai_hints_flag_allows_missing_command() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whatif").arg("--ai-hints");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI COMMAND FLOW"));
}

#[test]
fn bdd_whatif_requires_command_without_ai_flags() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zjj"));
    cmd.arg("whatif");

    cmd.assert().failure().stderr(predicate::str::contains(
        "required arguments were not provided",
    ));
}

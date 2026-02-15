//! Red Queen regression tests for pause, unlock, and rename commands
//!
//! These tests verify that parser validation and exit code behavior are correct
//! across missing args, empty strings, and invalid inputs.

use assert_cmd::Command;
use predicates::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// PAUSE COMMAND TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_pause_missing_required_session_name_rejects_with_exit_2() {
    // GIVEN: pause command without session name
    // WHEN: invoked without args
    // THEN: exits with code 2 (clap usage error) and shows usage
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .arg("pause")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("<name>"));
}

#[test]
fn test_pause_with_json_flag_missing_session_name_rejects_with_exit_1() {
    // GIVEN: pause command with --json but no session name
    // WHEN: invoked with only --json flag
    // THEN: exits with code 1 (validation error - empty session)
    // NOTE: --json errors go to stdout as JSON, not stderr
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["pause", "--json"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_pause_nonexistent_session_fails_with_exit_2() {
    // GIVEN: pause command with non-existent session name
    // WHEN: invoked with session that doesn't exist
    // THEN: exits with code 2 (not found error) with descriptive message
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["pause", "nonexistent-session-xyz"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

// ═══════════════════════════════════════════════════════════════════════════
// RESUME COMMAND TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_resume_missing_required_session_name_rejects_with_exit_2() {
    // GIVEN: resume command without session name
    // WHEN: invoked without args
    // THEN: exits with code 2 (clap usage error) and shows usage
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .arg("resume")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("<name>"));
}

#[test]
fn test_resume_with_json_flag_missing_session_name_rejects_with_exit_1() {
    // GIVEN: resume command with --json but no session name
    // WHEN: invoked with only --json flag
    // THEN: exits with code 1 (validation error - empty session)
    // NOTE: --json errors go to stdout as JSON, not stderr
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["resume", "--json"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_resume_nonexistent_session_fails_with_exit_2() {
    // GIVEN: resume command with non-existent session name
    // WHEN: invoked with session that doesn't exist
    // THEN: exits with code 2 (not found error) with descriptive message
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["resume", "nonexistent-session-xyz"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

// ═══════════════════════════════════════════════════════════════════════════
// UNLOCK COMMAND TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_unlock_missing_required_session_arg_rejects_with_exit_2() {
    // GIVEN: unlock command without session argument
    // WHEN: invoked without args
    // THEN: exits with code 2 (clap usage error) and shows usage
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .arg("unlock")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("<session>"));
}

#[test]
fn test_unlock_with_json_flag_missing_session_rejects_with_exit_1() {
    // GIVEN: unlock command with --json but no session
    // WHEN: invoked with only --json flag
    // THEN: exits with code 1 (validation error - empty session)
    // NOTE: --json errors go to stdout as JSON, not stderr
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["unlock", "--json"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_unlock_with_agent_id_but_missing_session_rejects_with_exit_2() {
    // GIVEN: unlock command with --agent-id but no session
    // WHEN: invoked with optional flag but missing required arg
    // THEN: exits with code 2 (clap usage error)
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["unlock", "--agent-id", "agent1"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn test_unlock_nonexistent_session_fails_with_exit_4() {
    // GIVEN: unlock command with non-existent session
    // WHEN: invoked with session that doesn't exist
    // THEN: exits with code 4 (SESSION_NOT_FOUND error) with descriptive message
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["unlock", "nonexistent-session-xyz"])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("does not exist").or(predicate::str::contains("not found")));
}

// ═══════════════════════════════════════════════════════════════════════════
// RENAME COMMAND TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_rename_missing_both_args_rejects_with_exit_2() {
    // GIVEN: rename command without any args
    // WHEN: invoked without old_name and new_name
    // THEN: exits with code 2 (clap usage error) and shows usage
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .arg("rename")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("<old_name>"))
        .stderr(predicate::str::contains("<new_name>"));
}

#[test]
fn test_rename_missing_new_name_rejects_with_exit_2() {
    // GIVEN: rename command with only old_name
    // WHEN: invoked with one arg but missing second required arg
    // THEN: exits with code 2 (clap usage error)
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["rename", "old-session"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("required arguments were not provided"))
        .stderr(predicate::str::contains("<new_name>"));
}

#[test]
fn test_rename_with_json_flag_missing_args_rejects_with_exit_1() {
    // GIVEN: rename command with --json but missing required args
    // WHEN: invoked with optional flag but no required args
    // THEN: exits with code 1 (validation error - empty args)
    // NOTE: --json errors go to stdout as JSON, not stderr
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["rename", "--json"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_rename_nonexistent_session_fails_with_exit_2() {
    // GIVEN: rename command with non-existent old session name
    // WHEN: invoked with session that doesn't exist
    // THEN: exits with code 2 (not found error) with descriptive message
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["rename", "nonexistent-old", "new-name"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

// ═══════════════════════════════════════════════════════════════════════════
// CROSS-COMMAND CONSISTENCY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_pause_resume_unlock_all_reject_invalid_flags_with_exit_2() {
    // GIVEN: commands with invalid/unknown flags
    // WHEN: invoked with nonsense flags
    // THEN: all consistently exit with code 2
    
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["pause", "session", "--invalid-flag"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["resume", "session", "--invalid-flag"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["unlock", "session", "--invalid-flag"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["rename", "old", "new", "--invalid-flag"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn test_help_flag_works_for_all_commands() {
    // GIVEN: commands with --help flag
    // WHEN: invoked with help
    // THEN: all exit 0 and show help text
    
    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["pause", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pause an active session"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["resume", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Resume a paused session"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["unlock", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Release exclusive lock"));

    Command::cargo_bin("zjj")
        .expect("zjj binary exists")
        .args(["rename", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rename an existing session"));
}

// Integration tests have relaxed clippy settings for brutal test scenarios.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

mod common;

use std::process::Command;

use common::{CommandResult, TestHarness};
use tempfile::TempDir;

fn run_zjj_with_env(
    harness: &TestHarness,
    args: &[&str],
    env_overrides: &[(&str, &str)],
) -> CommandResult {
    let path_with_system_dirs = format!(
        "/usr/bin:/usr/local/bin:{}",
        std::env::var("PATH").unwrap_or_default()
    );

    let mut command = Command::new(&harness.zjj_bin);
    command
        .args(args)
        .current_dir(&harness.current_dir)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .env("ZJJ_WORKSPACE_DIR", "workspaces")
        .env("PATH", path_with_system_dirs);

    for (key, value) in env_overrides {
        command.env(key, value);
    }

    command
        .output()
        .map(|output| CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
        .unwrap_or_else(|_| CommandResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "Command execution failed".to_string(),
        })
}

#[test]
fn given_clean_age_threshold_when_non_numeric_then_cli_rejects_value() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["clean", "--age-threshold", "abc"]);

    assert!(
        !result.success,
        "Expected clean to reject non-numeric age-threshold\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.stderr.contains("invalid value") || result.stderr.contains("unexpected argument"),
        "Expected parse error for invalid threshold\nStderr: {}",
        result.stderr
    );
}

#[test]
fn given_boolean_config_when_setting_string_value_then_command_fails_and_config_stays_readable() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let invalid_set = harness.zjj(&["config", "watch.enabled", "notabool"]);
    assert!(
        !invalid_set.success,
        "Expected invalid boolean value to be rejected\nStdout: {}\nStderr: {}",
        invalid_set.stdout, invalid_set.stderr
    );

    let lookup = harness.zjj(&["config", "watch.enabled"]);
    assert!(
        lookup.success,
        "Config should remain readable after rejected write\nStdout: {}\nStderr: {}",
        lookup.stdout, lookup.stderr
    );
}

#[test]
fn given_global_and_project_values_when_reading_with_global_flag_then_global_value_is_returned() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let global_home = TempDir::new().expect("tempdir should be created");
    let home_path = global_home.path().display().to_string();
    let envs = [
        ("HOME", home_path.as_str()),
        ("XDG_CONFIG_HOME", home_path.as_str()),
    ];

    let global_set = run_zjj_with_env(
        &harness,
        &["config", "--global", "main_branch", "trunk"],
        &envs,
    );
    assert!(
        global_set.success,
        "Global set should succeed\nStdout: {}\nStderr: {}",
        global_set.stdout, global_set.stderr
    );

    let project_set = run_zjj_with_env(&harness, &["config", "main_branch", "feature-x"], &envs);
    assert!(
        project_set.success,
        "Project set should succeed\nStdout: {}\nStderr: {}",
        project_set.stdout, project_set.stderr
    );

    let global_read = run_zjj_with_env(&harness, &["config", "--global", "main_branch"], &envs);
    assert!(
        global_read.success,
        "Global read should succeed\nStdout: {}\nStderr: {}",
        global_read.stdout, global_read.stderr
    );
    assert!(
        global_read.stdout.contains("main_branch = trunk"),
        "Global read should return the global value\nStdout: {}",
        global_read.stdout
    );
}

#[test]
fn given_completions_alias_when_using_pwsh_then_command_succeeds() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["completions", "pwsh"]);
    assert!(
        result.success,
        "Expected pwsh alias to be accepted\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        result.stdout.contains("Register-ArgumentCompleter"),
        "Expected powershell completion output\nStdout: {}",
        result.stdout
    );
}

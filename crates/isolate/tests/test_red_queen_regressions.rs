#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod common;

use common::{parse_json_output, payload, TestHarness};

#[test]
fn given_json_mode_when_parse_fails_then_attach_output_is_json_only() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["attach", "--json", "--bad-flag"]);
    assert!(!result.success, "command should fail");
    assert!(
        result.stderr.trim().is_empty(),
        "expected no human stderr in --json mode, got: {}",
        result.stderr
    );
    let parsed = parse_json_output(&result.stdout);
    assert!(
        parsed.is_ok(),
        "stdout should be valid JSON: {}",
        result.stdout
    );
}

#[test]
fn given_json_mode_when_parse_fails_then_agents_output_is_json_only() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["agents", "status", "--json", "--bad-flag"]);
    assert!(!result.success, "command should fail");
    assert!(
        result.stderr.trim().is_empty(),
        "expected no human stderr in --json mode, got: {}",
        result.stderr
    );
    let parsed = parse_json_output(&result.stdout);
    assert!(
        parsed.is_ok(),
        "stdout should be valid JSON: {}",
        result.stdout
    );
}

#[test]
fn given_contract_command_when_requesting_attach_then_contract_exists() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["contract", "attach", "--json"]);
    assert!(
        result.success,
        "attach contract should exist\nstderr={}\nstdout={}",
        result.stderr, result.stdout
    );

    let parsed = parse_json_output(&result.stdout).expect("expected valid JSON output");
    let contract_name = payload(&parsed)
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert_eq!(contract_name, "attach");
}

#[test]
fn given_add_contract_when_reading_args_then_name_is_not_unconditionally_required() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.isolate(&["contract", "add", "--json"]);
    assert!(
        result.success,
        "contract add should succeed: {}",
        result.stderr
    );

    let parsed = parse_json_output(&result.stdout).expect("expected valid JSON output");
    let contract = payload(&parsed);

    // name is listed as required because it is required for normal execution
    // but the description should mention it can be omitted with special flags
    let name_arg = contract
        .get("required_args")
        .and_then(serde_json::Value::as_array)
        .and_then(|args| {
            args.iter().find(|arg| {
                arg.get("name")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|name| name == "name")
            })
        })
        .expect("name should be in required_args");

    let description = name_arg
        .get("description")
        .and_then(serde_json::Value::as_str)
        .expect("name arg should have a description");

    assert!(
        description.contains("--example-json") || description.contains("omitted"),
        "description should mention that name is conditional. Got: {description}"
    );
}

#[test]
fn given_backup_create_when_timestamp_flag_present_then_clap_rejects_it() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.isolate(&["backup", "--create", "--timestamp", "20250101-010101"]);
    assert!(
        !result.success,
        "command should fail when --timestamp lacks --restore"
    );
    assert!(
        result.stderr.contains("--restore") || result.stdout.contains("--restore"),
        "expected error to mention --restore requirement\nstderr={}\nstdout={}",
        result.stderr,
        result.stdout
    );
}

#[test]
fn given_backup_restore_when_timestamp_is_malformed_then_error_is_actionable() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.isolate(&["backup", "--restore", "state.db", "--timestamp", "20250101"]);
    assert!(
        !result.success,
        "command should fail for malformed timestamp"
    );
    assert!(
        result.stderr.contains("Invalid --timestamp")
            || result.stdout.contains("Invalid --timestamp"),
        "expected explicit timestamp format error\nstderr={}\nstdout={}",
        result.stderr,
        result.stdout
    );
}

#[test]
fn given_unregistered_agent_when_unreg_then_returns_not_found() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.isolate(&["agents", "unregister", "--id", "ghost-agent", "--json"]);
    assert!(
        !result.success,
        "unregister should fail when agent does not exist"
    );
    assert!(
        result.stdout.contains("not found") || result.stderr.contains("not found"),
        "expected not found messaging\nstderr={}\nstdout={}",
        result.stderr,
        result.stdout
    );
}

#[test]
fn given_same_attach_failure_when_json_toggled_then_exit_code_is_identical() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let human = harness.isolate(&["attach", "nonexistent-session"]);
    let json = harness.isolate(&["attach", "nonexistent-session", "--json"]);

    assert!(!human.success, "human attach should fail in test env");
    assert!(!json.success, "json attach should fail in test env");
    assert_eq!(
        human.exit_code, json.exit_code,
        "exit code must not depend on --json\nhuman={:?}\njson={:?}\nhuman stderr={}\njson stdout={}\njson stderr={}",
        human.exit_code, json.exit_code, human.stderr, json.stdout, json.stderr
    );
}

#[test]
fn given_backup_unknown_database_when_json_enabled_then_code_is_invalid_argument() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let human = harness.isolate(&["backup", "--restore", "notadb.db"]);
    let json = harness.isolate(&["backup", "--restore", "notadb.db", "--json"]);

    assert!(!human.success, "human backup restore should fail");
    assert!(!json.success, "json backup restore should fail");
    assert_eq!(
        human.exit_code, json.exit_code,
        "exit code must not depend on --json"
    );

    let parsed = parse_json_output(&json.stdout).expect("expected valid JSON output");
    let code = payload(&parsed)
        .get("error")
        .and_then(|v| v.get("code"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    assert_eq!(
        code, "STATE_DB_CORRUPTED",
        "expected STATE_DB_CORRUPTED code for unknown database"
    );
}

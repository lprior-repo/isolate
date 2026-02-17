#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::bool_assert_comparison,
    clippy::duration_suboptimal_units,
    clippy::filter_map_bool_then
)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

mod common;

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use common::{payload, TestHarness};

#[derive(Clone, Copy)]
enum ErrorCategory {
    Usage,
    Runtime,
}

struct InvalidCase {
    name: &'static str,
    args: &'static [&'static str],
    category: ErrorCategory,
    expected_codes: &'static [&'static str],
}

struct FollowProcess {
    child: Child,
    stdout_rx: Receiver<String>,
}

fn assert_json_error_shape(result: &common::CommandResult, case: &InvalidCase) {
    assert!(!result.success, "{} should fail", case.name);
    assert!(
        result.exit_code.unwrap_or_default() != 0,
        "{} should return non-zero exit code",
        case.name
    );

    let parsed: serde_json::Value = serde_json::from_str(&result.stdout).unwrap_or_else(|_| {
        panic!(
            "{} stdout should be valid JSON: {}",
            case.name, result.stdout
        )
    });

    let schema = parsed
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{} should include $schema field", case.name));
    assert!(
        schema.contains("error-response"),
        "{} should use error-response schema, got: {schema}",
        case.name
    );
    assert_eq!(
        parsed
            .get("success")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        false,
        "{} should report success=false",
        case.name
    );

    let error = payload(&parsed)
        .get("error")
        .unwrap_or_else(|| panic!("{} should include data.error", case.name));
    let code = error
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{} should include error.code", case.name));
    let exit_code = error
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_else(|| panic!("{} should include error.exit_code", case.name));
    assert!(
        exit_code > 0,
        "{} should include positive error.exit_code, got {exit_code}",
        case.name
    );

    let usage_codes = ["INVALID_ARGUMENT", "VALIDATION_ERROR"];
    match case.category {
        ErrorCategory::Usage => {
            assert!(
                usage_codes.contains(&code),
                "{} should be usage/invalid-argument category, got code={code}",
                case.name
            );
        }
        ErrorCategory::Runtime => {
            assert!(
                !usage_codes.contains(&code),
                "{} should be runtime category, got usage code={code}",
                case.name
            );
        }
    }

    assert!(
        case.expected_codes.contains(&code),
        "{} expected one of {:?}, got {code}",
        case.name,
        case.expected_codes
    );
}

fn spawn_follow_process(harness: &TestHarness, args: &[&str]) -> FollowProcess {
    let path_with_system_dirs = format!(
        "/usr/bin:/usr/local/bin:{}",
        std::env::var("PATH").unwrap_or_default()
    );

    let mut child = Command::new(&harness.zjj_bin)
        .args(args)
        .current_dir(&harness.repo_path)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .env("ZJJ_WORKSPACE_DIR", "workspaces")
        .env("PATH", &path_with_system_dirs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn zjj follow process");

    let stdout = child
        .stdout
        .take()
        .expect("follow process should have piped stdout");
    let stderr = child
        .stderr
        .take()
        .expect("follow process should have piped stderr");

    let (stdout_tx, stdout_rx) = mpsc::channel::<String>();
    let (stderr_tx, _stderr_rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(value) = line {
                let _ = stdout_tx.send(value);
            }
        }
    });

    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(value) = line {
                let _ = stderr_tx.send(value);
            }
        }
    });

    FollowProcess { child, stdout_rx }
}

fn wait_for_line(rx: &Receiver<String>, needle: &str, timeout: Duration) -> Option<String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(line) = rx.recv_timeout(Duration::from_millis(250)) {
            if line.contains(needle) {
                return Some(line);
            }
        }
    }

    None
}

fn saw_line_containing(rx: &Receiver<String>, needle: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains(needle) {
                    return true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }

    false
}

fn append_events_line(events_file: &std::path::Path, line: &str) {
    use std::io::Write;

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_file)
        .expect("failed to open events file for append");
    let mut writer = std::io::BufWriter::new(file);
    writer
        .write_all(line.as_bytes())
        .expect("failed to append events line");
    writer.flush().expect("failed to flush events line");
}

#[test]
fn given_doctor_dry_run_without_fix_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["doctor", "--dry-run"]);

    assert!(
        !result.success,
        "doctor --dry-run without --fix should fail"
    );
    result.assert_output_contains("--fix");
}

#[test]
fn given_doctor_verbose_without_fix_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["doctor", "--verbose"]);

    assert!(
        !result.success,
        "doctor --verbose without --fix should fail"
    );
    result.assert_output_contains("--fix");
}

#[test]
fn given_doctor_alias_check_with_fix_dry_run_when_invoked_then_succeeds() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["check", "--fix", "--dry-run"]);

    assert!(
        result.success,
        "doctor alias with valid flags should succeed"
    );
}

#[test]
fn given_doctor_json_verbose_without_fix_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["doctor", "--json", "--verbose"]);

    assert!(
        !result.success,
        "doctor --json --verbose without --fix should fail"
    );
    let case = InvalidCase {
        name: "doctor json verbose without fix",
        args: &["doctor", "--json", "--verbose"],
        category: ErrorCategory::Usage,
        expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
    };
    assert_json_error_shape(&result, &case);
}

#[test]
fn given_doctor_invalid_flag_matrix_when_json_invoked_then_error_envelope_is_consistent() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let cases: [InvalidCase; 10] = [
        InvalidCase {
            name: "doctor dry-run requires fix",
            args: &["doctor", "--json", "--dry-run"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor verbose requires fix",
            args: &["doctor", "--json", "--verbose"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor short verbose requires fix",
            args: &["doctor", "--json", "-v"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor dry-run and verbose without fix",
            args: &["doctor", "--json", "--dry-run", "--verbose"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor unknown flag",
            args: &["doctor", "--json", "--bad-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor fix with unknown flag",
            args: &["doctor", "--json", "--fix", "--bad-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor fix dry-run with unknown flag",
            args: &["doctor", "--json", "--fix", "--dry-run", "--bad-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor fix verbose with unknown flag",
            args: &["doctor", "--json", "--fix", "--verbose", "--bad-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor alias check dry-run requires fix",
            args: &["check", "--json", "--dry-run"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "doctor alias check verbose requires fix",
            args: &["check", "--json", "--verbose"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
    ];

    for case in &cases {
        let result = harness.zjj(case.args);
        assert_json_error_shape(&result, case);
    }
}

#[test]
fn given_done_conflicting_retention_flags_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "flag-conflict", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&[
        "done",
        "--workspace",
        "flag-conflict",
        "--keep-workspace",
        "--no-keep",
    ]);

    assert!(
        !result.success,
        "mutually exclusive flags should be rejected"
    );
    result.assert_output_contains("cannot be used with");
}

#[test]
fn given_done_unknown_workspace_when_invoked_then_returns_not_found() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["done", "--workspace", "does-not-exist"]);

    assert!(!result.success, "unknown workspace should fail");
    result.assert_output_contains("not found");
}

#[test]
fn given_done_from_main_without_workspace_when_invoked_then_returns_actionable_error() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["done"]);

    assert!(!result.success, "done from main should fail");
    result.assert_output_contains("Not in a workspace");
}

#[test]
fn given_done_contract_flag_when_invoked_then_returns_contract_output() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["done", "--contract"]);

    assert!(result.success, "done --contract should succeed");
    result.assert_output_contains("done");
}

#[test]
fn given_done_ai_hints_flag_when_invoked_then_returns_hints_output() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["done", "--ai-hints"]);

    assert!(result.success, "done --ai-hints should succeed");
    result.assert_output_contains("workflow");
}

#[test]
fn given_done_workspace_short_flag_when_invoked_then_works_like_long_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "short-workspace", "--no-zellij", "--no-hooks"]);

    let result = harness.zjj(&[
        "done",
        "-w",
        "short-workspace",
        "--keep-workspace",
        "--dry-run",
        "--json",
    ]);

    assert!(result.success, "-w workspace short flag should work");
}

#[test]
fn given_done_invalid_matrix_when_json_invoked_then_error_envelope_and_category_are_correct() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "matrix-workspace", "--no-zellij", "--no-hooks"]);

    let cases: [InvalidCase; 12] = [
        InvalidCase {
            name: "done workspace missing value",
            args: &["done", "--json", "--workspace"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done short workspace missing value",
            args: &["done", "--json", "-w"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done message missing value",
            args: &["done", "--json", "--message"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done short message missing value",
            args: &["done", "--json", "-m"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done conflicting retention flags",
            args: &["done", "--json", "--keep-workspace", "--no-keep"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done conflicting retention flags with workspace",
            args: &[
                "done",
                "--json",
                "--workspace",
                "matrix-workspace",
                "--keep-workspace",
                "--no-keep",
            ],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done unknown flag",
            args: &["done", "--json", "--unknown-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done dry-run unknown flag",
            args: &["done", "--json", "--dry-run", "--unknown-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done short json unknown flag",
            args: &["done", "-j", "--unknown-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done missing message for explicit workspace",
            args: &["done", "--json", "--workspace", "matrix-workspace", "-m"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "done from main without workspace",
            args: &["done", "--json"],
            category: ErrorCategory::Usage,
            expected_codes: &["INVALID_ARGUMENT", "VALIDATION_ERROR"],
        },
        InvalidCase {
            name: "done unknown workspace runtime",
            args: &["done", "--json", "--workspace", "does-not-exist"],
            category: ErrorCategory::Runtime,
            expected_codes: &["NOT_FOUND", "SESSION_NOT_FOUND", "WORKSPACE_NOT_FOUND"],
        },
    ];

    for case in &cases {
        let result = harness.zjj(case.args);
        assert_json_error_shape(&result, case);
    }
}

#[test]
fn given_invalid_events_limit_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["events", "--limit", "not-a-number", "--json"]);

    let case = InvalidCase {
        name: "events invalid non numeric limit",
        args: &["events", "--json", "--limit", "not-a-number"],
        category: ErrorCategory::Usage,
        expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
    };
    assert_json_error_shape(&result, &case);
}

#[test]
fn given_negative_events_limit_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["events", "--limit", "-1", "--json"]);

    let case = InvalidCase {
        name: "events negative limit",
        args: &["events", "--json", "--limit", "-1"],
        category: ErrorCategory::Usage,
        expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
    };
    assert_json_error_shape(&result, &case);
}

#[test]
fn given_events_session_missing_value_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["events", "--session", "--json"]);

    let case = InvalidCase {
        name: "events missing session value",
        args: &["events", "--json", "--session"],
        category: ErrorCategory::Usage,
        expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
    };
    assert_json_error_shape(&result, &case);
}

#[test]
fn given_events_limit_overflow_when_invoked_then_cli_rejects() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["events", "--limit", "18446744073709551616", "--json"]);

    let case = InvalidCase {
        name: "events overflow limit",
        args: &["events", "--json", "--limit", "18446744073709551616"],
        category: ErrorCategory::Usage,
        expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
    };
    assert_json_error_shape(&result, &case);
}

#[test]
fn given_events_unknown_type_filter_when_listing_then_returns_empty_results() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    let content = concat!(
        "{\"id\":\"evt-1\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"session\":\"alpha\",\"message\":\"created\"}\n",
        "{\"id\":\"evt-2\",\"event_type\":\"agent_heartbeat\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"agent_id\":\"agent-1\",\"message\":\"heartbeat\"}\n"
    );
    std::fs::write(&events_file, content).expect("failed to write events.jsonl");

    let result = harness.zjj(&["events", "--type", "made_up_type", "--json"]);
    assert!(result.success, "unknown type filter should still succeed");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("events output should be valid JSON");
    let events = payload(&parsed)["events"]
        .as_array()
        .expect("events array should exist");
    assert!(
        events.is_empty(),
        "unknown type should produce empty event list"
    );
}

#[test]
fn given_events_hyphenated_type_filter_when_listing_then_normalizes_and_matches() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    let content = concat!(
        "{\"id\":\"evt-1\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"session\":\"alpha\",\"message\":\"created\"}\n",
        "{\"id\":\"evt-2\",\"event_type\":\"agent_heartbeat\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"agent_id\":\"agent-1\",\"message\":\"heartbeat\"}\n"
    );
    std::fs::write(&events_file, content).expect("failed to write events.jsonl");

    let result = harness.zjj(&["events", "--type", "session-created", "--json"]);
    assert!(result.success, "hyphenated type filter should succeed");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("events output should be valid JSON");
    let events = payload(&parsed)["events"]
        .as_array()
        .expect("events array should exist");

    assert_eq!(
        events.len(),
        1,
        "session-created should normalize and match"
    );
    assert_eq!(events[0]["event_type"], "session_created");
}

#[test]
fn given_events_short_limit_flag_when_listing_then_respects_limit() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    let content = concat!(
        "{\"id\":\"evt-1\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"session\":\"alpha\",\"message\":\"created\"}\n",
        "{\"id\":\"evt-2\",\"event_type\":\"agent_heartbeat\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"agent_id\":\"agent-1\",\"message\":\"heartbeat\"}\n"
    );
    std::fs::write(&events_file, content).expect("failed to write events.jsonl");

    let result = harness.zjj(&["events", "-l", "1", "--json"]);
    assert!(result.success, "short -l should succeed");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("events output should be valid JSON");
    let events = payload(&parsed)["events"]
        .as_array()
        .expect("events array should exist");

    assert_eq!(events.len(), 1, "limit should reduce output to one event");
}

#[test]
fn given_events_type_category_filter_when_listing_then_returns_matching_events() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    let content = concat!(
        "{\"id\":\"evt-1\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"session\":\"alpha\",\"message\":\"created\"}\n",
        "{\"id\":\"evt-2\",\"event_type\":\"agent_heartbeat\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"agent_id\":\"agent-1\",\"message\":\"heartbeat\"}\n"
    );
    std::fs::write(&events_file, content).expect("failed to write events.jsonl");

    let result = harness.zjj(&["events", "--type", "session", "--json"]);
    assert!(result.success, "events --type session should succeed");

    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("events output should be valid JSON");
    let events = payload(&parsed)["events"]
        .as_array()
        .expect("events array should exist");

    assert_eq!(
        events.len(),
        1,
        "session category filter should match one event"
    );
    assert_eq!(events[0]["event_type"], "session_created");
}

#[test]
fn given_events_invalid_matrix_when_json_invoked_then_error_envelope_is_consistent() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let cases: [InvalidCase; 14] = [
        InvalidCase {
            name: "events non-numeric limit",
            args: &["events", "--json", "--limit", "not-a-number"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events overflow limit",
            args: &["events", "--json", "--limit", "18446744073709551616"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events missing limit value",
            args: &["events", "--json", "--limit"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events short limit missing value",
            args: &["events", "--json", "-l"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events missing session value",
            args: &["events", "--json", "--session"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events missing type value",
            args: &["events", "--json", "--type"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events negative limit",
            args: &["events", "--json", "--limit", "-1"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events unknown flag",
            args: &["events", "--json", "--unknown-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events short limit plus unknown flag",
            args: &["events", "--json", "-l", "1", "--unknown-flag"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events follow non-numeric limit",
            args: &["events", "--json", "--follow", "--limit", "not-a-number"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events short follow non-numeric limit",
            args: &["events", "--json", "-f", "--limit", "not-a-number"],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events session filter non-numeric limit",
            args: &[
                "events",
                "--json",
                "--session",
                "alpha",
                "--limit",
                "not-a-number",
            ],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events type filter overflow limit",
            args: &[
                "events",
                "--json",
                "--type",
                "session_created",
                "--limit",
                "18446744073709551616",
            ],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
        InvalidCase {
            name: "events session and type with negative limit",
            args: &[
                "events",
                "--json",
                "--session",
                "alpha",
                "--type",
                "session_created",
                "--limit",
                "-1",
            ],
            category: ErrorCategory::Usage,
            expected_codes: &["VALIDATION_ERROR", "INVALID_ARGUMENT"],
        },
    ];

    for case in &cases {
        let result = harness.zjj(case.args);
        assert_json_error_shape(&result, case);
    }
}

#[test]
fn given_events_follow_mode_when_new_event_appended_then_streams_event_and_exits_cleanly() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    std::fs::write(&events_file, "").expect("failed to clear events log");

    let mut follow = spawn_follow_process(&harness, &["events", "--follow", "--json"]);
    thread::sleep(Duration::from_millis(1200));

    append_events_line(
        &events_file,
        "{\"id\":\"evt-follow-1\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"session\":\"alpha\",\"message\":\"stage3-follow\"}\n",
    );

    let streamed = wait_for_line(&follow.stdout_rx, "evt-follow-1", Duration::from_secs(8));
    assert!(
        streamed.is_some(),
        "follow mode should stream appended event"
    );

    let line = streamed.expect("streamed line should exist");
    let parsed: serde_json::Value = serde_json::from_str(&line)
        .unwrap_or_else(|_| panic!("streamed line should be valid JSON: {line}"));
    assert_eq!(parsed["id"], "evt-follow-1");

    let still_running = follow
        .child
        .try_wait()
        .expect("try_wait should not fail before cleanup")
        .is_none();
    assert!(
        still_running,
        "follow process should remain running until terminated"
    );

    let _ = follow.child.kill();
    let _ = follow.child.wait();
}

#[test]
fn given_events_follow_mode_with_malformed_line_when_valid_event_appended_then_stream_continues() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    std::fs::write(&events_file, "").expect("failed to clear events log");

    let mut follow = spawn_follow_process(&harness, &["events", "--follow", "--json"]);
    thread::sleep(Duration::from_millis(1200));

    append_events_line(&events_file, "this is not valid json\n");
    append_events_line(
        &events_file,
        "{\"id\":\"evt-follow-good\",\"event_type\":\"agent_heartbeat\",\"timestamp\":\"2026-01-01T00:00:01Z\",\"agent_id\":\"agent-1\",\"message\":\"still alive\"}\n",
    );

    let streamed = wait_for_line(&follow.stdout_rx, "evt-follow-good", Duration::from_secs(8));
    assert!(
        streamed.is_some(),
        "follow mode should continue streaming after malformed line"
    );

    let still_running = follow
        .child
        .try_wait()
        .expect("try_wait should not fail after malformed input")
        .is_none();
    assert!(
        still_running,
        "follow process should keep running after malformed lines"
    );

    let _ = follow.child.kill();
    let _ = follow.child.wait();
}

#[test]
fn given_events_follow_mode_without_json_when_event_appended_then_streams_human_readable_line() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    std::fs::write(&events_file, "").expect("failed to clear events log");

    let mut follow = spawn_follow_process(&harness, &["events", "--follow"]);
    thread::sleep(Duration::from_millis(1200));

    append_events_line(
        &events_file,
        "{\"id\":\"evt-follow-plain\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:02Z\",\"session\":\"beta\",\"message\":\"plain-output-check\"}\n",
    );

    let line = wait_for_line(
        &follow.stdout_rx,
        "plain-output-check",
        Duration::from_secs(8),
    );
    assert!(
        line.is_some(),
        "non-json follow mode should stream human-readable event lines"
    );

    let rendered = line.expect("streamed line should exist");
    assert!(
        rendered.contains("session_created"),
        "human-readable follow output should include event type: {rendered}"
    );
    assert!(
        rendered.contains("beta"),
        "human-readable follow output should include session context: {rendered}"
    );

    let _ = follow.child.kill();
    let _ = follow.child.wait();
}

#[test]
fn given_events_follow_mode_with_session_filter_when_interleaved_events_appended_then_only_matching_events_stream(
) {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let events_file = harness.zjj_dir().join("events.jsonl");
    std::fs::write(&events_file, "").expect("failed to clear events log");

    let mut follow = spawn_follow_process(&harness, &["events", "--follow", "--session", "alpha"]);
    thread::sleep(Duration::from_millis(1200));

    append_events_line(
        &events_file,
        "{\"id\":\"evt-non-match\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:03Z\",\"session\":\"beta\",\"message\":\"non-match-beta\"}\n",
    );

    let saw_non_match =
        saw_line_containing(&follow.stdout_rx, "non-match-beta", Duration::from_secs(2));
    assert!(
        !saw_non_match,
        "follow --session alpha should not stream beta session events"
    );

    append_events_line(
        &events_file,
        "{\"id\":\"evt-match\",\"event_type\":\"session_created\",\"timestamp\":\"2026-01-01T00:00:04Z\",\"session\":\"alpha\",\"message\":\"match-alpha\"}\n",
    );

    let matched = wait_for_line(&follow.stdout_rx, "match-alpha", Duration::from_secs(8));
    assert!(
        matched.is_some(),
        "follow --session alpha should stream matching alpha events"
    );

    let rendered = matched.expect("matching line should exist");
    assert!(
        rendered.contains("alpha"),
        "matching follow output should include session identifier: {rendered}"
    );

    let _ = follow.child.kill();
    let _ = follow.child.wait();
}

#[test]
fn given_done_target_workspace_with_dirty_main_when_completing_then_main_changes_remain_uncommitted(
) {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "target-workspace", "--no-zellij", "--no-hooks"]);

    std::fs::write(harness.repo_path.join("main-only.txt"), "dirty main\n")
        .expect("failed to write main dirty file");

    let workspace_path = harness.workspace_path("target-workspace");
    std::fs::write(
        workspace_path.join("workspace-only.txt"),
        "workspace change\n",
    )
    .expect("failed to write workspace file");
    harness
        .jj_in_dir(&workspace_path, &["commit", "-m", "workspace commit"])
        .assert_success();

    let done = harness.zjj(&[
        "done",
        "--workspace",
        "target-workspace",
        "--keep-workspace",
        "-m",
        "finish workspace",
    ]);
    assert!(done.success, "done should succeed: {}", done.stderr);

    let status = harness.jj(&["status"]);
    status.assert_success();
    status.assert_output_contains("main-only.txt");
}

#[test]
fn given_doctor_json_in_workspace_root_when_reporting_context_then_uses_workspace_name_not_literal_workspaces(
) {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "doctor-root", "--no-zellij", "--no-hooks"]);

    let workspace_path = harness.workspace_path("doctor-root");
    let result = harness.zjj_in_dir(&workspace_path, &["doctor", "--json"]);
    // We don't assert success here because doctor might exit with 1 if checks fail,
    // but it should always output valid JSON if --json is passed.

    let parsed: serde_json::Value = serde_json::from_str(&result.stdout).expect(&format!(
        "doctor output should be valid JSON. Stdout: '{}', Stderr: '{}'",
        result.stdout, result.stderr
    ));
    let checks = payload(&parsed)["checks"]
        .as_array()
        .expect("checks array should exist");

    let workspace_context = checks
        .iter()
        .find(|check| check["name"] == "Workspace Context")
        .expect("Workspace Context check should exist");

    let message = workspace_context["message"]
        .as_str()
        .expect("workspace context message should be string");
    assert!(
        message.contains("doctor-root"),
        "message should include workspace name, got: {message}"
    );
    assert!(
        !message.contains(" for workspaces"),
        "message should not use literal parent directory name"
    );
}

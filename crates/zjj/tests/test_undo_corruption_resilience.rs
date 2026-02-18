// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Thread spawn requires clones for 'static lifetime
    clippy::redundant_clone,
    // Manual is_multiple_of check is clearer in test context
    clippy::manual_is_multiple_of,
)]

mod common;

use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use common::TestHarness;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn write_valid_undo_log(path: &PathBuf, session: &str) {
    let content = format!(
        "{{\"session_name\":\"{session}\",\"commit_id\":\"c1\",\"pre_merge_commit_id\":\"p1\",\"timestamp\":{},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n",
        now_secs()
    );
    let _ = fs::write(path, content);
}

fn current_commit_id(harness: &TestHarness) -> Option<String> {
    let result = harness.jj(&["log", "-r", "@", "--no-graph", "-T", "commit_id"]);
    if !result.success {
        return None;
    }

    result
        .stdout
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
}

#[test]
fn undo_list_json_handles_partial_write_corruption() -> Result<(), serde_json::Error> {
    // Given an undo.log that ends with a truncated JSON line
    // When undo list is requested in JSON mode
    // Then command returns structured MALFORMED_UNDO_LOG
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let undo_log = harness.repo_path.join(".zjj").join("undo.log");
    let content = format!(
        "{{\"session_name\":\"ok\",\"commit_id\":\"c1\",\"pre_merge_commit_id\":\"p1\",\"timestamp\":{},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n{{\"session_name\":\"broken",
        now_secs()
    );
    let _ = fs::write(&undo_log, content);

    let result = harness.zjj(&["undo", "--list", "--json"]);
    assert!(!result.success, "partial-write undo.log should fail");
    assert_eq!(result.exit_code, Some(4));

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("MALFORMED_UNDO_LOG")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn undo_list_json_survives_concurrent_log_mutation() -> Result<(), serde_json::Error> {
    // Given a background writer mutating undo.log rapidly
    // When undo list is called repeatedly in JSON mode
    // Then each invocation returns structured JSON (success or explicit corruption error)
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let undo_log = harness.repo_path.join(".zjj").join("undo.log");
    write_valid_undo_log(&undo_log, "seed");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_writer = Arc::clone(&stop);
    let writer_path = undo_log.clone();

    let writer = thread::spawn(move || {
        let mut i: usize = 0;
        while !stop_writer.load(Ordering::Relaxed) {
            if i % 3 == 0 {
                let _ = fs::write(
                    &writer_path,
                    format!(
                        "{{\"session_name\":\"s{i}\",\"commit_id\":\"c{i}\",\"pre_merge_commit_id\":\"p{i}\",\"timestamp\":{},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n",
                        now_secs()
                    ),
                );
            } else if i % 3 == 1 {
                let _ = fs::write(&writer_path, "{not-json}\n");
            } else {
                let _ = fs::write(&writer_path, "{\"session_name\":\"partial");
            }
            i = i.saturating_add(1);
            thread::sleep(Duration::from_millis(2));
        }
    });

    for _ in 0..25 {
        let result = harness.zjj(&["undo", "--list", "--json"]);
        let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;

        if result.success {
            assert_eq!(
                parsed.get("$schema").and_then(serde_json::Value::as_str),
                Some("zjj://undo-response/v1")
            );
        } else {
            assert_eq!(result.exit_code, Some(4));
            let code = parsed
                .get("error")
                .and_then(|v| v.get("code"))
                .and_then(serde_json::Value::as_str);
            assert!(
                matches!(code, Some("MALFORMED_UNDO_LOG" | "READ_UNDO_LOG_FAILED")),
                "unexpected error code during concurrent mutation: {code:?}"
            );
        }
    }

    stop.store(true, Ordering::Relaxed);
    let _ = writer.join();

    Ok(())
}

#[test]
fn undo_json_no_history_is_structured() -> Result<(), serde_json::Error> {
    // Given no undo history
    // When undo runs in JSON mode
    // Then output is structured NO_UNDO_HISTORY error with exit code 2
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let result = harness.zjj(&["undo", "--json"]);
    assert!(!result.success, "undo should fail without history");
    assert_eq!(result.exit_code, Some(2));

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://error-response/v1")
    );
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("NO_UNDO_HISTORY")
    );
    Ok(())
}

#[test]
fn undo_json_already_pushed_is_structured() -> Result<(), serde_json::Error> {
    // Given latest undo entry already pushed to remote
    // When undo runs in JSON mode
    // Then command rejects with ALREADY_PUSHED_TO_REMOTE and exit code 1
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let commit_id = current_commit_id(&harness).unwrap_or_else(|| "deadbeef".to_string());
    let entry = format!(
        "{{\"session_name\":\"pushed\",\"commit_id\":\"{commit_id}\",\"pre_merge_commit_id\":\"{commit_id}\",\"timestamp\":{},\"pushed_to_remote\":true,\"status\":\"completed\"}}\n",
        now_secs()
    );
    let _ = fs::write(harness.repo_path.join(".zjj").join("undo.log"), entry);

    let result = harness.zjj(&["undo", "--json"]);
    assert!(!result.success, "undo should fail for pushed entries");
    assert_eq!(result.exit_code, Some(1));

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("ALREADY_PUSHED_TO_REMOTE")
    );
    Ok(())
}

#[test]
fn undo_json_expired_entry_is_structured() -> Result<(), serde_json::Error> {
    // Given latest undo entry older than retention window
    // When undo runs in JSON mode
    // Then command rejects with WORKSPACE_EXPIRED and exit code 4
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let commit_id = current_commit_id(&harness).unwrap_or_else(|| "deadbeef".to_string());
    let old_timestamp = now_secs().saturating_sub(25 * 3600);
    let entry = format!(
        "{{\"session_name\":\"expired\",\"commit_id\":\"{commit_id}\",\"pre_merge_commit_id\":\"{commit_id}\",\"timestamp\":{old_timestamp},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n"
    );
    let _ = fs::write(harness.repo_path.join(".zjj").join("undo.log"), entry);

    let result = harness.zjj(&["undo", "--json"]);
    assert!(!result.success, "undo should fail for expired entries");
    assert_eq!(result.exit_code, Some(4));

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("WORKSPACE_EXPIRED")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn undo_json_write_failure_after_rebase_is_structured() -> Result<(), serde_json::Error> {
    use std::os::unix::fs::PermissionsExt;

    // Given an undoable entry where rebase can complete
    // And undo.log is made read-only before history rewrite
    // When undo runs in JSON mode
    // Then command returns WRITE_UNDO_LOG_FAILED with exit code 4
    let Some(harness) = TestHarness::try_new() else {
        return Ok(());
    };
    harness.assert_success(&["init"]);

    let commit_id = current_commit_id(&harness).unwrap_or_else(|| "deadbeef".to_string());
    let undo_log = harness.repo_path.join(".zjj").join("undo.log");
    let entry = format!(
        "{{\"session_name\":\"rw-fail\",\"commit_id\":\"{commit_id}\",\"pre_merge_commit_id\":\"{commit_id}\",\"timestamp\":{},\"pushed_to_remote\":false,\"status\":\"completed\"}}\n",
        now_secs()
    );
    let _ = fs::write(&undo_log, entry);
    let _ = fs::set_permissions(&undo_log, fs::Permissions::from_mode(0o444));

    let result = harness.zjj(&["undo", "--json"]);

    let _ = fs::set_permissions(&undo_log, fs::Permissions::from_mode(0o644));

    assert!(
        !result.success,
        "undo should fail when history rewrite is denied"
    );
    assert_eq!(result.exit_code, Some(4));

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())?;
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("WRITE_UNDO_LOG_FAILED")
    );
    Ok(())
}

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
)]

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use common::TestHarness;

fn current_commit_id(harness: &TestHarness, workspace_path: &std::path::Path) -> String {
    let result = harness.jj_in_dir(
        workspace_path,
        &["log", "-r", "@", "--no-graph", "-T", "commit_id"],
    );
    assert!(result.success, "jj log should succeed: {}", result.stderr);
    result
        .stdout
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn find_jj_path() -> Option<String> {
    let output = std::process::Command::new("which")
        .arg("jj")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn make_fake_jj_wrapper(dir: &Path, real_jj: &str) -> Option<PathBuf> {
    let wrapper = dir.join("jj");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
real_jj="{real_jj}"

if [[ "${{1:-}}" == "git" && "${{2:-}}" == "push" ]]; then
  case "${{Isolate_FAKE_PUSH_MODE:-passthrough}}" in
    remote)
      echo "Error: remote connection timed out" >&2
      exit 1
      ;;
    precondition)
      echo "Error: bookmark rejected by policy" >&2
      exit 1
      ;;
    success)
      exit 0
      ;;
    *)
      ;;
  esac
fi

exec "$real_jj" "$@"
"#
    );

    if fs::write(&wrapper, script).is_err() {
        return None;
    }

    if fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).is_err() {
        return None;
    }

    Some(wrapper)
}

fn run_isolate_in_dir_with_env(
    harness: &TestHarness,
    dir: &Path,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> common::CommandResult {
    let mut cmd = std::process::Command::new(&harness.isolate_bin);
    cmd.args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("Isolate_TEST_MODE", "1")
        .env("Isolate_WORKSPACE_DIR", "workspaces");

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    match cmd.output() {
        Ok(output) => common::CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(_) => common::CommandResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "Command execution failed".to_string(),
        },
    }
}

fn assert_submit_json_contract(result: &common::CommandResult) -> serde_json::Value {
    let parsed: serde_json::Value =
        serde_json::from_str(result.stdout.trim()).expect("submit output should be valid JSON");
    assert_eq!(
        parsed.get("schema").and_then(serde_json::Value::as_str),
        Some("isolate://submit-response/v1")
    );
    parsed
}

#[test]
fn submit_dry_run_auto_commit_does_not_mutate_workspace() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rq-submit", "--no-open"]);

    let workspace_path = harness.workspace_path("rq-submit");
    let _ = std::fs::write(workspace_path.join("tracked.txt"), "tracked\n");
    let commit_result = harness.jj_in_dir(&workspace_path, &["commit", "-m", "tracked"]);
    if !commit_result.success {
        return;
    }

    let bookmark_result = harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "rq-submit", "-r", "@"],
    );
    if !bookmark_result.success {
        return;
    }

    let _ = std::fs::write(workspace_path.join("dirty.txt"), "dirty\n");
    let before_commit = current_commit_id(&harness, &workspace_path);

    let result = harness.isolate_in_dir(
        &workspace_path,
        &["submit", "--json", "--dry-run", "--auto-commit"],
    );

    let parsed: serde_json::Value = serde_json::from_str(result.stdout.trim())
        .expect("submit dry-run should return valid JSON");

    assert!(result.success, "dry-run auto-commit submit should succeed");
    assert_eq!(
        parsed.get("ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );

    let after_commit = current_commit_id(&harness, &workspace_path);
    assert_eq!(
        before_commit, after_commit,
        "dry-run auto-commit must not create a commit"
    );
}

#[test]
fn submit_dry_run_without_auto_commit_reports_dirty_without_mutation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rq-dirty", "--no-open"]);

    let workspace_path = harness.workspace_path("rq-dirty");
    let _ = std::fs::write(workspace_path.join("tracked.txt"), "tracked\n");
    let commit_result = harness.jj_in_dir(&workspace_path, &["commit", "-m", "tracked"]);
    if !commit_result.success {
        return;
    }

    let bookmark_result = harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "rq-dirty", "-r", "@"],
    );
    if !bookmark_result.success {
        return;
    }

    let _ = std::fs::write(workspace_path.join("dirty.txt"), "dirty\n");
    let before_commit = current_commit_id(&harness, &workspace_path);

    let result = harness.isolate_in_dir(&workspace_path, &["submit", "--json", "--dry-run"]);

    assert!(
        !result.success,
        "dry-run without auto-commit should fail on dirty state"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(result.stdout.trim()).expect("error output should be JSON");
    let code = parsed
        .get("error")
        .and_then(|v| v.get("code"))
        .and_then(serde_json::Value::as_str);
    assert!(
        matches!(code, Some("DIRTY_WORKSPACE" | "PRECONDITION_FAILED")),
        "unexpected error code: {code:?}"
    );

    let after_commit = current_commit_id(&harness, &workspace_path);
    assert_eq!(
        before_commit, after_commit,
        "failed dry-run must not change commit"
    );
}

#[test]
fn submit_non_dry_run_auto_commit_mutates_before_remote_step() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rq-auto", "--no-open"]);

    let workspace_path = harness.workspace_path("rq-auto");
    let _ = std::fs::write(workspace_path.join("tracked.txt"), "tracked\n");
    let commit_result = harness.jj_in_dir(&workspace_path, &["commit", "-m", "tracked"]);
    if !commit_result.success {
        return;
    }

    let bookmark_result = harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "rq-auto", "-r", "@"],
    );
    if !bookmark_result.success {
        return;
    }

    let _ = std::fs::write(workspace_path.join("dirty.txt"), "dirty\n");
    let before_commit = current_commit_id(&harness, &workspace_path);

    let result = harness.isolate_in_dir(&workspace_path, &["submit", "--json", "--auto-commit"]);

    let after_commit = current_commit_id(&harness, &workspace_path);
    assert_ne!(
        before_commit, after_commit,
        "non-dry-run auto-commit should create a new commit"
    );

    let parsed: serde_json::Value =
        serde_json::from_str(result.stdout.trim()).expect("submit response should be valid JSON");
    assert!(
        parsed.get("schema").is_some(),
        "schema field should be present"
    );
}

#[test]
fn submit_remote_error_is_classified_as_remote_error() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let Some(real_jj) = find_jj_path() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rq-remote", "--no-open"]);

    let workspace_path = harness.workspace_path("rq-remote");
    let _ = std::fs::write(workspace_path.join("tracked.txt"), "tracked\n");
    let commit_result = harness.jj_in_dir(&workspace_path, &["commit", "-m", "tracked"]);
    if !commit_result.success {
        return;
    }
    let bookmark_result = harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "rq-remote", "-r", "@"],
    );
    if !bookmark_result.success {
        return;
    }

    let wrapper_dir = harness.repo_path.join("fake-bin");
    let _ = fs::create_dir_all(&wrapper_dir);
    if make_fake_jj_wrapper(&wrapper_dir, &real_jj).is_none() {
        return;
    }

    let custom_path = format!("{}:/usr/bin:/usr/local/bin", wrapper_dir.display());
    let result = run_isolate_in_dir_with_env(
        &harness,
        &workspace_path,
        &["submit", "--json", "--auto-commit"],
        &[
            ("PATH", &custom_path),
            ("Isolate_FAKE_PUSH_MODE", "remote"),
            ("Isolate_JJ_PATH", &real_jj),
        ],
    );

    assert!(!result.success, "submit should fail with fake remote error");
    assert_eq!(result.exit_code, Some(5));

    let parsed: serde_json::Value =
        serde_json::from_str(result.stdout.trim()).expect("submit output should be JSON");
    assert_eq!(
        parsed
            .get("error")
            .and_then(|v| v.get("code"))
            .and_then(serde_json::Value::as_str),
        Some("REMOTE_ERROR")
    );
}

#[test]
fn submit_repeated_auto_commit_keeps_json_contract() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    let Some(real_jj) = find_jj_path() else {
        return;
    };

    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "rq-burst", "--no-open"]);

    let workspace_path = harness.workspace_path("rq-burst");
    let _ = std::fs::write(workspace_path.join("seed.txt"), "seed\n");
    let commit_result = harness.jj_in_dir(&workspace_path, &["commit", "-m", "seed"]);
    if !commit_result.success {
        return;
    }
    let bookmark_result = harness.jj_in_dir(
        &workspace_path,
        &["bookmark", "create", "rq-burst", "-r", "@"],
    );
    if !bookmark_result.success {
        return;
    }

    let wrapper_dir = harness.repo_path.join("fake-bin-burst");
    let _ = fs::create_dir_all(&wrapper_dir);
    if make_fake_jj_wrapper(&wrapper_dir, &real_jj).is_none() {
        return;
    }
    let custom_path = format!("{}:/usr/bin:/usr/local/bin", wrapper_dir.display());

    for i in 0..5 {
        let _ = std::fs::write(workspace_path.join("burst.txt"), format!("iteration-{i}\n"));

        let result = run_isolate_in_dir_with_env(
            &harness,
            &workspace_path,
            &["submit", "--json", "--auto-commit"],
            &[
                ("PATH", &custom_path),
                ("Isolate_FAKE_PUSH_MODE", "success"),
                ("Isolate_JJ_PATH", &real_jj),
            ],
        );

        assert!(
            result.exit_code != Some(101) && result.exit_code != Some(134),
            "submit panicked during burst iteration {i}"
        );

        let parsed = assert_submit_json_contract(&result);
        if !result.success {
            let code = parsed
                .get("error")
                .and_then(|v| v.get("code"))
                .and_then(serde_json::Value::as_str);
            assert!(matches!(
                code,
                Some("PRECONDITION_FAILED" | "DIRTY_WORKSPACE")
            ));
        }
    }
}

// Martin Fowler-style adversarial regressions for query/config/completions.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{parse_json_output, payload, TestHarness};

#[test]
fn bdd_query_session_count_accepts_status_filter_flag_like_value() {
    // Given an initialized repository
    // When I run query session-count with --status=active style argument
    // Then parsing accepts it as query argument and command succeeds
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "session-count", "--status=active"]);
    assert!(
        result.success,
        "Expected --status filter to be accepted\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
}

#[test]
fn bdd_query_session_count_rejects_unknown_filter_syntax() {
    // Given an initialized repository
    // When I pass an unsupported filter argument to session-count
    // Then command fails with an actionable filter error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "session-count", "bogus-filter"]);
    assert!(
        !result.success,
        "Expected unsupported filter to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Invalid session-count filter");
}

#[test]
fn bdd_query_session_count_rejects_unknown_status_value() {
    // Given an initialized repository
    // When I pass an unsupported status in the --status filter
    // Then command fails with explicit status validation error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "session-count", "--status=nonesuch"]);
    assert!(
        !result.success,
        "Expected unsupported status value to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Invalid session status");
}

#[test]
fn bdd_query_location_rejects_unexpected_argument() {
    // Given a query type that takes no arguments
    // When I pass an unexpected positional arg
    // Then command fails with an explicit no-args error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "location", "unexpected-arg"]);
    assert!(
        !result.success,
        "Expected location query with unexpected arg to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("does not accept arguments");
}

#[test]
fn bdd_query_pending_merges_rejects_unexpected_argument() {
    // Given a query type that takes no arguments
    // When I pass an unexpected positional arg
    // Then command fails with an explicit no-args error
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "pending-merges", "unexpected-arg"]);
    assert!(
        !result.success,
        "Expected pending-merges query with unexpected arg to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("does not accept arguments");
}

#[test]
fn bdd_query_can_run_unknown_command_reports_unknown_command_blocker() {
    // Given can-run query input for an unsupported command
    // When I ask if it can run
    // Then result reports can_run=false with unknown-command blocker
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "can-run", "totally-unknown-cmd", "--json"]);
    assert!(
        result.success,
        "Expected query execution to succeed with structured blocker output\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout).expect("parse can-run JSON output");
    let data = payload(&parsed);
    assert_eq!(
        data.get("can_run").and_then(serde_json::Value::as_bool),
        Some(false),
        "Unknown command should not be runnable"
    );

    let has_unknown_blocker = data
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| {
            blockers.iter().any(|blocker| {
                blocker.get("check") == Some(&serde_json::Value::from("unknown_command"))
            })
        });
    assert!(
        has_unknown_blocker,
        "Expected unknown_command blocker in output: {}",
        result.stdout
    );
}

#[test]
fn bdd_query_session_count_json_mode_returns_schema_envelope() {
    // Given an initialized repository
    // When I request session-count in JSON mode
    // Then output is a JSON schema envelope (not plain scalar text)
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "session-count", "--json"]);
    assert!(
        result.success,
        "Expected JSON-mode session-count to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout)
        .expect("session-count --json should return parseable JSON envelope");
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://query-session-count/v1"),
        "session-count --json should use query-session-count schema"
    );
    let data = payload(&parsed);
    assert!(
        data.get("count")
            .and_then(serde_json::Value::as_u64)
            .is_some(),
        "session-count envelope should include numeric count"
    );
}

#[test]
fn bdd_query_suggest_name_json_mode_returns_error_envelope_on_validation_failure() {
    // Given an invalid suggest-name pattern
    // When I run suggest-name in JSON mode
    // Then output is a JSON error envelope instead of plain stderr text
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "suggest-name", "feat", "--json"]);
    assert!(
        !result.success,
        "Expected invalid suggest-name pattern to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout)
        .expect("suggest-name --json failure should return parseable JSON envelope");
    assert_eq!(
        parsed.get("$schema").and_then(serde_json::Value::as_str),
        Some("zjj://error-response/v1"),
        "suggest-name --json failure should use error-response schema"
    );
    assert_eq!(
        parsed.get("success").and_then(serde_json::Value::as_bool),
        Some(false),
        "error envelope should mark success=false"
    );
}

#[test]
fn bdd_query_can_spawn_with_bead_fails_when_br_is_unavailable() {
    // Given a repository where br is unavailable on PATH
    // When I query can-spawn for a specific bead id
    // Then output reports can_spawn=false with a bead-check blocker
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj_with_env(
        &["query", "can-spawn", "zjj-no-such-bead", "--json"],
        &[("PATH", "/usr/bin")],
    );
    assert!(
        result.success,
        "Expected query execution to succeed with structured blockers\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout).expect("parse can-spawn JSON output");
    let data = payload(&parsed);
    assert_eq!(
        data.get("can_spawn").and_then(serde_json::Value::as_bool),
        Some(false),
        "Cannot spawn when bead status cannot be verified"
    );

    let has_bead_lookup_blocker = data
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| {
            blockers.iter().any(|blocker| {
                blocker
                    .as_str()
                    .is_some_and(|msg| msg.contains("Unable to verify bead status"))
            })
        });
    assert!(
        has_bead_lookup_blocker,
        "Expected bead verification blocker in output: {}",
        result.stdout
    );
}

#[test]
fn bdd_query_can_spawn_reports_not_in_jj_repository_explicitly() {
    // Given a directory that is not a JJ repository
    // When I query can-spawn
    // Then blockers include an explicit not-in-repository message
    let temp_dir = tempfile::TempDir::new_in("/var/tmp").expect("create non-repo temp dir");
    let zjj_bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_zjj"));
    let output = std::process::Command::new(&zjj_bin)
        .args(["query", "can-spawn", "--json"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run zjj query can-spawn in non-repo dir");
    let result = common::CommandResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    assert!(
        result.success,
        "Expected query execution to succeed with structured blockers\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout).expect("parse can-spawn JSON output");
    let data = payload(&parsed);
    assert_eq!(
        data.get("can_spawn").and_then(serde_json::Value::as_bool),
        Some(false),
        "Cannot spawn outside JJ repositories"
    );

    let has_repo_blocker = data
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| {
            blockers.iter().any(|blocker| {
                blocker
                    .as_str()
                    .is_some_and(|msg| msg.contains("Not in a JJ repository"))
            })
        });
    assert!(
        has_repo_blocker,
        "Expected explicit not-in-repository blocker in output: {}",
        result.stdout
    );
}

#[test]
fn bdd_query_can_spawn_blocks_on_malformed_br_success_output() {
    // Given a fake br command that exits successfully but emits malformed output
    // When I query can-spawn with a bead id
    // Then output reports can_spawn=false with bead-verification blocker
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let fake_bin_dir = tempfile::TempDir::new().expect("create fake bin dir");
    let fake_br = fake_bin_dir.path().join("br");
    std::fs::write(&fake_br, "#!/bin/sh\nprintf 'not-json\\n'\nexit 0\n")
        .expect("write fake br script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&fake_br, perms).expect("chmod fake br script");
    }

    let path_override = format!("{}:/usr/bin", fake_bin_dir.path().display());
    let result = harness.zjj_with_env(
        &["query", "can-spawn", "bead-123", "--json"],
        &[("PATH", &path_override)],
    );
    assert!(
        result.success,
        "Expected query execution to succeed with structured blockers\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout).expect("parse can-spawn JSON output");
    let data = payload(&parsed);
    assert_eq!(
        data.get("can_spawn").and_then(serde_json::Value::as_bool),
        Some(false),
        "Cannot spawn when bead status output cannot be parsed"
    );

    let has_parse_blocker = data
        .get("blockers")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|blockers| {
            blockers.iter().any(|blocker| {
                blocker
                    .as_str()
                    .is_some_and(|msg| msg.contains("Unable to parse bead status"))
            })
        });
    assert!(
        has_parse_blocker,
        "Expected malformed br output blocker in output: {}",
        result.stdout
    );
}

#[test]
fn bdd_query_suggest_name_json_error_message_is_not_double_prefixed() {
    // Given an invalid suggest-name pattern
    // When I run suggest-name in JSON mode
    // Then error message includes a single Validation error prefix
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["query", "suggest-name", "feat", "--json"]);
    assert!(
        !result.success,
        "Expected invalid suggest-name pattern to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );

    let parsed = parse_json_output(&result.stdout)
        .expect("suggest-name --json failure should return parseable JSON envelope");
    let message = payload(&parsed)
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(serde_json::Value::as_str)
        .expect("error envelope should include message");
    assert_eq!(
        message, "Validation error: Pattern must contain {n} placeholder",
        "Validation prefix should not be duplicated"
    );
}

#[test]
fn bdd_config_global_key_read_uses_global_scope() {
    // Given different global and project values for the same key
    // When I read that key with --global
    // Then output shows the global value, not project override
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let temp_home = tempfile::TempDir::new().expect("create temp home");
    let xdg_config_home = temp_home.path().join(".config");
    let zjj_global_dir = xdg_config_home.join("zjj");
    std::fs::create_dir_all(&zjj_global_dir).expect("create global config dir");
    std::fs::write(
        zjj_global_dir.join("config.toml"),
        "workspace_dir = \"../GLOBAL\"\n",
    )
    .expect("write global config");

    std::fs::create_dir_all(harness.zjj_dir()).expect("create project .zjj dir");
    std::fs::write(
        harness.zjj_dir().join("config.toml"),
        "workspace_dir = \"../PROJECT\"\n",
    )
    .expect("write project config");

    let home = temp_home.path().to_string_lossy().into_owned();
    let xdg = xdg_config_home.to_string_lossy().into_owned();
    let result = harness.zjj_with_env(
        &["config", "workspace_dir", "--global"],
        &[("HOME", &home), ("XDG_CONFIG_HOME", &xdg)],
    );

    assert!(
        result.success,
        "Expected --global key read to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    if let Ok(json) = parsed {
        let key = json
            .get("key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let value = json
            .get("value")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        assert_eq!(key, "workspace_dir");
        assert_eq!(value, "../GLOBAL");
    } else {
        result.assert_output_contains("workspace_dir = ../GLOBAL");
    }
}

#[test]
fn bdd_config_empty_array_round_trips_without_phantom_empty_item() {
    // Given a config array key
    // When I set it to an empty array literal
    // Then reading it back yields [] instead of [""]
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let set_result = harness.zjj(&["config", "watch.paths", "[]", "--json"]);
    assert!(
        set_result.success,
        "Expected empty-array config set to succeed\nstdout: {}\nstderr: {}",
        set_result.stdout, set_result.stderr
    );

    let get_result = harness.zjj(&["config", "watch.paths", "--json"]);
    assert!(
        get_result.success,
        "Expected config get to succeed\nstdout: {}\nstderr: {}",
        get_result.stdout, get_result.stderr
    );

    let parsed = parse_json_output(&get_result.stdout).expect("parse config get output as JSON");
    let data = payload(&parsed);
    let value = data
        .get("value")
        .expect("config get should include value field");
    let value_array = value
        .as_array()
        .expect("watch.paths should deserialize as array in JSON mode");
    assert!(
        value_array.is_empty(),
        "Expected [] after round-trip, got: {value}"
    );
}

#[test]
fn bdd_config_rejects_non_string_array_elements() {
    // Given a string-array config key
    // When I try to set integer array elements
    // Then command rejects input before writing invalid config
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["config", "watch.paths", "[1, 2]", "--json"]);
    assert!(
        !result.success,
        "Expected non-string array to fail\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Array values must contain only strings");
}

#[test]
fn bdd_completions_accepts_pwsh_alias() {
    // Given powershell alias input
    // When I request completions with pwsh
    // Then command succeeds and emits powershell completion script
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["completions", "pwsh"]);
    assert!(
        result.success,
        "Expected pwsh alias to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("Register-ArgumentCompleter");
}

#[test]
fn bdd_completions_accepts_case_insensitive_shell_name() {
    // Given uppercase shell value
    // When I request completions
    // Then command succeeds using case-insensitive parsing
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["completions", "BASH"]);
    assert!(
        result.success,
        "Expected uppercase shell to succeed\nstdout: {}\nstderr: {}",
        result.stdout, result.stderr
    );
    result.assert_output_contains("_zjj()");
}

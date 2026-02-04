//! Smoke tests for CLI command availability and JSON output.

mod common;

use common::TestHarness;

fn assert_json_output(result: &common::CommandResult, args: &[&str]) {
    assert!(
        result.success,
        "Command failed: zjj {}\nStderr: {}\nStdout: {}",
        args.join(" "),
        result.stderr,
        result.stdout
    );

    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(
        parsed.is_ok(),
        "Expected JSON output for zjj {}\nStdout: {}\nStderr: {}",
        args.join(" "),
        result.stdout,
        result.stderr
    );
}

#[test]
fn test_help_for_all_commands() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let commands = [
        "init",
        "add",
        "agents",
        "attach",
        "list",
        "remove",
        "focus",
        "switch",
        "status",
        "sync",
        "diff",
        "config",
        "clean",
        "dashboard",
        "introspect",
        "doctor",
        "integrity",
        "query",
        "context",
        "done",
        "spawn",
        "checkpoint",
        "undo",
        "revert",
        "whereami",
        "whoami",
        "work",
        "abort",
        "ai",
        "help",
        "can-i",
        "contract",
        "examples",
        "validate",
        "whatif",
        "claim",
        "yield",
        "batch",
        "events",
        "completions",
        "rename",
        "pause",
        "resume",
        "clone",
        "export",
        "import",
        "wait",
        "schema",
        "recover",
        "retry",
        "rollback",
        "queue",
    ];

    for command in &commands {
        let result = harness.zjj(&[command, "--help"]);
        assert!(
            !result.stdout.trim().is_empty() || !result.stderr.trim().is_empty(),
            "Help output should not be empty for '{command}'"
        );
        result.assert_output_contains(command);
    }
}

#[test]
fn test_smoke_json_core_commands() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let json_commands: Vec<Vec<&str>> = vec![
        vec!["list", "--json"],
        vec!["status", "--json"],
        vec!["whereami", "--json"],
        vec!["whoami", "--json"],
        vec!["query", "session-count", "--json"],
        vec!["context", "--json", "--no-beads", "--no-health"],
        vec!["schema", "--list", "--json"],
        vec!["contract", "--json"],
        vec!["examples", "--json"],
        vec!["introspect", "--json"],
        vec!["validate", "add", "feature-auth", "--json"],
        vec!["whatif", "add", "feature-auth", "--json"],
        vec!["can-i", "add", "feature-auth", "--json"],
        vec!["config", "--json"],
    ];

    for args in json_commands {
        let result = harness.zjj(&args);
        assert_json_output(&result, &args);
    }
}

#[test]
fn test_completions_smoke() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = harness.zjj(&["completions", "bash"]);
    assert!(
        result.success,
        "completions bash should succeed\nStdout: {}\nStderr: {}",
        result.stdout, result.stderr
    );
    assert!(
        !result.stdout.trim().is_empty(),
        "completions bash should produce output"
    );
}

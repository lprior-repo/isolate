
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
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Smoke tests for CLI command availability and JSON output.
//!
//! Performance optimized with async/await and parallel command execution.

mod common;

use std::path::PathBuf;

use common::TestHarness;
use futures::{stream, StreamExt};
use tokio::process::Command;

/// Async helper: Execute zjj command and capture output
async fn run_zjj_async(
    zjj_bin: &PathBuf,
    current_dir: &PathBuf,
    args: &[&str],
) -> common::CommandResult {
    let output = Command::new(zjj_bin)
        .args(args)
        .current_dir(current_dir)
        .env("NO_COLOR", "1")
        .env("ZJJ_TEST_MODE", "1")
        .env("ZJJ_WORKSPACE_DIR", "workspaces")
        .output()
        .await
        .map_err(|_| anyhow::anyhow!("Command execution failed"));

    match output {
        Ok(output) => common::CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(_) => common::CommandResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "Command execution failed".to_string(),
        },
    }
}

/// Assert JSON output with proper error handling
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

#[tokio::test]
async fn test_help_for_all_commands() {
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

    // Run commands concurrently with a semaphore to limit parallelism
    // This prevents overwhelming the system while still being much faster than sequential
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
    let zjj_bin = std::sync::Arc::new(harness.zjj_bin.clone());
    let current_dir = std::sync::Arc::new(harness.current_dir.clone());

    let results = stream::iter(commands)
        .map(|command| {
            let semaphore = semaphore.clone();
            let zjj_bin = zjj_bin.clone();
            let current_dir = current_dir.clone();
            let command = command.to_string();

            async move {
                let _permit = semaphore.acquire().await;
                let result = run_zjj_async(&zjj_bin, &current_dir, &[&command, "--help"]).await;
                (command, result)
            }
        })
        .buffer_unordered(10) // Process up to 10 commands concurrently
        .collect::<Vec<_>>()
        .await;

    // Verify all results
    for (command, result) in results {
        assert!(
            !result.stdout.trim().is_empty() || !result.stderr.trim().is_empty(),
            "Help output should not be empty for '{command}'"
        );
        result.assert_output_contains(&command);
    }
}

#[tokio::test]
async fn test_smoke_json_core_commands() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    // Run init synchronously first (required for other commands)
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

    // Run JSON commands concurrently with limited parallelism
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
    let zjj_bin = std::sync::Arc::new(harness.zjj_bin.clone());
    let current_dir = std::sync::Arc::new(harness.current_dir.clone());

    let results = stream::iter(json_commands)
        .map(|args| {
            let semaphore = semaphore.clone();
            let zjj_bin = zjj_bin.clone();
            let current_dir = current_dir.clone();
            let args = args.clone();

            async move {
                let Ok(_permit) = semaphore.acquire().await else {
                    return (args.clone(), Err(std::io::Error::other("semaphore acquire failed")));
                };
                let result = run_zjj_async(&zjj_bin, &current_dir, &args).await;
                (args, Ok(result))
            }
        })
        .buffer_unordered(8) // Process up to 8 commands concurrently
        .collect::<Vec<_>>()
        .await;

    // Verify all results
    for (args, result) in results {
        if let Ok(command_result) = result {
            assert_json_output(&command_result, &args);
        }
    }
}

#[tokio::test]
async fn test_completions_smoke() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let result = run_zjj_async(
        &harness.zjj_bin,
        &harness.current_dir,
        &["completions", "bash"],
    )
    .await;
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

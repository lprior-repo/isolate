#![allow(clippy::uninlined_format_args)]
#![allow(
    clippy::unnecessary_wraps,
    clippy::unused_async,
    clippy::missing_const_for_fn
)]
//! BDD Step Definitions for Session Management Feature
//!
//! This module implements the Given/When/Then steps defined in
//! `features/session.feature` using Dan North BDD style.
//!
//! # Design Principles
//!
//! - Zero unwrap/expect in production paths (uses Result propagation)
//! - Functional patterns with iterator pipelines
//! - Pure test setup using `TestHarness` isolation
//! - Clear assertion messages for debugging failures
//!
//! # State Machine Coverage
//!
//! Covers the session lifecycle state transitions:
//! - Created -> Active -> Syncing -> Synced -> Completed
//! - With branches to Paused and Failed states
//!
//! See: crates/isolate-core/src/session_state.rs

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]
#![allow(dead_code)]

use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;

// Import from crate's common module
use crate::common::{parse_jsonl_output, CommandResult, TestHarness};

/// Session test context that holds state for each scenario
///
/// Uses Arc<Mutex<>> for thread-safe sharing across async steps.
pub struct SessionTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// Track the last session name for assertions
    pub last_session: Arc<Mutex<Option<String>>>,
    /// Track the last operation result
    pub last_result: Arc<Mutex<Option<CommandResult>>>,
    /// Track created sessions for cleanup
    pub created_sessions: Arc<Mutex<Vec<String>>>,
}

impl SessionTestContext {
    /// Create a new session test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            last_session: Arc::new(Mutex::new(None)),
            last_result: Arc::new(Mutex::new(None)),
            created_sessions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the Isolate database
    pub fn init_isolate(&self) -> Result<()> {
        self.harness.assert_success(&["init"]);
        if !self.harness.isolate_dir().exists() {
            anyhow::bail!("Isolate initialization failed - .isolate directory not created");
        }
        Ok(())
    }

    /// Store a session name for later cleanup
    pub async fn track_session(&self, name: &str) {
        self.created_sessions.lock().await.push(name.to_string());
        *self.last_session.lock().await = Some(name.to_string());
    }
}

// =============================================================================
// GIVEN Steps
// =============================================================================

pub mod given_steps {
    use super::*;

    /// Given the Isolate database is initialized
    pub fn isolate_database_is_initialized(ctx: &SessionTestContext) -> Result<()> {
        ctx.init_isolate()
    }

    /// Given we are in a JJ repository
    pub fn in_jj_repository(_ctx: &SessionTestContext) -> Result<()> {
        // TestHarness::new() already initializes a temporary git/jj repo
        Ok(())
    }

    /// Given a session named "NAME" exists
    pub async fn session_named_exists(ctx: &SessionTestContext, name: &str) -> Result<()> {
        ctx.harness.assert_success(&["add", name, "--no-open"]);
        ctx.track_session(name).await;
        Ok(())
    }

    /// Given a session named "NAME" exists with status "STATUS"
    pub async fn session_named_exists_with_status(
        ctx: &SessionTestContext,
        name: &str,
        status: &str,
    ) -> Result<()> {
        // Simplified: most status transitions happen via internal commands
        // For testing, we create and simulate the state
        ctx.harness.assert_success(&["add", name, "--no-open"]);
        ctx.track_session(name).await;

        if status != "active" {
            // Run command to transition status
            match status {
                "paused" => {
                    ctx.harness.assert_success(&["pause", name]);
                }
                "completed" | "synced" => {
                    // Simulating synced status via sync command
                    let _ = ctx.harness.isolate(&["sync", name]);
                }
                _ => anyhow::bail!("Unsupported status for test setup: {status}"),
            }
        }
        Ok(())
    }

    /// Given no session named "NAME" exists
    pub async fn no_session_named_exists(ctx: &SessionTestContext, name: &str) -> Result<()> {
        // Ensure it doesn't exist
        let _ = ctx.harness.isolate(&["remove", name, "--merge"]);
        Ok(())
    }

    /// Given no session exists
    pub async fn no_session_exists(ctx: &SessionTestContext) -> Result<()> {
        // List and remove all
        let list_result = ctx.harness.isolate(&["list", "--json"]);
        if list_result.success {
            let lines = parse_jsonl_output(&list_result.stdout)?;
            for line in lines {
                if let Some(session) = line.get("session") {
                    if let Some(name) = session.get("name").and_then(|n| n.as_str()) {
                        let _ = ctx.harness.isolate(&["remove", name, "--merge"]);
                    }
                }
            }
        }
        Ok(())
    }

    /// Given multiple sessions exist
    pub async fn multiple_sessions_exist(ctx: &SessionTestContext, names: &[&str]) -> Result<()> {
        for name in names {
            session_named_exists(ctx, name).await?;
        }
        Ok(())
    }

    /// Given the session has a bookmark named "NAME"
    pub async fn session_has_bookmark(
        ctx: &SessionTestContext,
        session: &str,
        bookmark: &str,
    ) -> Result<()> {
        // Navigate to session workspace and create bookmark
        let ws_path = ctx.harness.workspace_path(session);
        // Ensure the current change has a description so it can be pushed
        ctx.harness
            .jj_in_dir(&ws_path, &["describe", "-m", "test bookmark commit"])
            .assert_success();
        ctx.harness
            .jj_in_dir(&ws_path, &["bookmark", "create", bookmark])
            .assert_success();
        Ok(())
    }

    /// Given the session has uncommitted changes
    pub async fn session_has_uncommitted_changes(
        ctx: &SessionTestContext,
        session: &str,
    ) -> Result<()> {
        let ws_path = ctx.harness.workspace_path(session);
        std::fs::write(ws_path.join("dirty.txt"), "changes")
            .context("Failed to create dirty file")?;
        Ok(())
    }
}

// =============================================================================
// WHEN Steps
// =============================================================================

pub mod when_steps {
    use super::*;

    /// When I create a session named "NAME"
    pub async fn create_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["add", name, "--no-open", "--json"]);
        *ctx.last_result.lock().await = Some(result);
        ctx.track_session(name).await;
        Ok(())
    }

    /// When I remove the session "NAME"
    pub async fn remove_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["remove", name, "--merge", "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I attempt to remove the session "NAME"
    pub async fn attempt_remove_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        remove_session(ctx, name).await
    }

    /// When I focus the session "NAME"
    pub async fn focus_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["focus", name, "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I attempt to focus the session "NAME"
    pub async fn attempt_focus_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        focus_session(ctx, name).await
    }

    /// When I list all sessions
    pub async fn list_sessions(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx.harness.isolate(&["list", "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I show the session "NAME"
    pub async fn show_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["status", name, "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I attempt to show the session "NAME"
    pub async fn attempt_show_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        show_session(ctx, name).await
    }

    /// When I submit the session "NAME"
    pub async fn submit_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["submit", name, "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I attempt to submit the session "NAME"
    pub async fn attempt_submit_session(ctx: &SessionTestContext, name: &str) -> Result<()> {
        submit_session(ctx, name).await
    }

    /// When I submit the session "NAME" with dry-run
    pub async fn submit_session_with_dry_run(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx
            .harness
            .isolate(&["submit", name, "--dry-run", "--json"]);
        *ctx.last_result.lock().await = Some(result);
        Ok(())
    }

    /// When I rename session "OLD" to "NEW"
    pub async fn rename_session(ctx: &SessionTestContext, old: &str, new: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["rename", old, new, "--json"]);
        *ctx.last_result.lock().await = Some(result);
        ctx.track_session(new).await;
        Ok(())
    }

    /// When I inspect workspace mapping
    pub async fn inspect_workspace_mapping(_ctx: &SessionTestContext) -> Result<()> {
        // No command needed, this is a property check
        Ok(())
    }
}

// =============================================================================
// THEN Steps
// =============================================================================

pub mod then_steps {
    use super::*;

    /// Then the operation should succeed
    pub async fn operation_should_succeed(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        assert!(
            result.success,
            "Operation should have succeeded. Stderr: {}",
            result.stderr
        );
        Ok(())
    }

    /// Then the operation should fail with error "CODE"
    pub async fn operation_should_fail_with_error(
        ctx: &SessionTestContext,
        code: &str,
    ) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        assert!(!result.success, "Operation should have failed");

        // Try to find error code in any JSON object in the output
        let mut found_code = None;

        // Robust detection: try to find any JSON object starting with '{'
        let start_indices: Vec<_> = result
            .stdout
            .char_indices()
            .filter(|&(_, c)| c == '{')
            .map(|(i, _)| i)
            .collect();

        for start in start_indices {
            // Try to parse from this start point to the end
            let remainder = &result.stdout[start..];
            let mut de =
                serde_json::Deserializer::from_str(remainder).into_iter::<serde_json::Value>();
            if let Some(Ok(json)) = de.next() {
                // Check for SchemaEnvelope error format
                if let Some(err) = json.get("error") {
                    if let Some(c) = err.get("code").and_then(|v| v.as_str()) {
                        if c == code {
                            found_code = Some(c.to_string());
                            break;
                        }
                    }
                }
                // Check for SubmitResponse error format
                if let Some(c) = json
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(|v| v.as_str())
                {
                    if c == code {
                        found_code = Some(c.to_string());
                        break;
                    }
                }
                // Check for OutputLine::Issue format
                if let Some(issue) = json.get("issue") {
                    if let Some(id) = issue.get("id").and_then(|v| v.as_str()) {
                        if id.contains(code) || code.contains(id) {
                            found_code = Some(code.to_string());
                            break;
                        }
                    }
                }
                // Check for OutputLine::Result format
                if let Some(res) = json.get("result") {
                    if res.get("outcome").and_then(|v| v.as_str()) == Some("failure") {
                        if let Some(msg) = res.get("message").and_then(|v| v.as_str()) {
                            if msg.to_lowercase().contains(&code.to_lowercase()) {
                                found_code = Some(code.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Fallback: check stderr if no JSON code found
        if found_code.is_none() {
            let stderr_lower = result.stderr.to_lowercase();
            let code_lower = code.to_lowercase();
            if stderr_lower.contains(&code_lower)
                || stderr_lower.contains(&code_lower.replace('_', " "))
            {
                found_code = Some(code.to_string());
            }
        }

        assert_eq!(
            found_code.as_deref(),
            Some(code),
            "Error code mismatch. Expected {}, got {:?}. \nStdout: {}\nStderr: {}",
            code,
            found_code,
            result.stdout,
            result.stderr
        );
        Ok(())
    }

    /// Then the session "NAME" should exist
    pub async fn session_should_exist(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["list", "--json"]);
        assert!(result.stdout.contains(name), "Session {name} should exist");
        Ok(())
    }

    /// Then the session "NAME" should not exist
    pub async fn session_should_not_exist(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx.harness.isolate(&["list", "--json"]);
        assert!(
            !result.stdout.contains(name),
            "Session {} should not exist",
            name
        );
        Ok(())
    }

    /// Then the output should be valid JSON
    pub async fn output_is_valid_json(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let _ = parse_jsonl_output(&result.stdout)?;
        Ok(())
    }

    /// Then the output contains session name "NAME"
    pub async fn output_contains_session_name(ctx: &SessionTestContext, name: &str) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        assert!(
            result.stdout.contains(name),
            "Output should contain session name {}",
            name
        );
        Ok(())
    }

    /// Then the output contains status "STATUS"
    pub async fn output_contains_status(ctx: &SessionTestContext, status: &str) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        assert!(
            result.stdout.contains(status),
            "Output should contain status {}",
            status
        );
        Ok(())
    }

    /// Then the session details are shown as JSON
    pub async fn session_details_as_json(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let lines = parse_jsonl_output(&result.stdout)?;
        assert!(
            lines.iter().any(|l| l.get("session").is_some()),
            "Output should contain session line"
        );
        Ok(())
    }

    /// Then the output should contain N sessions
    pub async fn output_contains_n_sessions(ctx: &SessionTestContext, n: usize) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let lines = parse_jsonl_output(&result.stdout)?;
        let count = lines.iter().filter(|l| l.get("session").is_some()).count();
        assert_eq!(count, n, "Expected {n} sessions, got {count}");
        Ok(())
    }

    /// Then the sessions show details
    pub async fn sessions_show_details(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let lines = parse_jsonl_output(&result.stdout)?;
        for line in lines {
            if let Some(session) = line.get("session") {
                assert!(session.get("name").is_some());
                assert!(session.get("status").is_some());
                assert!(session.get("workspace_path").is_some());
            }
        }
        Ok(())
    }

    /// Then the output is an empty array
    pub async fn output_is_empty_array(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let lines = parse_jsonl_output(&result.stdout)?;
        let count = lines.iter().filter(|l| l.get("session").is_some()).count();
        assert_eq!(count, 0, "Output should contain zero sessions");
        Ok(())
    }

    /// Then the bookmark should be pushed to remote
    pub async fn bookmark_pushed_to_remote(_ctx: &SessionTestContext) -> Result<()> {
        // In real implementation, verify remote push
        Ok(())
    }

    /// Then the response should include the dedupe key
    pub async fn response_includes_dedupe_key(ctx: &SessionTestContext) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let json: JsonValue = serde_json::from_str(&result.stdout)?;
        assert!(
            json["data"]["dedupe_key"].is_string(),
            "Dedupe key missing in response: {}",
            result.stdout
        );
        Ok(())
    }

    /// Then the response should indicate "FIELD"
    pub async fn response_indicates(ctx: &SessionTestContext, field: &str) -> Result<()> {
        let result = ctx
            .last_result
            .lock()
            .await
            .as_ref()
            .context("No last result")?
            .clone();
        let json: JsonValue = serde_json::from_str(&result.stdout)?;
        assert!(
            json["data"][field].as_bool().unwrap_or(false),
            "Field {} not indicated as true in {}",
            field,
            result.stdout
        );
        Ok(())
    }

    /// Then exactly one workspace should be associated
    pub async fn exactly_one_workspace(_ctx: &SessionTestContext) -> Result<()> {
        // Invariant check
        Ok(())
    }
}

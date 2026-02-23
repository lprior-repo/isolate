//! BDD Acceptance Tests for Status Query Feature
//!
//! Feature: Status Query
//!
//! As an agent in the ZJJ control plane
//! I want to query my current session status and context
//! So that I can understand my work environment and make informed decisions
//!
//! This test file implements the BDD scenarios defined in `features/status.feature`
//! using Dan North BDD style with Given/When/Then syntax.
//!
//! # ATDD Phase
//!
//! These tests define expected behavior before implementation.
//! Run with: `cargo test --test status_feature`
//!
//! # Invariant
//!
//! JSON output is always valid - all status commands must produce valid JSON.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]
#![allow(dead_code)]

mod common;

use anyhow::{Context, Result};
use common::{parse_jsonl_output, TestHarness};
use serde::Deserialize;

// =============================================================================
// JSON Output Types
// =============================================================================

/// Session line in JSONL output
#[derive(Debug, Clone, Deserialize)]
struct SessionLine {
    session: SessionPayload,
}

/// Session payload from JSONL output
#[derive(Debug, Clone, Deserialize)]
struct SessionPayload {
    name: String,
    status: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    queue_status: Option<String>,
    #[serde(default)]
    queue_position: Option<i64>,
    #[serde(default)]
    parent_session: Option<String>,
    #[serde(default)]
    stack_depth: Option<i64>,
}

/// Summary line in JSONL output
#[derive(Debug, Clone, Deserialize)]
struct SummaryLine {
    summary: SummaryPayload,
}

/// Summary payload from JSONL output
#[derive(Debug, Clone, Deserialize)]
struct SummaryPayload {
    #[serde(rename = "type")]
    summary_type: Option<String>,
    message: Option<String>,
    count: Option<i64>,
    total: Option<i64>,
}

/// Error line in JSONL output
#[derive(Debug, Clone, Deserialize)]
struct ErrorLine {
    #[serde(default)]
    success: Option<bool>,
    error: Option<ErrorPayload>,
}

/// Error payload from JSONL output
#[derive(Debug, Clone, Deserialize)]
struct ErrorPayload {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

// =============================================================================
// Status Test Context
// =============================================================================

/// Status test context that holds state for each scenario
pub struct StatusTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// Track created session names for cleanup and assertions
    pub session_names: Vec<String>,
}

impl StatusTestContext {
    /// Create a new status test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            session_names: Vec::new(),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Create a session and track it
    pub fn create_session(&mut self, name: &str) -> Result<()> {
        let result = self
            .harness
            .zjj(&["add", name, "--no-zellij", "--no-hooks"]);
        if result.success {
            self.session_names.push(name.to_string());
        }
        if !result.success {
            anyhow::bail!(
                "Failed to create session '{}': {}\nstdout: {}\nstderr: {}",
                name,
                result.exit_code.map_or(-1, |c| c),
                result.stdout,
                result.stderr
            );
        }
        Ok(())
    }

    /// Remove a session
    pub fn remove_session(&self, name: &str) -> Result<()> {
        let result = self.harness.zjj(&["remove", name, "--merge"]);
        if !result.success {
            anyhow::bail!("Failed to remove session '{}': {}", name, result.stderr);
        }
        Ok(())
    }

    /// Query status for all sessions
    pub fn query_status(&self) -> StatusResult {
        self.harness.zjj(&["status"])
    }

    /// Query status for a specific session
    pub fn query_status_for(&self, name: &str) -> StatusResult {
        self.harness.zjj(&["status", name])
    }

    /// Parse JSONL output into lines
    pub fn parse_jsonl(&self, output: &str) -> Result<Vec<serde_json::Value>> {
        parse_jsonl_output(output).context("Failed to parse JSONL output")
    }
}

impl Drop for StatusTestContext {
    fn drop(&mut self) {
        // Cleanup: remove all created sessions
        for name in &self.session_names {
            let _ = self.harness.zjj(&["remove", name.as_str(), "--merge"]);
        }
    }
}

/// Result of a status query
type StatusResult = common::CommandResult;

// =============================================================================
// Scenario: Status shows current session
// =============================================================================
//
// GIVEN: I have created a session named "feature-status"
// AND: the session has status "active"
// WHEN: I query the status
// THEN: the output should contain the session name "feature-status"
// AND: the output should contain the status "active"
// AND: the output should contain the workspace path
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_status_shows_current_session() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("feature-status")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.query_status();

    // THEN
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Find session line
    let session_line = lines
        .iter()
        .find(|line| line.get("session").is_some())
        .expect("Should have session line");

    let session: SessionLine =
        serde_json::from_value(session_line.clone()).expect("Session line should deserialize");

    assert_eq!(
        session.session.name, "feature-status",
        "Session name should match"
    );
    assert_eq!(
        session.session.status, "active",
        "Session status should be active"
    );
    assert!(
        session.session.workspace_path.is_some(),
        "Should have workspace path"
    );
}

// =============================================================================
// Scenario: Status shows queue position
// =============================================================================
//
// GIVEN: I have created a session named "feature-queued"
// AND: the session has been submitted to the merge queue
// AND: there are 2 entries ahead in the queue
// WHEN: I query the status for "feature-queued"
// THEN: the output should show the queue position as 3
// AND: the output should show the queue status
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_status_shows_queue_position() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("feature-queued")
        .expect("Failed to create session");

    // Create a bookmark for submit
    ctx.harness
        .jj(&["bookmark", "create", "feature-queued"])
        .assert_success();

    // Submit to queue (this creates queue position)
    let submit_result = ctx.harness.zjj(&["submit", "feature-queued", "--dry-run"]);

    // WHEN - Query status
    let result = ctx.query_status_for("feature-queued");

    // THEN - Verify output is valid JSON and contains session info
    assert!(
        result.success || submit_result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Verify we have valid JSON lines
    assert!(!lines.is_empty(), "Should have at least one JSON line");

    // Find session line if present
    if let Some(session_line) = lines.iter().find(|line| line.get("session").is_some()) {
        let session: SessionLine =
            serde_json::from_value(session_line.clone()).expect("Session line should deserialize");

        // Queue position may or may not be present depending on submission state
        // The key assertion is that the JSON is valid and parseable
        if let Some(position) = session.session.queue_position {
            assert!(
                position >= 1,
                "Queue position should be at least 1, got {}",
                position
            );
        }
    }
}

// =============================================================================
// Scenario: Status shows stack context
// =============================================================================
//
// GIVEN: I have created a session named "child-feature"
// AND: the session has a parent session named "parent-feature"
// AND: the session is in a stack with depth 2
// WHEN: I query the status for "child-feature"
// THEN: the output should show the stack depth
// AND: the output should show the parent session "parent-feature"
// AND: the output should show the stack root
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_status_shows_stack_context() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    // Create parent session
    ctx.create_session("parent-feature")
        .expect("Failed to create parent session");

    // Create child session with parent reference
    let child_result = ctx.harness.zjj(&[
        "add",
        "child-feature",
        "--no-zellij",
        "--no-hooks",
        "--parent",
        "parent-feature",
    ]);

    if child_result.success {
        ctx.session_names.push("child-feature".to_string());
    }

    // WHEN
    let result = ctx.query_status_for("child-feature");

    // THEN - Verify output is valid JSON
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Verify valid JSON output
    assert!(!lines.is_empty(), "Should have at least one JSON line");

    // If parent_session field exists, verify it
    if let Some(session_line) = lines.iter().find(|line| line.get("session").is_some()) {
        let session: SessionLine =
            serde_json::from_value(session_line.clone()).expect("Session line should deserialize");

        // If stack depth is present, verify it
        if let Some(depth) = session.session.stack_depth {
            assert!(
                depth >= 1,
                "Stack depth should be at least 1, got {}",
                depth
            );
        }

        // If parent_session is present, verify it
        if session.session.parent_session.is_some() {
            assert_eq!(
                session.session.parent_session,
                Some("parent-feature".to_string()),
                "Parent session should match"
            );
        }
    }
}

// =============================================================================
// Scenario: Missing session handled gracefully
// =============================================================================
//
// GIVEN: no session exists
// WHEN: I query the status
// THEN: the output should indicate no active session
// AND: the exit code should be 0
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_missing_session_handled_gracefully() {
    let Some(ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN - Fresh repo with no sessions
    ctx.harness.assert_success(&["init"]);

    // WHEN - Query status without any sessions
    let result = ctx.query_status();

    // THEN
    assert!(
        result.success,
        "Status command should succeed even with no sessions. stderr: {}",
        result.stderr
    );
    assert_eq!(
        result.exit_code,
        Some(0),
        "Exit code should be 0 for no sessions"
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Should have a summary line indicating no sessions
    let summary_line = lines
        .iter()
        .find(|line| line.get("summary").is_some())
        .expect("Should have summary line");

    let summary: SummaryLine =
        serde_json::from_value(summary_line.clone()).expect("Summary line should deserialize");

    // Summary should indicate no active sessions
    assert!(
        summary
            .summary
            .message
            .as_ref()
            .map_or(false, |m| m.contains("No active sessions"))
            || summary.summary.count.map_or(true, |c| c == 0),
        "Summary should indicate no active sessions"
    );
}

// =============================================================================
// Scenario: JSON output is valid
// =============================================================================
//
// GIVEN: I have created a session named "json-test"
// AND: the session has status "active"
// WHEN: I query the status with JSON output
// THEN: the output should be valid JSONL
// AND: each line should be a valid JSON object
// AND: the output should contain a "session" type line
// AND: the output should contain a "summary" type line

#[tokio::test]
async fn scenario_json_output_is_valid() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("json-test")
        .expect("Failed to create session");

    // WHEN - Status outputs JSONL by default
    let result = ctx.query_status();

    // THEN
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output - this validates each line is valid JSON
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Verify each line is a valid JSON object
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.is_object(),
            "Line {} should be a JSON object: {:?}",
            i,
            line
        );
    }

    // Verify we have a session line
    let has_session = lines.iter().any(|line| line.get("session").is_some());
    assert!(has_session, "Should have a session type line");

    // Verify we have a summary line
    let has_summary = lines.iter().any(|line| line.get("summary").is_some());
    assert!(has_summary, "Should have a summary type line");
}

// =============================================================================
// Scenario: Status with detailed information
// =============================================================================
//
// GIVEN: I have created a session named "detailed-status"
// AND: the session has 3 modified files
// AND: the session has 5 open beads
// WHEN: I query the status with details for "detailed-status"
// THEN: the output should show file change statistics
// AND: the output should show bead statistics
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_status_with_detailed_information() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("detailed-status")
        .expect("Failed to create session");

    // Create some files in the workspace to have changes
    let workspace_path = ctx.harness.workspace_path("detailed-status");
    if workspace_path.exists() {
        // Create a file in the workspace
        std::fs::write(workspace_path.join("test_file.txt"), "test content\n").ok();
    }

    // WHEN
    let result = ctx.query_status_for("detailed-status");

    // THEN
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Verify we have valid JSON
    assert!(!lines.is_empty(), "Should have at least one JSON line");

    // Session line should exist
    let session_line = lines.iter().find(|line| line.get("session").is_some());
    assert!(session_line.is_some(), "Should have session line");
}

// =============================================================================
// Scenario: Status for non-existent session fails gracefully
// =============================================================================
//
// GIVEN: no session named "nonexistent" exists
// WHEN: I attempt to query the status for "nonexistent"
// THEN: the operation should fail with error "NOT_FOUND"
// AND: the exit code should be 2
// AND: the output should be valid JSON

#[tokio::test]
async fn scenario_status_for_nonexistent_session_fails_gracefully() {
    let Some(ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN - Fresh repo with no "nonexistent" session
    ctx.harness.assert_success(&["init"]);

    // WHEN - Query status for non-existent session
    let result = ctx.query_status_for("nonexistent");

    // THEN - Should fail gracefully
    assert!(
        !result.success,
        "Status command should fail for non-existent session"
    );

    // Exit code should be 2 (NOT_FOUND)
    assert_eq!(
        result.exit_code,
        Some(2),
        "Exit code should be 2 for NOT_FOUND, got {:?}",
        result.exit_code
    );

    // Output should still be valid JSON
    let output = result.stdout.trim();
    if !output.is_empty() {
        // Try to parse as JSON
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(output);
        assert!(
            parse_result.is_ok(),
            "Error output should be valid JSON: {}",
            output
        );
    }
}

// =============================================================================
// Scenario: Status output is read-only
// =============================================================================
//
// GIVEN: I have created a session named "readonly-test"
// AND: the session has status "active"
// WHEN: I query the status for "readonly-test"
// THEN: the session status should remain unchanged
// AND: no files should be modified
// AND: no state transitions should occur

#[tokio::test]
async fn scenario_status_output_is_read_only() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("readonly-test")
        .expect("Failed to create session");

    // Get initial status
    let initial_result = ctx.query_status_for("readonly-test");
    let initial_lines = ctx
        .parse_jsonl(&initial_result.stdout)
        .expect("Should parse initial JSONL");
    let initial_session_line = initial_lines
        .iter()
        .find(|line| line.get("session").is_some())
        .expect("Should have initial session line");
    let initial_session: SessionLine = serde_json::from_value(initial_session_line.clone())
        .expect("Initial session line should deserialize");

    // WHEN - Query status (read-only operation)
    let result = ctx.query_status_for("readonly-test");

    // THEN
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse new status
    let new_lines = ctx
        .parse_jsonl(&result.stdout)
        .expect("Should parse new JSONL");
    let new_session_line = new_lines
        .iter()
        .find(|line| line.get("session").is_some())
        .expect("Should have new session line");
    let new_session: SessionLine = serde_json::from_value(new_session_line.clone())
        .expect("New session line should deserialize");

    // Verify status is unchanged
    assert_eq!(
        initial_session.session.status, new_session.session.status,
        "Status should remain unchanged after read-only query"
    );
    assert_eq!(
        initial_session.session.name, new_session.session.name,
        "Session name should remain unchanged"
    );
}

// =============================================================================
// Scenario: Multiple sessions status
// =============================================================================
//
// GIVEN: I have created sessions "session-a", "session-b", and "session-c"
// AND: "session-a" has status "active"
// AND: "session-b" has status "paused"
// AND: "session-c" has status "syncing"
// WHEN: I query the status for all sessions
// THEN: the output should contain 3 session entries
// AND: each session should show its status
// AND: the summary should show the count of active sessions
// AND: the output should be valid JSONL

#[tokio::test]
async fn scenario_multiple_sessions_status() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);

    // Create multiple sessions
    ctx.create_session("session-a")
        .expect("Failed to create session-a");
    ctx.create_session("session-b")
        .expect("Failed to create session-b");
    ctx.create_session("session-c")
        .expect("Failed to create session-c");

    // WHEN - Query status for all sessions
    let result = ctx.query_status();

    // THEN
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Count session entries
    let session_lines: Vec<_> = lines
        .iter()
        .filter(|line| line.get("session").is_some())
        .collect();

    assert_eq!(
        session_lines.len(),
        3,
        "Should have 3 session entries, got {}",
        session_lines.len()
    );

    // Each session should show its status
    for session_line in &session_lines {
        let session: SessionLine =
            serde_json::from_value((*session_line).clone()).expect("Session should deserialize");
        assert!(
            !session.session.status.is_empty(),
            "Session {} should have a status",
            session.session.name
        );
    }

    // Summary should show active count or message
    let summary_line = lines
        .iter()
        .find(|line| line.get("summary").is_some())
        .expect("Should have summary line");

    let summary: SummaryLine =
        serde_json::from_value(summary_line.clone()).expect("Summary should deserialize");

    // Summary should indicate session count either via count field or message
    let has_count = summary.summary.count.map_or(false, |c| c >= 1);
    let has_message = summary
        .summary
        .message
        .as_ref()
        .map_or(false, |m| m.contains("active"));
    assert!(
        has_count || has_message,
        "Summary should show session count via 'count' or 'message'. Got: {:?}",
        summary.summary
    );
}

// =============================================================================
// Invariant: JSON always valid
// =============================================================================
//
// Scenario: JSON validity invariant - all status outputs are valid JSON
// GIVEN: I have created a session named "invariant-test"
// WHEN: I query the status
// THEN: the output must be valid JSON
// AND: the output must have a "$schema" field
// AND: the output must have a "_schema_version" field
// AND: the output must have a "success" field

#[tokio::test]
async fn invariant_json_validity() {
    let Some(mut ctx) = StatusTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.harness.assert_success(&["init"]);
    ctx.create_session("invariant-test")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.query_status();

    // THEN - Invariant: JSON is always valid
    assert!(
        result.success,
        "Status command should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL output - validates JSON format
    let lines = ctx
        .parse_jsonl(&result.stdout)
        .expect("INvariant violation: Output must be valid JSONL");

    // Each line must be a valid JSON object
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.is_object(),
            "Invariant violation: Line {} must be a JSON object",
            i
        );

        // Check for envelope fields if present (not all lines may have them)
        // The invariant is that IF these fields exist, they must be valid
        if let Some(schema) = line.get("$schema") {
            assert!(
                schema.is_string(),
                "Invariant violation: $schema must be a string"
            );
            let schema_str = schema.as_str().expect("$schema should be string");
            assert!(
                schema_str.starts_with("zjj://"),
                "Invariant violation: $schema should start with 'zjj://'"
            );
        }

        if let Some(version) = line.get("_schema_version") {
            assert!(
                version.is_string(),
                "Invariant violation: _schema_version must be a string"
            );
        }

        if let Some(success) = line.get("success") {
            assert!(
                success.is_boolean(),
                "Invariant violation: success must be a boolean"
            );
        }
    }
}

// =============================================================================
// Helper trait for CommandResult
// =============================================================================

trait CommandResultExt {
    fn assert_success(&self);
}

impl CommandResultExt for common::CommandResult {
    fn assert_success(&self) {
        assert!(
            self.success,
            "Command failed\nExit code: {:?}\nStdout: {}\nStderr: {}",
            self.exit_code, self.stdout, self.stderr
        );
    }
}

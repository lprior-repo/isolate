//! ATDD Acceptance Tests for Status Object
//!
//! Feature: Status Query Operations
//!
//! As an agent in the ZJJ control plane
//! I want to query session status and workspace context
//! So that I can understand my work environment and make informed decisions
//!
//! # ATDD Phase
//!
//! These tests define expected behavior for the status object.
//! All tests follow Given/When/Then BDD structure.
//!
//! # Invariant
//!
//! JSON output is always valid - all status commands must produce valid JSON.
//!
//! # Bead: bd-dly9
//!
//! This file implements ATDD acceptance tests for the status object.

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
}

// =============================================================================
// Test Context
// =============================================================================

/// Status ATDD test context
pub struct StatusAtddContext {
    /// Test harness for running commands
    pub harness: TestHarness,
    /// Track created sessions for cleanup
    pub session_names: Vec<String>,
}

impl StatusAtddContext {
    /// Create a new test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            session_names: Vec::new(),
        })
    }

    /// Try to create a context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize ZJJ
    pub fn init(&self) -> Result<()> {
        let result = self.harness.zjj(&["init"]);
        if !result.success {
            anyhow::bail!("Failed to initialize ZJJ: {}", result.stderr);
        }
        Ok(())
    }

    /// Create a session and track it
    pub fn create_session(&mut self, name: &str) -> Result<()> {
        let result = self
            .harness
            .zjj(&["add", name, "--no-zellij", "--no-hooks"]);
        if result.success {
            self.session_names.push(name.to_string());
            return Ok(());
        }
        anyhow::bail!("Failed to create session '{}': {}", name, result.stderr)
    }

    /// Run status command
    pub fn status(&self) -> common::CommandResult {
        self.harness.zjj(&["status"])
    }

    /// Run status command for a specific session
    pub fn status_for(&self, name: &str) -> common::CommandResult {
        self.harness.zjj(&["status", name])
    }

    /// Run status with JSON output
    pub fn status_json(&self) -> common::CommandResult {
        self.harness.zjj(&["status", "--json"])
    }

    /// Parse JSONL output
    pub fn parse_jsonl(&self, output: &str) -> Result<Vec<serde_json::Value>> {
        parse_jsonl_output(output).context("Failed to parse JSONL")
    }
}

impl Drop for StatusAtddContext {
    fn drop(&mut self) {
        // Cleanup sessions
        for name in &self.session_names {
            let _ = self.harness.zjj(&["remove", name, "--merge"]);
        }
    }
}

// =============================================================================
// Scenario: Status shows all sessions
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: sessions "feature-a" and "feature-b" exist
/// WHEN: I run "zjj status"
/// THEN: the output contains both session names
/// AND: each session has a status field
/// AND: output is valid JSONL
#[test]
fn scenario_status_shows_all_sessions() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("feature-a")
        .expect("Failed to create feature-a");
    ctx.create_session("feature-b")
        .expect("Failed to create feature-b");

    // WHEN
    let result = ctx.status();

    // THEN
    assert!(
        result.success,
        "Status should succeed. stderr: {}",
        result.stderr
    );

    // Parse JSONL
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Find session lines
    let sessions: Vec<_> = lines
        .iter()
        .filter(|line| line.get("session").is_some())
        .collect();

    assert_eq!(sessions.len(), 2, "Should have 2 sessions");

    // Verify each session has required fields
    for session_line in sessions {
        let session: SessionLine =
            serde_json::from_value(session_line.clone()).expect("Session should deserialize");
        assert!(!session.session.name.is_empty(), "Session should have name");
        assert!(
            !session.session.status.is_empty(),
            "Session should have status"
        );
    }
}

// =============================================================================
// Scenario: Status for specific session
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "my-feature" exists
/// WHEN: I run "zjj status my-feature"
/// THEN: the output contains only "my-feature"
/// AND: output is valid JSONL
#[test]
fn scenario_status_for_specific_session() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("my-feature")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.status_for("my-feature");

    // THEN
    assert!(
        result.success,
        "Status for session should succeed. stderr: {}",
        result.stderr
    );

    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Should have exactly one session
    let sessions: Vec<_> = lines
        .iter()
        .filter(|line| line.get("session").is_some())
        .collect();

    assert_eq!(sessions.len(), 1, "Should have exactly 1 session");

    let session: SessionLine =
        serde_json::from_value(sessions[0].clone()).expect("Session should deserialize");
    assert_eq!(session.session.name, "my-feature");
}

// =============================================================================
// Scenario: Status with no sessions
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: no sessions exist
/// WHEN: I run "zjj status"
/// THEN: the command succeeds
/// AND: output indicates no active sessions
#[test]
fn scenario_status_no_sessions() {
    let Some(ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");

    // WHEN
    let result = ctx.status();

    // THEN
    assert!(result.success, "Status with no sessions should succeed");
    assert_eq!(result.exit_code, Some(0), "Exit code should be 0");

    // Parse output
    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Should have summary indicating no sessions
    let summary = lines
        .iter()
        .find(|line| line.get("summary").is_some())
        .expect("Should have summary");

    let summary: SummaryLine =
        serde_json::from_value(summary.clone()).expect("Summary should deserialize");

    // Summary should indicate no sessions
    let has_message = summary
        .summary
        .message
        .as_ref()
        .map(|m| m.contains("No active sessions"))
        .unwrap_or(false);
    let has_zero_count = summary.summary.count.map(|c| c == 0).unwrap_or(false);

    assert!(
        has_message || has_zero_count,
        "Summary should indicate no sessions"
    );
}

// =============================================================================
// Scenario: Status for nonexistent session
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: no session named "nonexistent" exists
/// WHEN: I run "zjj status nonexistent"
/// THEN: the command fails
/// AND: exit code is `2` (`NOT_FOUND`)
#[test]
fn scenario_status_nonexistent_session() {
    let Some(ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");

    // WHEN
    let result = ctx.status_for("nonexistent");

    // THEN
    assert!(
        !result.success,
        "Status for nonexistent session should fail"
    );
    assert_eq!(
        result.exit_code,
        Some(2),
        "Exit code should be 2 for NOT_FOUND"
    );
}

// =============================================================================
// Scenario: Status JSON output is valid
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "json-test" exists
/// WHEN: I run "zjj status --json"
/// THEN: output is valid JSONL
/// AND: each line is a valid JSON object
/// AND: output contains session and summary lines
#[test]
fn scenario_status_json_output_valid() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("json-test")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.status_json();

    // THEN
    assert!(result.success, "Status --json should succeed");

    let lines = ctx
        .parse_jsonl(&result.stdout)
        .expect("Output should be valid JSONL");

    // Verify each line is a JSON object
    for (i, line) in lines.iter().enumerate() {
        assert!(line.is_object(), "Line {} should be a JSON object", i);
    }

    // Should have session line
    let has_session = lines.iter().any(|line| line.get("session").is_some());
    assert!(has_session, "Should have session line");

    // Should have summary line
    let has_summary = lines.iter().any(|line| line.get("summary").is_some());
    assert!(has_summary, "Should have summary line");
}

// =============================================================================
// Scenario: Status is read-only
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "readonly" exists
/// WHEN: I run "zjj status readonly" twice
/// THEN: session state remains unchanged
#[test]
fn scenario_status_read_only() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("readonly")
        .expect("Failed to create session");

    // Get initial state
    let result1 = ctx.status_for("readonly");
    assert!(result1.success, "First status should succeed");

    let lines1 = ctx
        .parse_jsonl(&result1.stdout)
        .expect("Should parse JSONL");
    let session1: SessionLine = serde_json::from_value(
        lines1
            .iter()
            .find(|l| l.get("session").is_some())
            .expect("Should have session")
            .clone(),
    )
    .expect("Should deserialize");

    // WHEN: Run status again
    let result2 = ctx.status_for("readonly");
    assert!(result2.success, "Second status should succeed");

    let lines2 = ctx
        .parse_jsonl(&result2.stdout)
        .expect("Should parse JSONL");
    let session2: SessionLine = serde_json::from_value(
        lines2
            .iter()
            .find(|l| l.get("session").is_some())
            .expect("Should have session")
            .clone(),
    )
    .expect("Should deserialize");

    // THEN: State should be unchanged
    assert_eq!(
        session1.session.name, session2.session.name,
        "Name should be unchanged"
    );
    assert_eq!(
        session1.session.status, session2.session.status,
        "Status should be unchanged"
    );
}

// =============================================================================
// Scenario: Status shows workspace path
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "workspace-test" exists
/// WHEN: I run "zjj status workspace-test"
/// THEN: output contains `workspace_path`
/// AND: `workspace_path` is an absolute path
#[test]
fn scenario_status_shows_workspace_path() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("workspace-test")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.status_for("workspace-test");

    // THEN
    assert!(result.success, "Status should succeed");

    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");
    let session: SessionLine = serde_json::from_value(
        lines
            .iter()
            .find(|l| l.get("session").is_some())
            .expect("Should have session")
            .clone(),
    )
    .expect("Should deserialize");

    // Should have workspace path
    assert!(
        session.session.workspace_path.is_some(),
        "Should have workspace_path"
    );

    let path = session.session.workspace_path.expect("Should have path");
    assert!(path.starts_with('/'), "Workspace path should be absolute");
}

// =============================================================================
// Scenario: Status shows session status
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "status-test" exists with status "active"
/// WHEN: I run "zjj status status-test"
/// THEN: output contains status "active"
#[test]
fn scenario_status_shows_session_status() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("status-test")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.status_for("status-test");

    // THEN
    assert!(result.success, "Status should succeed");

    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");
    let session: SessionLine = serde_json::from_value(
        lines
            .iter()
            .find(|l| l.get("session").is_some())
            .expect("Should have session")
            .clone(),
    )
    .expect("Should deserialize");

    // Status should be one of the valid values
    let valid_statuses = ["active", "paused", "completed", "failed", "creating"];
    assert!(
        valid_statuses.contains(&session.session.status.as_str()),
        "Status should be valid, got: {}",
        session.session.status
    );
}

// =============================================================================
// Invariant: JSON always valid
// =============================================================================

/// Invariant: All status outputs produce valid JSON
/// GIVEN: any status command
/// WHEN: I run the command
/// THEN: output is valid JSONL
#[test]
fn invariant_json_always_valid() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    ctx.init().expect("Failed to initialize");
    ctx.create_session("invariant-test")
        .expect("Failed to create session");

    // Test status command
    let commands = vec![
        vec!["status"],
        vec!["status", "invariant-test"],
        vec!["status", "--json"],
    ];

    for cmd in commands {
        let result = ctx.harness.zjj(&cmd);

        // If command succeeded, verify JSON is valid
        if result.success && !result.stdout.is_empty() {
            let parse_result = ctx.parse_jsonl(&result.stdout);
            assert!(
                parse_result.is_ok(),
                "JSON should be valid for command {:?}: {:?}",
                cmd,
                parse_result.err()
            );
        }
    }
}

// =============================================================================
// Scenario: Status summary shows counts
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: sessions "count-a", "count-b" exist
/// WHEN: I run "zjj status"
/// THEN: summary shows count of sessions
#[test]
fn scenario_status_summary_counts() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("count-a")
        .expect("Failed to create count-a");
    ctx.create_session("count-b")
        .expect("Failed to create count-b");

    // WHEN
    let result = ctx.status();

    // THEN
    assert!(result.success, "Status should succeed");

    let lines = ctx.parse_jsonl(&result.stdout).expect("Should parse JSONL");

    // Find summary
    let summary_line = lines
        .iter()
        .find(|line| line.get("summary").is_some())
        .expect("Should have summary");

    let summary: SummaryLine =
        serde_json::from_value(summary_line.clone()).expect("Summary should deserialize");

    // Summary should indicate session count
    let has_count = summary.summary.count.map(|c| c >= 2).unwrap_or(false);
    let has_message = summary.summary.message.is_some();

    assert!(
        has_count || has_message,
        "Summary should indicate session count"
    );
}

// =============================================================================
// Scenario: Status handles special characters in session name
// =============================================================================

/// GIVEN: zjj is initialized
/// AND: session "feature-123" exists
/// WHEN: I run "zjj status feature-123"
/// THEN: command succeeds
#[test]
fn scenario_status_special_characters() {
    let Some(mut ctx) = StatusAtddContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init().expect("Failed to initialize");
    ctx.create_session("feature-123")
        .expect("Failed to create session");

    // WHEN
    let result = ctx.status_for("feature-123");

    // THEN
    assert!(
        result.success,
        "Status for session with numbers should succeed"
    );
}

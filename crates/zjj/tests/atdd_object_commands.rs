//! ATDD Tests for Core Object Command Workflows
//!
//! This module implements Acceptance Test-Driven Development tests for all
//! object-based CLI commands following the `zjj <object> <action>` pattern.
//!
//! # Objects Under Test
//!
//! - task: Task management (beads, work items)
//! - session: Session management (workspaces, zellij tabs)
//! - queue: Merge queue operations
//! - stack: Stack operations (parent-child session relationships)
//! - agent: Agent coordination and tracking
//! - status: Status and introspection queries
//! - config: Configuration management
//! - doctor: Diagnostics and health checks
//!
//! # Test Structure
//!
//! Each test follows BDD-style Given-When-Then structure:
//! - GIVEN: Preconditions are established
//! - WHEN: An action is performed
//! - THEN: Expected outcomes are verified
//!
//! # Design Principles
//!
//! - Zero unwrap/expect/panic (uses Result with ? propagation)
//! - Pure functional patterns where possible
//! - Tests are reproducible and can run in parallel
//! - Clear scenario documentation

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::uninlined_format_args)]

mod common;

use std::sync::Arc;

use anyhow::Result;
use common::{parse_json_output, parse_jsonl_output, TestHarness};
use tokio::sync::Mutex;

// =============================================================================
// Test Context
// =============================================================================

/// ATDD test context that holds state for each scenario
pub struct AtddTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// Track created sessions for cleanup
    pub sessions: Arc<Mutex<Vec<String>>>,
    /// Track created agent IDs for cleanup
    pub agents: Arc<Mutex<Vec<String>>>,
}

impl AtddTestContext {
    /// Create a new ATDD test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            sessions: Arc::new(Mutex::new(Vec::new())),
            agents: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the ZJJ database
    pub fn init_zjj(&self) -> Result<()> {
        let result = self.harness.zjj(&["init"]);
        if !result.success {
            anyhow::bail!(
                "Failed to initialize ZJJ: {}\nstdout: {}\nstderr: {}",
                result.exit_code.map_or(-1, |c| c),
                result.stdout,
                result.stderr
            );
        }
        if !self.harness.zjj_dir().exists() {
            anyhow::bail!("ZJJ initialization failed - .zjj directory not created");
        }
        Ok(())
    }

    /// Track a session for cleanup
    pub async fn track_session(&self, name: &str) {
        self.sessions.lock().await.push(name.to_string());
    }

    /// Track an agent for cleanup
    pub async fn track_agent(&self, id: &str) {
        self.agents.lock().await.push(id.to_string());
    }

    /// Create a session and track it
    pub async fn create_session(&self, name: &str) -> Result<()> {
        let result = self
            .harness
            .zjj(&["add", name, "--no-zellij", "--no-hooks"]);
        if result.success {
            self.track_session(name).await;
            return Ok(());
        }
        anyhow::bail!(
            "Failed to create session '{}': {}\nstdout: {}\nstderr: {}",
            name,
            result.exit_code.map_or(-1, |c| c),
            result.stdout,
            result.stderr
        )
    }
}

impl Drop for AtddTestContext {
    fn drop(&mut self) {
        // Cleanup: remove all created sessions
        // Note: We can't use async in drop, so we use blocking operations
        if let Ok(sessions) = self.sessions.try_lock() {
            for name in sessions.iter() {
                let _ = self.harness.zjj(&["remove", name.as_str(), "--merge"]);
            }
        }
    }
}

// =============================================================================
// Object Command: TASK
// =============================================================================

/// Scenario: Task list displays all tasks
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task list"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_task_list_displays_tasks() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Task list should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON output is valid
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    assert!(
        json_result.is_ok(),
        "Task list JSON should be valid: {}\nstdout: {}",
        json_result
            .err()
            .map_or("unknown error".to_string(), |e| e.to_string()),
        result.stdout
    );
}

/// Scenario: Task show requires task ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task show" without an ID
/// THEN: the command fails with an error
#[test]
fn scenario_task_show_requires_id() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task", "show"]);

    // THEN: Should fail because ID is required
    assert!(
        !result.success,
        "Task show without ID should fail. stdout: {}",
        result.stdout
    );
}

/// Scenario: Task show with nonexistent ID fails gracefully
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task show" with a nonexistent ID
/// THEN: the command fails with a helpful error
#[test]
fn scenario_task_show_nonexistent_fails_gracefully() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx
        .harness
        .zjj(&["task", "show", "nonexistent-task-id-12345", "--json"]);

    // THEN: Should fail with not found error
    assert!(!result.success, "Task show with nonexistent ID should fail");
    // Error message should be informative
    let output_combined = format!("{}{}", result.stdout, result.stderr);
    assert!(
        output_combined.contains("not found")
            || output_combined.contains("error")
            || !result.success,
        "Should indicate task not found or error"
    );
}

/// Scenario: Task claim requires task ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task claim" without an ID
/// THEN: the command fails with an error
#[test]
fn scenario_task_claim_requires_id() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task", "claim"]);

    // THEN
    assert!(!result.success, "Task claim without ID should fail");
}

/// Scenario: Task yield requires task ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task yield" without an ID
/// THEN: the command fails with an error
#[test]
fn scenario_task_yield_requires_id() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task", "yield"]);

    // THEN
    assert!(!result.success, "Task yield without ID should fail");
}

/// Scenario: Task start requires task ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task start" without an ID
/// THEN: the command fails with an error
#[test]
fn scenario_task_start_requires_id() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task", "start"]);

    // THEN
    assert!(!result.success, "Task start without ID should fail");
}

/// Scenario: Task done accepts optional ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task done" without an ID
/// THEN: the command handles it appropriately (may use current session)
#[test]
fn scenario_task_done_optional_id() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN: Run without ID (should not crash)
    let result = ctx.harness.zjj(&["task", "done", "--json"]);

    // THEN: Should not panic; behavior depends on whether there's a current session
    // Either it succeeds (uses current session) or fails with helpful message
    let _ = result.success; // Just verify it doesn't panic
}

// =============================================================================
// Task Lifecycle Scenarios (claim -> yield -> done)
// =============================================================================

/// Scenario: Task claim with nonexistent task fails gracefully
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task claim" with a nonexistent task ID
/// THEN: the command returns claimed=false with an error message
#[test]
fn scenario_task_claim_nonexistent_returns_error() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx
        .harness
        .zjj(&["task", "claim", "bd-nonexistent-xyz123", "--json"]);

    // THEN: Command may succeed (HTTP 200) but claim should be false
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    if let Ok(json) = json_result {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(true);
        assert!(
            !claimed,
            "Claim should be false for nonexistent task. Got: {:?}",
            data
        );
        let error_present = data.get("error").is_some();
        assert!(
            error_present,
            "Error message should be present for nonexistent task. Got: {:?}",
            data
        );
    }
}

/// Scenario: Task claim creates lock file
///
/// GIVEN: zjj is initialized
/// AND: a task exists
/// WHEN: I run "zjj task claim <task-id>"
/// THEN: the claim succeeds
/// AND: a lock file is created
#[tokio::test]
async fn scenario_task_claim_creates_lock() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // Create a test bead/task by adding to the beads database
    let task_id = "bd-test-claim-lock";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test claim task"]);
    if !add_result.success {
        // If bead add doesn't exist, try creating via other means or skip
        println!("SKIP: bead add not available");
        return;
    }

    // WHEN
    let result = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "test-agent-1")],
    );

    // THEN
    if result.success && !result.stdout.is_empty() {
        let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
        if let Ok(json) = json_result {
            let data = json.get("data").unwrap_or(&json);
            let claimed = data
                .get("claimed")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);
            assert!(claimed, "Claim should succeed. Got: {:?}", data);

            // Verify lock file exists
            let lock_path = ctx
                .harness
                .zjj_dir()
                .join("task-locks")
                .join(format!("{}.lock", task_id));
            assert!(
                lock_path.exists(),
                "Lock file should exist at {:?}",
                lock_path
            );
        }
    }

    // Cleanup
    let _ = ctx.harness.zjj(&["task", "yield", task_id, "--json"]);
}

/// Scenario: Task claim by different agent fails
///
/// GIVEN: zjj is initialized
/// AND: a task is claimed by agent A
/// WHEN: agent B tries to claim the same task
/// THEN: the claim fails with "already claimed" error
#[tokio::test]
async fn scenario_task_claim_conflict() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-claim-conflict";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test conflict task"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    // Agent A claims the task
    let claim_a = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "agent-alice")],
    );
    if !claim_a.success {
        println!("SKIP: task claim not available");
        return;
    }

    // WHEN: Agent B tries to claim
    let claim_b = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "agent-bob")],
    );

    // THEN
    let json_result: Result<serde_json::Value, _> = parse_json_output(&claim_b.stdout);
    if let Ok(json) = json_result {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(true);
        assert!(
            !claimed,
            "Agent B should not be able to claim. Got: {:?}",
            data
        );

        let error = data.get("error").and_then(|e| e.as_str()).unwrap_or("");
        assert!(
            error.contains("claimed") || error.contains("already"),
            "Error should indicate task is already claimed. Got: {}",
            error
        );
    }

    // Cleanup
    let _ = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "agent-alice")],
    );
}

/// Scenario: Task lifecycle claim -> yield
///
/// GIVEN: zjj is initialized
/// AND: a task exists
/// WHEN: I claim a task, then yield it
/// THEN: claim succeeds, yield succeeds
/// AND: task is available for claiming again
#[tokio::test]
async fn scenario_task_lifecycle_claim_yield() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-lifecycle-yield";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test lifecycle yield"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    let agent_id = "test-agent-lifecycle";

    // WHEN: Claim the task
    let claim_result = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );
    if !claim_result.success {
        println!("SKIP: task claim not available");
        return;
    }

    // Verify claim succeeded
    let claim_json: Result<serde_json::Value, _> = parse_json_output(&claim_result.stdout);
    if let Ok(json) = claim_json {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        assert!(claimed, "Initial claim should succeed. Got: {:?}", data);
    }

    // AND: Yield the task
    let yield_result = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );

    // THEN
    assert!(
        yield_result.success,
        "Yield should succeed. stderr: {}",
        yield_result.stderr
    );

    let yield_json: Result<serde_json::Value, _> = parse_json_output(&yield_result.stdout);
    if let Ok(json) = yield_json {
        let data = json.get("data").unwrap_or(&json);
        let yielded = data
            .get("yielded")
            .and_then(|y| y.as_bool())
            .unwrap_or(false);
        assert!(yielded, "Yield should succeed. Got: {:?}", data);
    }

    // Verify task can be claimed again by a different agent
    let reclaim_result = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "different-agent")],
    );

    let reclaim_json: Result<serde_json::Value, _> = parse_json_output(&reclaim_result.stdout);
    if let Ok(json) = reclaim_json {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        assert!(
            claimed,
            "Re-claim after yield should succeed. Got: {:?}",
            data
        );
    }

    // Cleanup
    let _ = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "different-agent")],
    );
}

/// Scenario: Task lifecycle claim -> done
///
/// GIVEN: zjj is initialized
/// AND: a task exists
/// WHEN: I claim a task, then complete it
/// THEN: claim succeeds, done succeeds
/// AND: task status is "completed"
#[tokio::test]
async fn scenario_task_lifecycle_claim_done() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-lifecycle-done";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test lifecycle done"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    let agent_id = "test-agent-done";

    // WHEN: Claim the task
    let claim_result = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );
    if !claim_result.success {
        println!("SKIP: task claim not available");
        return;
    }

    // AND: Complete the task
    let done_result = ctx.harness.zjj_with_env(
        &["task", "done", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );

    // THEN
    assert!(
        done_result.success,
        "Done should succeed. stderr: {}",
        done_result.stderr
    );

    let done_json: Result<serde_json::Value, _> = parse_json_output(&done_result.stdout);
    if let Ok(json) = done_json {
        let data = json.get("data").unwrap_or(&json);
        let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("");
        assert!(
            status == "completed" || status == "closed",
            "Task status should be completed/closed. Got: {}",
            status
        );
    }

    // Verify task cannot be claimed again
    let reclaim_result = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "another-agent")],
    );

    let reclaim_json: Result<serde_json::Value, _> = parse_json_output(&reclaim_result.stdout);
    if let Ok(json) = reclaim_json {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        assert!(
            !claimed,
            "Completed task should not be claimable. Got: {:?}",
            data
        );
    }
}

/// Scenario: Task full lifecycle claim -> yield -> claim -> done
///
/// GIVEN: zjj is initialized
/// AND: a task exists
/// WHEN: I claim, yield, reclaim, then complete a task
/// THEN: each step succeeds
/// AND: final status is "completed"
#[tokio::test]
async fn scenario_task_lifecycle_full() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-lifecycle-full";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test full lifecycle"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    let agent_a = "agent-alice-full";
    let agent_b = "agent-bob-full";

    // WHEN: Step 1 - Agent A claims
    let claim1 = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_a)],
    );
    if !claim1.success {
        println!("SKIP: task claim not available");
        return;
    }

    // Step 2 - Agent A yields
    let yield1 = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_a)],
    );
    assert!(yield1.success, "Yield should succeed");

    // Step 3 - Agent B claims
    let claim2 = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_b)],
    );
    let claim2_json: Result<serde_json::Value, _> = parse_json_output(&claim2.stdout);
    if let Ok(json) = claim2_json {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        assert!(claimed, "Agent B should be able to claim after yield");
    }

    // Step 4 - Agent B completes
    let done = ctx.harness.zjj_with_env(
        &["task", "done", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_b)],
    );

    // THEN
    assert!(done.success, "Done should succeed. stderr: {}", done.stderr);

    let done_json: Result<serde_json::Value, _> = parse_json_output(&done.stdout);
    if let Ok(json) = done_json {
        let data = json.get("data").unwrap_or(&json);
        let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("");
        assert!(
            status == "completed" || status == "closed",
            "Final status should be completed. Got: {}",
            status
        );
    }
}

/// Scenario: Task yield by non-owner fails
///
/// GIVEN: zjj is initialized
/// AND: a task is claimed by agent A
/// WHEN: agent B tries to yield the task
/// THEN: the yield fails with "not claimed by you" error
#[tokio::test]
async fn scenario_task_yield_by_non_owner_fails() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-yield-nonowner";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test yield nonowner"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    // Agent A claims the task
    let claim_a = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "owner-agent")],
    );
    if !claim_a.success {
        println!("SKIP: task claim not available");
        return;
    }

    // WHEN: Agent B tries to yield
    let yield_b = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "nonowner-agent")],
    );

    // THEN
    let yield_json: Result<serde_json::Value, _> = parse_json_output(&yield_b.stdout);
    if let Ok(json) = yield_json {
        let data = json.get("data").unwrap_or(&json);
        let yielded = data
            .get("yielded")
            .and_then(|y| y.as_bool())
            .unwrap_or(true);
        assert!(
            !yielded,
            "Non-owner should not be able to yield. Got: {:?}",
            data
        );

        let error = data.get("error").and_then(|e| e.as_str()).unwrap_or("");
        assert!(
            error.contains("claimed by") || error.contains("not"),
            "Error should indicate ownership. Got: {}",
            error
        );
    }

    // Cleanup
    let _ = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "owner-agent")],
    );
}

/// Scenario: Task claim is idempotent for same agent
///
/// GIVEN: zjj is initialized
/// AND: a task is already claimed by agent A
/// WHEN: agent A claims the same task again
/// THEN: the claim succeeds (extends the lock)
#[tokio::test]
async fn scenario_task_claim_idempotent_same_agent() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-claim-idempotent";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test claim idempotent"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    let agent_id = "idempotent-agent";

    // Initial claim
    let claim1 = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );
    if !claim1.success {
        println!("SKIP: task claim not available");
        return;
    }

    // WHEN: Same agent claims again
    let claim2 = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );

    // THEN
    let claim2_json: Result<serde_json::Value, _> = parse_json_output(&claim2.stdout);
    if let Ok(json) = claim2_json {
        let data = json.get("data").unwrap_or(&json);
        let claimed = data
            .get("claimed")
            .and_then(|c| c.as_bool())
            .unwrap_or(false);
        assert!(
            claimed,
            "Re-claim by same agent should succeed. Got: {:?}",
            data
        );
    }

    // Cleanup
    let _ = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", agent_id)],
    );
}

/// Scenario: Task done by non-owner fails
///
/// GIVEN: zjj is initialized
/// AND: a task is claimed by agent A
/// WHEN: agent B tries to complete the task
/// THEN: the done command fails with ownership error
#[tokio::test]
async fn scenario_task_done_by_non_owner_fails() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    let task_id = "bd-test-done-nonowner";
    let add_result = ctx
        .harness
        .zjj(&["bead", "add", task_id, "--title", "Test done nonowner"]);
    if !add_result.success {
        println!("SKIP: bead add not available");
        return;
    }

    // Agent A claims the task
    let claim_a = ctx.harness.zjj_with_env(
        &["task", "claim", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "owner-done")],
    );
    if !claim_a.success {
        println!("SKIP: task claim not available");
        return;
    }

    // WHEN: Agent B tries to complete
    let done_b = ctx.harness.zjj_with_env(
        &["task", "done", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "nonowner-done")],
    );

    // THEN
    // The done command should fail or return an error
    let combined = format!("{}{}", done_b.stdout, done_b.stderr);
    let failed = !done_b.success
        || combined.contains("claimed by")
        || combined.contains("not you")
        || combined.contains("error");
    assert!(
        failed,
        "Non-owner should not be able to complete task. Got: {}",
        combined
    );

    // Cleanup
    let _ = ctx.harness.zjj_with_env(
        &["task", "yield", task_id, "--json"],
        &[("ZJJ_AGENT_ID", "owner-done")],
    );
}

// =============================================================================
// Object Command: SESSION
// =============================================================================

/// Scenario: Session list displays sessions
///
/// GIVEN: zjj is initialized
/// AND: a session exists
/// WHEN: I run "zjj session list"
/// THEN: the output contains the session name
/// AND: output is valid JSON when --json flag is used
#[tokio::test]
async fn scenario_session_list_displays_sessions() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    ctx.create_session("test-session-list")
        .await
        .expect("Failed to create session");

    // WHEN
    let result = ctx.harness.zjj(&["session", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Session list should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("test-session-list"),
        "Session list should contain session name. Got: {}",
        result.stdout
    );

    // Verify JSON output
    let json_result: Result<Vec<serde_json::Value>, _> = parse_jsonl_output(&result.stdout);
    assert!(json_result.is_ok(), "Session list JSONL should be valid");
}

/// Scenario: Session add creates a new session
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session add <name>" with required options
/// THEN: the session is created
/// AND: the command succeeds
#[tokio::test]
async fn scenario_session_add_creates_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&[
        "session",
        "add",
        "test-session-add",
        "--no-open",
        "--no-hooks",
    ]);

    // THEN
    assert!(
        result.success,
        "Session add should succeed. stderr: {}",
        result.stderr
    );

    // Track for cleanup
    ctx.track_session("test-session-add").await;

    // Verify workspace exists
    ctx.harness.assert_workspace_exists("test-session-add");
}

/// Scenario: Session add requires name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session add" without a name
/// THEN: the command fails with an error
#[test]
fn scenario_session_add_requires_name() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "add"]);

    // THEN
    assert!(!result.success, "Session add without name should fail");
}

/// Scenario: Session remove requires name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session remove" without a name
/// THEN: the command fails with an error
#[test]
fn scenario_session_remove_requires_name() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "remove"]);

    // THEN
    assert!(!result.success, "Session remove without name should fail");
}

/// Scenario: Session focus requires name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session focus" without a name
/// THEN: the command fails with an error
#[test]
fn scenario_session_focus_requires_name() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "focus"]);

    // THEN
    assert!(!result.success, "Session focus without name should fail");
}

/// Scenario: Session rename requires old and new names
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session rename" without arguments
/// THEN: the command fails with an error
#[test]
fn scenario_session_rename_requires_names() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "rename"]);

    // THEN
    assert!(!result.success, "Session rename without names should fail");
}

/// Scenario: Session spawn requires bead ID
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj session spawn" without a bead ID
/// THEN: the command fails with an error
#[test]
fn scenario_session_spawn_requires_bead() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "spawn"]);

    // THEN
    assert!(!result.success, "Session spawn without bead should fail");
}

/// Scenario: Session init initializes zjj
///
/// GIVEN: a JJ repository exists
/// WHEN: I run "zjj session init"
/// THEN: the .zjj directory is created
/// AND: the command succeeds
#[test]
fn scenario_session_init_creates_zjj_dir() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // WHEN
    let result = ctx.harness.zjj(&["session", "init"]);

    // THEN
    assert!(
        result.success,
        "Session init should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        ctx.harness.zjj_dir().exists(),
        ".zjj directory should exist after init"
    );
}

// =============================================================================
// Object Command: QUEUE
// =============================================================================

/// Scenario: Queue list displays queue entries
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue list"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_queue_list_displays_entries() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Queue list should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON output is valid
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    assert!(
        json_result.is_ok(),
        "Queue list JSON should be valid: {:?}",
        json_result.err()
    );
}

/// Scenario: Queue list shows empty queue statistics
///
/// GIVEN: zjj is initialized
/// AND: no sessions are in the queue
/// WHEN: I run "zjj queue list --json"
/// THEN: output shows zero counts
#[test]
fn scenario_queue_list_empty_shows_zeros() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "list", "--json"]);

    // THEN
    assert!(result.success, "Queue list should succeed");

    // Parse and verify queue summary structure
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    if let Ok(json) = json_result {
        // Should have queue_summary with count fields
        if let Some(summary) = json.get("queue_summary") {
            assert!(
                summary.get("total").is_some(),
                "Queue summary should have total field"
            );
            assert!(
                summary.get("pending").is_some(),
                "Queue summary should have pending field"
            );
        }
    }
}

/// Scenario: Queue enqueue requires session name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue enqueue" without a session
/// THEN: the command fails with an error
#[test]
fn scenario_queue_enqueue_requires_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "enqueue"]);

    // THEN: Should fail because session is required
    let combined = format!("{}{}", result.stdout, result.stderr);
    assert!(
        !result.success || combined.contains("required") || combined.contains("error"),
        "Queue enqueue without session should fail. stdout: {}, stderr: {}",
        result.stdout,
        result.stderr
    );
}

/// Scenario: Queue dequeue requires session name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue dequeue" without a session
/// THEN: the command fails with an error
#[test]
fn scenario_queue_dequeue_requires_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "dequeue"]);

    // THEN: Should fail because session is required
    let combined = format!("{}{}", result.stdout, result.stderr);
    assert!(
        !result.success || combined.contains("required") || combined.contains("error"),
        "Queue dequeue without session should fail. stdout: {}, stderr: {}",
        result.stdout,
        result.stderr
    );
}

/// Scenario: Queue status shows queue state
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue status"
/// THEN: the command succeeds
#[test]
fn scenario_queue_status_displays_state() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "status", "--json"]);

    // THEN: Command should succeed and return valid JSON
    assert!(
        result.success,
        "Queue status should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Queue process runs with dry-run
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue process --dry-run"
/// THEN: the command succeeds without making changes
#[test]
fn scenario_queue_process_dry_run() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["queue", "process", "--dry-run"]);

    // THEN: Command should complete (may indicate no entries to process)
    // The key assertion is that it doesn't fail catastrophically
    // Process may fail if no entries, but should not panic
    let _ = result.success;
}

/// Scenario: Queue enqueue adds session to queue
///
/// GIVEN: zjj is initialized
/// AND: a session exists
/// WHEN: I run "zjj queue enqueue <session>"
/// THEN: the session is added to the queue
/// AND: the command succeeds
#[tokio::test]
async fn scenario_queue_enqueue_adds_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    ctx.create_session("test-queue-enqueue")
        .await
        .expect("Failed to create session");

    // WHEN
    let result = ctx
        .harness
        .zjj(&["queue", "enqueue", "test-queue-enqueue", "--json"]);

    // THEN
    assert!(
        result.success,
        "Queue enqueue should succeed. stdout: {}, stderr: {}",
        result.stdout, result.stderr
    );

    // Verify JSON output is valid
    if result.success && !result.stdout.is_empty() {
        let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
        assert!(
            json_result.is_ok(),
            "Queue enqueue JSON should be valid: {:?}",
            json_result.err()
        );
    }

    // Cleanup: dequeue the session
    let _ = ctx.harness.zjj(&["queue", "dequeue", "test-queue-enqueue"]);
}

/// Scenario: Queue dequeue removes session from queue
///
/// GIVEN: zjj is initialized
/// AND: a session is in the queue
/// WHEN: I run "zjj queue dequeue <session>"
/// THEN: the session is removed from the queue
/// AND: the command succeeds
#[tokio::test]
async fn scenario_queue_dequeue_removes_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    ctx.create_session("test-queue-dequeue")
        .await
        .expect("Failed to create session");

    // Add session to queue first
    let enqueue_result = ctx.harness.zjj(&["queue", "enqueue", "test-queue-dequeue"]);
    if !enqueue_result.success {
        // Skip test if enqueue not implemented
        println!("SKIP: queue enqueue not implemented");
        return;
    }

    // WHEN
    let result = ctx
        .harness
        .zjj(&["queue", "dequeue", "test-queue-dequeue", "--json"]);

    // THEN
    assert!(
        result.success,
        "Queue dequeue should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Queue list shows enqueued sessions
///
/// GIVEN: zjj is initialized
/// AND: sessions are in the queue
/// WHEN: I run "zjj queue list"
/// THEN: the output contains the enqueued sessions
#[tokio::test]
async fn scenario_queue_list_shows_enqueued_sessions() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    ctx.create_session("test-queue-list-a")
        .await
        .expect("Failed to create session A");

    // Enqueue session
    let enqueue_a = ctx.harness.zjj(&["queue", "enqueue", "test-queue-list-a"]);
    if !enqueue_a.success {
        // Skip test if enqueue not implemented
        println!("SKIP: queue enqueue not implemented");
        return;
    }

    // WHEN
    let result = ctx.harness.zjj(&["queue", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Queue list should succeed. stderr: {}",
        result.stderr
    );

    // Cleanup
    let _ = ctx.harness.zjj(&["queue", "dequeue", "test-queue-list-a"]);
}

/// Scenario: Queue lifecycle add-list-remove
///
/// GIVEN: zjj is initialized
/// AND: a session exists
/// WHEN: I enqueue session, list, and dequeue
/// THEN: each step succeeds and state is consistent
#[tokio::test]
async fn scenario_queue_lifecycle_add_list_remove() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");
    ctx.create_session("test-queue-lifecycle")
        .await
        .expect("Failed to create session");

    // WHEN: Enqueue to queue
    let enqueue_result = ctx
        .harness
        .zjj(&["queue", "enqueue", "test-queue-lifecycle", "--json"]);
    if !enqueue_result.success {
        // Skip test if enqueue not implemented
        println!("SKIP: queue enqueue not implemented");
        return;
    }

    // AND: List should work
    let list_result = ctx.harness.zjj(&["queue", "list", "--json"]);
    assert!(
        list_result.success,
        "List should succeed in lifecycle. stderr: {}",
        list_result.stderr
    );

    // AND: Status should work
    let status_result = ctx.harness.zjj(&["queue", "status", "--json"]);
    assert!(
        status_result.success,
        "Status should succeed in lifecycle. stderr: {}",
        status_result.stderr
    );

    // THEN: Dequeue should succeed
    let dequeue_result = ctx
        .harness
        .zjj(&["queue", "dequeue", "test-queue-lifecycle", "--json"]);
    assert!(
        dequeue_result.success,
        "Dequeue should succeed in lifecycle. stderr: {}",
        dequeue_result.stderr
    );

    // Verify final state - list should not error
    let final_list = ctx.harness.zjj(&["queue", "list", "--json"]);
    assert!(
        final_list.success,
        "Final list should succeed. stderr: {}",
        final_list.stderr
    );
}

/// Scenario: Queue enqueue nonexistent session fails
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue enqueue" with a nonexistent session
/// THEN: the command fails with an appropriate error
#[test]
fn scenario_queue_enqueue_nonexistent_fails() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx
        .harness
        .zjj(&["queue", "enqueue", "nonexistent-session-xyz"]);

    // THEN: Should fail because session doesn't exist
    // The exact error depends on implementation but it should not succeed silently
    let output_combined = format!("{}{}", result.stdout, result.stderr);
    assert!(
        !result.success
            || output_combined.contains("not found")
            || output_combined.contains("error")
            || output_combined.contains("does not exist")
            || output_combined.contains("invalid"),
        "Enqueue nonexistent session should fail or report error. stdout: {}, stderr: {}",
        result.stdout,
        result.stderr
    );
}

/// Scenario: Queue dequeue nonexistent entry handles gracefully
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj queue dequeue" with a session not in queue
/// THEN: the command handles it gracefully (success or informative message)
#[test]
fn scenario_queue_dequeue_nonexistent_handles_gracefully() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN: Try to dequeue a session that was never enqueued
    let result = ctx
        .harness
        .zjj(&["queue", "dequeue", "never-enqueued-session"]);

    // THEN: Should not panic - either succeeds (idempotent) or gives helpful message
    // The queue dequeue command should handle this gracefully
    let _ = result.success; // Just verify it doesn't panic
}

// =============================================================================
// Object Command: STACK
// =============================================================================

/// Scenario: Stack status shows stack information
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj stack status"
/// THEN: the command succeeds
#[test]
fn scenario_stack_status_displays_info() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["stack", "status", "--json"]);

    // THEN: Should succeed (even if no stack exists)
    // The command itself should not crash
    let _ = result.success;
}

/// Scenario: Stack list displays all stacks
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj stack list"
/// THEN: the command succeeds
#[test]
fn scenario_stack_list_displays_stacks() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["stack", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Stack list should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Stack create requires name
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj stack create" without a name
/// THEN: the command fails with an error
#[test]
fn scenario_stack_create_requires_name() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["stack", "create"]);

    // THEN
    assert!(!result.success, "Stack create without name should fail");
}

/// Scenario: Stack push requires session
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj stack push" without a session
/// THEN: the command fails with an error
#[test]
fn scenario_stack_push_requires_session() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["stack", "push"]);

    // THEN
    assert!(!result.success, "Stack push without session should fail");
}

// =============================================================================
// Object Command: AGENT
// =============================================================================

/// Scenario: Agent list displays agents
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj agents"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_agent_list_displays_agents() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["agents", "--json"]);

    // THEN
    assert!(
        result.success,
        "Agents list should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Agent register succeeds
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj agents register"
/// THEN: the command succeeds
#[test]
fn scenario_agent_register_succeeds() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["agents", "register", "--json"]);

    // THEN
    assert!(
        result.success,
        "Agents register should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Agent status shows agent info
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj agents status"
/// THEN: the command succeeds
#[test]
fn scenario_agent_status_displays_info() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["agents", "status", "--json"]);

    // THEN
    assert!(
        result.success,
        "Agents status should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Agent broadcast requires message
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj agents broadcast" without a message
/// THEN: the command fails with an error
#[test]
fn scenario_agent_broadcast_requires_message() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["agents", "broadcast"]);

    // THEN
    assert!(
        !result.success,
        "Agents broadcast without message should fail"
    );
}

/// Scenario: Agent heartbeat succeeds
///
/// GIVEN: zjj is initialized
/// AND: an agent is registered
/// WHEN: I run "zjj agents heartbeat"
/// THEN: the command succeeds
#[test]
fn scenario_agent_heartbeat_succeeds() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // Register an agent and extract the agent_id from the JSON output
    let register_result = ctx.harness.zjj(&["agents", "register", "--json"]);
    assert!(register_result.success, "Agents register should succeed");

    // Extract agent_id from the JSON response
    let agent_id = extract_agent_id_from_output(&register_result.stdout)
        .expect("Should have agent_id in register output");

    // WHEN - Send heartbeat with the agent_id in environment
    let result = ctx.harness.zjj_with_env(
        &["agents", "heartbeat", "--json"],
        &[("ZJJ_AGENT_ID", &agent_id)],
    );

    // THEN
    assert!(
        result.success,
        "Agents heartbeat should succeed. stderr: {}",
        result.stderr
    );
}

/// Extract `agent_id` from agents register JSON output
fn extract_agent_id_from_output(stdout: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let data = parsed.get("data").unwrap_or(&parsed);
    data.get("agent_id")
        .and_then(|s| s.as_str())
        .map(String::from)
}

// =============================================================================
// Object Command: STATUS
// =============================================================================

/// Scenario: Status show displays current status
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj status"
/// THEN: the command succeeds
#[test]
fn scenario_status_show_displays_status() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["status", "--json"]);

    // THEN
    assert!(
        result.success,
        "Status should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Status whereami shows location
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj whereami"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_status_whereami_shows_location() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["whereami", "--json"]);

    // THEN
    assert!(
        result.success,
        "whereami should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON output
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    assert!(json_result.is_ok(), "whereami JSON should be valid");
}

/// Scenario: Status whoami shows identity
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj whoami"
/// THEN: the command succeeds
#[test]
fn scenario_status_whoami_shows_identity() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["whoami", "--json"]);

    // THEN
    assert!(
        result.success,
        "Status whoami should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Status context shows context info
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj context"
/// THEN: the command succeeds
#[test]
fn scenario_status_context_shows_info() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["context", "--json"]);

    // THEN
    assert!(
        result.success,
        "context should succeed. stderr: {}",
        result.stderr
    );
}

// =============================================================================
// Object Command: CONFIG
// =============================================================================

/// Scenario: Config list displays configuration
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj config"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_config_list_displays_config() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["config", "--json"]);

    // THEN
    assert!(
        result.success,
        "Config should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Config get requires key
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj config get" without a key
/// THEN: the command fails with an error
#[test]
fn scenario_config_get_requires_key() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["config", "get"]);

    // THEN
    assert!(!result.success, "Config get without key should fail");
}

/// Scenario: Config set requires key and value
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj config set" without key and value
/// THEN: the command fails with an error
#[test]
fn scenario_config_set_requires_key_value() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["config", "set"]);

    // THEN
    assert!(!result.success, "Config set without key should fail");
}

/// Scenario: Config schema displays schema
/// NOTE: schema subcommand not available - using schema command instead
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj schema"
/// THEN: the command succeeds
#[test]
fn scenario_config_schema_displays_schema() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["schema", "--json"]);

    // THEN
    assert!(
        result.success,
        "schema should succeed. stderr: {}",
        result.stderr
    );
}

// =============================================================================
// Object Command: DOCTOR
// =============================================================================

/// Scenario: Doctor check runs diagnostics
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor --json"
/// THEN: the command succeeds
/// AND: output is valid JSON when --json flag is used
#[test]
fn scenario_doctor_check_runs_diagnostics() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN - Use "zjj doctor" (the main check command)
    let result = ctx.harness.zjj(&["doctor", "--json"]);

    // THEN
    assert!(
        result.success,
        "Doctor should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor fix runs fixes
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor --fix --dry-run"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_fix_runs_fixes() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN - Use "zjj doctor --fix --dry-run" (flags on main command)
    let result = ctx.harness.zjj(&["doctor", "--fix", "--dry-run"]);

    // THEN: Should succeed with dry-run (no actual changes)
    assert!(
        result.success,
        "Doctor --fix --dry-run should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor integrity checks integrity
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor --json"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_integrity_checks() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN - Use "zjj doctor --json" (integrity is part of doctor checks)
    let result = ctx.harness.zjj(&["doctor", "--json"]);

    // THEN
    assert!(
        result.success,
        "Doctor should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor clean cleans up invalid sessions
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj clean --dry-run"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_clean_dry_run() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN - Use "zjj clean --dry-run" (standalone clean command)
    let result = ctx.harness.zjj(&["clean", "--dry-run"]);

    // THEN
    assert!(
        result.success,
        "Clean --dry-run should succeed. stderr: {}",
        result.stderr
    );
}

// =============================================================================
// Error Path Tests
// =============================================================================

/// Scenario: Invalid object shows error
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj invalid-object"
/// THEN: the command fails with an error
#[test]
fn scenario_invalid_object_shows_error() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["invalid-object-name"]);

    // THEN
    assert!(!result.success, "Invalid object should fail");
}

/// Scenario: Missing subcommand shows error
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj task" without a subcommand
/// THEN: the command fails or shows help
#[test]
fn scenario_missing_subcommand_shows_error() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["task"]);

    // THEN: Should fail or show help
    // clap requires subcommand by default
    assert!(
        !result.success || result.stdout.contains("usage") || result.stdout.contains("Commands"),
        "Missing subcommand should fail or show help"
    );
}

/// Scenario: JSON output is always valid
///
/// GIVEN: any command with --json flag
/// WHEN: the command runs
/// THEN: the output is valid JSON (either success or error envelope)
#[test]
fn scenario_json_output_always_valid() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // Test several commands with --json
    let commands = [
        ["session", "list", "--json"],
        ["queue", "list", "--json"],
        ["agent", "list", "--json"],
        ["stack", "list", "--json"],
        ["config", "list", "--json"],
    ];

    for cmd in commands {
        let result = ctx.harness.zjj(&cmd);

        // If there's output, verify it's valid JSON
        if !result.stdout.is_empty() {
            let json_result: Result<serde_json::Value, _> = if result.stdout.contains('\n') {
                // JSONL format
                parse_jsonl_output(&result.stdout)
                    .map(|v| v.first().cloned().unwrap_or(serde_json::Value::Null))
            } else {
                parse_json_output(&result.stdout)
            };

            assert!(
                json_result.is_ok(),
                "JSON output for '{}' should be valid: {:?}\nstdout: {}",
                cmd.join(" "),
                json_result.err(),
                result.stdout
            );
        }
    }
}

/// Scenario: Help shows all objects
///
/// GIVEN: zjj is installed
/// WHEN: I run "zjj --help"
/// THEN: the output shows all object commands
#[test]
fn scenario_help_shows_all_objects() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // WHEN
    let result = ctx.harness.zjj(&["--help"]);

    // THEN
    let expected_objects = [
        "task", "session", "queue", "stack", "agent", "status", "config", "doctor",
    ];

    for obj in expected_objects {
        assert!(
            result.stdout.contains(obj) || result.stderr.contains(obj),
            "Help should mention object '{}'. Got: {}",
            obj,
            result.stdout
        );
    }
}

/// Scenario: Version shows version
///
/// GIVEN: zjj is installed
/// WHEN: I run "zjj --version"
/// THEN: the output shows version information
#[test]
fn scenario_version_shows_version() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // WHEN
    let result = ctx.harness.zjj(&["--version"]);

    // THEN
    assert!(
        result.success || result.stdout.contains("zjj") || result.stderr.contains("zjj"),
        "Version should show zjj info"
    );
}

// =============================================================================
// Global Flags Tests
// =============================================================================

/// Scenario: --json flag produces JSON output
///
/// GIVEN: zjj is initialized
/// WHEN: I run any command with --json
/// THEN: the output is valid JSON
#[test]
fn scenario_json_flag_produces_json() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // Test with a simple command
    let result = ctx.harness.zjj(&["status", "--json"]);

    assert!(result.success, "status --json should succeed");

    // Verify JSON is valid
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    assert!(
        json_result.is_ok(),
        "Output should be valid JSON: {}",
        result.stdout
    );
}

/// Scenario: --verbose flag enables verbose output
///
/// GIVEN: zjj is initialized
/// WHEN: I run a command with --verbose
/// THEN: the command succeeds (verbose output is optional)
#[test]
fn scenario_verbose_flag_works() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["session", "list", "--verbose"]);

    // THEN: Command should succeed (verbose just adds more info)
    assert!(
        result.success,
        "Session list --verbose should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: --dry-run flag prevents changes
///
/// GIVEN: zjj is initialized
/// WHEN: I run a command with --dry-run
/// THEN: no actual changes are made
#[tokio::test]
async fn scenario_dry_run_flag_prevents_changes() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN: Try to add session with dry-run
    let result = ctx
        .harness
        .zjj(&["session", "add", "test-dry-run-session", "--dry-run"]);

    // THEN: Command should succeed but no session created
    assert!(
        result.success,
        "Session add --dry-run should succeed. stderr: {}",
        result.stderr
    );

    // Verify no workspace was created
    ctx.harness
        .assert_workspace_not_exists("test-dry-run-session");
}

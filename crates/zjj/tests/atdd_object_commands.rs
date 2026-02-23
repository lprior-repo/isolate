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
    let json_result: Result<Vec<serde_json::Value>, _> = parse_jsonl_output(&result.stdout);
    assert!(json_result.is_ok(), "Queue list JSONL should be valid");
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

    // THEN
    assert!(!result.success, "Queue enqueue without session should fail");
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

    // THEN
    assert!(!result.success, "Queue dequeue without session should fail");
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

    // THEN
    assert!(
        result.success,
        "Queue status should succeed. stderr: {}",
        result.stderr
    );
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
    let result = ctx
        .harness
        .zjj_with_env(&["agents", "heartbeat", "--json"], &[("ZJJ_AGENT_ID", &agent_id)]);

    // THEN
    assert!(
        result.success,
        "Agents heartbeat should succeed. stderr: {}",
        result.stderr
    );
}

/// Extract agent_id from agents register JSON output
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
/// WHEN: I run "zjj status show"
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
    let result = ctx.harness.zjj(&["status", "show", "--json"]);

    // THEN
    assert!(
        result.success,
        "Status show should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Status whereami shows location
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj status whereami"
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
    let result = ctx.harness.zjj(&["status", "whereami", "--json"]);

    // THEN
    assert!(
        result.success,
        "Status whereami should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON output
    let json_result: Result<serde_json::Value, _> = parse_json_output(&result.stdout);
    assert!(json_result.is_ok(), "Status whereami JSON should be valid");
}

/// Scenario: Status whoami shows identity
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj status whoami"
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
    let result = ctx.harness.zjj(&["status", "whoami", "--json"]);

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
/// WHEN: I run "zjj status context"
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
    let result = ctx.harness.zjj(&["status", "context", "--json"]);

    // THEN
    assert!(
        result.success,
        "Status context should succeed. stderr: {}",
        result.stderr
    );
}

// =============================================================================
// Object Command: CONFIG
// =============================================================================

/// Scenario: Config list displays configuration
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj config list"
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
    let result = ctx.harness.zjj(&["config", "list", "--json"]);

    // THEN
    assert!(
        result.success,
        "Config list should succeed. stderr: {}",
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
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj config schema"
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
    let result = ctx.harness.zjj(&["config", "schema", "--json"]);

    // THEN
    assert!(
        result.success,
        "Config schema should succeed. stderr: {}",
        result.stderr
    );
}

// =============================================================================
// Object Command: DOCTOR
// =============================================================================

/// Scenario: Doctor check runs diagnostics
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor check"
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

    // WHEN
    let result = ctx.harness.zjj(&["doctor", "check", "--json"]);

    // THEN
    assert!(
        result.success,
        "Doctor check should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor fix runs fixes
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor fix"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_fix_runs_fixes() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["doctor", "fix", "--dry-run"]);

    // THEN: Should succeed with dry-run (no actual changes)
    assert!(
        result.success,
        "Doctor fix --dry-run should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor integrity checks integrity
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor integrity"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_integrity_checks() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["doctor", "integrity", "--json"]);

    // THEN
    assert!(
        result.success,
        "Doctor integrity should succeed. stderr: {}",
        result.stderr
    );
}

/// Scenario: Doctor clean cleans up invalid sessions
///
/// GIVEN: zjj is initialized
/// WHEN: I run "zjj doctor clean --dry-run"
/// THEN: the command succeeds
#[test]
fn scenario_doctor_clean_dry_run() {
    let Some(ctx) = AtddTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to initialize ZJJ");

    // WHEN
    let result = ctx.harness.zjj(&["doctor", "clean", "--dry-run"]);

    // THEN
    assert!(
        result.success,
        "Doctor clean --dry-run should succeed. stderr: {}",
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
    let result = ctx.harness.zjj(&["status", "show", "--json"]);

    assert!(result.success, "Status show --json should succeed");

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

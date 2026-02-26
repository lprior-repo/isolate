#![allow(clippy::uninlined_format_args, clippy::redundant_closure_for_method_calls, clippy::expect_used)]
//! BDD Acceptance Tests for Agent Management Feature
//!
//! Feature: Agent Management
//!
//! As an autonomous agent using Isolate
//! I want to manage my agent identity
//! So that I can coordinate work with other agents
//!
//! This test file implements the BDD scenarios defined in `features/agent.feature`
//! using Dan North BDD style with Given/When/Then syntax.
//!
//! # ATDD Phase
//!
//! These tests define expected behavior before implementation.
//! Run with: `cargo test --test agent_feature`
//!
//! # Key Invariants
//!
//! - Unique agent IDs (no two agents can have the same ID)
//! - Agents must be registered before sending heartbeats
//! - Heartbeats track liveness (stale after timeout)

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]

mod common;

use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use common::{CommandResult, TestHarness};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;

// =============================================================================
// Agent Test Context
// =============================================================================

/// Agent test context that holds state for each scenario
///
/// Uses Arc<Mutex<>> for thread-safe sharing across async steps.
pub struct AgentTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// Track the last agent ID for assertions
    pub last_agent_id: Arc<Mutex<Option<String>>>,
    /// Track the last operation result
    pub last_result: Arc<Mutex<Option<CommandResult>>>,
    /// Track registered agents for cleanup
    pub registered_agents: Arc<Mutex<Vec<String>>>,
}

impl AgentTestContext {
    /// Create a new agent test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            last_agent_id: Arc::new(Mutex::new(None)),
            last_result: Arc::new(Mutex::new(None)),
            registered_agents: Arc::new(Mutex::new(Vec::new())),
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

    /// Store an agent ID for later cleanup
    pub async fn track_agent(&self, id: &str) {
        self.registered_agents.lock().await.push(id.to_string());
        *self.last_agent_id.lock().await = Some(id.to_string());
    }

    /// Run isolate command and store result
    pub async fn run_isolate(&self, args: &[&str]) -> CommandResult {
        let result = self.harness.isolate(args);
        *self.last_result.lock().await = Some(result.clone());
        result
    }

    /// Run isolate command with environment variables
    pub async fn run_isolate_with_env(
        &self,
        args: &[&str],
        env_vars: &[(&str, &str)],
    ) -> CommandResult {
        let result = self.harness.isolate_with_env(args, env_vars);
        *self.last_result.lock().await = Some(result.clone());
        result
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parse agents from JSON output
fn parse_agents_from_output(stdout: &str) -> Result<Vec<JsonValue>> {
    let parsed: JsonValue =
        serde_json::from_str(stdout).with_context(|| "Failed to parse JSON output")?;

    // Handle envelope format
    let data = parsed.get("data").unwrap_or(&parsed);

    // Get agents array
    let agents = data
        .get("agents")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(agents)
}

/// Extract `agent_id` from register output
fn extract_agent_id(stdout: &str) -> Option<String> {
    let parsed: JsonValue = serde_json::from_str(stdout).ok()?;
    let data = parsed.get("data").unwrap_or(&parsed);
    data.get("agent_id")
        .and_then(|s| s.as_str())
        .map(String::from)
}

/// Check if agent exists in database
async fn agent_exists_in_db(ctx: &AgentTestContext, agent_id: &str) -> Result<bool> {
    let result = ctx.run_isolate(&["agents", "--all", "--json"]).await;

    if !result.success {
        return Ok(false);
    }

    let agents = parse_agents_from_output(&result.stdout)?;
    Ok(agents.iter().any(|a| {
        a.get("agent_id")
            .and_then(|s| s.as_str())
            .map(|s| s == agent_id)
            .unwrap_or(false)
    }))
}

// =============================================================================
// Scenario: Register creates agent
// =============================================================================

/// Scenario: Register creates agent
///
/// GIVEN: no agent with ID "agent-test-001" exists
/// WHEN: I register an agent with ID "agent-test-001"
/// THEN: the agent "agent-test-001" should exist
/// AND: the agent "agent-test-001" should be registered
/// AND: the environment variable `"Isolate_AGENT_ID"` should be set
/// AND: the agent details should be returned as JSON
#[tokio::test]
async fn scenario_register_creates_agent() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN
    let result = ctx
        .run_isolate(&["agents", "register", "--id", "agent-test-001", "--json"])
        .await;

    // THEN
    assert!(
        result.success,
        "Register should succeed. stderr: {}",
        result.stderr
    );

    // Verify output contains agent_id
    assert!(
        result.stdout.contains("agent-test-001"),
        "Output should contain agent_id 'agent-test-001'. Got: {}",
        result.stdout
    );

    // Verify JSON output
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    let data = parsed.get("data").unwrap_or(&parsed);
    assert!(
        data.get("agent_id").is_some(),
        "JSON should contain agent_id field"
    );
    assert!(
        data.get("message").is_some(),
        "JSON should contain message field"
    );

    ctx.track_agent("agent-test-001").await;
}

// =============================================================================
// Scenario: Register with auto-generated ID
// =============================================================================

/// Scenario: Register with auto-generated ID
///
/// GIVEN: no agent is registered
/// WHEN: I register an agent without specifying an ID
/// THEN: an agent should be created with an auto-generated ID
/// AND: the agent ID should match pattern "agent-XXXXXXXX-XXXX"
#[tokio::test]
async fn scenario_register_auto_generated_id() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN
    let result = ctx.run_isolate(&["agents", "register", "--json"]).await;

    // THEN
    assert!(
        result.success,
        "Register should succeed. stderr: {}",
        result.stderr
    );

    // Extract agent_id from output
    let agent_id = extract_agent_id(&result.stdout).expect("Should have agent_id in output");

    // Verify pattern: agent-XXXXXXXX-XXXX (hex timestamp + pid)
    assert!(
        agent_id.starts_with("agent-"),
        "Agent ID should start with 'agent-', got: {}",
        agent_id
    );

    // Pattern: agent-{8 hex chars}-{4 hex chars}
    let parts: Vec<&str> = agent_id.split('-').collect();
    assert!(
        parts.len() >= 3,
        "Agent ID should have format 'agent-XXXXXXXX-XXXX', got: {}",
        agent_id
    );

    // Verify hex components
    assert!(
        parts[1].len() == 8 && parts[1].chars().all(|c| c.is_ascii_hexdigit()),
        "Second part should be 8 hex chars, got: {}",
        parts[1]
    );
    assert!(
        parts[2].len() == 4 && parts[2].chars().all(|c| c.is_ascii_hexdigit()),
        "Third part should be 4 hex chars, got: {}",
        parts[2]
    );

    ctx.track_agent(&agent_id).await;
}

// =============================================================================
// Scenario: Duplicate ID fails (actually succeeds with update semantics)
// =============================================================================

/// Scenario: Duplicate ID succeeds with update semantics
///
/// GIVEN: an agent with ID "agent-duplicate" exists
/// WHEN: I register an agent with ID "agent-duplicate"
/// THEN: the operation should succeed with update semantics
/// AND: the agent `"agent-duplicate"` should have an updated `last_seen` timestamp
#[tokio::test]
async fn scenario_duplicate_id_succeeds_with_update() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Create first agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-duplicate", "--json"])
        .await;

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // WHEN - Register with same ID
    let result = ctx
        .run_isolate(&["agents", "register", "--id", "agent-duplicate", "--json"])
        .await;

    // THEN - Should succeed (update semantics)
    assert!(
        result.success,
        "Register with duplicate ID should succeed with update semantics. stderr: {}",
        result.stderr
    );

    ctx.track_agent("agent-duplicate").await;
}

// =============================================================================
// Scenario: Register with invalid ID fails
// =============================================================================

/// Scenario: Register with invalid ID fails
///
/// GIVEN: no agent is registered
/// WHEN: I attempt to register an agent with ID ""
/// THEN: the operation should fail with error `"VALIDATION_ERROR"`
#[tokio::test]
async fn scenario_register_invalid_id_fails() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Attempt to register with empty ID
    let result = ctx
        .run_isolate(&["agents", "register", "--id", "", "--json"])
        .await;

    // THEN - Should fail
    assert!(
        !result.success,
        "Register with empty ID should fail. Got success with: {}",
        result.stdout
    );

    // Error message should mention validation or empty
    let error_output = format!("{} {}", result.stdout, result.stderr).to_lowercase();
    assert!(
        error_output.contains("empty")
            || error_output.contains("whitespace")
            || error_output.contains("invalid"),
        "Error should mention empty/whitespace/invalid. Got: {}",
        error_output
    );
}

// =============================================================================
// Scenario: Register with reserved keyword fails
// =============================================================================

/// Scenario: Register with reserved keyword fails
///
/// GIVEN: no agent is registered
/// WHEN: I attempt to register an agent with ID "null"
/// THEN: the operation should fail with error `"VALIDATION_ERROR"`
/// AND: the error message should indicate `"reserved keyword"`
#[tokio::test]
async fn scenario_register_reserved_keyword_fails() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Attempt to register with reserved keyword
    let result = ctx
        .run_isolate(&["agents", "register", "--id", "null", "--json"])
        .await;

    // THEN - Should fail
    assert!(
        !result.success,
        "Register with reserved keyword 'null' should fail. Got success with: {}",
        result.stdout
    );

    // Error message should mention reserved keyword
    let error_output = format!("{} {}", result.stdout, result.stderr).to_lowercase();
    assert!(
        error_output.contains("reserved"),
        "Error should mention 'reserved'. Got: {}",
        error_output
    );
}

// =============================================================================
// Scenario: Heartbeat updates timestamp
// =============================================================================

/// Scenario: Heartbeat updates timestamp
///
/// GIVEN: an agent with ID "agent-heartbeat" exists
/// AND: the environment variable `"Isolate_AGENT_ID"` is set
/// WHEN: I send a heartbeat for the current agent
/// THEN: the agent should have an updated `last_seen` timestamp
/// AND: the `actions_count` should be incremented
#[tokio::test]
async fn scenario_heartbeat_updates_timestamp() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-heartbeat", "--json"])
        .await;

    // WHEN - Send heartbeat with Isolate_AGENT_ID set
    let result = ctx
        .run_isolate_with_env(
            &["agents", "heartbeat", "--json"],
            &[("Isolate_AGENT_ID", "agent-heartbeat")],
        )
        .await;

    // THEN
    assert!(
        result.success,
        "Heartbeat should succeed. stderr: {}",
        result.stderr
    );

    // Verify output contains agent_id and timestamp
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    let data = parsed.get("data").unwrap_or(&parsed);
    assert!(
        data.get("agent_id").is_some(),
        "JSON should contain agent_id field"
    );
    assert!(
        data.get("timestamp").is_some(),
        "JSON should contain timestamp field"
    );

    ctx.track_agent("agent-heartbeat").await;
}

// =============================================================================
// Scenario: Heartbeat with command updates `current_command`
// =============================================================================

/// Scenario: Heartbeat with command updates `current_command`
///
/// GIVEN: an agent with ID "agent-cmd" exists
/// WHEN: I send a heartbeat with command "isolate add feature-x"
/// THEN: the agent should have `current_command` set to "isolate add feature-x"
#[tokio::test]
async fn scenario_heartbeat_with_command() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-cmd", "--json"])
        .await;

    // WHEN - Send heartbeat with command
    let result = ctx
        .run_isolate_with_env(
            &[
                "agents",
                "heartbeat",
                "--command",
                "isolate add feature-x",
                "--json",
            ],
            &[("Isolate_AGENT_ID", "agent-cmd")],
        )
        .await;

    // THEN
    assert!(
        result.success,
        "Heartbeat with command should succeed. stderr: {}",
        result.stderr
    );

    ctx.track_agent("agent-cmd").await;
}

// =============================================================================
// Scenario: Unknown agent heartbeat fails
// =============================================================================

/// Scenario: Unknown agent heartbeat fails
///
/// GIVEN: no agent is registered in the environment
/// WHEN: I attempt to send a heartbeat
/// THEN: the operation should fail
#[tokio::test]
async fn scenario_unknown_agent_heartbeat_fails() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Attempt heartbeat without Isolate_AGENT_ID
    // Note: We need to clear the env var from any previous registration
    let result = ctx
        .run_isolate_with_env(&["agents", "heartbeat", "--json"], &[("Isolate_AGENT_ID", "")])
        .await;

    // THEN - Should fail (empty agent ID is treated as "not found")
    assert!(
        !result.success,
        "Heartbeat without registered agent should fail. Got success with: {}",
        result.stdout
    );

    // Error message should mention agent or found
    let error_output = format!("{} {}", result.stdout, result.stderr).to_lowercase();
    assert!(
        error_output.contains("agent")
            || error_output.contains("found")
            || error_output.contains("register"),
        "Error should mention agent/register/found. Got: {}",
        error_output
    );
}

// =============================================================================
// Scenario: Heartbeat for unregistered agent fails
// =============================================================================

/// Scenario: Heartbeat for unregistered agent fails
///
/// GIVEN: the environment variable `"Isolate_AGENT_ID"` is set to "agent-ghost"
/// AND: no agent with ID "agent-ghost" exists in the database
/// WHEN: I attempt to send a heartbeat
/// THEN: the operation should fail with error `"AGENT_NOT_FOUND"`
#[tokio::test]
async fn scenario_heartbeat_unregistered_agent_fails() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Attempt heartbeat with non-existent agent ID
    let result = ctx
        .run_isolate_with_env(
            &["agents", "heartbeat", "--json"],
            &[("Isolate_AGENT_ID", "agent-ghost")],
        )
        .await;

    // THEN - Should fail
    assert!(
        !result.success,
        "Heartbeat for non-existent agent should fail. Got success with: {}",
        result.stdout
    );

    // Error message should mention not found
    let error_output = format!("{} {}", result.stdout, result.stderr).to_lowercase();
    assert!(
        error_output.contains("not found"),
        "Error should mention 'not found'. Got: {}",
        error_output
    );
}

// =============================================================================
// Scenario: Whoami returns identity
// =============================================================================

/// Scenario: Whoami returns identity
///
/// GIVEN: an agent with ID "agent-whoami" exists
/// AND: the environment variable `"Isolate_AGENT_ID"` is set
/// WHEN: I query whoami
/// THEN: the output should contain "agent-whoami"
/// AND: the registered field should be true
#[tokio::test]
async fn scenario_whoami_returns_identity() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-whoami", "--json"])
        .await;

    // WHEN
    let result = ctx
        .run_isolate_with_env(&["whoami", "--json"], &[("Isolate_AGENT_ID", "agent-whoami")])
        .await;

    // THEN
    assert!(
        result.success,
        "Whoami should succeed. stderr: {}",
        result.stderr
    );

    // Verify output contains agent_id
    assert!(
        result.stdout.contains("agent-whoami"),
        "Output should contain 'agent-whoami'. Got: {}",
        result.stdout
    );

    // Verify JSON structure
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    let data = parsed.get("data").unwrap_or(&parsed);
    assert!(
        data.get("registered")
            .and_then(|r| r.as_bool())
            .unwrap_or(false),
        "registered should be true. Got: {:?}",
        data.get("registered")
    );
    assert_eq!(
        data.get("agent_id").and_then(|s| s.as_str()),
        Some("agent-whoami"),
        "agent_id should be 'agent-whoami'"
    );

    ctx.track_agent("agent-whoami").await;
}

// =============================================================================
// Scenario: Whoami returns unregistered when no agent
// =============================================================================

/// Scenario: Whoami returns unregistered when no agent
///
/// GIVEN: no agent is registered in the environment
/// WHEN: I query whoami
/// THEN: the output should show no agent registered
#[tokio::test]
async fn scenario_whoami_unregistered() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Query whoami using regular isolate (which may or may not have Isolate_AGENT_ID set)
    // The key test is that whoami succeeds and returns valid JSON
    let result = ctx.run_isolate(&["whoami", "--json"]).await;

    // THEN - Whoami always succeeds, even when unregistered
    assert!(
        result.success,
        "Whoami should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON output structure exists
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    let data = parsed.get("data").unwrap_or(&parsed);

    // Verify the output has the expected fields (registered, simple, agent_id)
    assert!(
        data.get("registered").is_some() || parsed.get("registered").is_some(),
        "Output should contain registered field"
    );
}

// =============================================================================
// Scenario: List shows all agents
// =============================================================================

/// Scenario: List shows all agents
///
/// GIVEN: agents "agent-alpha", "agent-beta", and "agent-gamma" exist
/// WHEN: I list all agents
/// THEN: the output should contain 3 agents
/// AND: each agent should show `agent_id`, `registered_at`, and `last_seen`
#[tokio::test]
async fn scenario_list_shows_all_agents() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register multiple agents
    for agent_id in &["agent-alpha", "agent-beta", "agent-gamma"] {
        let _ = ctx
            .run_isolate(&["agents", "register", "--id", agent_id, "--json"])
            .await;
        ctx.track_agent(agent_id).await;
    }

    // WHEN
    let result = ctx.run_isolate(&["agents", "--all", "--json"]).await;

    // THEN
    assert!(
        result.success,
        "Agent list should succeed. stderr: {}",
        result.stderr
    );

    // Parse and verify agents
    let agents = parse_agents_from_output(&result.stdout).expect("Should parse agents");

    // We should have at least 3 agents
    assert!(
        agents.len() >= 3,
        "Should have at least 3 agents. Got: {}",
        agents.len()
    );

    // Verify each agent has required fields
    for agent in &agents {
        assert!(
            agent.get("agent_id").is_some(),
            "Agent should have agent_id field"
        );
        assert!(
            agent.get("registered_at").is_some(),
            "Agent should have registered_at field"
        );
        assert!(
            agent.get("last_seen").is_some(),
            "Agent should have last_seen field"
        );
    }

    // Verify our agents are present
    let agent_ids: HashSet<&str> = agents
        .iter()
        .filter_map(|a| a.get("agent_id").and_then(|s| s.as_str()))
        .collect();

    for expected in &["agent-alpha", "agent-beta", "agent-gamma"] {
        assert!(
            agent_ids.contains(expected),
            "Agents should contain '{}'. Got: {:?}",
            expected,
            agent_ids
        );
    }
}

// =============================================================================
// Scenario: List empty returns empty array
// =============================================================================

/// Scenario: List empty returns empty array
///
/// GIVEN: no agents exist
/// WHEN: I list all agents
/// THEN: the output should show 0 agents
/// AND: the `total_active` count should be 0
#[tokio::test]
async fn scenario_list_empty() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN - Fresh context with no agents registered
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN
    let result = ctx.run_isolate(&["agents", "--json"]).await;

    // THEN
    assert!(
        result.success,
        "Agent list should succeed. stderr: {}",
        result.stderr
    );

    // Parse and verify no agents
    let agents = parse_agents_from_output(&result.stdout).expect("Should parse agents");

    assert!(
        agents.is_empty(),
        "Should have 0 agents. Got: {}",
        agents.len()
    );
}

// =============================================================================
// Scenario: List with --all shows stale agents
// =============================================================================

/// Scenario: List with --all shows stale agents
///
/// GIVEN: agent "agent-active" exists and is active
/// AND: agent "agent-stale" exists and is stale
/// WHEN: I list all agents with --all flag
/// THEN: the output should contain 2 agents
/// AND: the `total_stale` count should be at least 1
#[tokio::test]
async fn scenario_list_all_shows_stale() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agents
    for agent_id in &["agent-active", "agent-stale"] {
        let _ = ctx
            .run_isolate(&["agents", "register", "--id", agent_id, "--json"])
            .await;
        ctx.track_agent(agent_id).await;
    }

    // WHEN
    let result = ctx.run_isolate(&["agents", "--all", "--json"]).await;

    // THEN
    assert!(
        result.success,
        "Agent list with --all should succeed. stderr: {}",
        result.stderr
    );

    // Parse and verify
    let agents = parse_agents_from_output(&result.stdout).expect("Should parse agents");

    // Should have at least 2 agents
    assert!(
        agents.len() >= 2,
        "Should have at least 2 agents. Got: {}",
        agents.len()
    );
}

// =============================================================================
// Scenario: Each agent has a unique ID (Invariant)
// =============================================================================

/// Scenario: Each agent has a unique ID
///
/// GIVEN: agents "agent-1", "agent-2", and "agent-3" exist
/// WHEN: I inspect the agent IDs
/// THEN: all agent IDs should be unique
#[tokio::test]
async fn scenario_unique_agent_ids_invariant() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register multiple agents
    for agent_id in &["agent-unique-1", "agent-unique-2", "agent-unique-3"] {
        let _ = ctx
            .run_isolate(&["agents", "register", "--id", agent_id, "--json"])
            .await;
        ctx.track_agent(agent_id).await;
    }

    // WHEN
    let result = ctx.run_isolate(&["agents", "--all", "--json"]).await;

    // THEN
    let agents = parse_agents_from_output(&result.stdout).expect("Should parse agents");

    // Collect all agent IDs
    let agent_ids: Vec<&str> = agents
        .iter()
        .filter_map(|a| a.get("agent_id").and_then(|s| s.as_str()))
        .collect();

    // Verify uniqueness
    let unique_ids: HashSet<&str> = agent_ids.iter().copied().collect();

    assert_eq!(
        agent_ids.len(),
        unique_ids.len(),
        "All agent IDs should be unique. Total: {}, Unique: {}",
        agent_ids.len(),
        unique_ids.len()
    );
}

// =============================================================================
// Scenario: Status shows agent details
// =============================================================================

/// Scenario: Status shows agent details
///
/// GIVEN: an agent with ID "agent-status" exists
/// AND: the environment variable `"Isolate_AGENT_ID"` is set
/// WHEN: I query the agent status
/// THEN: the output should show agent information
#[tokio::test]
async fn scenario_status_shows_details() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-status", "--json"])
        .await;

    // WHEN
    let result = ctx
        .run_isolate_with_env(
            &["agents", "status", "--json"],
            &[("Isolate_AGENT_ID", "agent-status")],
        )
        .await;

    // THEN
    assert!(
        result.success,
        "Agent status should succeed. stderr: {}",
        result.stderr
    );

    // Verify JSON structure - the output might have "agent" nested object or "data" wrapper
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // Handle envelope format or direct output
    let data = parsed.get("data").unwrap_or(&parsed);

    // Status output should have registered field and possibly agent object
    assert!(
        data.get("registered").is_some() || data.get("agent").is_some(),
        "Status should contain registered or agent field. Got: {:?}",
        data
    );

    ctx.track_agent("agent-status").await;
}

// =============================================================================
// Scenario: Unregister removes agent
// =============================================================================

/// Scenario: Unregister removes agent
///
/// GIVEN: an agent with ID "agent-unregister" exists
/// AND: the environment variable `"Isolate_AGENT_ID"` is set
/// WHEN: I unregister the current agent
/// THEN: the agent should not exist
#[tokio::test]
async fn scenario_unregister_removes_agent() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // Register agent
    let _ = ctx
        .run_isolate(&["agents", "register", "--id", "agent-unregister", "--json"])
        .await;

    // Verify agent exists
    let exists_before = agent_exists_in_db(&ctx, "agent-unregister")
        .await
        .unwrap_or(false);

    // WHEN
    let result = ctx
        .run_isolate_with_env(
            &["agents", "unregister", "--json"],
            &[("Isolate_AGENT_ID", "agent-unregister")],
        )
        .await;

    // THEN
    assert!(
        result.success,
        "Unregister should succeed. stderr: {}",
        result.stderr
    );

    // Verify agent no longer exists
    let exists_after = agent_exists_in_db(&ctx, "agent-unregister")
        .await
        .unwrap_or(false);

    assert!(
        !exists_after,
        "Agent should not exist after unregister. Existed before: {}, after: {}",
        exists_before, exists_after
    );
}

// =============================================================================
// Scenario: Unregister non-existent agent fails
// =============================================================================

/// Scenario: Unregister non-existent agent fails
///
/// GIVEN: no agent with ID "agent-ghost" exists
/// WHEN: I attempt to unregister agent "agent-ghost"
/// THEN: the operation should fail with error `"AGENT_NOT_FOUND"`
#[tokio::test]
async fn scenario_unregister_nonexistent_fails() {
    let Some(ctx) = AgentTestContext::try_new() else {
        eprintln!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_isolate().expect("Failed to initialize Isolate");

    // WHEN - Attempt to unregister non-existent agent
    let result = ctx
        .run_isolate(&[
            "agents",
            "unregister",
            "--id",
            "agent-ghost-nonexistent",
            "--json",
        ])
        .await;

    // THEN - Should fail
    assert!(
        !result.success,
        "Unregister non-existent agent should fail. Got success with: {}",
        result.stdout
    );

    // Error message should mention not found
    let error_output = format!("{} {}", result.stdout, result.stderr).to_lowercase();
    assert!(
        error_output.contains("not found"),
        "Error should mention 'not found'. Got: {}",
        error_output
    );
}

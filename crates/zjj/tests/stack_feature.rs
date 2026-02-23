//! BDD Acceptance Tests for Stack Management Feature
//!
//! Feature: Stack Management
//!
//! As a developer using ZJJ
//! I want to manage stacked sessions for hierarchical work
//! So that I can build features on top of other features
//!
//! This test file implements the BDD scenarios defined in `features/stack.feature`
//! using Dan North BDD style with Given/When/Then syntax.
//!
//! # ATDD Phase
//!
//! These tests define expected behavior before implementation.
//! Run with: `cargo test --test stack_feature`
//!
//! # Key Invariants
//!
//! - Acyclic graph: No workspace can be its own ancestor
//! - Parent always exists for non-root workspaces
//! - Depth = parent's depth + 1

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::bool_assert_comparison)]

mod common;

use std::sync::Arc;

use anyhow::{Context, Result};
use common::{CommandResult, TestHarness};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;
use zjj_core::{coordination::StackError, MergeQueue};

// =============================================================================
// Stack Test Context
// =============================================================================

/// Stack test context that holds state for each scenario
pub struct StackTestContext {
    /// The test harness for running commands
    pub harness: TestHarness,
    /// The merge queue for direct operations
    pub queue: Arc<Mutex<Option<MergeQueue>>>,
    /// Track the last workspace for assertions
    pub last_workspace: Arc<Mutex<Option<String>>>,
    /// Track the last operation result
    pub last_result: Arc<Mutex<Option<CommandResult>>>,
}

impl StackTestContext {
    /// Create a new stack test context
    pub fn new() -> Result<Self> {
        let harness = TestHarness::new()?;
        Ok(Self {
            harness,
            queue: Arc::new(Mutex::new(None)),
            last_workspace: Arc::new(Mutex::new(None)),
            last_result: Arc::new(Mutex::new(None)),
        })
    }

    /// Try to create a new context, returning None if jj is unavailable
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }

    /// Initialize the ZJJ database
    pub fn init_zjj(&self) -> Result<()> {
        self.harness.assert_success(&["init"]);
        if !self.harness.zjj_dir().exists() {
            anyhow::bail!("ZJJ initialization failed - .zjj directory not created");
        }
        Ok(())
    }

    /// Initialize the queue database
    pub async fn init_queue(&self) -> Result<MergeQueue> {
        let queue_db = self.harness.repo_path.join(".zjj").join("state.db");
        let queue = MergeQueue::open(&queue_db)
            .await
            .context("Failed to open merge queue database")?;
        *self.queue.lock().await = Some(queue.clone());
        Ok(queue)
    }

    /// Get the queue, initializing if necessary
    pub async fn get_queue(&self) -> Result<MergeQueue> {
        let guard = self.queue.lock().await;
        if let Some(queue) = guard.as_ref() {
            return Ok(queue.clone());
        }
        drop(guard);
        self.init_queue().await
    }

    /// Store a workspace name for later assertions
    pub async fn track_workspace(&self, name: &str) {
        *self.last_workspace.lock().await = Some(name.to_string());
    }
}

// =============================================================================
// Scenario: List shows tree structure (via status for each workspace)
// =============================================================================

/// Scenario: List shows tree structure
///
/// GIVEN: a root session named "main-feature" exists
/// AND: a child session named "sub-feature-a" exists with parent "main-feature"
/// AND: a child session named "sub-feature-b" exists with parent "main-feature"
/// AND: a grandchild session named "nested-feature" exists with parent "sub-feature-a"
/// WHEN: I show the stack status for each workspace
/// THEN: the output should show parent-child relationships
#[tokio::test]
async fn scenario_list_shows_tree_structure() {
    let Some(ctx) = StackTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to init zjj");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    // Create root
    queue
        .add("main-feature", None, 0, None)
        .await
        .expect("Failed to add root");

    // Create children with parent
    queue
        .add("sub-feature-a", None, 0, Some("main-feature"))
        .await
        .expect("Failed to add sub-feature-a");
    queue
        .add("sub-feature-b", None, 0, Some("main-feature"))
        .await
        .expect("Failed to add sub-feature-b");
    queue
        .add("nested-feature", None, 0, Some("sub-feature-a"))
        .await
        .expect("Failed to add nested-feature");

    // WHEN - Check status of nested-feature to verify tree
    let result = ctx
        .harness
        .zjj(&["stack", "status", "nested-feature", "--json"]);
    *ctx.last_result.lock().await = Some(result.clone());

    // THEN
    assert!(
        result.success,
        "Stack status should succeed. stderr: {}",
        result.stderr
    );

    // Verify tree structure via status
    assert!(
        result.stdout.contains("nested-feature"),
        "Output should contain 'nested-feature'. Got: {}",
        result.stdout
    );

    // Parse JSON and verify parent chain
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // Verify parent is sub-feature-a
    let parent = parsed
        .get("payload")
        .and_then(|p| p.get("parent"))
        .or_else(|| parsed.get("parent"))
        .and_then(|p| p.as_str());
    assert_eq!(
        parent,
        Some("sub-feature-a"),
        "Parent should be 'sub-feature-a', got: {:?}",
        parent
    );

    // Verify root is main-feature
    let root = parsed
        .get("payload")
        .and_then(|p| p.get("root"))
        .or_else(|| parsed.get("root"))
        .and_then(|r| r.as_str());
    assert_eq!(
        root,
        Some("main-feature"),
        "Root should be 'main-feature', got: {:?}",
        root
    );

    // Verify depth is 2 (nested -> sub-feature-a -> main-feature)
    let depth = parsed
        .get("payload")
        .and_then(|p| p.get("depth"))
        .or_else(|| parsed.get("depth"))
        .and_then(|d| d.as_i64());
    assert_eq!(depth, Some(2), "Depth should be 2, got: {:?}", depth);
}

// =============================================================================
// Scenario: Show displays stack context
// =============================================================================

/// Scenario: Show displays stack context
///
/// GIVEN: a root session named "root-session" exists
/// AND: a child session named "child-session" exists with parent "root-session"
/// AND: a grandchild session named "grandchild-session" exists with parent "child-session"
/// WHEN: I show the stack status for "grandchild-session"
/// THEN: the output should contain the workspace name
/// AND: the output should show depth 2
/// AND: the output should show parent "child-session"
/// AND: the output should show root "root-session"
#[tokio::test]
async fn scenario_show_displays_stack_context() {
    let Some(ctx) = StackTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to init zjj");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    // Create hierarchy
    queue
        .add("root-session", None, 0, None)
        .await
        .expect("Failed to add root-session");
    queue
        .add("child-session", None, 0, Some("root-session"))
        .await
        .expect("Failed to add child-session");
    queue
        .add("grandchild-session", None, 0, Some("child-session"))
        .await
        .expect("Failed to add grandchild-session");

    // WHEN
    let result = ctx
        .harness
        .zjj(&["stack", "status", "grandchild-session", "--json"]);
    *ctx.last_result.lock().await = Some(result.clone());

    // THEN
    assert!(
        result.success,
        "Stack status should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("grandchild-session"),
        "Output should contain workspace name. Got: {}",
        result.stdout
    );

    // Parse JSON and verify structure
    let parsed: JsonValue =
        serde_json::from_str(&result.stdout).expect("Output should be valid JSON");

    // Verify depth (should be 2: grandchild -> child -> root)
    let depth = parsed
        .get("payload")
        .and_then(|p| p.get("depth"))
        .or_else(|| parsed.get("depth"))
        .and_then(|d| d.as_i64());
    assert!(
        depth == Some(2),
        "Depth should be 2, got: {:?}. Output: {}",
        depth,
        result.stdout
    );

    // Verify parent
    let parent = parsed
        .get("payload")
        .and_then(|p| p.get("parent"))
        .or_else(|| parsed.get("parent"))
        .and_then(|p| p.as_str());
    assert!(
        parent == Some("child-session"),
        "Parent should be 'child-session', got: {:?}",
        parent
    );

    // Verify root
    let root = parsed
        .get("payload")
        .and_then(|p| p.get("root"))
        .or_else(|| parsed.get("root"))
        .and_then(|r| r.as_str());
    assert!(
        root == Some("root-session"),
        "Root should be 'root-session', got: {:?}",
        root
    );
}

// =============================================================================
// Scenario: Restack updates children (tested via build_dependent_list)
// =============================================================================

/// Scenario: Restack updates children
///
/// GIVEN: a root session named "base-feature" exists
/// AND: a child session named "dependent-feature" exists with parent "base-feature"
/// WHEN: I restack the stack rooted at "base-feature"
/// THEN: all children should be identified for restacking
///
/// Note: This tests the pure function that identifies dependents for restacking.
/// The CLI restack command may not be fully implemented yet.
#[tokio::test]
async fn scenario_restack_updates_children() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState,
        stack_depth::build_dependent_list, QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN - Create parent-child relationship
    let entries = vec![
        create_entry("base-feature", None),
        create_entry("dependent-feature", Some("base-feature")),
    ];

    // WHEN - Get dependents that would need restacking
    let dependents = build_dependent_list("base-feature", &entries);

    // THEN - All children should be identified
    assert_eq!(dependents.len(), 1, "base-feature should have 1 dependent");
    assert_eq!(
        dependents[0], "dependent-feature",
        "dependent should be identified for restacking"
    );
}

// =============================================================================
// Scenario: Cycle detection prevents creation
// =============================================================================

/// Scenario: Cycle detection prevents creation
///
/// GIVEN: a root session named "ancestor" exists
/// AND: a child session named "descendant" exists with parent "ancestor"
/// WHEN: I attempt to set the parent of "ancestor" to "descendant"
/// THEN: the operation should fail with error `"CYCLE_DETECTED"`
///
/// Note: Tests the pure cycle detection function directly.
#[tokio::test]
async fn scenario_cycle_detection_prevents_creation() {
    // GIVEN - Use pure function for cycle detection test
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState, stack_depth::validate_no_cycle,
        QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // Create entries with ancestor <- descendant relationship
    let entries = vec![
        create_entry("ancestor", None),
        create_entry("descendant", Some("ancestor")),
    ];

    // WHEN - Attempt to create cycle (set ancestor's parent to descendant)
    let result = validate_no_cycle("ancestor", "descendant", &entries);

    // THEN
    assert!(result.is_err(), "Should detect cycle");

    match result {
        Err(StackError::CycleDetected {
            workspace,
            cycle_path,
        }) => {
            assert_eq!(
                workspace, "ancestor",
                "Cycle should be detected for 'ancestor'"
            );
            assert!(
                !cycle_path.is_empty(),
                "Cycle path should not be empty: {:?}",
                cycle_path
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Should have detected cycle"),
    }
}

// =============================================================================
// Scenario: Restack root with no children is no-op
// =============================================================================

/// Scenario: Restack root with no children is no-op
///
/// GIVEN: a root session named "lonely-root" exists
/// AND: the session has no children
/// WHEN: I check for dependents to restack
/// THEN: the operation should return empty list (no-op)
#[tokio::test]
async fn scenario_restack_root_no_children_is_noop() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState,
        stack_depth::build_dependent_list, QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN - A root with no children
    let entries = vec![create_entry("lonely-root")];

    // WHEN - Get dependents
    let dependents = build_dependent_list("lonely-root", &entries);

    // THEN - No children to restack
    assert!(
        dependents.is_empty(),
        "lonely-root should have no dependents - restack would be no-op"
    );
}

// =============================================================================
// Scenario: Create stacked session with parent (via queue.add)
// =============================================================================

/// Scenario: Create stacked session with parent
///
/// GIVEN: a root session named "parent-session" exists
/// WHEN: I create a stacked session named "child-session" with parent "parent-session"
/// THEN: the session "child-session" should exist
/// AND: the session "child-session" should have parent "parent-session"
/// AND: the session "child-session" should have depth 1
#[tokio::test]
async fn scenario_create_stacked_session_with_parent() {
    let Some(ctx) = StackTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to init zjj");
    let queue = ctx.get_queue().await.expect("Failed to get queue");

    // Create parent
    queue
        .add("parent-session", None, 0, None)
        .await
        .expect("Failed to add parent-session");

    // WHEN - Create child with parent using queue API
    let result = queue
        .add("child-session", None, 0, Some("parent-session"))
        .await;
    *ctx.last_result.lock().await = Some(CommandResult {
        success: result.is_ok(),
        exit_code: if result.is_ok() { Some(0) } else { Some(1) },
        stdout: String::new(),
        stderr: result
            .as_ref()
            .map_or_else(|e| e.to_string(), |_| String::new()),
    });

    // THEN
    assert!(result.is_ok(), "Create stacked session should succeed");

    // Verify child exists with correct parent
    let child = queue
        .get_by_workspace("child-session")
        .await
        .expect("Failed to query child");
    assert!(child.is_some(), "child-session should exist in queue");

    let child = child.expect("Child should exist");
    assert_eq!(
        child.parent_workspace,
        Some("parent-session".to_string()),
        "Child should have parent-session as parent"
    );

    // Verify depth calculation
    let all_entries = queue.list(None).await.expect("Failed to list entries");
    let depth = zjj_core::coordination::calculate_stack_depth("child-session", &all_entries)
        .expect("Failed to calculate depth");
    assert_eq!(depth, 1, "Child should have depth 1");
}

// =============================================================================
// Scenario: Create stacked session with non-existent parent (depth calculation fails)
// =============================================================================

/// Scenario: Create stacked session with non-existent parent
///
/// Test that when a session references a non-existent parent,
/// the depth calculation returns `ParentNotFound` error.
#[tokio::test]
async fn scenario_create_stacked_nonexistent_parent_fails() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState,
        stack_depth::calculate_stack_depth, QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN - An orphan with non-existent parent
    let entries = vec![create_entry("orphan-session", Some("nonexistent-parent"))];

    // WHEN - Try to calculate depth
    let result = calculate_stack_depth("orphan-session", &entries);

    // THEN - Should fail with ParentNotFound
    assert!(result.is_err(), "Should fail for non-existent parent");

    match result {
        Err(StackError::ParentNotFound { parent_workspace }) => {
            assert_eq!(
                parent_workspace, "nonexistent-parent",
                "Error should reference the missing parent"
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Should have failed for non-existent parent"),
    }
}

// =============================================================================
// Scenario: Depth calculation is consistent
// =============================================================================

/// Scenario: Depth calculation is consistent
///
/// Test the pure depth calculation function directly
#[tokio::test]
async fn scenario_depth_calculation_is_consistent() {
    // GIVEN - Create chain using pure functions
    use zjj_core::coordination::{
        queue_entities::Dependents,
        queue_status::StackMergeState,
        stack_depth::{calculate_stack_depth, find_stack_root},
        QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // Create chain: depth-0 <- depth-1 <- depth-2 <- depth-3
    let entries = vec![
        create_entry("depth-0", None),
        create_entry("depth-1", Some("depth-0")),
        create_entry("depth-2", Some("depth-1")),
        create_entry("depth-3", Some("depth-2")),
    ];

    // WHEN & THEN - Verify depths
    assert_eq!(
        calculate_stack_depth("depth-0", &entries).expect("depth-0"),
        0,
        "Root should have depth 0"
    );
    assert_eq!(
        calculate_stack_depth("depth-1", &entries).expect("depth-1"),
        1,
        "First child should have depth 1"
    );
    assert_eq!(
        calculate_stack_depth("depth-2", &entries).expect("depth-2"),
        2,
        "Second child should have depth 2"
    );
    assert_eq!(
        calculate_stack_depth("depth-3", &entries).expect("depth-3"),
        3,
        "Third child should have depth 3"
    );

    // Verify root
    assert_eq!(
        find_stack_root("depth-3", &entries).expect("root for depth-3"),
        "depth-0",
        "Root of depth-3 should be depth-0"
    );
}

// =============================================================================
// Scenario: Self-parent cycle detection
// =============================================================================

/// Scenario: Self-parent cycle detection
///
/// Test that setting parent to self is detected as a cycle
#[tokio::test]
async fn scenario_self_parent_cycle_detection() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState, stack_depth::validate_no_cycle,
        QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: None,
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN
    let entries = vec![create_entry("self-loop-test")];

    // WHEN - Attempt to set parent to self
    let result = validate_no_cycle("self-loop-test", "self-loop-test", &entries);

    // THEN
    assert!(result.is_err(), "Self-parent should be detected as cycle");

    match result {
        Err(StackError::CycleDetected {
            workspace,
            cycle_path,
        }) => {
            assert_eq!(
                workspace, "self-loop-test",
                "Cycle should be detected for self-loop-test"
            );
            assert!(
                cycle_path.contains(&"self-loop-test".to_string()),
                "Cycle path should contain self-loop-test: {:?}",
                cycle_path
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Should have detected self-parent cycle"),
    }
}

// =============================================================================
// Scenario: Acyclic invariant is enforced
// =============================================================================

/// Scenario: Acyclic invariant is enforced
///
/// Test that indirect cycles are also detected
#[tokio::test]
async fn scenario_acyclic_invariant_enforced() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState, stack_depth::validate_no_cycle,
        QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN - Chain: a <- b <- c
    let entries = vec![
        create_entry("a", None),
        create_entry("b", Some("a")),
        create_entry("c", Some("b")),
    ];

    // WHEN - Attempt to create cycle (set a's parent to c)
    let result = validate_no_cycle("a", "c", &entries);

    // THEN
    assert!(result.is_err(), "Should detect indirect cycle");

    match result {
        Err(StackError::CycleDetected {
            workspace,
            cycle_path,
        }) => {
            assert_eq!(workspace, "a", "Cycle should be detected for 'a'");
            // Cycle path should show the loop
            assert!(
                !cycle_path.is_empty(),
                "Cycle path should not be empty: {:?}",
                cycle_path
            );
        }
        Err(e) => panic!("Unexpected error type: {:?}", e),
        Ok(_) => panic!("Should have detected cycle"),
    }
}

// =============================================================================
// Scenario: Stack status for non-existent workspace
// =============================================================================

/// Scenario: Stack status for non-existent workspace
///
/// GIVEN: no sessions exist
/// WHEN: I check stack status for a workspace
/// THEN: the operation should indicate the workspace is not in queue
#[tokio::test]
async fn scenario_status_nonexistent_workspace() {
    let Some(ctx) = StackTestContext::try_new() else {
        println!("SKIP: jj not available");
        return;
    };

    // GIVEN
    ctx.init_zjj().expect("Failed to init zjj");

    // Verify queue is empty
    let queue = ctx.get_queue().await.expect("Failed to get queue");
    let entries = queue.list(None).await.expect("Failed to list");
    assert!(entries.is_empty(), "Queue should be empty initially");

    // WHEN - Check status for non-existent workspace
    let result = ctx
        .harness
        .zjj(&["stack", "status", "nonexistent-workspace", "--json"]);
    *ctx.last_result.lock().await = Some(result.clone());

    // THEN - Should indicate not found (exit code 2 based on handler)
    // The operation should not succeed (workspace not in queue)
    assert!(
        !result.success || result.exit_code == Some(2),
        "Status for nonexistent workspace should fail or return exit code 2. exit_code: {:?}, stderr: {}",
        result.exit_code,
        result.stderr
    );
}

// =============================================================================
// Scenario: Build dependent list
// =============================================================================

/// Scenario: Build dependent list (children of a workspace)
///
/// Test the pure `build_dependent_list` function
#[tokio::test]
async fn scenario_build_dependent_list() {
    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState,
        stack_depth::build_dependent_list, QueueEntry, QueueStatus, WorkspaceQueueState,
    };

    fn create_entry(workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: None,
            priority: 0,
            status: QueueStatus::Pending,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    // GIVEN - Tree structure:
    //     root
    //     /   \
    //   child1 child2
    //    |
    //  grandchild
    let entries = vec![
        create_entry("root", None),
        create_entry("child1", Some("root")),
        create_entry("child2", Some("root")),
        create_entry("grandchild", Some("child1")),
    ];

    // WHEN - Build dependent list for root
    let dependents = build_dependent_list("root", &entries);

    // THEN - Should include all descendants in BFS order
    assert_eq!(
        dependents.len(),
        3,
        "Root should have 3 dependents (children + grandchildren)"
    );
    assert!(
        dependents.contains(&"child1".to_string()),
        "Should contain child1"
    );
    assert!(
        dependents.contains(&"child2".to_string()),
        "Should contain child2"
    );
    assert!(
        dependents.contains(&"grandchild".to_string()),
        "Should contain grandchild"
    );

    // Verify BFS order: child1 and child2 should come before grandchild
    let child1_pos = dependents
        .iter()
        .position(|s| s == "child1")
        .expect("child1 position");
    let grandchild_pos = dependents
        .iter()
        .position(|s| s == "grandchild")
        .expect("grandchild position");
    assert!(
        child1_pos < grandchild_pos,
        "child1 should appear before grandchild in BFS order"
    );

    // Verify child1 has only grandchild as dependent
    let child1_dependents = build_dependent_list("child1", &entries);
    assert_eq!(child1_dependents.len(), 1, "child1 should have 1 dependent");
    assert_eq!(
        child1_dependents[0], "grandchild",
        "child1's dependent should be grandchild"
    );

    // Verify leaf has no dependents
    let grandchild_dependents = build_dependent_list("grandchild", &entries);
    assert!(
        grandchild_dependents.is_empty(),
        "grandchild should have no dependents"
    );
}

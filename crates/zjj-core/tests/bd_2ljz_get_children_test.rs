//! ATDD Test for bd-2ljz: Add get_children method to QueueRepository
//!
//! BEAD: bd-2ljz
//! REQUIREMENT: Add a `get_children` method to QueueRepository trait
//! CONTRACT: `async fn get_children(&self, workspace: &str) -> Result<Vec<QueueEntry>>`
//! EARS:
//!   - THE SYSTEM SHALL provide get_children(workspace) -> Result<Vec<QueueEntry>>
//!   - WHEN given workspace, THE SYSTEM SHALL find all entries where parent_workspace ==
//!     Some(workspace)
//!   - IF no children exist, THE SYSTEM SHALL return Ok(Vec::new()) (empty is valid, not an error)
//!   - THE SYSTEM SHALL return only direct children (not grandchildren or deeper)
//!
//! This test file should:
//!   1. COMPILE (method signature is valid Rust)
//!   2. FAIL initially (method doesn't exist yet)
//!   3. PASS after implementation

#![allow(
    clippy::doc_markdown,
    clippy::unreadable_literal,
    clippy::unimplemented
)]

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;
use zjj_core::{
    coordination::{
        queue::{QueueAddResponse, QueueControlError, QueueStats, RecoveryStats},
        queue_entities::{Dependents, ProcessingLock, QueueEntry, QueueEvent},
        queue_repository::QueueRepository,
        queue_status::{QueueEventType, QueueStatus, StackMergeState, WorkspaceQueueState},
    },
    Result,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MOCK IMPLEMENTATION: InMemoryQueueRepository for testing
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// In-memory implementation of QueueRepository for ATDD testing.
///
/// This mock stores entries in a HashMap and provides deterministic
/// behavior for testing the get_children method.
struct InMemoryQueueRepository {
    entries: Arc<RwLock<HashMap<String, QueueEntry>>>,
}

impl InMemoryQueueRepository {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn insert_entry(&self, entry: QueueEntry) {
        let mut entries = self.entries.write().await;
        entries.insert(entry.workspace.clone(), entry);
    }
}

#[async_trait]
impl QueueRepository for InMemoryQueueRepository {
    async fn add(
        &self,
        _workspace: &str,
        _bead_id: Option<&str>,
        _priority: i32,
        _agent_id: Option<&str>,
    ) -> Result<QueueAddResponse> {
        unimplemented!("not needed for get_children tests")
    }

    async fn add_with_dedupe(
        &self,
        _workspace: &str,
        _bead_id: Option<&str>,
        _priority: i32,
        _agent_id: Option<&str>,
        _dedupe_key: Option<&str>,
    ) -> Result<QueueAddResponse> {
        unimplemented!("not needed for get_children tests")
    }

    async fn upsert_for_submit(
        &self,
        _workspace: &str,
        _bead_id: Option<&str>,
        _priority: i32,
        _agent_id: Option<&str>,
        _dedupe_key: &str,
        _head_sha: &str,
    ) -> Result<QueueEntry> {
        unimplemented!("not needed for get_children tests")
    }

    async fn get_by_id(&self, _id: i64) -> Result<Option<QueueEntry>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(workspace).cloned())
    }

    async fn list(&self, _filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.values().cloned().collect())
    }

    async fn next(&self) -> Result<Option<QueueEntry>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn remove(&self, _workspace: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn position(&self, _workspace: &str) -> Result<Option<usize>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn count_pending(&self) -> Result<usize> {
        unimplemented!("not needed for get_children tests")
    }

    async fn stats(&self) -> Result<QueueStats> {
        unimplemented!("not needed for get_children tests")
    }

    async fn acquire_processing_lock(&self, _agent_id: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn release_processing_lock(&self, _agent_id: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn get_processing_lock(&self) -> Result<Option<ProcessingLock>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn extend_lock(&self, _agent_id: &str, _extra_secs: i64) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn mark_processing(&self, _workspace: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn mark_completed(&self, _workspace: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn mark_failed(&self, _workspace: &str, _error: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn next_with_lock(&self, _agent_id: &str) -> Result<Option<QueueEntry>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn transition_to(&self, _workspace: &str, _new_status: QueueStatus) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn transition_to_failed(
        &self,
        _workspace: &str,
        _error_message: &str,
        _is_retryable: bool,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn update_rebase_metadata(
        &self,
        _workspace: &str,
        _head_sha: &str,
        _tested_against_sha: &str,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn update_rebase_metadata_with_count(
        &self,
        _workspace: &str,
        _head_sha: &str,
        _tested_against_sha: &str,
        _rebase_count: i32,
        _rebase_timestamp: i64,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn is_fresh(&self, _workspace: &str, _current_main_sha: &str) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    async fn return_to_rebasing(&self, _workspace: &str, _new_main_sha: &str) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn begin_merge(&self, _workspace: &str) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn complete_merge(&self, _workspace: &str, _merged_sha: &str) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn fail_merge(
        &self,
        _workspace: &str,
        _error_message: &str,
        _is_retryable: bool,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn retry_entry(&self, _id: i64) -> std::result::Result<QueueEntry, QueueControlError> {
        unimplemented!("not needed for get_children tests")
    }

    async fn cancel_entry(&self, _id: i64) -> std::result::Result<QueueEntry, QueueControlError> {
        unimplemented!("not needed for get_children tests")
    }

    async fn append_event(
        &self,
        _queue_id: i64,
        _event_type: &str,
        _details: Option<&str>,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn append_typed_event(
        &self,
        _queue_id: i64,
        _event_type: QueueEventType,
        _details: Option<&str>,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn fetch_events(&self, _queue_id: i64) -> Result<Vec<QueueEvent>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn fetch_recent_events(&self, _queue_id: i64, _limit: usize) -> Result<Vec<QueueEvent>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn cleanup(&self, _max_age: std::time::Duration) -> Result<usize> {
        unimplemented!("not needed for get_children tests")
    }

    async fn reclaim_stale(&self, _stale_threshold_secs: i64) -> Result<usize> {
        unimplemented!("not needed for get_children tests")
    }

    async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
        unimplemented!("not needed for get_children tests")
    }

    async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
        unimplemented!("not needed for get_children tests")
    }

    async fn is_lock_stale(&self) -> Result<bool> {
        unimplemented!("not needed for get_children tests")
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // METHOD UNDER TEST: get_children
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    async fn get_children(&self, workspace: &str) -> Result<Vec<QueueEntry>> {
        let entries = self.entries.read().await;
        let children: Vec<QueueEntry> = entries
            .values()
            .filter(|entry| entry.parent_workspace.as_deref() == Some(workspace))
            .cloned()
            .collect();
        Ok(children)
    }

    async fn get_stack_root(&self, _workspace: &str) -> Result<Option<QueueEntry>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn update_dependents(&self, _workspace: &str, _dependents: &[String]) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn transition_stack_state(
        &self,
        _workspace: &str,
        _new_state: StackMergeState,
    ) -> Result<()> {
        unimplemented!("not needed for get_children tests")
    }

    async fn find_blocked(&self) -> Result<Vec<QueueEntry>> {
        unimplemented!("not needed for get_children tests")
    }

    async fn cascade_unblock(&self, _merged_workspace: &str) -> Result<usize> {
        unimplemented!("not needed for get_children tests")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER: Create test QueueEntry with parent relationship
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Create a test QueueEntry with the specified workspace and optional parent.
fn create_entry(id: i64, workspace: &str, parent_workspace: Option<&str>) -> QueueEntry {
    QueueEntry {
        id,
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Trait method signature compile-time check
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children method exists on QueueRepository trait.
///
/// GIVEN: The QueueRepository trait
/// WHEN: Calling get_children on a trait object
/// THEN: The method compiles and has correct signature
#[test]
fn test_get_children_method_exists_on_trait() {
    // This test verifies the method exists by ensuring the trait object
    // can call the method. If the method doesn't exist, this won't compile.
    use std::future::Future;
    fn _assert_trait_has_get_children<'a>(
        repo: &'a dyn QueueRepository,
    ) -> impl Future<Output = Result<Vec<QueueEntry>>> + 'a {
        repo.get_children("test")
    }
    // If we get here, the trait has the method with correct signature
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Returns children when they exist
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children returns direct children of a workspace.
///
/// GIVEN: A workspace with direct children
/// WHEN: Calling get_children
/// THEN: Returns Vec containing all direct children
#[tokio::test]
async fn test_get_children_returns_direct_children() {
    let repo = InMemoryQueueRepository::new();

    // Setup: root -> child1, child2
    repo.insert_entry(create_entry(1, "root", None)).await;
    repo.insert_entry(create_entry(2, "child1", Some("root")))
        .await;
    repo.insert_entry(create_entry(3, "child2", Some("root")))
        .await;

    let result = repo.get_children("root").await;

    assert!(result.is_ok(), "get_children should return Ok");
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    assert_eq!(children.len(), 2, "Should have exactly 2 children");
    let workspaces: std::collections::HashSet<&str> =
        children.iter().map(|e| e.workspace.as_str()).collect();
    assert!(workspaces.contains("child1"), "Should contain child1");
    assert!(workspaces.contains("child2"), "Should contain child2");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Happy path - Single child
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children returns a single child correctly.
///
/// GIVEN: A workspace with exactly one child
/// WHEN: Calling get_children
/// THEN: Returns Vec with exactly one entry
#[tokio::test]
async fn test_get_children_single_child() {
    let repo = InMemoryQueueRepository::new();

    repo.insert_entry(create_entry(1, "parent", None)).await;
    repo.insert_entry(create_entry(2, "only_child", Some("parent")))
        .await;

    let result = repo.get_children("parent").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].workspace, "only_child");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Empty case - Returns empty Vec when no children exist
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children returns empty Vec when workspace has no children.
///
/// GIVEN: A workspace with no children
/// WHEN: Calling get_children
/// THEN: Returns Ok(Vec::new()) - empty is valid, not an error
#[tokio::test]
async fn test_get_children_returns_empty_vec_when_no_children() {
    let repo = InMemoryQueueRepository::new();

    // Workspace exists but has no children
    repo.insert_entry(create_entry(1, "lonely", None)).await;
    // Other unrelated workspaces
    repo.insert_entry(create_entry(2, "other", None)).await;
    repo.insert_entry(create_entry(3, "unrelated_child", Some("other")))
        .await;

    let result = repo.get_children("lonely").await;

    assert!(result.is_ok(), "Should return Ok even with no children");
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };
    assert!(
        children.is_empty(),
        "Workspace with no children should return empty vec"
    );
}

/// Test that get_children returns empty Vec for workspace with only grandchildren.
///
/// GIVEN: A workspace whose grandchildren exist but no direct children
/// WHEN: Calling get_children
/// THEN: Returns Ok(Vec::new()) - no direct children
#[tokio::test]
async fn test_get_children_empty_when_only_grandchildren_exist() {
    let repo = InMemoryQueueRepository::new();

    // root -> (no direct children) -> grandchild is orphaned
    repo.insert_entry(create_entry(1, "root", None)).await;
    // grandchild's parent doesn't exist in the tree, it's not a child of root
    repo.insert_entry(create_entry(2, "orphan", Some("nonexistent")))
        .await;

    let result = repo.get_children("root").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };
    assert!(children.is_empty());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Only direct children returned (not grandchildren)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children returns ONLY direct children, not grandchildren.
///
/// GIVEN: A workspace with children AND grandchildren
/// WHEN: Calling get_children
/// THEN: Returns only direct children, grandchildren are excluded
#[tokio::test]
async fn test_get_children_excludes_grandchildren() {
    let repo = InMemoryQueueRepository::new();

    // Tree: root -> child -> grandchild -> greatgrandchild
    repo.insert_entry(create_entry(1, "root", None)).await;
    repo.insert_entry(create_entry(2, "child", Some("root")))
        .await;
    repo.insert_entry(create_entry(3, "grandchild", Some("child")))
        .await;
    repo.insert_entry(create_entry(4, "greatgrandchild", Some("grandchild")))
        .await;

    let result = repo.get_children("root").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    // Only "child" should be returned, not grandchild or greatgrandchild
    assert_eq!(children.len(), 1, "Should only have direct children");
    assert_eq!(children[0].workspace, "child");
}

/// Test get_children on a middle node returns only its direct children.
///
/// GIVEN: A middle workspace with children and a parent
/// WHEN: Calling get_children on the middle workspace
/// THEN: Returns only its direct children, not siblings or parent
#[tokio::test]
async fn test_get_children_middle_node_only_direct() {
    let repo = InMemoryQueueRepository::new();

    // Tree:
    //       root
    //      /    \
    //   child1  child2
    //     |        |
    //  gc1       gc2
    repo.insert_entry(create_entry(1, "root", None)).await;
    repo.insert_entry(create_entry(2, "child1", Some("root")))
        .await;
    repo.insert_entry(create_entry(3, "child2", Some("root")))
        .await;
    repo.insert_entry(create_entry(4, "gc1", Some("child1")))
        .await;
    repo.insert_entry(create_entry(5, "gc2", Some("child2")))
        .await;

    // Query child1 - should only get gc1
    let result = repo.get_children("child1").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].workspace, "gc1");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Multiple direct children
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children returns all direct children when there are many.
///
/// GIVEN: A workspace with 5 direct children
/// WHEN: Calling get_children
/// THEN: Returns all 5 children
#[tokio::test]
async fn test_get_children_multiple_direct_children() {
    let repo = InMemoryQueueRepository::new();

    repo.insert_entry(create_entry(1, "parent", None)).await;
    repo.insert_entry(create_entry(2, "child1", Some("parent")))
        .await;
    repo.insert_entry(create_entry(3, "child2", Some("parent")))
        .await;
    repo.insert_entry(create_entry(4, "child3", Some("parent")))
        .await;
    repo.insert_entry(create_entry(5, "child4", Some("parent")))
        .await;
    repo.insert_entry(create_entry(6, "child5", Some("parent")))
        .await;

    let result = repo.get_children("parent").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    assert_eq!(children.len(), 5);

    let workspaces: std::collections::HashSet<&str> =
        children.iter().map(|e| e.workspace.as_str()).collect();

    for i in 1..=5 {
        assert!(
            workspaces.contains(&format!("child{i}").as_str()),
            "Should contain child{i}"
        );
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Complex tree structure
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test get_children with complex branching tree.
///
/// GIVEN: A complex tree with multiple branches
/// WHEN: Calling get_children on root
/// THEN: Returns only root's direct children
#[tokio::test]
async fn test_get_children_complex_tree() {
    let repo = InMemoryQueueRepository::new();

    // Tree structure:
    //           root
    //         /   |   \
    //       a     b     c
    //      / \    |     |
    //     d   e   f     g
    repo.insert_entry(create_entry(1, "root", None)).await;
    repo.insert_entry(create_entry(2, "a", Some("root"))).await;
    repo.insert_entry(create_entry(3, "b", Some("root"))).await;
    repo.insert_entry(create_entry(4, "c", Some("root"))).await;
    repo.insert_entry(create_entry(5, "d", Some("a"))).await;
    repo.insert_entry(create_entry(6, "e", Some("a"))).await;
    repo.insert_entry(create_entry(7, "f", Some("b"))).await;
    repo.insert_entry(create_entry(8, "g", Some("c"))).await;

    let result = repo.get_children("root").await;

    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };

    // Only a, b, c should be returned (3 direct children)
    assert_eq!(children.len(), 3);

    let workspaces: std::collections::HashSet<&str> =
        children.iter().map(|e| e.workspace.as_str()).collect();

    assert!(workspaces.contains("a"));
    assert!(workspaces.contains("b"));
    assert!(workspaces.contains("c"));
    assert!(!workspaces.contains("d")); // grandchild
    assert!(!workspaces.contains("e")); // grandchild
    assert!(!workspaces.contains("f")); // grandchild
    assert!(!workspaces.contains("g")); // grandchild
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Non-existent workspace
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children for non-existent workspace returns empty Vec.
///
/// GIVEN: A non-existent workspace name
/// WHEN: Calling get_children
/// THEN: Returns Ok(Vec::new()) - no error for missing workspace
#[tokio::test]
async fn test_get_children_nonexistent_workspace_returns_empty() {
    let repo = InMemoryQueueRepository::new();

    // Empty repository, workspace doesn't exist
    let result = repo.get_children("nonexistent").await;

    // Should return Ok with empty vec, not an error
    assert!(result.is_ok());
    let children = if let Ok(c) = result {
        c
    } else {
        unreachable!()
    };
    assert!(children.is_empty());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Independent stacks don't interfere
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that independent stack trees don't interfere with each other.
///
/// GIVEN: Two independent stack trees
/// WHEN: Calling get_children on each root
/// THEN: Each returns only its own children
#[tokio::test]
async fn test_get_children_independent_stacks() {
    let repo = InMemoryQueueRepository::new();

    // Stack A: root-a -> child-a
    repo.insert_entry(create_entry(1, "root-a", None)).await;
    repo.insert_entry(create_entry(2, "child-a", Some("root-a")))
        .await;

    // Stack B: root-b -> child-b1, child-b2
    repo.insert_entry(create_entry(3, "root-b", None)).await;
    repo.insert_entry(create_entry(4, "child-b1", Some("root-b")))
        .await;
    repo.insert_entry(create_entry(5, "child-b2", Some("root-b")))
        .await;

    // Check root-a
    let result_a = repo.get_children("root-a").await;
    let result_a = if let Ok(c) = result_a {
        c
    } else {
        unreachable!()
    };
    assert_eq!(result_a.len(), 1);
    assert_eq!(result_a[0].workspace, "child-a");

    // Check root-b
    let result_b = repo.get_children("root-b").await;
    let result_b = if let Ok(c) = result_b {
        c
    } else {
        unreachable!()
    };
    assert_eq!(result_b.len(), 2);
    let workspaces: std::collections::HashSet<&str> =
        result_b.iter().map(|e| e.workspace.as_str()).collect();
    assert!(workspaces.contains("child-b1"));
    assert!(workspaces.contains("child-b2"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: Deterministic behavior (pure function semantics)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that get_children is deterministic for same input.
///
/// GIVEN: Same repository state
/// WHEN: Calling get_children multiple times
/// THEN: Returns identical results each time
#[tokio::test]
async fn test_get_children_is_deterministic() {
    let repo = InMemoryQueueRepository::new();

    repo.insert_entry(create_entry(1, "parent", None)).await;
    repo.insert_entry(create_entry(2, "child1", Some("parent")))
        .await;
    repo.insert_entry(create_entry(3, "child2", Some("parent")))
        .await;

    let result1 = repo.get_children("parent").await;
    let result1 = if let Ok(c) = result1 {
        c
    } else {
        unreachable!()
    };
    let result2 = repo.get_children("parent").await;
    let result2 = if let Ok(c) = result2 {
        c
    } else {
        unreachable!()
    };
    let result3 = repo.get_children("parent").await;
    let result3 = if let Ok(c) = result3 {
        c
    } else {
        unreachable!()
    };

    // All results should have same length
    assert_eq!(result1.len(), result2.len());
    assert_eq!(result2.len(), result3.len());

    // All should contain the same workspaces
    let ws1: std::collections::HashSet<&str> =
        result1.iter().map(|e| e.workspace.as_str()).collect();
    let ws2: std::collections::HashSet<&str> =
        result2.iter().map(|e| e.workspace.as_str()).collect();
    let ws3: std::collections::HashSet<&str> =
        result3.iter().map(|e| e.workspace.as_str()).collect();

    assert_eq!(ws1, ws2);
    assert_eq!(ws2, ws3);
}

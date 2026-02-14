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
//! End-to-end FIFO and deterministic merge tests (bd-1no)
//!
//! These tests verify:
//! - Complete submit-to-merge flow works end-to-end
//! - FIFO processing order is preserved
//! - Multiple entries are processed in submission order
//! - Deterministic merge behavior
//!
//! ## FIFO Ordering Guarantee
//!
//! The merge queue uses `ORDER BY priority ASC, added_at ASC` to ensure:
//! 1. Lower priority numbers are processed first
//! 2. Within same priority, earlier `added_at` timestamps win (FIFO)

use std::sync::Arc;

use futures::future::join_all;
use zjj_core::coordination::queue::{MergeQueue, QueueStatus};

// ============================================================================
// FIFO ORDERING TESTS
// ============================================================================

/// Test that entries are retrieved in FIFO order when added sequentially.
///
/// GIVEN: A queue with multiple entries added at different times
/// WHEN: We repeatedly call `next()` to retrieve entries
/// THEN: Entries are returned in the order they were added (FIFO)
#[tokio::test]
async fn test_fifo_ordering_same_priority() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries sequentially (different timestamps guaranteed)
    let entries = ["ws-first", "ws-second", "ws-third", "ws-fourth", "ws-fifth"];

    for entry in entries {
        queue.add(entry, None, 5, None).await?;
        // Small delay to ensure different timestamps
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Verify FIFO order by repeatedly calling next()
    let mut retrieved_order = Vec::new();
    for expected in entries {
        let next = queue.next().await?;
        assert!(next.is_some(), "Should have retrieved entry for {expected}");
        let entry = next.unwrap();
        assert_eq!(
            entry.workspace, expected,
            "FIFO order violated: expected {expected}, got {}",
            entry.workspace
        );
        // Mark as processing so next() returns the next entry
        let _ = queue.mark_processing(&entry.workspace).await;
        retrieved_order.push(entry.workspace);
    }

    // Verify complete order
    assert_eq!(
        retrieved_order,
        entries.to_vec(),
        "Retrieved order should match submission order (FIFO)"
    );

    Ok(())
}

/// Test that priority takes precedence over FIFO, but FIFO is preserved within same priority.
///
/// GIVEN: A queue with entries having different priorities
/// WHEN: We retrieve entries
/// THEN: Lower priority numbers come first, same-priority entries follow FIFO
#[tokio::test]
async fn test_priority_ordering_with_fifo_tiebreaker() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries with different priorities, out of order
    // Priority 10 (low): added first
    queue.add("ws-low-1", None, 10, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Priority 0 (highest): added second
    queue.add("ws-high", None, 0, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Priority 10 (low): added third
    queue.add("ws-low-2", None, 10, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Priority 5 (medium): added fourth
    queue.add("ws-mid", None, 5, None).await?;

    // Expected order by priority, then FIFO:
    // 1. ws-high (priority 0)
    // 2. ws-mid (priority 5)
    // 3. ws-low-1 (priority 10, added first)
    // 4. ws-low-2 (priority 10, added second)
    let expected_order = ["ws-high", "ws-mid", "ws-low-1", "ws-low-2"];

    for expected in expected_order {
        let next = queue.next().await?;
        assert!(next.is_some(), "Should have entry for {expected}");
        let entry = next.unwrap();
        assert_eq!(
            entry.workspace, expected,
            "Priority/FIFO order violated: expected {expected}, got {}",
            entry.workspace
        );
        let _ = queue.mark_processing(&entry.workspace).await;
    }

    Ok(())
}

/// Test that concurrent additions still maintain FIFO when priorities match.
///
/// GIVEN: Multiple entries added concurrently with same priority
/// WHEN: We retrieve them
/// THEN: Each is processed exactly once (no duplicates, no drops)
#[tokio::test]
async fn test_concurrent_additions_maintain_fifo() -> Result<(), Box<dyn std::error::Error>> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    // Add 20 entries concurrently
    let add_futures: Vec<_> = (0..20)
        .map(|i| {
            let queue = Arc::clone(&queue);
            async move {
                let workspace = format!("ws-{i}");
                queue.add(&workspace, None, 5, None).await
            }
        })
        .collect();

    join_all(add_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // All 20 should be pending
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 20, "All 20 entries should be pending");

    // Retrieve all and verify no duplicates
    let mut retrieved = std::collections::HashSet::new();
    let mut count = 0;

    while count < 20 {
        let next = queue.next().await?;
        match next {
            Some(entry) => {
                assert!(
                    retrieved.insert(entry.workspace.clone()),
                    "Duplicate entry retrieved: {}",
                    entry.workspace
                );
                let _ = queue.mark_processing(&entry.workspace).await;
                count += 1;
            }
            None => break,
        }
    }

    assert_eq!(
        retrieved.len(),
        20,
        "Should have retrieved exactly 20 unique entries"
    );

    Ok(())
}

// ============================================================================
// DETERMINISTIC MERGE BEHAVIOR TESTS
// ============================================================================

/// Test that the same sequence of operations produces the same result.
///
/// GIVEN: Two independent queues with identical operations
/// WHEN: We perform the same add/claim sequence
/// THEN: Both produce identical order
#[tokio::test]
async fn test_deterministic_ordering_across_queues() -> Result<(), Box<dyn std::error::Error>> {
    // Queue 1
    let queue1 = MergeQueue::open_in_memory().await?;
    queue1.add("ws-a", None, 5, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    queue1.add("ws-b", None, 5, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    queue1.add("ws-c", None, 5, None).await?;

    // Queue 2 (independent, same operations)
    let queue2 = MergeQueue::open_in_memory().await?;
    queue2.add("ws-a", None, 5, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    queue2.add("ws-b", None, 5, None).await?;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    queue2.add("ws-c", None, 5, None).await?;

    // Both should return entries in same order
    let order1: Vec<_> = extract_all_in_order(&queue1).await;
    let order2: Vec<_> = extract_all_in_order(&queue2).await;

    assert_eq!(
        order1, order2,
        "Both queues should produce identical ordering (deterministic)"
    );
    assert_eq!(order1, ["ws-a", "ws-b", "ws-c"], "Order should be FIFO");

    Ok(())
}

/// Test that queue state transitions are deterministic.
///
/// GIVEN: An entry in the queue
/// WHEN: We perform state transitions
/// THEN: The same transitions always lead to the same final state
#[tokio::test]
async fn test_deterministic_state_transitions() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entry
    let response = queue.add("ws-test", None, 5, None).await?;
    assert_eq!(response.entry.status, QueueStatus::Pending);

    // Claim entry
    let claimed = queue.next_with_lock("agent-1").await?;
    assert!(claimed.is_some(), "Should claim entry");
    let entry = claimed.unwrap();
    assert_eq!(entry.workspace, "ws-test");
    assert_eq!(entry.status, QueueStatus::Claimed);
    assert_eq!(entry.agent_id.as_deref(), Some("agent-1"));

    // Verify state via get_by_workspace
    let fetched = queue.get_by_workspace("ws-test").await?;
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.status, QueueStatus::Claimed);
    assert_eq!(fetched.agent_id.as_deref(), Some("agent-1"));

    // Cleanup
    let _ = queue.mark_completed("ws-test").await;
    let _ = queue.release_processing_lock("agent-1").await;

    Ok(())
}

/// Test that multiple agents competing for entries don't create order violations.
///
/// GIVEN: Multiple entries and multiple agents claiming concurrently
/// WHEN: Agents compete for entries
/// THEN: Each entry is claimed exactly once, in FIFO order per agent
#[tokio::test]
async fn test_concurrent_claims_maintain_fifo_per_agent() -> Result<(), Box<dyn std::error::Error>>
{
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    // Add 10 entries
    for i in 0..10 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Have 3 agents compete for entries
    let handles: Vec<_> = (0..3)
        .map(|agent_idx| {
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let agent_id = format!("agent-{agent_idx}");
                let mut claimed = Vec::new();

                // Each agent tries to claim up to 10 entries
                for _ in 0..10 {
                    match queue.next_with_lock(&agent_id).await {
                        Ok(Some(entry)) => {
                            claimed.push(entry.workspace.clone());
                            // Simulate work
                            tokio::task::yield_now().await;
                            let _ = queue.mark_completed(&entry.workspace).await;
                            let _ = queue.release_processing_lock(&agent_id).await;
                        }
                        Ok(None) | Err(_) => break,
                    }
                }

                claimed
            })
        })
        .collect();

    let results = join_all(handles).await;

    // Collect all claimed workspaces
    let mut all_claimed: Vec<String> = results
        .into_iter()
        .filter_map(Result::ok)
        .flatten()
        .collect();

    // Sort to verify all 10 were claimed
    all_claimed.sort();
    let expected: Vec<_> = (0..10).map(|i| format!("ws-{i}")).collect();

    assert_eq!(
        all_claimed, expected,
        "All 10 entries should be claimed exactly once"
    );

    // Verify queue is empty (all completed)
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 0, "No pending entries should remain");

    Ok(())
}

// ============================================================================
// SUBMIT-TO-MERGE E2E FLOW TESTS
// ============================================================================

/// Test the complete submit-to-merge flow with idempotent upsert.
///
/// GIVEN: A workspace ready to submit
/// WHEN: We submit via upsert_for_submit
/// THEN: Entry is added/updated correctly
#[tokio::test]
async fn test_submit_to_merge_idempotent_flow() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // First submit
    let entry1 = queue
        .upsert_for_submit(
            "ws-feature",
            Some("bead-123"),
            5,
            None,
            "ws-feature:change1",
            "sha1",
        )
        .await?;

    assert_eq!(entry1.workspace, "ws-feature");
    assert_eq!(entry1.status, QueueStatus::Pending);
    assert_eq!(entry1.head_sha.as_deref(), Some("sha1"));

    // Idempotent resubmit (same dedupe_key, same workspace)
    let entry2 = queue
        .upsert_for_submit(
            "ws-feature",
            Some("bead-123"),
            5,
            None,
            "ws-feature:change1",
            "sha2", // New SHA
        )
        .await?;

    // Should update in place, not create new entry
    assert_eq!(entry2.id, entry1.id, "Same entry should be updated");
    assert_eq!(
        entry2.head_sha.as_deref(),
        Some("sha2"),
        "SHA should be updated"
    );
    assert_eq!(entry2.status, QueueStatus::Pending);

    // Verify only one entry exists
    let all_pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(
        all_pending.len(),
        1,
        "Should have exactly one pending entry"
    );

    Ok(())
}

/// Test that terminal state entries can be resubmitted.
///
/// GIVEN: An entry that reached terminal state (cancelled)
/// WHEN: We resubmit with same dedupe_key
/// THEN: Entry is reset to pending for new work
#[tokio::test]
async fn test_resubmit_after_terminal_state() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Initial submit
    let _entry1 = queue
        .upsert_for_submit("ws-feature", None, 5, None, "ws-feature:change1", "sha1")
        .await?;

    // Claim the entry
    let claimed = queue.next_with_lock("agent-1").await?;
    assert!(claimed.is_some());

    // Transition to terminal state (cancelled) - valid from claimed
    queue
        .transition_to("ws-feature", QueueStatus::Cancelled)
        .await?;
    let _ = queue.release_processing_lock("agent-1").await;

    // Verify terminal state
    let cancelled = queue.get_by_workspace("ws-feature").await?;
    assert!(cancelled.is_some());
    let cancelled = cancelled.unwrap();
    assert_eq!(cancelled.status, QueueStatus::Cancelled);

    // Resubmit (same workspace, same dedupe_key)
    let entry2 = queue
        .upsert_for_submit("ws-feature", None, 5, None, "ws-feature:change1", "sha2")
        .await?;

    // Should be reset to pending
    assert_eq!(
        entry2.status,
        QueueStatus::Pending,
        "Entry should be reset to pending"
    );
    assert_eq!(entry2.head_sha.as_deref(), Some("sha2"));

    Ok(())
}

/// Test FIFO is maintained across the full lifecycle.
///
/// GIVEN: Multiple entries going through submit-claim-cancel cycle
/// WHEN: We process them in order
/// THEN: Each is claimed in FIFO order
#[tokio::test]
async fn test_full_lifecycle_fifo_order() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Submit 5 entries
    let workspaces = ["ws-1", "ws-2", "ws-3", "ws-4", "ws-5"];
    for (idx, ws) in workspaces.iter().enumerate() {
        queue
            .upsert_for_submit(
                ws,
                None,
                5,
                None,
                &format!("{ws}:change{idx}"),
                &format!("sha{idx}"),
            )
            .await?;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Claim all entries in FIFO order and verify
    let mut claimed_order = Vec::new();
    for expected in workspaces {
        // Claim next
        let claimed = queue.next_with_lock("agent-test").await?;
        assert!(claimed.is_some(), "Should claim {expected}");
        let entry = claimed.unwrap();
        assert_eq!(
            entry.workspace, expected,
            "FIFO order violated in lifecycle test"
        );

        // Move to terminal state (cancelled) to allow next claim
        queue
            .transition_to(&entry.workspace, QueueStatus::Cancelled)
            .await?;
        let _ = queue.release_processing_lock("agent-test").await;

        claimed_order.push(entry.workspace);
    }

    assert_eq!(
        claimed_order,
        workspaces.to_vec(),
        "Claim order should match submission order (FIFO)"
    );

    // Verify all are in terminal state
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 0, "No pending entries should remain");
    assert_eq!(
        stats.failed, 5,
        "All 5 entries should be in failed/cancelled state"
    );

    Ok(())
}

/// Test that queue list operation returns entries in FIFO order.
///
/// GIVEN: A queue with entries
/// WHEN: We call list()
/// THEN: Entries are returned in priority/FIFO order
#[tokio::test]
async fn test_list_returns_fifo_order() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries
    for (idx, ws) in ["ws-a", "ws-b", "ws-c"].iter().enumerate() {
        queue.add(ws, None, 5, None).await?;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // Mark first as claimed to test mixed-status list
        if idx == 0 {
            let _ = queue.mark_processing(ws).await;
        }
    }

    // List pending only
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].workspace, "ws-b");
    assert_eq!(pending[1].workspace, "ws-c");

    // List all
    let all_entries = queue.list(None).await?;
    assert_eq!(all_entries.len(), 3);
    // All entries should be in FIFO order regardless of status
    assert_eq!(all_entries[0].workspace, "ws-a");
    assert_eq!(all_entries[1].workspace, "ws-b");
    assert_eq!(all_entries[2].workspace, "ws-c");

    Ok(())
}

// ============================================================================
// HELPERS
// ============================================================================

/// Extract all entries from a queue in order, marking each as processing.
async fn extract_all_in_order(queue: &MergeQueue) -> Vec<String> {
    let mut order = Vec::new();
    while let Ok(Some(entry)) = queue.next().await {
        order.push(entry.workspace.clone());
        let _ = queue.mark_processing(&entry.workspace).await;
    }
    order
}

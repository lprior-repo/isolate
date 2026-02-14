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
    // Async and concurrency relaxations
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue,
)]
//! Adversarial tests for queue - boundary conditions and edge cases
//!
//! These tests push the queue to its limits with:
//! - Maximum concurrency stress
//! - Edge case boundary conditions
//! - Failure mode testing
//! - Data corruption scenarios
//! - Race condition detection
//! - Dedup key adversarial cases

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::future::join_all;
use zjj_core::{
    coordination::queue::{MergeQueue, QueueControlError, QueueStatus},
    Result,
};

// ============================================================================
// DEDUPE KEY ADVERSARIAL TESTS
// ============================================================================

/// Test that upsert_for_submit with SAME workspace updates the entry.
/// This is the idempotent submit case - same change resubmitted should succeed.
#[tokio::test]
async fn adversarial_dedupe_same_workspace_updates() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let dedupe_key = "workspace:change123";
    let head_sha = "sha1111111111111111111111111111111111111";

    // First submit using upsert_for_submit (the idempotent method)
    let entry1 = queue
        .upsert_for_submit("ws-1", None, 5, None, dedupe_key, head_sha)
        .await?;
    let entry1_id = entry1.id;

    // Second submit with SAME workspace and dedupe_key - should UPDATE, not create new
    let head_sha2 = "sha2222222222222222222222222222222222222";
    let entry2 = queue
        .upsert_for_submit("ws-1", None, 5, None, dedupe_key, head_sha2)
        .await?;
    let entry2_id = entry2.id;

    // Should be the SAME entry (updated)
    assert_eq!(
        entry1_id, entry2_id,
        "Same workspace + same dedupe_key should update existing entry"
    );

    // Verify only ONE entry exists
    let entries = queue.list(None).await?;
    let active_count = entries
        .iter()
        .filter(|e| e.dedupe_key.as_deref() == Some(dedupe_key))
        .count();

    assert_eq!(
        active_count, 1,
        "Should have exactly one active entry with this dedupe_key"
    );

    Ok(())
}

/// Test that duplicate dedupe_key from DIFFERENT workspace is rejected.
/// This prevents duplicate work from different sources.
#[tokio::test]
async fn adversarial_dedupe_different_workspace_rejected() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let dedupe_key = "feature-x:change123";

    // First submit from workspace 1
    queue
        .add_with_dedupe("ws-1", None, 5, None, Some(dedupe_key))
        .await?;

    // Second submit from DIFFERENT workspace with SAME dedupe_key - should fail
    let result = queue
        .add_with_dedupe("ws-2", None, 5, None, Some(dedupe_key))
        .await;

    // Should be an error about duplicate dedupe_key
    assert!(
        result.is_err(),
        "Different workspace with same dedupe_key should be rejected"
    );

    Ok(())
}

/// Test that terminal entries (merged, failed, cancelled) allow dedupe_key reuse.
/// Using upsert_for_submit which handles this case.
#[tokio::test]
async fn adversarial_dedupe_reuse_after_terminal() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let dedupe_key = "ws:change123";

    // First submit
    let entry1 = queue
        .upsert_for_submit(
            "ws-1",
            None,
            5,
            None,
            dedupe_key,
            "sha1111111111111111111111111111111111111",
        )
        .await?;

    // Move to processing
    queue.mark_processing(&entry1.workspace).await?;

    // Mark as merged (terminal state)
    queue.mark_completed(&entry1.workspace).await?;

    // Now same dedupe_key should be allowed again (entry is terminal)
    // Using upsert_for_submit which handles terminal state
    let resubmitted = queue
        .upsert_for_submit(
            "ws-1",
            None,
            5,
            None,
            dedupe_key,
            "sha2222222222222222222222222222222222222",
        )
        .await;

    let resubmitted = resubmitted?;
    assert_eq!(
        resubmitted.id, entry1.id,
        "Resubmitting same workspace should reset existing terminal entry"
    );
    assert_eq!(
        resubmitted.dedupe_key.as_deref(),
        Some(dedupe_key),
        "Resubmitted entry should preserve dedupe_key"
    );

    Ok(())
}

/// Test concurrent submissions with SAME dedupe_key - only one should succeed.
#[tokio::test]
async fn adversarial_dedupe_concurrent_submissions_race() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);
    let dedupe_key = "race:test-key";

    // Spawn 10 concurrent submissions with SAME dedupe_key
    let mut handles = vec![];
    for i in 0..10 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            q.add_with_dedupe(&format!("ws-{i}"), None, 5, None, Some(dedupe_key))
                .await
        });
        handles.push(handle);
    }

    let joined = join_all(handles).await;
    assert!(
        joined.iter().all(std::result::Result::is_ok),
        "All dedupe race tasks should complete without panic"
    );
    let results = joined
        .into_iter()
        .map(|r| r.expect("join handle should not panic"))
        .collect::<Vec<_>>();

    // Count successes
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    // Exactly ONE should succeed (the first one to acquire the unique constraint).
    // Others should fail with dedupe_key conflict.
    assert_eq!(success_count, 1, "Exactly one submission should succeed");

    // Verify only one active entry exists with this dedupe_key
    let entries = queue.list(None).await?;
    let active_with_key = entries
        .iter()
        .filter(|e| {
            e.dedupe_key.as_deref() == Some(dedupe_key)
                && !matches!(
                    e.status,
                    QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
                )
        })
        .count();

    assert_eq!(
        active_with_key, 1,
        "Only one active entry should exist with this dedupe_key"
    );

    Ok(())
}

/// Test upsert_for_submit with dedupe key - the primary submit path.
#[tokio::test]
async fn adversarial_upsert_submit_idempotent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let dedupe_key = "feature-branch:abc123";
    let head_sha = "sha1111111111111111111111111111111111111";

    // First upsert
    let entry1 = queue
        .upsert_for_submit("ws-feature", None, 5, None, dedupe_key, head_sha)
        .await?;
    assert_eq!(entry1.dedupe_key.as_deref(), Some(dedupe_key));

    // Second upsert with same dedupe_key - should return existing entry
    let entry2 = queue
        .upsert_for_submit("ws-feature", None, 5, None, dedupe_key, head_sha)
        .await?;

    assert_eq!(
        entry1.id, entry2.id,
        "Should return same entry on idempotent upsert"
    );

    // Different head_sha should update the entry
    let head_sha2 = "sha2222222222222222222222222222222222222";
    let entry3 = queue
        .upsert_for_submit("ws-feature", None, 5, None, dedupe_key, head_sha2)
        .await?;

    assert_eq!(
        entry1.id, entry3.id,
        "Same dedupe_key should update, not create new"
    );

    Ok(())
}

// ============================================================================
// BOUNDARY CONDITION TESTS
// ============================================================================

/// Test queue with maximum possible entries.
#[tokio::test]
async fn adversarial_max_entries() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add 1000 entries
    for i in 0..1000 {
        queue.add(&format!("ws-{}", i), None, 5, None).await?;
    }

    let stats = queue.stats().await?;
    assert_eq!(stats.total, 1000, "Should have 1000 entries");

    // Verify all can be retrieved
    // Use next() + mark_processing to actually consume entries (next() is non-mutating peek)
    let mut count = 0;
    while let Some(entry) = queue.next().await? {
        // Mark as processing to remove it from pending queue
        queue.mark_processing(&entry.workspace).await?;
        count += 1;
    }

    assert_eq!(count, 1000, "Should be able to retrieve all 1000 entries");

    Ok(())
}

/// Test queue with extreme priority values.
#[tokio::test]
async fn adversarial_extreme_priorities() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add with extreme priorities
    queue.add("ws-min", None, i32::MIN, None).await?;
    queue.add("ws-neg", None, -1, None).await?;
    queue.add("ws-zero", None, 0, None).await?;
    queue.add("ws-pos", None, 1, None).await?;
    queue.add("ws-max", None, i32::MAX, None).await?;

    // Should come out in priority order (lowest first).
    // Note: next() is non-mutating peek, so we must mark_processing to advance the queue.
    let expected_order = ["ws-min", "ws-neg", "ws-zero", "ws-pos", "ws-max"];
    for expected_workspace in expected_order {
        let entry = queue.next().await?.expect("should have entry");
        assert_eq!(
            entry.workspace, expected_workspace,
            "Priority ordering mismatch"
        );
        let marked = queue.mark_processing(&entry.workspace).await?;
        assert!(marked, "Entry should transition to processing");
    }

    Ok(())
}

/// Test workspace names with unusual characters.
#[tokio::test]
async fn adversarial_special_workspace_names() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Test various edge case workspace names
    let names = [
        "ws-simple",
        "ws-with-dash",
        "ws_with_underscore",
        "ws.with.dots",
        "ws123numeric",
        "ws-UPPERCASE",
        "ws-mixed-Case-123",
    ];

    for name in names {
        queue.add(name, None, 5, None).await?;
    }

    let stats = queue.stats().await?;
    assert_eq!(
        stats.total,
        names.len(),
        "All workspace names should be accepted"
    );

    Ok(())
}

/// Test empty and null edge cases.
#[tokio::test]
async fn adversarial_null_handling() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add with null bead_id and agent_id
    queue.add("ws-1", None, 5, None).await?;
    queue.add("ws-2", None, 5, Some("agent-1")).await?;
    queue
        .add_with_dedupe("ws-3", Some("bead-1"), 5, None, None)
        .await?;
    queue.add_with_dedupe("ws-4", None, 5, None, None).await?;

    let stats = queue.stats().await?;
    assert_eq!(stats.total, 4, "Should accept null variants");

    Ok(())
}

// ============================================================================
// CONCURRENCY STRESS TESTS
// ============================================================================

/// Test concurrent agents fighting for entries with lock contention.
#[tokio::test]
async fn adversarial_massive_contention_1000_agents() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    // Add only 10 entries
    for i in 0..10 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
    }

    // Spawn 100 concurrent agents with enough retry budget to cover serialized
    // lock handoffs for all 10 entries.
    let mut handles = vec![];
    for i in 0..100 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            // Retry enough times so all entries can be claimed under contention.
            for _ in 0..400 {
                let result = q.next_with_lock(&format!("agent-{i}")).await;
                match result {
                    Ok(Some(entry)) => {
                        // Hold briefly then release
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        let _ = q.release_processing_lock(&format!("agent-{i}")).await;
                        return Some(entry.workspace);
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }
            None
        });
        handles.push(handle);
    }

    let results = join_all(handles)
        .await
        .into_iter()
        .filter_map(std::result::Result::ok)
        .flatten()
        .collect::<Vec<_>>();

    // Exactly 10 unique entries should be claimed (one per queue item).
    let unique: std::collections::HashSet<_> = results.iter().collect();
    assert_eq!(
        unique.len(),
        10,
        "Expected all 10 entries to be claimed, got {}",
        unique.len()
    );
    assert_eq!(
        unique.len(),
        results.len(),
        "No entry should be claimed twice"
    );

    Ok(())
}

/// Test rapid add/remove cycles from multiple tasks.
#[tokio::test]
async fn adversarial_rapid_add_remove_cycles() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);
    let successful_adds = Arc::new(AtomicUsize::new(0));
    let failed_adds = Arc::new(AtomicUsize::new(0));
    let successful_removes = Arc::new(AtomicUsize::new(0));

    // 50 tasks each doing 20 rapid cycles
    let mut handles = vec![];
    for i in 0..50 {
        let q = queue.clone();
        let adds = successful_adds.clone();
        let add_failures = failed_adds.clone();
        let removes = successful_removes.clone();
        let handle = tokio::spawn(async move {
            for j in 0..20 {
                let workspace = format!("ws-{i}-{j}");
                match q.add(&workspace, None, 5, None).await {
                    Ok(_) => {
                        adds.fetch_add(1, Ordering::Relaxed);
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        if matches!(q.remove(&workspace).await, Ok(true)) {
                            removes.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        add_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
        handles.push(handle);
    }

    join_all(handles).await;

    let adds = successful_adds.load(Ordering::Relaxed);
    let add_failures = failed_adds.load(Ordering::Relaxed);
    let removes = successful_removes.load(Ordering::Relaxed);

    assert_eq!(
        adds + add_failures,
        1000,
        "All add attempts should be accounted for"
    );

    // Residual entries must exactly equal successful adds minus successful removes.
    let stats = queue.stats().await?;
    let expected_residual = adds.saturating_sub(removes);
    assert_eq!(
        stats.total, expected_residual,
        "Queue total should match successful add/remove accounting"
    );
    assert!(
        stats.total <= 50,
        "Residual entries should remain small under contention, got {}",
        stats.total
    );

    Ok(())
}

/// Test concurrent state transitions.
#[tokio::test]
async fn adversarial_concurrent_state_transitions() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    // Add 50 entries
    for i in 0..50 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
    }

    // Spawn tasks that randomly transition states
    let mut handles = vec![];
    for i in 0..50 {
        let q = queue.clone();
        let workspace = format!("ws-{i}");
        let handle = tokio::spawn(async move {
            // Random sequence of operations
            for _ in 0..3 {
                if matches!(q.mark_processing(&workspace).await, Ok(true)) {
                    tokio::time::sleep(Duration::from_micros(50)).await;
                    let _ = q.mark_completed(&workspace).await;
                }
            }
        });
        handles.push(handle);
    }

    join_all(handles).await;

    // Should end in consistent state
    let stats = queue.stats().await?;
    println!("Final stats: {:?}", stats);

    // Total should remain stable and status buckets should account for all entries.
    assert_eq!(
        stats.total, 50,
        "No entries should be lost during transitions"
    );
    assert_eq!(
        stats.completed + stats.failed + stats.processing + stats.pending,
        50,
        "All entries should be represented in status counts"
    );

    Ok(())
}

// ============================================================================
// FAILURE MODE TESTS
// ============================================================================

/// Test behavior when removing non-existent entry.
#[tokio::test]
async fn adversarial_remove_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Remove non-existent workspace
    let result = queue.remove("nonexistent-workspace").await?;

    assert!(!result, "Removing nonexistent should return false");

    Ok(())
}

/// Test get_by_workspace for non-existent entry.
#[tokio::test]
async fn adversarial_get_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let result = queue.get_by_workspace("nonexistent").await?;

    assert!(
        result.is_none(),
        "Non-existent workspace should return None"
    );

    Ok(())
}

/// Test mark_processing on non-existent entry.
#[tokio::test]
async fn adversarial_mark_processing_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // mark_processing returns Ok(false) when no matching entry exists
    let result = queue.mark_processing("nonexistent-workspace").await?;

    assert!(!result, "Marking nonexistent should return false");

    Ok(())
}

/// Test mark_completed on non-existent entry.
#[tokio::test]
async fn adversarial_mark_completed_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // mark_completed returns Ok(false) when no matching entry exists
    let result = queue.mark_completed("nonexistent-workspace").await?;

    assert!(!result, "Marking nonexistent should return false");

    Ok(())
}

/// Test double completion (idempotency).
#[tokio::test]
async fn adversarial_double_complete_idempotent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    queue.add("ws-1", None, 5, None).await?;
    queue.mark_processing("ws-1").await?;

    // First completion
    let result1 = queue.mark_completed("ws-1").await?;
    assert!(result1, "First complete should succeed");

    // Second completion - entry is already completed, so returns false (idempotent but no update)
    let result2 = queue.mark_completed("ws-1").await?;

    // Returns false because entry is already in completed state (not an error, but no rows updated)
    assert!(
        !result2,
        "Double complete should be idempotent (returns false on already-completed)"
    );

    Ok(())
}

// ============================================================================
// LOCK CONTENTION TESTS
// ============================================================================

/// Test lock timeout and reacquisition.
#[tokio::test]
async fn adversarial_lock_timeout_reacquisition() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    queue.add("ws-1", None, 5, None).await?;

    // Agent 1 acquires lock
    let acquired = queue.acquire_processing_lock("agent-1").await?;
    assert!(acquired, "First agent should acquire lock");

    // Agent 2 tries but fails
    let acquired2 = queue.acquire_processing_lock("agent-2").await?;
    assert!(!acquired2, "Second agent should fail to acquire");

    // Agent 1 releases
    let released = queue.release_processing_lock("agent-1").await?;
    assert!(released, "Release should succeed");

    // Now agent 2 should be able to acquire
    let acquired3 = queue.acquire_processing_lock("agent-2").await?;
    assert!(acquired3, "Agent 2 should acquire after release");

    Ok(())
}

/// Test lock release by non-owner.
#[tokio::test]
async fn adversarial_lock_release_by_non_owner() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    queue.add("ws-1", None, 5, None).await?;

    // Agent 1 acquires lock
    queue.acquire_processing_lock("agent-1").await?;

    // Agent 2 tries to release (should fail)
    let released = queue.release_processing_lock("agent-2").await?;

    assert!(!released, "Non-owner should not be able to release");

    // Agent 1 should still hold it - check the lock
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still be held by agent-1");
    if let Some(lock) = lock {
        assert_eq!(lock.agent_id, "agent-1", "Lock should be held by agent-1");
    }

    // Agent 1 releases
    queue.release_processing_lock("agent-1").await?;

    // Now lock should be gone
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_none(), "Lock should be released");

    Ok(())
}

// ============================================================================
// CLEANUP STRESS TESTS
// ============================================================================

/// Test cleanup with many old entries.
#[tokio::test]
async fn adversarial_cleanup_many_old_entries() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add 100 entries and mark them as completed
    for i in 0..100 {
        queue.add(&format!("ws-old-{}", i), None, 5, None).await?;
        queue.mark_processing(&format!("ws-old-{}", i)).await?;
        queue.mark_completed(&format!("ws-old-{}", i)).await?;
    }

    // Ensure completed entries become older than the cutoff used below.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Add 10 fresh pending entries
    for i in 0..10 {
        queue.add(&format!("ws-new-{}", i), None, 5, None).await?;
    }

    // Add 5 fresh completed entries that should NOT be cleaned.
    for i in 0..5 {
        let ws = format!("ws-fresh-completed-{i}");
        queue.add(&ws, None, 5, None).await?;
        queue.mark_processing(&ws).await?;
        queue.mark_completed(&ws).await?;
    }

    // Cleanup entries older than 1 second.
    let cleaned = queue.cleanup(Duration::from_secs(1)).await?;

    assert_eq!(cleaned, 100, "Should clean all 100 old completed entries");

    // Verify fresh entries remain.
    let stats = queue.stats().await?;
    assert_eq!(
        stats.pending, 10,
        "Should have 10 pending entries remaining"
    );
    assert_eq!(stats.completed, 5, "Should keep 5 fresh completed entries");

    Ok(())
}

/// Test that cleanup doesn't remove processing entries.
#[tokio::test]
async fn adversarial_cleanup_preserves_processing() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add entries in various states
    queue.add("ws-pending", None, 5, None).await?;
    queue.add("ws-processing", None, 5, None).await?;
    queue.mark_processing("ws-processing").await?;
    queue.add("ws-completed", None, 5, None).await?;
    queue.mark_processing("ws-completed").await?;
    queue.mark_completed("ws-completed").await?;

    // Use max_age=0 for deterministic cleanup of terminal states only.
    let cleaned = queue.cleanup(Duration::ZERO).await?;

    // Only completed should be cleaned
    assert_eq!(cleaned, 1, "Should only clean completed entry");

    // Verify others remain
    let ws_pending = queue.get_by_workspace("ws-pending").await?;
    let ws_processing = queue.get_by_workspace("ws-processing").await?;

    assert!(ws_pending.is_some(), "Pending should remain");
    assert!(ws_processing.is_some(), "Processing should remain");

    Ok(())
}

// ============================================================================
// RACE CONDITION DETECTION TESTS
// ============================================================================

/// Test that only one concurrent retry can win for the same entry.
#[tokio::test]
async fn adversarial_retry_entry_concurrent_single_winner() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    queue.add("ws-retry-race", None, 5, None).await?;
    queue
        .transition_to("ws-retry-race", QueueStatus::Claimed)
        .await?;
    queue
        .transition_to("ws-retry-race", QueueStatus::FailedRetryable)
        .await?;

    let entry = queue
        .get_by_workspace("ws-retry-race")
        .await?
        .ok_or_else(|| zjj_core::Error::DatabaseError("Entry should exist".to_string()))?;

    let mut handles = vec![];
    for _ in 0..2 {
        let q = queue.clone();
        let id = entry.id;
        handles.push(tokio::spawn(async move { q.retry_entry(id).await }));
    }

    let results = join_all(handles)
        .await
        .into_iter()
        .map(|h| h.expect("Retry task should not panic"))
        .collect::<Vec<_>>();

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let not_retryable_count = results
        .iter()
        .filter(|r| {
            matches!(
                r,
                Err(QueueControlError::NotRetryable {
                    status: QueueStatus::Pending,
                    ..
                })
            )
        })
        .count();

    assert_eq!(
        success_count, 1,
        "Exactly one concurrent retry should succeed"
    );
    assert_eq!(
        not_retryable_count, 1,
        "The loser should observe entry already moved out of failed_retryable"
    );

    let final_entry = queue
        .get_by_id(entry.id)
        .await?
        .ok_or_else(|| zjj_core::Error::DatabaseError("Updated entry should exist".to_string()))?;
    assert_eq!(final_entry.status, QueueStatus::Pending);
    assert_eq!(
        final_entry.attempt_count, 1,
        "Concurrent retries must not double-increment attempts"
    );

    Ok(())
}

/// Test that next_with_lock does not leak lock state when queue is empty.
#[tokio::test]
async fn adversarial_next_with_lock_empty_releases_lock() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    let next = queue.next_with_lock("agent-empty").await?;
    assert!(next.is_none(), "Empty queue should return None");

    let lock = queue.get_processing_lock().await?;
    assert!(
        lock.is_none(),
        "Lock should be released when no entry is claimed"
    );

    Ok(())
}

/// Test that next_with_lock() atomically claims entries - no duplicates.
#[tokio::test]
async fn adversarial_next_no_duplicate_returns() -> Result<()> {
    let queue = Arc::new(MergeQueue::open_in_memory().await?);

    // Add 10 entries
    for i in 0..10 {
        queue.add(&format!("ws-{}", i), None, 5, None).await?;
    }

    // 100 concurrent callers using next_with_lock (atomic claim).
    // Each caller retries to ensure all 10 entries get a chance to be claimed.
    let mut handles = vec![];
    for i in 0..100 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..200 {
                if let Ok(Some(entry)) = q.next_with_lock(&format!("agent-{i}")).await {
                    // Release the lock so others can proceed
                    let _ = q.release_processing_lock(&format!("agent-{i}")).await;
                    return Some(entry.id);
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            None
        });
        handles.push(handle);
    }

    let results = join_all(handles)
        .await
        .into_iter()
        .filter_map(std::result::Result::ok)
        .flatten()
        .collect::<Vec<_>>();

    // All 10 queue entries should be claimed exactly once.
    assert_eq!(results.len(), 10, "Should claim exactly 10 entries");

    // All returned IDs must be unique (atomic claim prevents duplicates).
    let unique_ids: std::collections::HashSet<_> = results.iter().collect();
    assert_eq!(
        unique_ids.len(),
        10,
        "next_with_lock() should atomically claim entries - no duplicates, got {} duplicates",
        results.len() - unique_ids.len()
    );

    Ok(())
}

/// Test position tracking accuracy under concurrency.
#[tokio::test]
async fn adversarial_position_tracking_concurrent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;

    // Add 20 entries
    for i in 0..20 {
        queue.add(&format!("ws-{}", i), None, 5, None).await?;
    }

    // Check positions concurrently
    let mut handles = vec![];
    for i in 0..20 {
        let q = queue.clone();
        let workspace = format!("ws-{}", i);
        let handle = tokio::spawn(async move { q.position(&workspace).await.ok().flatten() });
        handles.push(handle);
    }

    let positions = join_all(handles)
        .await
        .into_iter()
        .filter_map(std::result::Result::ok)
        .flatten()
        .collect::<Vec<_>>();

    // All should have positions
    assert_eq!(positions.len(), 20, "All entries should have positions");

    // Positions should be 1-20 (1-indexed)
    let mut sorted = positions;
    sorted.sort_unstable();
    for (i, pos) in sorted.iter().enumerate() {
        assert_eq!(*pos, i + 1, "Position should be 1-indexed");
    }

    Ok(())
}

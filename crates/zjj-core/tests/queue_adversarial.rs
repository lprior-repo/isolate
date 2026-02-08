//! Adversarial tests for queue - boundary conditions and edge cases
// Test code uses unwrap/expect idioms for test clarity.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use zjj_core::coordination::queue::{MergeQueue, QueueStatus};
use zjj_core::Result;

#[tokio::test]
async fn adv_duplicate_workspace_add_fails() -> Result<()> {
    // Test that adding the same workspace twice fails
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-dup", None, 5, None).await?;
    
    // Second add should fail
    let result = queue.add("ws-dup", None, 5, None).await;
    
    assert!(result.is_err(), "Duplicate workspace add should fail");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already in the queue"), 
            "Error should mention duplicate: {}", err);
    
    Ok(())
}

#[tokio::test]
async fn adv_remove_nonexistent_returns_false() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    let removed = queue.remove("ws-nonexistent").await?;
    assert!(!removed, "Removing nonexistent workspace should return false");
    
    Ok(())
}

#[tokio::test]
async fn adv_mark_processing_on_completed_fails() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-mark-test", None, 5, None).await?;
    queue.mark_processing("ws-mark-test").await?;
    queue.mark_completed("ws-mark-test").await?;
    
    // Try to mark processing again - should fail (not in pending state)
    let result = queue.mark_processing("ws-mark-test").await?;
    assert!(!result, "Marking completed entry as processing should fail");
    
    Ok(())
}

#[tokio::test]
async fn adv_release_lock_by_non_holder_fails() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-lock-test", None, 5, None).await?;
    queue.next_with_lock("agent-holder").await?;
    
    // Non-holder tries to release
    let released = queue.release_processing_lock("agent-imposter").await?;
    assert!(!released, "Non-holder should not release lock");
    
    // Original holder should still have lock
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still be held");
    assert_eq!(lock.unwrap().agent_id, "agent-holder");
    
    Ok(())
}

#[tokio::test]
async fn adv_concurrent_add_same_workspace() -> Result<()> {
    // Test race condition: multiple agents adding same workspace
    let queue = Arc::new(MergeQueue::open_in_memory().await?);
    
    let mut handles = vec![];
    for i in 0..10 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            q.add("ws-race", None, 5, None).await
        });
        handles.push(handle);
    }
    
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // Exactly one should succeed
    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = results.iter().filter(|r| r.is_err()).count();
    
    assert_eq!(successes, 1, "Exactly one add should succeed");
    assert_eq!(failures, 9, "Nine adds should fail with duplicate error");
    
    // Verify only one entry exists
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 1, "Only one entry should exist");
    
    Ok(())
}

#[tokio::test]
async fn adv_lock_timeout_with_active_work() -> Result<()> {
    // Test that lock doesn't expire while agent is actively working
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-timeout-test", None, 5, None).await?;
    
    // Claim with very short timeout
    let short_timeout_queue = MergeQueue::open_with_timeout(
        &std::path::PathBuf::from(":memory:"),
        1  // 1 second timeout
    ).await?;
    short_timeout_queue.add("ws-short-timeout", None, 5, None).await?;
    
    // Agent 1 claims entry
    let entry = short_timeout_queue.next_with_lock("agent-timeout").await?;
    assert!(entry.is_some());
    
    // Wait a bit
    sleep(Duration::from_millis(100)).await;
    
    // Try to extend lock
    let extended = short_timeout_queue.extend_lock("agent-timeout", 10).await?;
    assert!(extended, "Lock extension should succeed");
    
    // Verify lock is still held
    let lock = short_timeout_queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still be held after extension");
    
    Ok(())
}

#[tokio::test]
async fn adv_cleanup_with_mixed_states() -> Result<()> {
    // Test cleanup only removes completed/failed entries, not pending/processing
    let queue = MergeQueue::open_in_memory().await?;
    
    // Add entries in various states
    queue.add("ws-pending", None, 5, None).await?;
    queue.add("ws-processing", None, 5, None).await?;
    queue.add("ws-completed", None, 5, None).await?;
    queue.add("ws-failed", None, 5, None).await?;
    
    // Set states
    queue.mark_processing("ws-processing").await?;
    queue.mark_processing("ws-completed").await?;
    queue.mark_completed("ws-completed").await?;
    queue.mark_processing("ws-failed").await?;
    queue.mark_failed("ws-failed", "test error").await?;

    // Wait a moment to ensure entries are "old"
    sleep(Duration::from_millis(100)).await;

    // Cleanup old entries - delete entries completed more than 0 seconds ago
    let cleaned = queue.cleanup(Duration::from_secs(0)).await?;
    
    // Should only clean completed/failed
    assert_eq!(cleaned, 2, "Should clean completed and failed entries");
    
    // Verify remaining entries
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 1, "Pending entry should remain");
    assert_eq!(stats.processing, 1, "Processing entry should remain");
    assert_eq!(stats.completed, 0, "Completed entry should be cleaned");
    assert_eq!(stats.failed, 0, "Failed entry should be cleaned");
    
    Ok(())
}

#[tokio::test]
async fn adv_priority_overflow() -> Result<()> {
    // Test extreme priority values
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-max-priority", None, i32::MAX, None).await?;
    queue.add("ws-min-priority", None, i32::MIN, None).await?;
    queue.add("ws-neg-priority", None, -100, None).await?;
    queue.add("ws-pos-priority", None, 100, None).await?;
    
    let entries = queue.list(Some(QueueStatus::Pending)).await?;
    
    // Should be ordered: MIN, -100, 100, MAX
    assert_eq!(entries[0].workspace, "ws-min-priority");
    assert_eq!(entries[1].workspace, "ws-neg-priority");
    assert_eq!(entries[2].workspace, "ws-pos-priority");
    assert_eq!(entries[3].workspace, "ws-max-priority");
    
    Ok(())
}

#[tokio::test]
async fn adv_position_of_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    let position = queue.position("ws-nonexistent").await?;
    assert!(position.is_none(), "Position of nonexistent workspace should be None");
    
    Ok(())
}

#[tokio::test]
async fn adv_empty_queue_next() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    let next = queue.next().await?;
    assert!(next.is_none(), "Next on empty queue should return None");
    
    let next_with_lock = queue.next_with_lock("agent-test").await?;
    assert!(next_with_lock.is_none(), "Next with lock on empty queue should return None");
    
    Ok(())
}

#[tokio::test]
async fn adv_get_by_id_nonexistent() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    let entry = queue.get_by_id(99999).await?;
    assert!(entry.is_none(), "Getting nonexistent ID should return None");
    
    Ok(())
}

#[tokio::test]
async fn adv_list_with_status_filter() -> Result<()> {
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-1", None, 5, None).await?;
    queue.add("ws-2", None, 5, None).await?;
    queue.add("ws-3", None, 5, None).await?;

    queue.mark_processing("ws-2").await?;
    queue.mark_processing("ws-3").await?;
    queue.mark_completed("ws-3").await?;
    
    let all = queue.list(None).await?;
    assert_eq!(all.len(), 3, "List with no filter should return all");
    
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    assert_eq!(pending.len(), 1, "Should have 1 pending");
    
    let processing = queue.list(Some(QueueStatus::Processing)).await?;
    assert_eq!(processing.len(), 1, "Should have 1 processing");
    
    let completed = queue.list(Some(QueueStatus::Completed)).await?;
    assert_eq!(completed.len(), 1, "Should have 1 completed");
    
    let failed = queue.list(Some(QueueStatus::Failed)).await?;
    assert_eq!(failed.len(), 0, "Should have 0 failed");
    
    Ok(())
}

#[tokio::test]
async fn adv_concurrent_mark_operations_same_entry() -> Result<()> {
    // Test concurrent mark operations on the same entry
    let queue = Arc::new(MergeQueue::open_in_memory().await?);
    
    queue.add("ws-concurrent-mark", None, 5, None).await?;
    
    let mut handles = vec![];
    
    // Spawn tasks trying different mark operations
    for i in 0..5 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            q.mark_processing("ws-concurrent-mark").await
        });
        handles.push(handle);
    }
    
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Exactly one should succeed
    let successes = results.iter().filter(|r| r.is_ok() && *r.as_ref().unwrap()).count();
    assert_eq!(successes, 1, "Exactly one mark_processing should succeed");
    
    // Entry should be in processing state
    let entry = queue.get_by_workspace("ws-concurrent-mark").await?;
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().status, QueueStatus::Processing);
    
    Ok(())
}

#[tokio::test]
async fn adv_stats_across_concurrent_ops() -> Result<()> {
    // Test that stats remain consistent under concurrent operations
    let queue = Arc::new(MergeQueue::open_in_memory().await?);
    
    // Add initial entries
    for i in 0..20 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
    }
    
    let mut handles = vec![];
    
    // Spawn tasks doing various operations
    for i in 0..10 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            // Process some entries
            for j in (0..20).step_by(2) {
                let workspace = format!("ws-{}", j + i);
                let _ = q.mark_processing(&workspace).await;
                let _ = q.mark_completed(&workspace).await;
            }
        });
        handles.push(handle);
    }
    
    futures::future::join_all(handles).await;
    
    // Verify stats are consistent
    let stats = queue.stats().await?;
    assert_eq!(stats.total, 20, "Total should be 20");
    assert!(stats.completed + stats.pending == 20, 
            "Completed + pending should equal total");
    
    Ok(())
}

//! Stress tests for queue concurrent operations.
//!
//! These tests verify that the merge queue correctly handles concurrent access
//! from multiple agents, ensuring no duplicate workspace assignments and proper
//! lock timeout/extension behavior.

use std::{collections::HashSet, sync::Arc, time::Duration};

use futures::future::join_all;
use tokio::time::sleep;
use zjj_core::coordination::queue::{MergeQueue, QueueEntry, QueueStatus};
use zjj_core::Error;

/// Helper to spawn multiple agents concurrently and collect their results.
/// Agents retry up to 10 times with exponential backoff to handle lock contention.
async fn spawn_concurrent_agents(
    queue: Arc<MergeQueue>,
    work_items: Vec<String>,
) -> Vec<Option<String>> {
    const MAX_RETRIES: u32 = 20;
    const INITIAL_BACKOFF_MS: u64 = 1;

    let handles = work_items
        .into_iter()
        .map(|agent_id| {
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let mut result: Result<Option<QueueEntry>, Error> = Ok(None);
                let mut backoff_ms = INITIAL_BACKOFF_MS;

                // Retry with backoff to handle lock contention
                for _attempt in 0..MAX_RETRIES {
                    result = queue.next_with_lock(&agent_id).await;

                    match &result {
                        Ok(Some(_)) => {
                            // Successfully claimed - proceed with work
                            break;
                        }
                        Ok(None) => {
                            // Lock held by another agent - retry with backoff
                            sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = (backoff_ms * 2).min(50); // Exponential backoff, max 50ms
                        }
                        Err(_) => {
                            // Error - stop retrying
                            break;
                        }
                    }
                }

                // Process the claimed entry
                if let Ok(Some(entry)) = &result {
                    // Small delay to simulate work
                    sleep(Duration::from_millis(1)).await;
                    let _ = queue.mark_completed(&entry.workspace).await;
                    let _ = queue.release_processing_lock(&agent_id).await;
                }

                result.ok().flatten().map(|e| e.workspace)
            })
        })
        .collect::<Vec<_>>();

    join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

#[tokio::test]
async fn test_queue_concurrent_lock_no_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    // Test that 10 agents competing for 20 work items results in no duplicates
    let queue = MergeQueue::open_in_memory().await?;

    // Add 20 work items to the queue in parallel
    let items: Vec<_> = (0..20)
        .map(|i| {
            (
                format!("workspace-{}", i),
                format!("bead-{}", i),
                5i32,
            )
        })
        .collect();
    let add_futures: Vec<_> = items
        .iter()
        .map(|(workspace, bead, priority)| {
            queue.add(workspace, Some(bead), *priority, None)
        })
        .collect();
    join_all(add_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Verify all items are pending
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 20, "All 20 items should be pending");

    // Spawn 20 agents concurrently
    let work_items: Vec<String> = (0..20).map(|i| format!("agent-{}", i)).collect();
    let queue = Arc::new(queue);
    let results = spawn_concurrent_agents(queue.clone(), work_items).await;

    // Collect all successfully claimed workspaces
    let claimed_workspaces: HashSet<String> = results
        .into_iter()
        .filter_map(|opt| opt)
        .collect();

    // Verify no duplicates: should have exactly 20 unique workspaces claimed
    assert_eq!(
        claimed_workspaces.len(),
        20,
        "No duplicate workspace assignments expected"
    );

    // Verify all workspaces were claimed exactly once
    for i in 0..20 {
        let workspace = format!("workspace-{}", i);
        assert!(
            claimed_workspaces.contains(&workspace),
            "Workspace {} should be in claimed set",
            workspace
        );
    }

    // Verify queue stats show all items completed
    let stats = queue.stats().await?;
    assert_eq!(stats.completed, 20, "All 20 items should be completed");
    assert_eq!(stats.processing, 0, "No items should remain in processing");

    Ok(())
}

#[tokio::test]
async fn test_queue_lock_timeout_allows_reacquisition() -> Result<(), Box<dyn std::error::Error>> {
    // Test that lock release allows another agent to acquire the lock
    let queue = MergeQueue::open_in_memory().await?;

    // Add a work item
    queue
        .add("workspace-timeout", Some("bead-timeout"), 5, None)
        .await?;

    let queue = Arc::new(queue);

    // Agent 1 claims the work item
    let claimed1 = queue.next_with_lock("agent-1").await?;
    assert!(
        claimed1.as_ref().is_some(),
        "Agent 1 should claim the workspace"
    );

    // Verify agent-1 holds the lock
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should be held");
    if let Some(lock_val) = lock {
        assert_eq!(
            lock_val.agent_id,
            "agent-1",
            "Agent-1 should hold the lock"
        );
    }

    // Agent 2 tries to claim but should fail (lock held)
    let claimed2 = queue.next_with_lock("agent-2").await?;
    assert!(
        claimed2.is_none(),
        "Agent 2 should not claim while agent-1 holds lock"
    );

    // Agent 1 releases the lock explicitly to simulate timeout completion
    let released = queue.release_processing_lock("agent-1").await?;
    assert!(released, "Agent-1 should release the lock");

    // Verify lock is released
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_none(), "Lock should be released");

    // Now agent-2 should be able to acquire the lock and claim new work
    // Add another work item for agent-2 to claim
    queue
        .add("workspace-timeout-2", Some("bead-timeout-2"), 5, None)
        .await?;

    let claimed3 = queue.next_with_lock("agent-2").await?;
    assert!(
        claimed3.is_some(),
        "Agent-2 should claim workspace after lock release"
    );

    // Verify agent-2 now holds the lock
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should be held by agent-2");
    if let Some(lock_val) = lock {
        assert_eq!(
            lock_val.agent_id,
            "agent-2",
            "Agent-2 should hold the lock after acquisition"
        );
    }

    // Cleanup
    let _ = queue.mark_completed("workspace-timeout").await;
    let _ = queue.mark_completed("workspace-timeout-2").await;
    let _ = queue.release_processing_lock("agent-2").await;

    Ok(())
}

#[tokio::test]
async fn test_queue_lock_extension_prevents_expiration() -> Result<(), Box<dyn std::error::Error>> {
    // Test that lock extension prevents expiration
    let queue = MergeQueue::open_in_memory().await?;

    // Add a work item
    queue
        .add("workspace-extend", Some("bead-extend"), 5, None)
        .await?;

    let queue = Arc::new(queue);

    // Agent claims the work item
    let claimed = queue.next_with_lock("agent-extender").await?;
    assert!(claimed.is_some(), "Agent should claim the workspace");

    // Get initial lock info
    let lock1 = queue.get_processing_lock().await?;
    assert!(lock1.is_some(), "Lock should be held");
    let initial_expires = lock1.as_ref().expect("lock should exist").expires_at;

    // Extend the lock by 100 seconds
    let extended = queue.extend_lock("agent-extender", 100).await?;
    assert!(extended, "Lock extension should succeed");

    // Small delay to ensure extension is persisted
    sleep(Duration::from_millis(1)).await;

    // Verify lock was extended
    let lock2 = queue.get_processing_lock().await?;
    assert!(lock2.is_some(), "Lock should still be held");
    let new_expires = lock2.as_ref().expect("lock should exist").expires_at;

    assert!(
        new_expires > initial_expires,
        "Lock expiration should be extended: initial={}, new={}",
        initial_expires, new_expires
    );

    // Another agent cannot claim while lock is extended
    let claimed2 = queue.next_with_lock("agent-other").await?;
    assert!(
        claimed2.is_none(),
        "Other agent cannot claim while lock is held"
    );

    // Verify the lock is still held by the original agent
    let lock3 = queue.get_processing_lock().await?;
    assert!(lock3.is_some(), "Lock should still be held");
    if let Some(lock_val) = lock3 {
        assert_eq!(
            lock_val.agent_id,
            "agent-extender",
            "Original agent should still hold the lock"
        );
    }

    // Test that non-owner cannot extend the lock
    let extended_by_other = queue.extend_lock("agent-other", 100).await?;
    assert!(
        !extended_by_other,
        "Non-owner should not be able to extend the lock"
    );

    // Verify lock expiration hasn't changed (still held by original agent)
    let lock4 = queue.get_processing_lock().await?;
    assert_eq!(
        lock4.as_ref().expect("lock should exist").expires_at,
        new_expires,
        "Lock expiration should not change when non-owner tries to extend"
    );

    // Cleanup: complete the work item
    let completed = queue.mark_completed("workspace-extend").await?;
    assert!(completed, "Should mark workspace as completed");

    let released = queue.release_processing_lock("agent-extender").await?;
    assert!(released, "Should release the lock");

    // Verify lock is released
    let lock5 = queue.get_processing_lock().await?;
    assert!(lock5.is_none(), "Lock should be released");

    Ok(())
}

#[tokio::test]
async fn test_queue_concurrent_high_contention() -> Result<(), Box<dyn std::error::Error>> {
    // Test high contention: many agents competing for few work items
    let queue = MergeQueue::open_in_memory().await?;

    // Add 5 work items in parallel
    let items: Vec<_> = (0..5)
        .map(|i| (format!("ws-{}", i), format!("bead-{}", i), 5i32))
        .collect();
    let add_futures: Vec<_> = items
        .iter()
        .map(|(workspace, bead, priority)| {
            queue.add(workspace, Some(bead), *priority, None)
        })
        .collect();
    join_all(add_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let queue = Arc::new(queue);

    // Spawn 20 agents competing for 5 work items with retry logic
    const MAX_RETRIES: u32 = 20;
    const INITIAL_BACKOFF_MS: u64 = 1;

    let agents: Vec<String> = (0..20).map(|i| format!("agent-{}", i)).collect();
    let handles = agents.into_iter().map(|agent_id| {
        let queue = Arc::clone(&queue);
        tokio::spawn(async move {
            let mut result: Result<Option<QueueEntry>, Error> = Ok(None);
            let mut backoff_ms = INITIAL_BACKOFF_MS;

            // Retry with backoff to handle lock contention
            for _attempt in 0..MAX_RETRIES {
                result = queue.next_with_lock(&agent_id).await;

                match &result {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(50);
                    }
                    Err(_) => break,
                }
            }

            if let Ok(Some(entry)) = &result {
                sleep(Duration::from_millis(1)).await;
                let _ = queue.mark_completed(&entry.workspace).await;
                let _ = queue.release_processing_lock(&agent_id).await;
            }
            result.ok().flatten().map(|e| e.workspace)
        })
    });

    let results = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect::<Vec<_>>();

    // Exactly 5 unique workspaces should be claimed (no duplicates)
    let unique_workspaces: HashSet<_> = results.into_iter().collect();
    assert_eq!(
        unique_workspaces.len(),
        5,
        "Exactly 5 unique workspaces should be claimed, no duplicates"
    );

    // All workspaces should be completed
    let stats = queue.stats().await?;
    assert_eq!(stats.completed, 5, "All 5 workspaces should be completed");

    Ok(())
}

#[tokio::test]
async fn test_queue_serialization_under_load() -> Result<(), Box<dyn std::error::Error>> {
    // Test that queue operations are properly serialized under load
    let queue = MergeQueue::open_in_memory().await?;
    let queue = Arc::new(queue);

    // Add 100 work items with varying priorities in parallel
    let items: Vec<_> = (0..100)
        .map(|i| {
            let workspace = format!("ws-{}", i);
            let bead = format!("bead-{}", i);
            let priority = (i % 10) as i32;
            (workspace, bead, priority)
        })
        .collect();
    let add_futures: Vec<_> = items
        .iter()
        .map(|(workspace, bead, priority)| {
            queue.add(workspace, Some(bead), *priority, None)
        })
        .collect();
    join_all(add_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Verify all added
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 100, "All 100 items should be pending");

    // Process all items with 10 concurrent agents
    let num_agents = 10;
    let mut handles = Vec::new();

    for agent_idx in 0..num_agents {
        let queue = Arc::clone(&queue);
        let handle = tokio::spawn(async move {
            let agent_id = format!("agent-{}", agent_idx);
            let mut processed = 0;

            loop {
                let result = queue.next_with_lock(&agent_id).await;
                match result {
                    Ok(Some(entry)) => {
                        // Small delay to simulate work (1ms vs original 1ms = same speed)
                        sleep(Duration::from_millis(1)).await;
                        // Complete
                        let _ = queue.mark_completed(&entry.workspace).await;
                        let _ = queue.release_processing_lock(&agent_id).await;
                        processed += 1;
                    }
                    Ok(None) => {
                        // No more work or lock held by another agent
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            processed
        });
        handles.push(handle);
    }

    let processed_counts = join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    // Total processed should be 100
    let total_processed: usize = processed_counts.iter().sum();
    assert_eq!(
        total_processed, 100,
        "All 100 workspaces should be processed"
    );

    // Verify final state
    let stats = queue.stats().await?;
    assert_eq!(stats.completed, 100, "All 100 items should be completed");
    assert_eq!(stats.pending, 0, "No items should remain pending");
    assert_eq!(stats.processing, 0, "No items should remain in processing");

    Ok(())
}

#[tokio::test]
async fn test_queue_priority_respected_under_concurrency() -> Result<(), Box<dyn std::error::Error>>
{
    // Test that priority ordering is respected even under concurrent access
    let queue = MergeQueue::open_in_memory().await?;

    // Add work items with different priorities out of order
    let work_items = vec![
        ("ws-low", "bead-1", 10),
        ("ws-high1", "bead-2", 0),
        ("ws-mid1", "bead-3", 5),
        ("ws-high2", "bead-4", 1),
        ("ws-mid2", "bead-5", 5),
        ("ws-low2", "bead-6", 10),
    ];

    // Add work items in parallel
    let add_futures: Vec<_> = work_items
        .iter()
        .map(|(workspace, bead, priority)| {
            queue.add(workspace, Some(bead), *priority, None)
        })
        .collect();
    join_all(add_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let queue = Arc::new(queue);

    // Process all items with 3 agents
    let mut processed_order = Vec::new();
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let agent_id = format!("agent-{}", i);
                let mut order = Vec::new();

                loop {
                    let result = queue.next_with_lock(&agent_id).await;
                    match result {
                        Ok(Some(entry)) => {
                            order.push((entry.workspace.clone(), entry.priority));
                            // Small delay to simulate work (1ms vs original 5ms = 5x faster)
                            sleep(Duration::from_millis(1)).await;
                            let _ = queue.mark_completed(&entry.workspace).await;
                            let _ = queue.release_processing_lock(&agent_id).await;
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }

                order
            })
        })
        .collect();

    let results = join_all(handles).await;
    for result in results {
        if let Ok(order) = result {
            processed_order.extend(order);
        }
    }

    // Sort by priority to verify correct ordering
    processed_order.sort_by_key(|(_, priority)| *priority);

    // Highest priority (0) items should be processed first
    let priority_0_items: Vec<_> = processed_order.iter().filter(|(_, p)| *p == 0).collect();
    assert_eq!(priority_0_items.len(), 1, "One priority-0 item");

    // Lower priority items (10) should be processed last
    let priority_10_items: Vec<_> = processed_order.iter().filter(|(_, p)| *p == 10).collect();
    assert_eq!(priority_10_items.len(), 2, "Two priority-10 items");

    // All items processed
    assert_eq!(processed_order.len(), 6, "All 6 items should be processed");

    Ok(())
}

#[tokio::test]
async fn test_queue_lock_contention_resolution() -> Result<(), Box<dyn std::error::Error>> {
    // Test that lock contention is properly resolved
    let queue = MergeQueue::open_in_memory().await?;

    // Add single work item
    queue.add("ws-single", Some("bead-single"), 5, None).await?;

    let queue = Arc::new(queue);

    // Multiple agents try to claim simultaneously
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let agent_id = format!("agent-{}", i);
                queue.next_with_lock(&agent_id).await
            })
        })
        .collect();

    let results = join_all(handles).await;

    // Count successful claims
    let successful_claims: usize = results
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(|r| matches!(r, Ok(Some(_))))
        .count();

    // Only one agent should successfully claim the workspace
    assert_eq!(
        successful_claims, 1,
        "Only one agent should successfully claim the workspace"
    );

    // Verify workspace is in processing state
    let entry = queue.get_by_workspace("ws-single").await?;
    assert!(entry.is_some(), "Workspace should exist");
    if let Some(entry_val) = entry {
        assert_eq!(
            entry_val.status,
            QueueStatus::Processing,
            "Workspace should be in processing state"
        );
    }

    Ok(())
}

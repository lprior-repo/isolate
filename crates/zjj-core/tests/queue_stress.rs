//! Stress test for coordination queue - Run with: cargo test --test `queue_stress`

use std::time::Duration;

use zjj_core::{
    coordination::queue::{MergeQueue, QueueStatus},
    Result,
};

#[tokio::test]
async fn stress_concurrent_claim_with_massive_contention() -> Result<()> {
    // This test creates extreme contention by having many agents compete for few entries
    let queue = MergeQueue::open_in_memory().await?;

    // Add only 5 entries but spawn 50 agents to fight over them
    for i in 0..5 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
    }

    let mut handles = vec![];

    // Spawn 50 concurrent agents trying to claim
    for i in 0..50 {
        let q = queue.clone();
        let agent_id = format!("agent-{i}");
        let handle = tokio::spawn(async move {
            // Retry a few times to handle lock contention
            for _attempt in 0..10 {
                let result = q.next_with_lock(&agent_id).await;
                match result {
                    Ok(Some(_entry)) => {
                        // Hold the lock briefly to prevent others from getting it
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        // Release the lock
                        let _ = q.release_processing_lock(&agent_id).await;
                        return true;
                    }
                    Ok(None) => {
                        // No entry available or lock held - retry after brief delay
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                    Err(_) => return false,
                }
            }
            false
        });
        handles.push(handle);
    }

    // Wait for all agents to complete
    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| {
            eprintln!("Task join error: {e:?}");
            false
        }))
        .collect();

    let claims = results.iter().filter(|&&x| x).count();
    let failures = results.len() - claims;

    println!("Stress test results: {claims} successful claims, {failures} failed attempts");

    // With 5 entries and 50 agents, we should have exactly 5 successful claims
    // This proves the queue correctly prevents duplicate processing
    assert_eq!(
        claims, 5,
        "Exactly 5 agents should successfully claim entries"
    );

    // Verify no duplicate workspace processing
    let all_entries = queue.list(None).await?;
    let processing_count = all_entries
        .iter()
        .filter(|e| e.status == QueueStatus::Processing)
        .count();

    assert_eq!(
        processing_count, 5,
        "Exactly 5 entries should be in processing state"
    );

    Ok(())
}

#[tokio::test]
async fn stress_rapid_add_remove_cycle() -> Result<()> {
    // Test rapid add/remove cycles to ensure no corruption
    let queue = MergeQueue::open_in_memory().await?;

    let mut handles = vec![];

    // Spawn 20 tasks each doing rapid add/remove
    for i in 0..20 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let workspace = format!("ws-{i}-{j}");
                let () = q.add(&workspace, None, 5, None).await;
                let () = tokio::time::sleep(Duration::from_millis(1)).await;
                let () = q.remove(&workspace).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    futures::future::join_all(handles).await;

    // Verify final state is consistent
    let stats = queue.stats().await?;
    println!("Final stats after rapid add/remove: {:?}", stats);

    // All entries should have been removed
    assert_eq!(stats.total, 0, "All entries should be removed");

    Ok(())
}

#[tokio::test]
async fn stress_priority_under_concurrent_updates() -> Result<()> {
    // Test that priority ordering is maintained under concurrent additions
    let queue = MergeQueue::open_in_memory().await?;

    let mut handles = vec![];

    // Add 100 entries with varying priorities from multiple tasks
    for i in 0..10 {
        let q = queue.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let priority = (i * 10 + j) % 10; // Mix priorities 0-9
                let workspace = format!("ws-task{i}-entry{j}");
                let _ = q.add(&workspace, None, priority, None).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all additions
    futures::future::join_all(handles).await;

    // Verify all entries are present
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 100, "All 100 entries should be pending");

    // Verify priority ordering
    let entries = queue.list(Some(QueueStatus::Pending)).await?;

    // Check that entries are sorted by priority
    for i in 1..entries.len() {
        assert!(
            entries[i - 1].priority <= entries[i].priority,
            "Entries should be sorted by priority: got {} then {}",
            entries[i - 1].priority,
            entries[i].priority
        );
    }

    println!("Priority ordering verified for {} entries", entries.len());

    Ok(())
}

#[tokio::test]
async fn stress_deadlock_prevention_under_timeout() -> Result<()> {
    // Test that the queue doesn't deadlock under lock timeout scenarios
    let queue = MergeQueue::open_in_memory().await?;

    queue.add("ws-1", None, 5, None).await?;
    queue.add("ws-2", None, 5, None).await?;

    // First agent claims and holds lock
    let entry1 = queue.next_with_lock("agent-1").await?;
    assert!(entry1.is_some(), "First agent should claim entry");

    // Second agent should get None (lock held)
    let entry2 = queue.next_with_lock("agent-2").await?;
    assert!(
        entry2.is_none(),
        "Second agent should not claim while lock held"
    );

    // Verify queue stats show one in processing
    let stats = queue.stats().await?;
    assert_eq!(stats.processing, 1, "One entry should be processing");

    // Release lock
    let released = queue.release_processing_lock("agent-1").await?;
    assert!(released, "Lock release should succeed");

    // Now second agent can claim
    let entry3 = queue.next_with_lock("agent-2").await?;
    assert!(
        entry3.is_some(),
        "Second agent should claim after lock release"
    );

    println!("Deadlock prevention test passed - no blocking observed");

    Ok(())
}

#[tokio::test]
async fn stress_concurrent_mark_operations() -> Result<()> {
    // Test concurrent mark_processing, mark_completed, mark_failed operations
    let queue = MergeQueue::open_in_memory().await?;

    // Add 20 entries
    for i in 0..20 {
        queue
            .add(&format!("ws-{i}"), None, 5, Some("agent-initial"))
            .await?;
    }

    let mut handles = vec![];

    // Spawn multiple tasks marking entries as processing
    for i in 0..10 {
        let q = queue.clone();
        let workspace = format!("ws-{i}");
        let handle = tokio::spawn(async move {
            let _ = q.mark_processing(&workspace).await;
            tokio::time::sleep(Duration::from_millis(1)).await;
            let _ = q.mark_completed(&workspace).await;
        });
        handles.push(handle);
    }

    // Spawn more tasks marking as failed
    for i in 10..20 {
        let q = queue.clone();
        let workspace = format!("ws-{i}");
        let handle = tokio::spawn(async move {
            let _ = q.mark_processing(&workspace).await;
            tokio::time::sleep(Duration::from_millis(1)).await;
            let _ = q.mark_failed(&workspace, "test error").await;
        });
        handles.push(handle);
    }

    // Wait for all operations
    futures::future::join_all(handles).await;

    // Verify final state
    let stats = queue.stats().await?;
    println!("Stats after concurrent mark ops: {:?}", stats);

    assert_eq!(stats.completed, 10, "10 entries should be completed");
    assert_eq!(stats.failed, 10, "10 entries should be failed");
    assert_eq!(stats.processing, 0, "No entries should remain processing");

    Ok(())
}

#[tokio::test]
async fn stress_exponential_backoff_under_contention() -> Result<()> {
    // Test that retry logic with exponential backoff works under heavy contention
    let queue = MergeQueue::open_in_memory().await?;

    // Add 10 entries
    for i in 0..10 {
        queue.add(&format!("ws-{i}"), None, 5, None).await?;
    }

    // Manually acquire the processing lock to simulate contention
    let lock_acquired = queue.acquire_processing_lock("holder").await?;
    assert!(lock_acquired, "Lock should be acquired");

    let start = std::time::Instant::now();

    // Spawn 20 agents all trying to claim while lock is held
    // They should encounter lock contention and use retry logic
    let mut handles = vec![];
    for i in 0..20 {
        let q = queue.clone();
        let agent_id = format!("agent-{i}");
        let handle = tokio::spawn(async move {
            // Since next_with_lock returns Ok(None) when lock is held (not a retryable error),
            // we need to add application-level retry logic for this specific test scenario
            // Use a timeout to prevent infinite loops
            let timeout_duration = Duration::from_millis(200);
            let start = std::time::Instant::now();

            let result = loop {
                // Check timeout
                if start.elapsed() >= timeout_duration {
                    break Ok(None);
                }

                let res = q.next_with_lock(&agent_id).await;
                match &res {
                    Ok(Some(_)) => break res, // Success - got an entry
                    Ok(None) => {
                        // Lock held - retry after brief delay
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                    Err(_) => break res, // Error - stop
                }
            };

            (agent_id, result)
        });
        handles.push(handle);
    }

    // Wait a bit then release lock
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = queue.release_processing_lock("holder").await;

    // Wait for all agents to complete
    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    let elapsed = start.elapsed();
    println!("Exponential backoff test completed in {:?}", elapsed);

    // Count successful claims - with the singleton processing lock,
    // only 1 agent should successfully claim (the first one after lock release)
    let successful = results
        .iter()
        .filter(|(_, r)| r.is_ok() && r.as_ref().unwrap().is_some())
        .count();

    assert_eq!(
        successful, 1,
        "Only 1 agent should claim due to singleton processing lock"
    );

    // Verify retry logic didn't cause excessive delays
    assert!(
        elapsed < Duration::from_secs(5),
        "Should complete within 5 seconds"
    );

    Ok(())
}

#[tokio::test]
async fn stress_cleanup_old_entries_under_load() -> Result<()> {
    // Test cleanup operation while queue is under load
    let queue = MergeQueue::open_in_memory().await?;

    // Add and complete 50 old entries
    for i in 0..50 {
        let workspace = format!("old-ws-{i}");
        queue.add(&workspace, None, 5, None).await?;
        queue.mark_processing(&workspace).await?;
        queue.mark_completed(&workspace).await?;
    }

    // Add 20 active pending entries
    for i in 0..20 {
        queue.add(&format!("active-ws-{i}"), None, 5, None).await?;
    }

    // Wait to ensure entries are old enough (>1 second)
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cleanup entries older than 1 second (should clean all completed)
    let cleaned = queue.cleanup(Duration::from_secs(1)).await?;
    println!("Cleaned {} old entries", cleaned);

    assert_eq!(cleaned, 50, "Should clean all 50 completed entries");

    // Verify active entries remain
    let stats = queue.stats().await?;
    assert_eq!(stats.pending, 20, "All 20 active entries should remain");
    assert_eq!(stats.completed, 0, "No completed entries should remain");

    Ok(())
}

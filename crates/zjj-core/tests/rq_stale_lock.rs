//! Test stale lock reclamation
use zjj_core::coordination::queue::MergeQueue;
use zjj_core::QueueStatus;

#[tokio::test]
async fn reclaim_stale_does_not_corrupt_ready_to_merge() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    let q2 = queue.clone();
    
    // Add entry and advance to ready_to_merge via full flow
    queue.add("ws-stale", None, 5, None).await?;
    queue.transition_to("ws-stale", QueueStatus::Claimed).await?;
    queue.transition_to("ws-stale", QueueStatus::Rebasing).await?;
    queue.transition_to("ws-stale", QueueStatus::Testing).await?;
    queue.transition_to("ws-stale", QueueStatus::ReadyToMerge).await?;
    
    // Acquire new processing lock
    let acquired = queue.acquire_processing_lock("agent-holder").await?;
    assert!(acquired, "Lock should be acquired");
    
    // Reclaim with 0 second threshold (our lock should be stale)
    let reclaimed = queue.reclaim_stale(0).await?;
    
    // Should NOT reclaim - entry is ready_to_merge (no active processing)
    assert_eq!(reclaimed, 0, "Should not reclaim ready_to_merge entry");
    
    // Lock should still be held by us
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still be held");
    assert_eq!(lock.unwrap().agent_id, "agent-holder");
    
    // Entry should still be ready_to_merge
    let ready = queue.list(Some(QueueStatus::ReadyToMerge)).await?;
    assert_eq!(ready.len(), 1, "Entry should still be ready_to_merge");
    
    Ok(())
}

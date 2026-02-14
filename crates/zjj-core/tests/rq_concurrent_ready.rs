//! Test concurrent ready_to_merge handling
use zjj_core::coordination::queue::MergeQueue;
use zjj_core::QueueStatus;

#[tokio::test]
async fn two_agents_cannot_claim_same_ready_entry() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    let queue1 = queue.clone();
    let queue2 = queue.clone();
    
    // Add entry and advance it to ready_to_merge
    queue1.add("ws-race", None, 5, None).await?;
    queue1.transition_to("ws-race", QueueStatus::Testing).await?;
    queue1.transition_to("ws-race", QueueStatus::ReadyToMerge).await?;
    
    // Two agents try to process it simultaneously
    let (result1, result2) = tokio::join!(
        queue1.next_with_lock("agent-1"),
        queue2.next_with_lock("agent-2")
    );
    
    // Both should get None - ready_to_merge not claimable
    assert!(result1?.is_none(), "agent-1 should get None");
    assert!(result2?.is_none(), "agent-2 should get None");
    
    // Verify it's still ready_to_merge
    let ready = queue.list(Some(QueueStatus::ReadyToMerge)).await?;
    assert_eq!(ready.len(), 1, "Entry should still be ready_to_merge");
    
    Ok(())
}

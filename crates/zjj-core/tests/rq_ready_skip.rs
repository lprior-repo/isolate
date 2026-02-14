//! Test ready_to_merge entries are skipped by next_with_lock
use zjj_core::coordination::queue::MergeQueue;
use zjj_core::QueueStatus;

#[tokio::test]
async fn ready_to_merge_skipped_by_next_with_lock() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    
    // Add two entries
    queue.add("ws-pending", None, 5, None).await?;
    queue.add("ws-ready", None, 6, None).await?;
    
    // Transition ws-pending through full flow to ready_to_merge
    queue.transition_to("ws-pending", QueueStatus::Claimed).await?;
    queue.transition_to("ws-pending", QueueStatus::Rebasing).await?;
    queue.transition_to("ws-pending", QueueStatus::Testing).await?;
    queue.transition_to("ws-pending", QueueStatus::ReadyToMerge).await?;
    
    // Try to claim - should get ws-ready (higher priority) NOT ws-pending (ready_to_merge)
    let result = queue.next_with_lock("agent-test").await?;
    
    assert!(result.is_some(), "Should claim an entry");
    let entry = result.unwrap();
    assert_eq!(entry.workspace, "ws-ready", "Should claim ws-ready, not ws-pending (ready_to_merge)");
    assert_eq!(entry.status, QueueStatus::Claimed, "Entry should be claimed");
    
    // Verify ws-pending is still ready_to_merge
    let ready_list = queue.list(Some(QueueStatus::ReadyToMerge)).await?;
    assert_eq!(ready_list.len(), 1, "ws-pending should still be ready_to_merge");
    
    Ok(())
}

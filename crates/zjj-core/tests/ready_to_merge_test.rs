// Test ready_to_merge isolation
use zjj_core::coordination::queue::MergeQueue;
use zjj_core::QueueStatus;

#[tokio::test]
async fn ready_to_merge_not_in_pending_list() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    
    queue.add("ws-pending", None, 5, None).await?;
    queue.add("ws-ready", None, 5, None).await?;
    
    // Transition one to ready_to_merge
    queue.transition_to("ws-ready", QueueStatus::ReadyToMerge).await?;
    
    // List pending entries
    let pending = queue.list(Some(QueueStatus::Pending)).await?;
    
    assert_eq!(pending.len(), 1, "Only 1 entry should be pending");
    assert_eq!(pending[0].workspace, "ws-pending", "ws-pending should be the only pending entry");
    
    // Verify ready_to_merge exists in its own list
    let ready = queue.list(Some(QueueStatus::ReadyToMerge)).await?;
    assert_eq!(ready.len(), 1, "1 entry should be ready_to_merge");
    assert_eq!(ready[0].workspace, "ws-ready", "ws-ready should be ready_to_merge");
    
    Ok(())
}

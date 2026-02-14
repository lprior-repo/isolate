//! Edge case tests for processing lock
use zjj_core::coordination::queue::MergeQueue;

#[tokio::test]
async fn release_lock_without_holding_returns_false() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    
    // Try to release a lock we never acquired
    let result = queue.release_processing_lock("agent-imposter").await?;
    
    assert!(!result, "Should return false when releasing lock we don't hold");
    
    Ok(())
}

#[tokio::test]
async fn double_acquire_same_agent_overwrites() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    
    // First acquire
    let acquired1 = queue.acquire_processing_lock("agent-1").await?;
    assert!(acquired1, "First acquire should succeed");
    
    // Second acquire by same agent should overwrite (refresh lock)
    let acquired2 = queue.acquire_processing_lock("agent-1").await?;
    assert!(acquired2, "Same agent should be able to re-acquire (refresh)");
    
    // Verify lock is still held
    let lock = queue.get_processing_lock().await?;
    assert!(lock.is_some(), "Lock should still be held");
    assert_eq!(lock.unwrap().agent_id, "agent-1");
    
    Ok(())
}

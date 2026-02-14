//! Basic lock tests
use zjj_core::coordination::queue::MergeQueue;

#[tokio::test]
async fn first_acquire_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    let acquired = queue.acquire_processing_lock("agent-1").await?;
    assert!(acquired, "First acquire should succeed");
    Ok(())
}

#[tokio::test]
async fn release_by_non_holder_returns_false() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    let result = queue.release_processing_lock("agent-imposter").await?;
    assert!(!result, "Release by non-holder should return false");
    Ok(())
}

#[tokio::test]
async fn second_acquire_different_agent_fails() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    queue.acquire_processing_lock("agent-1").await?;
    let acquired2 = queue.acquire_processing_lock("agent-2").await?;
    assert!(!acquired2, "Second agent should not acquire lock");
    Ok(())
}

//! Test ready_to_merge to rebasing transition (freshen_guard)
use zjj_core::coordination::queue::MergeQueue;
use zjj_core::QueueStatus;

#[tokio::test]
async fn ready_to_merge_can_return_to_rebasing_on_main_change() -> Result<(), Box<dyn std::error::Error>> {
    let queue = MergeQueue::open_in_memory().await?;
    
    // Add entry and advance to ready_to_merge
    queue.add("ws-rebase-test", None, 5, None).await?;
    queue.transition_to("ws-rebase-test", QueueStatus::Claimed).await?;
    queue.transition_to("ws-rebase-test", QueueStatus::Rebasing).await?;
    queue.transition_to("ws-rebase-test", QueueStatus::Testing).await?;
    queue.transition_to("ws-rebase-test", QueueStatus::ReadyToMerge).await?;
    
    // Set head_sha and tested_against_sha to simulate previous testing
    sqlx::query(
        "UPDATE merge_queue SET head_sha = 'abc123', tested_against_sha = 'def456' \
         WHERE workspace = 'ws-rebase-test'"
    ).execute(queue.pool()).await?;
    
    // Try to return to rebasing due to main change
    let result = queue.return_to_rebasing_if_main_changed("ws-rebase-test", "new_main_sha_xyz").await;
    
    assert!(result.is_ok(), "Should be able to return to rebasing from ready_to_merge");
    
    // Verify it's in rebasing state now
    let entries = queue.list(Some(QueueStatus::Rebasing)).await?;
    assert_eq!(entries.len(), 1, "Entry should be in rebasing state");
    assert_eq!(entries[0].workspace, "ws-rebase-test");
    
    Ok(())
}

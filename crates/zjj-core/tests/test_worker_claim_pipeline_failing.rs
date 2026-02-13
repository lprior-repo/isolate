// Tests for worker claim pipeline and state transitions (GREEN phase)
//
// These tests verify that the state transition and audit event functionality
// is correctly implemented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]

use zjj_core::coordination::queue::{MergeQueue, QueueStatus};

#[tokio::test]
async fn test_state_transitions_through_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    // Test state transitions: claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
    let queue = MergeQueue::open_in_memory().await?;

    // Add and claim an entry
    queue
        .add("workspace-pipeline", Some("bead-pipeline"), 5, None)
        .await?;
    let claimed = queue.next_with_lock("agent-pipeline").await?;
    assert!(claimed.is_some());
    let entry = claimed.unwrap();
    assert_eq!(entry.status, QueueStatus::Claimed);

    // 1. claimed -> rebasing
    queue
        .transition_to("workspace-pipeline", QueueStatus::Rebasing)
        .await?;
    let entry = queue
        .get_by_workspace("workspace-pipeline")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Rebasing);

    // 2. rebasing -> testing
    queue
        .transition_to("workspace-pipeline", QueueStatus::Testing)
        .await?;
    let entry = queue
        .get_by_workspace("workspace-pipeline")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Testing);

    // 3. testing -> ready_to_merge
    queue
        .transition_to("workspace-pipeline", QueueStatus::ReadyToMerge)
        .await?;
    let entry = queue
        .get_by_workspace("workspace-pipeline")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::ReadyToMerge);

    // 4. ready_to_merge -> merging
    queue
        .transition_to("workspace-pipeline", QueueStatus::Merging)
        .await?;
    let entry = queue
        .get_by_workspace("workspace-pipeline")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Merging);

    // 5. merging -> merged
    queue
        .transition_to("workspace-pipeline", QueueStatus::Merged)
        .await?;
    let entry = queue
        .get_by_workspace("workspace-pipeline")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::Merged);

    Ok(())
}

#[tokio::test]
async fn test_audit_events_emitted_on_each_transition() -> Result<(), Box<dyn std::error::Error>> {
    // Test that audit events are emitted on each state transition
    let queue = MergeQueue::open_in_memory().await?;

    // Add and claim an entry
    queue
        .add("workspace-events", Some("bead-events"), 5, None)
        .await?;
    let claimed = queue.next_with_lock("agent-events").await?;
    assert!(claimed.is_some());
    let entry_id = claimed.unwrap().id;

    // Perform transitions
    queue
        .transition_to("workspace-events", QueueStatus::Rebasing)
        .await?;
    queue
        .transition_to("workspace-events", QueueStatus::Testing)
        .await?;
    queue
        .transition_to("workspace-events", QueueStatus::ReadyToMerge)
        .await?;
    queue
        .transition_to("workspace-events", QueueStatus::Merging)
        .await?;
    queue
        .transition_to("workspace-events", QueueStatus::Merged)
        .await?;

    // Verify audit events were recorded
    let events = queue.fetch_events(entry_id).await?;

    // Expected transitions:
    // 1. pending -> claimed (by next_with_lock)
    // 2. claimed -> rebasing
    // 3. rebasing -> testing
    // 4. testing -> ready_to_merge
    // 5. ready_to_merge -> merging
    // 6. merging -> merged
    assert_eq!(events.len(), 6, "Should have 6 audit events");

    Ok(())
}

#[tokio::test]
async fn test_error_classification_retryable_vs_terminal() -> Result<(), Box<dyn std::error::Error>>
{
    // Test that errors are classified correctly as retryable vs terminal
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue
        .add("workspace-errors", Some("bead-errors"), 5, None)
        .await?;

    // Claim the entry
    queue.next_with_lock("agent-error").await?;

    // Test retryable failure
    queue
        .transition_to_failed("workspace-errors", "conflict", true)
        .await?;

    let entry = queue
        .get_by_workspace("workspace-errors")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::FailedRetryable);
    assert_eq!(entry.error_message, Some("conflict".to_string()));

    // Release the lock before claiming again
    queue.release_processing_lock("agent-error").await?;

    // Test terminal failure
    // Re-add the entry for terminal test
    queue
        .add("workspace-terminal", Some("bead-terminal"), 5, None)
        .await?;
    queue.next_with_lock("agent-terminal").await?;

    queue
        .transition_to_failed("workspace-terminal", "invalid configuration", false)
        .await?;

    let entry = queue
        .get_by_workspace("workspace-terminal")
        .await?
        .expect("Entry should exist");
    assert_eq!(entry.status, QueueStatus::FailedTerminal);
    assert_eq!(
        entry.error_message,
        Some("invalid configuration".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_failed_claim_prevents_processing() -> Result<(), Box<dyn std::error::Error>> {
    // Test that if claim fails, processing does not proceed
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue
        .add("workspace-fail", Some("bead-fail"), 5, None)
        .await?;

    // Simulate a failed claim by directly setting status to failed without proper lock
    // This should fail because we can't transition from Pending to FailedTerminal directly
    let failed_claim = queue
        .transition_to("workspace-fail", QueueStatus::FailedTerminal)
        .await;
    assert!(failed_claim.is_err());

    // The entry should still be pending
    let entry = queue
        .get_by_workspace("workspace-fail")
        .await?
        .expect("Entry should exist");
    assert_eq!(
        entry.status,
        QueueStatus::Pending,
        "Entry should remain pending"
    );

    // Verify that a subsequent successful claim works
    let successful_claim = queue.next_with_lock("agent-1").await?;
    assert!(
        successful_claim.is_some(),
        "Should be able to claim after failed attempt"
    );

    Ok(())
}

#[tokio::test]
async fn test_state_transition_validation() -> Result<(), Box<dyn std::error::Error>> {
    // Test that invalid state transitions are rejected
    let queue = MergeQueue::open_in_memory().await?;

    // Add an entry
    queue
        .add("workspace-invalid", Some("bead-invalid"), 5, None)
        .await?;

    // Invalid transition: Pending -> Merged (should fail)
    let result = queue
        .transition_to("workspace-invalid", QueueStatus::Merged)
        .await;
    assert!(
        result.is_err(),
        "Should reject invalid transition from Pending to Merged"
    );

    // Claim the entry first
    queue.next_with_lock("agent-test").await?;

    // Invalid transition: Claimed -> Merged (should fail)
    let result = queue
        .transition_to("workspace-invalid", QueueStatus::Merged)
        .await;
    assert!(
        result.is_err(),
        "Should reject invalid transition from Claimed to Merged"
    );

    // Valid transition: Claimed -> Rebasing (should succeed)
    let result = queue
        .transition_to("workspace-invalid", QueueStatus::Rebasing)
        .await;
    assert!(
        result.is_ok(),
        "Should accept valid transition from Claimed to Rebasing"
    );

    Ok(())
}

#[tokio::test]
async fn test_terminal_state_no_outgoing_transitions() -> Result<(), Box<dyn std::error::Error>> {
    // Test that terminal states have no outgoing transitions
    let queue = MergeQueue::open_in_memory().await?;

    // Add and process an entry to merged state
    queue
        .add("workspace-terminal", Some("bead-terminal"), 5, None)
        .await?;
    queue.next_with_lock("agent-terminal").await?;
    queue
        .transition_to("workspace-terminal", QueueStatus::Rebasing)
        .await?;
    queue
        .transition_to("workspace-terminal", QueueStatus::Testing)
        .await?;
    queue
        .transition_to("workspace-terminal", QueueStatus::ReadyToMerge)
        .await?;
    queue
        .transition_to("workspace-terminal", QueueStatus::Merging)
        .await?;
    queue
        .transition_to("workspace-terminal", QueueStatus::Merged)
        .await?;

    // Verify merged is terminal - no outgoing transitions allowed
    let result = queue
        .transition_to("workspace-terminal", QueueStatus::Testing)
        .await;
    assert!(
        result.is_err(),
        "Should reject transition from Merged terminal state"
    );

    // Release lock before next claim
    queue.release_processing_lock("agent-terminal").await?;

    // Test FailedTerminal is also terminal
    queue
        .add("workspace-terminal-2", Some("bead-terminal-2"), 5, None)
        .await?;

    // Use a different agent since the first one still holds the lock
    queue.next_with_lock("agent-terminal-2").await?;

    queue
        .transition_to_failed("workspace-terminal-2", "test error", false)
        .await?;

    let result = queue
        .transition_to("workspace-terminal-2", QueueStatus::Rebasing)
        .await;
    assert!(
        result.is_err(),
        "Should reject transition from FailedTerminal terminal state"
    );

    Ok(())
}

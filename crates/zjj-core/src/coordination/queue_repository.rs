//! Queue Repository Trait - Abstraction boundary for queue persistence.
//!
//! This module defines the `QueueRepository` trait which abstracts the
//! persistence layer for the merge queue, allowing different implementations
//! (SQLite, in-memory, etc.) while preserving the same domain operations.
//!
//! The existing `MergeQueue` implements this trait.

use std::time::Duration;

use super::{
    queue::{QueueAddResponse, QueueControlError, QueueStats},
    queue_entities::{ProcessingLock, QueueEntry, QueueEvent},
    queue_status::{QueueEventType, QueueStatus},
};
use crate::Result;

/// Trait defining the persistence boundary for the merge queue.
///
/// This abstraction allows swapping out the storage backend (SQLite, in-memory,
/// Postgres, etc.) while preserving the same queue operations semantics.
///
/// # Error Handling
///
/// All methods return `Result<T>` from the crate's error type, ensuring
/// consistent error handling across implementations.
///
/// # Async Support
///
/// This trait uses `async_trait` for async method support.
#[async_trait::async_trait]
pub trait QueueRepository: Send + Sync {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // ENTRY OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Add a workspace to the queue.
    async fn add(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
    ) -> Result<QueueAddResponse>;

    /// Add a workspace to the queue with a deduplication key.
    async fn add_with_dedupe(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: Option<&str>,
    ) -> Result<QueueAddResponse>;

    /// Idempotent upsert for submit operations.
    async fn upsert_for_submit(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: &str,
        head_sha: &str,
    ) -> Result<QueueEntry>;

    /// Get a queue entry by its ID.
    async fn get_by_id(&self, id: i64) -> Result<Option<QueueEntry>>;

    /// Get a queue entry by workspace name.
    async fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>>;

    /// List queue entries, optionally filtered by status.
    async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>>;

    /// Get the next pending entry (highest priority, oldest first).
    async fn next(&self) -> Result<Option<QueueEntry>>;

    /// Remove a workspace from the queue.
    async fn remove(&self, workspace: &str) -> Result<bool>;

    /// Get the position of a workspace in the pending queue (1-indexed).
    async fn position(&self, workspace: &str) -> Result<Option<usize>>;

    /// Count pending entries in the queue.
    async fn count_pending(&self) -> Result<usize>;

    /// Get queue statistics.
    async fn stats(&self) -> Result<QueueStats>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // PROCESSING LOCK OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Acquire the processing lock for an agent.
    async fn acquire_processing_lock(&self, agent_id: &str) -> Result<bool>;

    /// Release the processing lock held by an agent.
    async fn release_processing_lock(&self, agent_id: &str) -> Result<bool>;

    /// Get the current processing lock, if any.
    async fn get_processing_lock(&self) -> Result<Option<ProcessingLock>>;

    /// Extend the processing lock expiration.
    async fn extend_lock(&self, agent_id: &str, extra_secs: i64) -> Result<bool>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATUS TRANSITION OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Mark a workspace as processing (claimed).
    async fn mark_processing(&self, workspace: &str) -> Result<bool>;

    /// Mark a workspace as completed.
    async fn mark_completed(&self, workspace: &str) -> Result<bool>;

    /// Mark a workspace as failed.
    async fn mark_failed(&self, workspace: &str, error: &str) -> Result<bool>;

    /// Get the next pending entry and claim it with the processing lock.
    async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>>;

    /// Transition a queue entry to a new status.
    async fn transition_to(&self, workspace: &str, new_status: QueueStatus) -> Result<()>;

    /// Transition a queue entry to a failed state.
    async fn transition_to_failed(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // REBASE OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Update rebase metadata after a successful rebase.
    async fn update_rebase_metadata(
        &self,
        workspace: &str,
        head_sha: &str,
        tested_against_sha: &str,
    ) -> Result<()>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FRESHNESS GUARD OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Check if the entry is fresh (SHA matches current main).
    async fn is_fresh(&self, workspace: &str, current_main_sha: &str) -> Result<bool>;

    /// Return entry to rebasing state when freshness check fails.
    async fn return_to_rebasing(&self, workspace: &str, new_main_sha: &str) -> Result<()>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MERGE OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Begin the merge phase for a workspace.
    async fn begin_merge(&self, workspace: &str) -> Result<()>;

    /// Complete the merge and record the merge commit SHA.
    async fn complete_merge(&self, workspace: &str, merged_sha: &str) -> Result<()>;

    /// Mark a merge as failed.
    async fn fail_merge(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // CONTROL OPERATIONS (Retry & Cancel)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Retry a failed_retryable entry.
    async fn retry_entry(&self, id: i64) -> std::result::Result<QueueEntry, QueueControlError>;

    /// Cancel an active (non-terminal) entry.
    async fn cancel_entry(&self, id: i64) -> std::result::Result<QueueEntry, QueueControlError>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EVENT AUDIT TRAIL
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Append an event to the audit trail.
    async fn append_event(
        &self,
        queue_id: i64,
        event_type: &str,
        details: Option<&str>,
    ) -> Result<()>;

    /// Append a typed event to the audit trail.
    async fn append_typed_event(
        &self,
        queue_id: i64,
        event_type: QueueEventType,
        details: Option<&str>,
    ) -> Result<()>;

    /// Fetch all events for a queue entry.
    async fn fetch_events(&self, queue_id: i64) -> Result<Vec<QueueEvent>>;

    /// Fetch recent events for a queue entry.
    async fn fetch_recent_events(&self, queue_id: i64, limit: usize) -> Result<Vec<QueueEvent>>;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MAINTENANCE OPERATIONS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Clean up old completed/failed entries.
    async fn cleanup(&self, max_age: Duration) -> Result<usize>;

    /// Reclaim stale entries that have been claimed but whose lease has expired.
    async fn reclaim_stale(&self, stale_threshold_secs: i64) -> Result<usize>;
}

//! Worker lifecycle management for graceful shutdown.
//!
//! This module provides signal handling and claim management for worker processes.
//! WHEN a worker receives a shutdown signal, it releases all active claims gracefully.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::coordination::queue::{MergeQueue, QueueStatus};

/// Represents an active claim held by a worker.
#[derive(Debug, Clone)]
pub struct ActiveClaim {
    /// The workspace that was claimed.
    pub workspace: String,
    /// The queue entry ID.
    pub entry_id: i64,
    /// The agent ID that holds the claim.
    pub agent_id: String,
}

/// Tracks active claims for a worker.
#[derive(Debug, Clone, Default)]
pub struct ClaimTracker {
    claims: Arc<RwLock<Vec<ActiveClaim>>>,
}

impl ClaimTracker {
    /// Create a new empty claim tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            claims: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new active claim.
    pub async fn register(&self, claim: ActiveClaim) {
        let mut claims = self.claims.write().await;
        claims.push(claim);
    }

    /// Remove a claim by workspace.
    pub async fn release(&self, workspace: &str) -> Option<ActiveClaim> {
        let mut claims = self.claims.write().await;
        let idx = claims.iter().position(|c| c.workspace == workspace)?;
        Some(claims.remove(idx))
    }

    /// Get all active claims.
    pub async fn all(&self) -> Vec<ActiveClaim> {
        let claims = self.claims.read().await;
        claims.clone()
    }

    /// Check if there are any active claims.
    pub async fn is_empty(&self) -> bool {
        let claims = self.claims.read().await;
        claims.is_empty()
    }

    /// Get the count of active claims.
    pub async fn count(&self) -> usize {
        let claims = self.claims.read().await;
        claims.len()
    }
}

/// Result of a graceful shutdown operation.
#[derive(Debug, Clone)]
pub struct ShutdownResult {
    /// Number of claims successfully released.
    pub released_count: usize,
    /// Number of claims that failed to release.
    pub failed_count: usize,
    /// List of workspaces that failed to release (if any).
    pub failed_workspaces: Vec<String>,
}

/// Gracefully shutdown a worker by releasing all active claims.
///
/// WHEN a worker receives a shutdown signal, this function releases all
/// active claims by transitioning them back to `pending` status.
///
/// # Arguments
/// * `queue` - The merge queue to update
/// * `claims` - The claim tracker with active claims
///
/// # Returns
/// A `ShutdownResult` indicating how many claims were released.
pub async fn graceful_shutdown(queue: &MergeQueue, claims: &ClaimTracker) -> ShutdownResult {
    let active_claims = claims.all().await;

    if active_claims.is_empty() {
        return ShutdownResult {
            released_count: 0,
            failed_count: 0,
            failed_workspaces: Vec::new(),
        };
    }

    let mut released_count = 0;
    let mut failed_workspaces = Vec::new();

    for claim in active_claims {
        match release_claim(queue, &claim).await {
            Ok(true) => {
                released_count += 1;
                tracing::info!(
                    workspace = %claim.workspace,
                    entry_id = claim.entry_id,
                    "Released claim during graceful shutdown"
                );
            }
            Ok(false) => {
                tracing::debug!(
                    workspace = %claim.workspace,
                    entry_id = claim.entry_id,
                    "No claim release needed during graceful shutdown"
                );
            }
            Err(e) => {
                tracing::warn!(
                    workspace = %claim.workspace,
                    entry_id = claim.entry_id,
                    error = %e,
                    "Failed to release claim during shutdown"
                );
                failed_workspaces.push(claim.workspace);
            }
        }
    }

    let failed_count = failed_workspaces.len();

    ShutdownResult {
        released_count,
        failed_count,
        failed_workspaces,
    }
}

/// Release a single claim by transitioning it back to pending.
///
/// This transitions the entry from its current status to `pending`
/// so another worker can reclaim it after shutdown.
async fn release_claim(queue: &MergeQueue, claim: &ActiveClaim) -> crate::Result<bool> {
    let entry = queue
        .get_by_workspace(&claim.workspace)
        .await?
        .ok_or_else(|| {
            crate::Error::NotFound(format!(
                "Workspace '{}' not found in queue",
                claim.workspace
            ))
        })?;

    if entry.status.is_terminal() {
        tracing::debug!(
            workspace = %claim.workspace,
            status = %entry.status.as_str(),
            "Skipping terminal entry during shutdown"
        );
        return Ok(false);
    }

    if entry.agent_id.as_deref() != Some(&claim.agent_id) {
        tracing::debug!(
            workspace = %claim.workspace,
            expected_agent = %claim.agent_id,
            actual_agent = ?entry.agent_id,
            "Claim no longer held by this agent"
        );
        return Ok(false);
    }

    queue
        .transition_to(&claim.workspace, QueueStatus::Pending)
        .await
        .map_err(|e| {
            crate::Error::DatabaseError(format!(
                "Failed to release claim for '{}': {e}",
                claim.workspace
            ))
        })?;

    Ok(true)
}

/// Shutdown signal handler using tokio signal handling.
///
/// Returns a future that resolves when a shutdown signal is received.
#[cfg(unix)]
pub async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).ok();
    let mut sigint = signal(SignalKind::interrupt()).ok();

    tokio::select! {
        () = async {
            if let Some(ref mut sig) = sigterm {
                sig.recv().await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
        () = async {
            if let Some(ref mut sig) = sigint {
                sig.recv().await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            tracing::info!("Received SIGINT, initiating graceful shutdown");
        }
    }
}

#[cfg(not(unix))]
pub async fn wait_for_shutdown_signal() {
    #[cfg(windows)]
    {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Received Ctrl+C, initiating graceful shutdown");
    }
    #[cfg(not(windows))]
    {
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::queue::MergeQueue;

    #[tokio::test]
    async fn test_claim_tracker_register_and_release() {
        let tracker = ClaimTracker::new();

        tracker
            .register(ActiveClaim {
                workspace: "ws-1".to_string(),
                entry_id: 1,
                agent_id: "agent-1".to_string(),
            })
            .await;

        assert_eq!(tracker.count().await, 1);
        assert!(!tracker.is_empty().await);

        let released = tracker.release("ws-1").await;
        assert!(released.is_some());
        assert_eq!(tracker.count().await, 0);
        assert!(tracker.is_empty().await);
    }

    #[tokio::test]
    async fn test_claim_tracker_multiple_claims() {
        let tracker = ClaimTracker::new();

        for i in 0..5 {
            tracker
                .register(ActiveClaim {
                    workspace: format!("ws-{i}"),
                    entry_id: i,
                    agent_id: "agent-1".to_string(),
                })
                .await;
        }

        assert_eq!(tracker.count().await, 5);

        let all = tracker.all().await;
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_empty_claims() {
        let queue = MergeQueue::open_in_memory()
            .await
            .expect("Failed to create queue");
        let tracker = ClaimTracker::new();

        let result = graceful_shutdown(&queue, &tracker).await;

        assert_eq!(result.released_count, 0);
        assert_eq!(result.failed_count, 0);
        assert!(result.failed_workspaces.is_empty());
    }

    #[tokio::test]
    async fn test_graceful_shutdown_releases_claimed_entry() {
        let queue = MergeQueue::open_in_memory()
            .await
            .expect("Failed to create queue");
        let tracker = ClaimTracker::new();

        queue
            .add("ws-1", None, 5, None)
            .await
            .expect("Failed to add");

        let entry = queue
            .next_with_lock("agent-1")
            .await
            .expect("Failed to get next")
            .expect("Expected entry");

        assert_eq!(entry.status, QueueStatus::Claimed);

        tracker
            .register(ActiveClaim {
                workspace: "ws-1".to_string(),
                entry_id: entry.id,
                agent_id: "agent-1".to_string(),
            })
            .await;

        queue.release_processing_lock("agent-1").await.ok();

        let result = graceful_shutdown(&queue, &tracker).await;

        assert_eq!(result.released_count, 1);
        assert_eq!(result.failed_count, 0);

        let updated = queue
            .get_by_workspace("ws-1")
            .await
            .expect("Failed to get entry")
            .expect("Expected entry");
        assert_eq!(updated.status, QueueStatus::Pending);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_skips_terminal_entries() {
        let queue = MergeQueue::open_in_memory()
            .await
            .expect("Failed to create queue");
        let tracker = ClaimTracker::new();

        queue
            .add("ws-1", None, 5, None)
            .await
            .expect("Failed to add");

        queue
            .transition_to("ws-1", QueueStatus::Claimed)
            .await
            .expect("Failed to transition");
        queue
            .transition_to("ws-1", QueueStatus::Cancelled)
            .await
            .expect("Failed to transition");

        tracker
            .register(ActiveClaim {
                workspace: "ws-1".to_string(),
                entry_id: 1,
                agent_id: "agent-1".to_string(),
            })
            .await;

        let result = graceful_shutdown(&queue, &tracker).await;

        assert_eq!(result.released_count, 0);
        assert_eq!(result.failed_count, 0);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_multiple_claims() {
        let queue = MergeQueue::open_in_memory()
            .await
            .expect("Failed to create queue");
        let tracker = ClaimTracker::new();

        for i in 0..3 {
            let ws = format!("ws-{i}");
            queue.add(&ws, None, 5, None).await.expect("Failed to add");
        }

        for i in 0..3 {
            let entry = queue
                .next_with_lock(&format!("agent-{i}"))
                .await
                .expect("Failed to get next")
                .expect("Expected entry");

            tracker
                .register(ActiveClaim {
                    workspace: entry.workspace.clone(),
                    entry_id: entry.id,
                    agent_id: format!("agent-{i}"),
                })
                .await;

            queue
                .release_processing_lock(&format!("agent-{i}"))
                .await
                .ok();
        }

        let result = graceful_shutdown(&queue, &tracker).await;

        assert_eq!(result.released_count, 3);
        assert_eq!(result.failed_count, 0);

        for i in 0..3 {
            let updated = queue
                .get_by_workspace(&format!("ws-{i}"))
                .await
                .expect("Failed to get entry")
                .expect("Expected entry");
            assert_eq!(updated.status, QueueStatus::Pending);
        }
    }
}

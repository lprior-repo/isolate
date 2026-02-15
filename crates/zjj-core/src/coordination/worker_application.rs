//! Application-layer worker pipeline orchestration.
//!
//! This module defines the seam between queue state transitions (domain + persistence)
//! and gate execution runtime (shell adapter). The goal is to keep worker behavior
//! portable so the runtime can be swapped (local process today, durable runtime later).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{future::Future, path::Path};

use super::{QueueRepository, QueueStatus};
use crate::{
    moon_gates::{format_failure_message, GatesOutcome, GatesStatus},
    Result,
};

/// Port for executing quality gates.
///
/// Implementations are shell adapters (`moon`, remote executors, durable workflow engines).
pub trait QualityGateRuntime: Send + Sync {
    /// Execute the full quality gate sequence for a workspace.
    fn execute_quality_gates(
        &self,
        working_dir: &Path,
    ) -> impl Future<Output = Result<GatesOutcome>> + Send;
}

/// Outcome of processing a claimed queue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerPipelineOutcome {
    /// Entry passed all gates and transitioned to `ready_to_merge`.
    ReadyToMerge,
    /// Entry failed gates and transitioned to `failed_retryable`.
    FailedRetryable { message: String },
}

/// Application service for worker pipeline execution.
pub struct WorkerPipelineService<'a, Q, R>
where
    Q: QueueRepository,
    R: QualityGateRuntime,
{
    queue: &'a Q,
    runtime: &'a R,
}

impl<'a, Q, R> WorkerPipelineService<'a, Q, R>
where
    Q: QueueRepository,
    R: QualityGateRuntime,
{
    /// Create a new worker pipeline service.
    #[must_use]
    pub const fn new(queue: &'a Q, runtime: &'a R) -> Self {
        Self { queue, runtime }
    }

    /// Process a claimed queue entry through gates and status transitions.
    ///
    /// # Errors
    ///
    /// Returns an error if queue status transitions fail, gate execution fails,
    /// or lock release fails.
    pub async fn process_claimed_entry(
        &self,
        workspace: &str,
        worker_id: &str,
        working_dir: &Path,
    ) -> Result<WorkerPipelineOutcome> {
        self.queue
            .transition_to(workspace, QueueStatus::Testing)
            .await?;

        let outcome = self.runtime.execute_quality_gates(working_dir).await?;

        let pipeline_outcome = self.apply_outcome_transition(workspace, &outcome).await?;

        self.queue.release_processing_lock(worker_id).await?;

        Ok(pipeline_outcome)
    }

    async fn apply_outcome_transition(
        &self,
        workspace: &str,
        outcome: &GatesOutcome,
    ) -> Result<WorkerPipelineOutcome> {
        match outcome.status {
            GatesStatus::AllPassed => {
                self.queue
                    .transition_to(workspace, QueueStatus::ReadyToMerge)
                    .await?;
                Ok(WorkerPipelineOutcome::ReadyToMerge)
            }
            GatesStatus::QuickFailed | GatesStatus::TestFailed => {
                let failure_message = format_failure_message(outcome);
                self.queue
                    .transition_to_failed(workspace, &failure_message, true)
                    .await?;
                Ok(WorkerPipelineOutcome::FailedRetryable {
                    message: failure_message,
                })
            }
        }
    }
}

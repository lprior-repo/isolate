//! Merge train processing for sequential session merging.
//!
//! This module implements Graphite-style merge train processing that:
//! - Processes queue entries in priority order (lowest priority number first)
//! - Emits `TrainStep` events for each processing step
//! - Runs quality gates and conflict checks
//! - Merges sessions that pass all checks
//! - Handles failures gracefully with proper status updates
//!
//! # Architecture
//!
//! The train processor is pure functional core that operates on a `QueueRepository`.
//! All I/O is delegated to the repository trait, making the core logic testable.
//!
//! # State Machine
//!
//! ```text
//! pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
//!     |          |          |           |              |            |
//!     v          v          v           v              v            v
//! cancelled  failed_retryable/failed_terminal (from any non-terminal state)
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
// Timing values from as_millis() fit in u64 for any practical duration
#![allow(clippy::cast_possible_truncation)]
// Long functions are intentional for pipeline readability
#![allow(clippy::too_many_lines)]

use itertools::Itertools;
use thiserror::Error;

use super::{queue_entities::QueueEntry, queue_status::QueueStatus, QueueRepository};
use crate::{Error, Result};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TRAIN PROCESSING ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for train processing operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TrainError {
    /// Failed to acquire processing lock.
    #[error("failed to acquire processing lock")]
    LockAcquisitionFailed,

    /// Entry processing failed.
    #[error("entry processing failed for workspace '{workspace}': {reason}")]
    EntryFailed { workspace: String, reason: String },

    /// Quality gate failed.
    #[error("quality gate failed for workspace '{workspace}': {gate}")]
    QualityGateFailed { workspace: String, gate: String },

    /// Conflict detected during merge.
    #[error("conflict detected for workspace '{workspace}': {files:?}")]
    ConflictDetected {
        workspace: String,
        files: Vec<String>,
    },

    /// Invalid state transition.
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),

    /// Repository error.
    #[error("repository error: {0}")]
    RepositoryError(String),

    /// Timeout exceeded.
    #[error("operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TRAIN OUTPUT TYPES
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Step in the train processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainStepKind {
    /// Claiming entry for processing.
    Claim,
    /// Rebasing onto target branch.
    Rebase,
    /// Running quality gates.
    Test,
    /// Checking for conflicts.
    ConflictCheck,
    /// Freshness check against main.
    FreshnessCheck,
    /// Performing merge.
    Merge,
    /// Cleanup after processing.
    Cleanup,
}

impl std::fmt::Display for TrainStepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claim => write!(f, "claim"),
            Self::Rebase => write!(f, "rebase"),
            Self::Test => write!(f, "test"),
            Self::ConflictCheck => write!(f, "conflict_check"),
            Self::FreshnessCheck => write!(f, "freshness_check"),
            Self::Merge => write!(f, "merge"),
            Self::Cleanup => write!(f, "cleanup"),
        }
    }
}

/// Status of a train step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainStepStatus {
    /// Step started.
    Started,
    /// Step completed successfully.
    Completed,
    /// Step failed.
    Failed,
    /// Step was skipped.
    Skipped,
}

impl std::fmt::Display for TrainStepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// A step event emitted during train processing.
#[derive(Debug, Clone)]
pub struct TrainStep {
    /// Kind of step being performed.
    pub step: TrainStepKind,
    /// Current status of the step.
    pub status: TrainStepStatus,
    /// Workspace being processed.
    pub workspace: String,
    /// Position in queue (1-indexed).
    pub position: usize,
    /// Optional message with details.
    pub message: Option<String>,
    /// Duration of step in milliseconds (if completed).
    pub duration_ms: Option<u64>,
}

/// Result of processing a single entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryResultKind {
    /// Entry was merged successfully.
    Merged,
    /// Entry failed tests.
    TestsFailed,
    /// Entry has conflicts.
    Conflicts,
    /// Entry is stale (needs rebase).
    Stale,
    /// Entry failed with retryable error.
    FailedRetryable,
    /// Entry failed with terminal error.
    FailedTerminal,
    /// Entry was skipped.
    Skipped,
    /// Entry was cancelled.
    Cancelled,
}

impl std::fmt::Display for EntryResultKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Merged => write!(f, "merged"),
            Self::TestsFailed => write!(f, "tests_failed"),
            Self::Conflicts => write!(f, "conflicts"),
            Self::Stale => write!(f, "stale"),
            Self::FailedRetryable => write!(f, "failed_retryable"),
            Self::FailedTerminal => write!(f, "failed_terminal"),
            Self::Skipped => write!(f, "skipped"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Result of processing a single queue entry.
#[derive(Debug, Clone)]
pub struct EntryResult {
    /// Workspace that was processed.
    pub workspace: String,
    /// Position in queue.
    pub position: usize,
    /// Result kind.
    pub result: EntryResultKind,
    /// Final status after processing.
    pub final_status: QueueStatus,
    /// Optional error message.
    pub error: Option<String>,
    /// Total processing time in milliseconds.
    pub duration_ms: u64,
}

/// Summary of train processing run.
#[derive(Debug, Clone, Default)]
pub struct TrainResult {
    /// Total entries processed.
    pub total_processed: usize,
    /// Entries successfully merged.
    pub merged: usize,
    /// Entries that failed (retryable or terminal).
    pub failed: usize,
    /// Entries that were skipped.
    pub skipped: usize,
    /// Individual entry results.
    pub entries: Vec<EntryResult>,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

impl TrainResult {
    /// Create an empty train result.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total_processed: 0,
            merged: 0,
            failed: 0,
            skipped: 0,
            entries: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Add an entry result to the summary.
    #[must_use]
    pub fn add_entry(&self, entry: EntryResult) -> Self {
        let merged = self.merged + usize::from(entry.result == EntryResultKind::Merged);
        let failed = self.failed
            + usize::from(
                entry.result == EntryResultKind::FailedRetryable
                    || entry.result == EntryResultKind::FailedTerminal,
            );
        let skipped = self.skipped
            + usize::from(
                entry.result == EntryResultKind::Skipped
                    || entry.result == EntryResultKind::Cancelled,
            );

        let mut entries = self.entries.clone();
        entries.push(entry);

        Self {
            total_processed: self.total_processed + 1,
            merged,
            failed,
            skipped,
            entries,
            duration_ms: self.duration_ms,
        }
    }

    /// Set total duration.
    #[must_use]
    pub fn with_duration(self, duration_ms: u64) -> Self {
        Self {
            duration_ms,
            ..self
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUALITY GATE TRAIT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Trait for quality gate implementations.
///
/// Quality gates are pure functions that determine if an entry is ready
/// to proceed to the next stage. Implementations should be:
/// - Deterministic (same input = same output)
/// - Side-effect free (no I/O in the gate itself)
/// - Fast (defer expensive operations to shell layer)
#[async_trait::async_trait]
pub trait QualityGate: Send + Sync {
    /// Check if the entry passes this quality gate.
    ///
    /// Returns `Ok(())` if the gate passes, or an error describing the failure.
    async fn check(&self, entry: &QueueEntry) -> std::result::Result<(), TrainError>;

    /// Human-readable name for this gate.
    fn name(&self) -> &str;
}

/// Trait for merge execution.
///
/// This abstracts the actual merge operation, allowing different
/// implementations (jj, git, mock for testing).
#[async_trait::async_trait]
pub trait MergeExecutor: Send + Sync {
    /// Execute a merge for the given workspace.
    ///
    /// Returns the merged SHA on success, or an error on failure.
    async fn merge(&self, workspace: &str) -> std::result::Result<String, TrainError>;

    /// Check if the workspace has conflicts.
    async fn has_conflicts(&self, workspace: &str) -> std::result::Result<bool, TrainError>;

    /// Get the current HEAD SHA of main branch.
    async fn get_main_sha(&self) -> std::result::Result<String, TrainError>;

    /// Rebase the workspace onto main.
    async fn rebase(&self, workspace: &str) -> std::result::Result<String, TrainError>;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TRAIN PROCESSOR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuration for train processing.
#[derive(Debug, Clone)]
pub struct TrainConfig {
    /// Maximum time to spend on a single entry.
    pub entry_timeout_secs: u64,
    /// Whether to stop on first failure.
    pub stop_on_failure: bool,
    /// Maximum consecutive failures before stopping.
    pub max_consecutive_failures: usize,
    /// Whether to process in dry-run mode (no actual merges).
    pub dry_run: bool,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            entry_timeout_secs: 300, // 5 minutes
            stop_on_failure: false,
            max_consecutive_failures: 3,
            dry_run: false,
        }
    }
}

/// Functional train processor that merges sessions in priority order.
///
/// This is the core domain type for merge train processing. It operates
/// purely on the abstract `QueueRepository` trait, making it testable
/// without database dependencies.
pub struct TrainProcessor<G, M>
where
    G: QualityGate,
    M: MergeExecutor,
{
    repository: std::sync::Arc<dyn QueueRepository>,
    quality_gates: Vec<G>,
    merge_executor: M,
    config: TrainConfig,
}

impl<G, M> TrainProcessor<G, M>
where
    G: QualityGate,
    M: MergeExecutor,
{
    /// Create a new train processor.
    #[must_use]
    pub fn new(
        repository: std::sync::Arc<dyn QueueRepository>,
        quality_gates: Vec<G>,
        merge_executor: M,
        config: TrainConfig,
    ) -> Self {
        Self {
            repository,
            quality_gates,
            merge_executor,
            config,
        }
    }

    /// Process all entries in the merge train.
    ///
    /// This is the main entry point for train processing. It:
    /// 1. Fetches entries in priority order
    /// 2. Processes each entry through the pipeline
    /// 3. Returns a summary of results
    ///
    /// # Errors
    ///
    /// Returns an error if the repository operation fails catastrophically.
    /// Individual entry failures are captured in the result, not as errors.
    pub async fn process(&self) -> Result<TrainResult> {
        let start = std::time::Instant::now();

        // Fetch all pending entries ordered by priority
        let entries = self.fetch_pending_entries().await?;

        // Process entries in order
        let mut result = TrainResult::new();

        for (idx, entry) in entries.into_iter().enumerate() {
            let position = idx + 1;

            // Process this entry
            let entry_result = self.process_entry(&entry, position).await;

            // Check if we should continue
            match entry_result {
                Ok(er) => {
                    result = result.add_entry(er);

                    // Check stop conditions
                    if self.should_stop(&result) {
                        break;
                    }
                }
                Err(e) => {
                    // Catastrophic error - record and continue or stop
                    let failed_result = EntryResult {
                        workspace: entry.workspace.clone(),
                        position,
                        result: EntryResultKind::FailedTerminal,
                        final_status: QueueStatus::FailedTerminal,
                        error: Some(e.to_string()),
                        duration_ms: 0,
                    };
                    result = result.add_entry(failed_result);
                }
            }
        }

        Ok(result.with_duration(start.elapsed().as_millis() as u64))
    }

    /// Fetch pending entries in priority order.
    async fn fetch_pending_entries(&self) -> Result<Vec<QueueEntry>> {
        self.repository
            .list(Some(QueueStatus::Pending))
            .await
            .map(|entries| {
                entries
                    .into_iter()
                    .sorted_by(|a, b| {
                        // Sort by priority (ascending), then by added_at (ascending)
                        a.priority
                            .cmp(&b.priority)
                            .then_with(|| a.added_at.cmp(&b.added_at))
                    })
                    .collect()
            })
    }

    /// Process a single entry through the pipeline.
    async fn process_entry(&self, entry: &QueueEntry, position: usize) -> Result<EntryResult> {
        let start = std::time::Instant::now();

        // Emit step: Claim started
        Self::emit_step(&TrainStep {
            step: TrainStepKind::Claim,
            status: TrainStepStatus::Started,
            workspace: entry.workspace.clone(),
            position,
            message: None,
            duration_ms: None,
        });

        // Try to claim the entry
        let claimed = self.claim_entry(entry).await?;

        if !claimed {
            return Ok(EntryResult {
                workspace: entry.workspace.clone(),
                position,
                result: EntryResultKind::Skipped,
                final_status: entry.status,
                error: Some("Could not claim entry".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Run through the pipeline
        let result = self.run_pipeline(entry, position).await;

        // Update final status based on result
        match &result {
            Ok(er) => {
                // Ensure status is persisted
                let _ = self.update_final_status(entry, er.result).await;
                Ok(EntryResult {
                    duration_ms: start.elapsed().as_millis() as u64,
                    ..er.clone()
                })
            }
            Err(e) => {
                let _ = self
                    .update_final_status(entry, EntryResultKind::FailedTerminal)
                    .await;
                Ok(EntryResult {
                    workspace: entry.workspace.clone(),
                    position,
                    result: EntryResultKind::FailedTerminal,
                    final_status: QueueStatus::FailedTerminal,
                    error: Some(e.to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }

    /// Run the processing pipeline for an entry.
    async fn run_pipeline(&self, entry: &QueueEntry, position: usize) -> Result<EntryResult> {
        let start = std::time::Instant::now();

        // Step 1: Run quality gates
        for gate in &self.quality_gates {
            let gate_name = gate.name().to_string();
            Self::emit_step(&TrainStep {
                step: TrainStepKind::Test,
                status: TrainStepStatus::Started,
                workspace: entry.workspace.clone(),
                position,
                message: Some(format!("Running gate: {gate_name}")),
                duration_ms: None,
            });

            match gate.check(entry).await {
                Ok(()) => {
                    Self::emit_step(&TrainStep {
                        step: TrainStepKind::Test,
                        status: TrainStepStatus::Completed,
                        workspace: entry.workspace.clone(),
                        position,
                        message: Some(format!("Gate passed: {gate_name}")),
                        duration_ms: Some(0),
                    });
                }
                Err(e) => {
                    Self::emit_step(&TrainStep {
                        step: TrainStepKind::Test,
                        status: TrainStepStatus::Failed,
                        workspace: entry.workspace.clone(),
                        position,
                        message: Some(format!("Gate failed: {gate_name} - {e}")),
                        duration_ms: Some(0),
                    });

                    return Ok(EntryResult {
                        workspace: entry.workspace.clone(),
                        position,
                        result: EntryResultKind::TestsFailed,
                        final_status: QueueStatus::FailedRetryable,
                        error: Some(format!("Quality gate '{gate_name}' failed: {e}")),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        // Step 2: Check for conflicts
        Self::emit_step(&TrainStep {
            step: TrainStepKind::ConflictCheck,
            status: TrainStepStatus::Started,
            workspace: entry.workspace.clone(),
            position,
            message: None,
            duration_ms: None,
        });

        match self.merge_executor.has_conflicts(&entry.workspace).await {
            Ok(false) => {
                Self::emit_step(&TrainStep {
                    step: TrainStepKind::ConflictCheck,
                    status: TrainStepStatus::Completed,
                    workspace: entry.workspace.clone(),
                    position,
                    message: Some("No conflicts detected".to_string()),
                    duration_ms: Some(0),
                });
            }
            Ok(true) => {
                Self::emit_step(&TrainStep {
                    step: TrainStepKind::ConflictCheck,
                    status: TrainStepStatus::Failed,
                    workspace: entry.workspace.clone(),
                    position,
                    message: Some("Conflicts detected".to_string()),
                    duration_ms: Some(0),
                });

                return Ok(EntryResult {
                    workspace: entry.workspace.clone(),
                    position,
                    result: EntryResultKind::Conflicts,
                    final_status: QueueStatus::FailedRetryable,
                    error: Some("Conflicts detected in workspace".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                return Ok(EntryResult {
                    workspace: entry.workspace.clone(),
                    position,
                    result: EntryResultKind::FailedRetryable,
                    final_status: QueueStatus::FailedRetryable,
                    error: Some(format!("Conflict check failed: {e}")),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Step 3: Freshness check
        Self::emit_step(&TrainStep {
            step: TrainStepKind::FreshnessCheck,
            status: TrainStepStatus::Started,
            workspace: entry.workspace.clone(),
            position,
            message: None,
            duration_ms: None,
        });

        let main_sha = self
            .merge_executor
            .get_main_sha()
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to get main SHA: {e}")))?;

        let is_fresh = self
            .repository
            .is_fresh(&entry.workspace, &main_sha)
            .await?;

        if !is_fresh {
            Self::emit_step(&TrainStep {
                step: TrainStepKind::FreshnessCheck,
                status: TrainStepStatus::Failed,
                workspace: entry.workspace.clone(),
                position,
                message: Some("Entry is stale, needs rebase".to_string()),
                duration_ms: Some(0),
            });

            // Return to rebasing state
            let _ = self
                .repository
                .return_to_rebasing(&entry.workspace, &main_sha)
                .await;

            return Ok(EntryResult {
                workspace: entry.workspace.clone(),
                position,
                result: EntryResultKind::Stale,
                final_status: QueueStatus::Rebasing,
                error: Some("Entry is stale, needs rebase".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Self::emit_step(&TrainStep {
            step: TrainStepKind::FreshnessCheck,
            status: TrainStepStatus::Completed,
            workspace: entry.workspace.clone(),
            position,
            message: Some("Entry is fresh".to_string()),
            duration_ms: Some(0),
        });

        // Step 4: Transition to ready_to_merge
        self.repository
            .transition_to(&entry.workspace, QueueStatus::ReadyToMerge)
            .await?;

        // Step 5: Perform merge (if not dry run)
        if self.config.dry_run {
            Self::emit_step(&TrainStep {
                step: TrainStepKind::Merge,
                status: TrainStepStatus::Skipped,
                workspace: entry.workspace.clone(),
                position,
                message: Some("Skipped (dry run)".to_string()),
                duration_ms: Some(0),
            });

            return Ok(EntryResult {
                workspace: entry.workspace.clone(),
                position,
                result: EntryResultKind::Skipped,
                final_status: QueueStatus::ReadyToMerge,
                error: Some("Dry run - merge skipped".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        Self::emit_step(&TrainStep {
            step: TrainStepKind::Merge,
            status: TrainStepStatus::Started,
            workspace: entry.workspace.clone(),
            position,
            message: None,
            duration_ms: None,
        });

        // Begin merge phase
        self.repository.begin_merge(&entry.workspace).await?;

        match self.merge_executor.merge(&entry.workspace).await {
            Ok(merged_sha) => {
                // Complete the merge
                self.repository
                    .complete_merge(&entry.workspace, &merged_sha)
                    .await?;

                Self::emit_step(&TrainStep {
                    step: TrainStepKind::Merge,
                    status: TrainStepStatus::Completed,
                    workspace: entry.workspace.clone(),
                    position,
                    message: Some(format!("Merged as {merged_sha}")),
                    duration_ms: Some(0),
                });

                Ok(EntryResult {
                    workspace: entry.workspace.clone(),
                    position,
                    result: EntryResultKind::Merged,
                    final_status: QueueStatus::Merged,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            Err(e) => {
                // Mark merge as failed
                let _ = self
                    .repository
                    .fail_merge(
                        &entry.workspace,
                        &e.to_string(),
                        true, // is_retryable
                    )
                    .await;

                Self::emit_step(&TrainStep {
                    step: TrainStepKind::Merge,
                    status: TrainStepStatus::Failed,
                    workspace: entry.workspace.clone(),
                    position,
                    message: Some(format!("Merge failed: {e}")),
                    duration_ms: Some(0),
                });

                Ok(EntryResult {
                    workspace: entry.workspace.clone(),
                    position,
                    result: EntryResultKind::FailedRetryable,
                    final_status: QueueStatus::FailedRetryable,
                    error: Some(format!("Merge failed: {e}")),
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }

    /// Attempt to claim an entry for processing.
    async fn claim_entry(&self, entry: &QueueEntry) -> Result<bool> {
        match self
            .repository
            .transition_to(&entry.workspace, QueueStatus::Claimed)
            .await
        {
            Ok(()) => Ok(true),
            Err(Error::InvalidConfig(_)) => Ok(false), // Invalid transition
            Err(e) => Err(e),
        }
    }

    /// Update the final status of an entry.
    async fn update_final_status(&self, entry: &QueueEntry, result: EntryResultKind) -> Result<()> {
        let target_status = match result {
            EntryResultKind::Merged => QueueStatus::Merged,
            EntryResultKind::TestsFailed
            | EntryResultKind::Conflicts
            | EntryResultKind::FailedRetryable => QueueStatus::FailedRetryable,
            EntryResultKind::Stale => QueueStatus::Rebasing,
            EntryResultKind::FailedTerminal => QueueStatus::FailedTerminal,
            EntryResultKind::Skipped => return Ok(()),
            EntryResultKind::Cancelled => QueueStatus::Cancelled,
        };

        self.repository
            .transition_to(&entry.workspace, target_status)
            .await?;
        Ok(())
    }

    /// Emit a train step event.
    fn emit_step(step: &TrainStep) {
        // In a real implementation, this would emit to an event stream
        // For now, we just log it
        tracing::info!(
            step = %step.step,
            status = %step.status,
            workspace = %step.workspace,
            position = step.position,
            "Train step"
        );
    }

    /// Check if processing should stop based on results.
    fn should_stop(&self, result: &TrainResult) -> bool {
        if self.config.stop_on_failure && result.failed > 0 {
            return true;
        }

        // Check consecutive failures
        let consecutive_failures = result
            .entries
            .iter()
            .rev()
            .take_while(|e| {
                e.result == EntryResultKind::FailedRetryable
                    || e.result == EntryResultKind::FailedTerminal
            })
            .count();

        consecutive_failures >= self.config.max_consecutive_failures
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PURE HELPER FUNCTIONS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Sort queue entries by priority (lowest number = highest priority).
///
/// This is a pure function that can be used to pre-sort entries
/// before processing.
#[must_use]
pub fn sort_by_priority(entries: Vec<QueueEntry>) -> Vec<QueueEntry> {
    entries
        .into_iter()
        .sorted_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.added_at.cmp(&b.added_at))
        })
        .collect()
}

/// Filter entries that are ready for processing.
///
/// Returns only entries that are in a processable state.
#[must_use]
pub fn filter_processable(entries: Vec<QueueEntry>) -> Vec<QueueEntry> {
    entries
        .into_iter()
        .filter(|e| e.status == QueueStatus::Pending)
        .collect()
}

/// Calculate the position of each entry in the queue.
///
/// Returns a map from workspace name to 1-indexed position.
#[must_use]
pub fn calculate_positions(entries: &[QueueEntry]) -> std::collections::HashMap<String, usize> {
    entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| (entry.workspace.clone(), idx + 1))
        .collect()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_train_step_kind_display() {
        assert_eq!(TrainStepKind::Claim.to_string(), "claim");
        assert_eq!(TrainStepKind::Rebase.to_string(), "rebase");
        assert_eq!(TrainStepKind::Test.to_string(), "test");
        assert_eq!(TrainStepKind::ConflictCheck.to_string(), "conflict_check");
        assert_eq!(TrainStepKind::FreshnessCheck.to_string(), "freshness_check");
        assert_eq!(TrainStepKind::Merge.to_string(), "merge");
        assert_eq!(TrainStepKind::Cleanup.to_string(), "cleanup");
    }

    #[test]
    fn test_train_step_status_display() {
        assert_eq!(TrainStepStatus::Started.to_string(), "started");
        assert_eq!(TrainStepStatus::Completed.to_string(), "completed");
        assert_eq!(TrainStepStatus::Failed.to_string(), "failed");
        assert_eq!(TrainStepStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn test_entry_result_kind_display() {
        assert_eq!(EntryResultKind::Merged.to_string(), "merged");
        assert_eq!(EntryResultKind::TestsFailed.to_string(), "tests_failed");
        assert_eq!(EntryResultKind::Conflicts.to_string(), "conflicts");
        assert_eq!(EntryResultKind::Stale.to_string(), "stale");
        assert_eq!(
            EntryResultKind::FailedRetryable.to_string(),
            "failed_retryable"
        );
        assert_eq!(
            EntryResultKind::FailedTerminal.to_string(),
            "failed_terminal"
        );
        assert_eq!(EntryResultKind::Skipped.to_string(), "skipped");
        assert_eq!(EntryResultKind::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_train_result_new() {
        let result = TrainResult::new();
        assert_eq!(result.total_processed, 0);
        assert_eq!(result.merged, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);
        assert!(result.entries.is_empty());
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_train_result_add_entry() {
        let result = TrainResult::new();

        let entry = EntryResult {
            workspace: "test-workspace".to_string(),
            position: 1,
            result: EntryResultKind::Merged,
            final_status: QueueStatus::Merged,
            error: None,
            duration_ms: 100,
        };

        let updated = result.add_entry(entry);

        assert_eq!(updated.total_processed, 1);
        assert_eq!(updated.merged, 1);
        assert_eq!(updated.failed, 0);
        assert_eq!(updated.skipped, 0);
        assert_eq!(updated.entries.len(), 1);
    }

    #[test]
    fn test_train_result_add_failed_entry() {
        let result = TrainResult::new();

        let entry = EntryResult {
            workspace: "test-workspace".to_string(),
            position: 1,
            result: EntryResultKind::FailedTerminal,
            final_status: QueueStatus::FailedTerminal,
            error: Some("Something went wrong".to_string()),
            duration_ms: 50,
        };

        let updated = result.add_entry(entry);

        assert_eq!(updated.total_processed, 1);
        assert_eq!(updated.merged, 0);
        assert_eq!(updated.failed, 1);
        assert_eq!(updated.skipped, 0);
    }

    #[test]
    fn test_train_result_with_duration() {
        let result = TrainResult::new().with_duration(5000);
        assert_eq!(result.duration_ms, 5000);
    }

    #[test]
    fn test_train_error_display() {
        let err = TrainError::LockAcquisitionFailed;
        assert!(err.to_string().contains("lock"));

        let err = TrainError::EntryFailed {
            workspace: "test".to_string(),
            reason: "timeout".to_string(),
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("timeout"));

        let err = TrainError::QualityGateFailed {
            workspace: "test".to_string(),
            gate: "lint".to_string(),
        };
        assert!(err.to_string().contains("lint"));

        let err = TrainError::ConflictDetected {
            workspace: "test".to_string(),
            files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
        };
        assert!(err.to_string().contains("conflict"));
        assert!(err.to_string().contains("file1.rs"));
    }

    #[test]
    fn test_train_config_default() {
        let config = TrainConfig::default();
        assert_eq!(config.entry_timeout_secs, 300);
        assert!(!config.stop_on_failure);
        assert_eq!(config.max_consecutive_failures, 3);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_sort_by_priority() {
        let entries = vec![
            QueueEntry {
                id: 1,
                workspace: "low-priority".to_string(),
                bead_id: None,
                priority: 10,
                status: QueueStatus::Pending,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
            QueueEntry {
                id: 2,
                workspace: "high-priority".to_string(),
                bead_id: None,
                priority: 1,
                status: QueueStatus::Pending,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
        ];

        let sorted = sort_by_priority(entries);

        assert_eq!(sorted[0].workspace, "high-priority");
        assert_eq!(sorted[1].workspace, "low-priority");
    }

    #[test]
    fn test_filter_processable() {
        let entries = vec![
            QueueEntry {
                id: 1,
                workspace: "pending".to_string(),
                bead_id: None,
                priority: 1,
                status: QueueStatus::Pending,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
            QueueEntry {
                id: 2,
                workspace: "claimed".to_string(),
                bead_id: None,
                priority: 1,
                status: QueueStatus::Claimed,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
        ];

        let filtered = filter_processable(entries);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].workspace, "pending");
    }

    #[test]
    fn test_calculate_positions() {
        let entries = vec![
            QueueEntry {
                id: 1,
                workspace: "first".to_string(),
                bead_id: None,
                priority: 1,
                status: QueueStatus::Pending,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
            QueueEntry {
                id: 2,
                workspace: "second".to_string(),
                bead_id: None,
                priority: 2,
                status: QueueStatus::Pending,
                added_at: Utc::now().timestamp(),
                started_at: None,
                completed_at: None,
                error_message: None,
                agent_id: None,
                dedupe_key: None,
                workspace_state: super::super::queue_status::WorkspaceQueueState::Created,
                previous_state: None,
                state_changed_at: None,
                head_sha: None,
                tested_against_sha: None,
                attempt_count: 0,
                max_attempts: 3,
                rebase_count: 0,
                last_rebase_at: None,
                parent_workspace: None,
            },
        ];

        let positions = calculate_positions(&entries);

        assert_eq!(positions.get("first"), Some(&1));
        assert_eq!(positions.get("second"), Some(&2));
    }
}

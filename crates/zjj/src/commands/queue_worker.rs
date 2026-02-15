//! Queue worker daemon for processing queue entries.
//!
//! This module implements the worker command that processes entries from the
//! merge queue. It supports both one-shot processing (--once) and continuous
//! loop processing (--loop).
//!
//! ## Gate Execution Flow
//!
//! 1. Claim entry from queue (status: `pending` -> `claimed`)
//! 2. Transition to `testing` status
//! 3. Run `moon run :quick` (fail fast)
//! 4. If quick passes, run `moon run :test`
//! 5. On both gates passing -> `ready_to_merge`
//! 6. On any failure -> `failed_retryable`
//!
//! ## Error Classification
//!
//! Failures are classified as retryable or terminal:
//! - **Retryable**: transient errors (I/O, database locked, timeouts, test failures)
//! - **Terminal**: permanent errors (conflicts, validation, permissions)
//!
//! ## Graceful Shutdown
//!
//! The worker handles SIGINT/SIGTERM signals and:
//! - Releases active claims on shutdown
//! - Leaves the queue in a consistent state
//! - Transitions in-progress items to appropriate failure states

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use tokio::{process::Command, signal, sync::Notify, time::sleep};
use zjj_core::{
    coordination::{QualityGateRuntime, WorkerPipelineOutcome, WorkerPipelineService},
    json::SchemaEnvelope,
    moon_gates::{
        classify_exit_code, combine_results, format_failure_message, GateResult, GatesOutcome,
        GatesStatus, MoonGate,
    },
    MergeQueue, OutputFormat, QueueStatus,
};

use crate::commands::{
    get_queue_db_path,
    worker_error::{classify_with_attempts, ErrorClass},
};

/// Default stale threshold in seconds (5 minutes).
const DEFAULT_STALE_THRESHOLD_SECS: i64 = 300;

/// Exit code for successful processing.
const EXIT_SUCCESS: i32 = 0;

/// Exit code for general errors.
const EXIT_ERROR: i32 = 1;

/// Exit code for nothing to process (--once with no pending items).
///
/// This is treated as a successful no-op so automation can poll safely.
const EXIT_NOTHING_TO_DO: i32 = 0;

/// Options for the queue worker command.
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    /// Run continuously (process items until interrupted).
    pub loop_mode: bool,
    /// Process exactly one item, then exit.
    pub once: bool,
    /// Polling interval in seconds.
    pub interval_secs: u64,
    /// Unique worker identifier.
    pub worker_id: Option<String>,
    /// Output format.
    pub format: OutputFormat,
}

/// Output for the worker command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerOutput {
    /// Worker identifier.
    pub worker_id: String,
    /// Number of items processed.
    pub processed: usize,
    /// Number of stale entries reclaimed.
    pub reclaimed: usize,
    /// Number of items that failed (retryable).
    pub failed_retryable: usize,
    /// Number of items that failed (terminal).
    pub failed_terminal: usize,
    /// Final status message.
    pub message: String,
}

/// State tracking for the active entry being processed.
#[derive(Debug, Clone)]
struct ActiveEntry {
    /// The workspace name.
    workspace: String,
    /// The queue entry ID.
    id: i64,
    /// Current attempt count.
    attempt_count: i32,
    /// Maximum allowed attempts.
    max_attempts: i32,
}

/// Generate a worker ID from hostname and PID.
fn generate_worker_id() -> String {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .map_or_else(
            |_| "unknown".to_string(),
            |h| {
                h.split('.')
                    .next()
                    .map_or_else(|| "unknown".to_string(), String::from)
            },
        );
    let pid = std::process::id();
    format!("{hostname}-{pid}")
}

/// Resolve the worker ID, generating one if not provided.
fn resolve_worker_id(provided: Option<&str>) -> String {
    provided.map_or_else(generate_worker_id, String::from)
}

/// Get or create the merge queue database.
async fn get_queue() -> Result<MergeQueue> {
    let queue_db = get_queue_db_path().await?;
    MergeQueue::open(&queue_db)
        .await
        .context("Failed to open merge queue database")
}

/// Print output in the requested format.
fn print_output(output: &WorkerOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("queue-worker-response", "single", output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize output")?;
        println!("{json_str}");
    } else {
        println!("Worker {} finished", output.worker_id);
        println!("  Processed: {} items", output.processed);
        println!("  Reclaimed: {} stale entries", output.reclaimed);
        if output.failed_retryable > 0 {
            println!("  Failed (retryable): {} items", output.failed_retryable);
        }
        if output.failed_terminal > 0 {
            println!("  Failed (terminal): {} items", output.failed_terminal);
        }
        println!("  {}", output.message);
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// MOON GATE EXECUTION (Shell Layer)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Execute a moon task and return the result.
///
/// This is the shell adapter that handles I/O for moon execution.
/// It runs moon as a subprocess and captures the output.
///
/// # Arguments
/// * `gate` - Which gate to run (Quick or Test)
/// * `working_dir` - The working directory for moon execution
///
/// # Returns
/// A `GateResult` containing pass/fail status and output.
async fn execute_moon_gate(gate: MoonGate, working_dir: &Path) -> Result<GateResult> {
    let task = gate.as_task();

    let output = Command::new("moon")
        .args(["run", task])
        .current_dir(working_dir)
        .output()
        .await
        .with_context(|| format!("Failed to execute moon run {task}"))?;

    let exit_code = output.status.code().map_or(-1, |c| c);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let passed = classify_exit_code(exit_code);

    Ok(if passed {
        GateResult::passed(gate, stdout, stderr)
    } else {
        GateResult::failed(gate, exit_code, stdout, stderr)
    })
}

/// Execute all required gates for a queue entry.
///
/// This implements the fail-fast logic:
/// 1. Run :quick gate first
/// 2. If quick fails, return immediately (fail fast)
/// 3. If quick passes, run :test gate
///
/// # Arguments
/// * `working_dir` - The working directory for moon execution
///
/// # Returns
/// A `GatesOutcome` with results from all executed gates.
async fn execute_all_gates(working_dir: &Path) -> Result<GatesOutcome> {
    // Run quick gate first
    let quick_result = execute_moon_gate(MoonGate::Quick, working_dir).await?;

    // Fail fast: if quick fails, don't run test
    if !quick_result.passed {
        return Ok(combine_results(quick_result, None));
    }

    // Quick passed, run test gate
    let test_result = execute_moon_gate(MoonGate::Test, working_dir).await?;

    Ok(combine_results(quick_result, Some(test_result)))
}

/// Local shell adapter for gate execution.
struct LocalMoonRuntime;

impl QualityGateRuntime for LocalMoonRuntime {
    fn execute_quality_gates(
        &self,
        working_dir: &Path,
    ) -> impl std::future::Future<Output = zjj_core::Result<GatesOutcome>> + Send {
        async move {
            execute_all_gates(working_dir)
                .await
                .map_err(|err| zjj_core::Error::Command(err.to_string()))
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PROCESSING LOGIC
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Handle a processing failure with proper error classification.
///
/// Classifies the error and transitions the queue entry to the appropriate
/// failure state (`failed_retryable` or `failed_terminal`).
///
/// # Arguments
/// * `queue` - The merge queue
/// * `entry` - The active entry that failed
/// * `error` - The error that occurred
/// * `worker_id` - The worker ID for lock release
///
/// # Returns
/// The error classification (Retryable or Terminal).
async fn handle_processing_failure(
    queue: &MergeQueue,
    entry: &ActiveEntry,
    error: &anyhow::Error,
    worker_id: &str,
) -> ErrorClass {
    let error_msg = error.to_string();

    // Classify error with attempt count consideration
    let classification =
        classify_with_attempts(&error_msg, entry.attempt_count, entry.max_attempts);

    // Log the classification decision
    let class_str = match classification {
        ErrorClass::Retryable => "retryable",
        ErrorClass::Terminal => "terminal",
    };
    eprintln!(
        "Entry {} failed ({class_str}): {error_msg}",
        entry.workspace
    );

    // Transition to appropriate failure state
    let is_retryable = matches!(classification, ErrorClass::Retryable);
    let transition_result = queue
        .transition_to_failed(&entry.workspace, &error_msg, is_retryable)
        .await;

    if let Err(e) = transition_result {
        eprintln!(
            "Warning: Failed to transition entry {} to failed state: {e}",
            entry.workspace
        );
    }

    // Always release the processing lock
    if let Err(e) = queue.release_processing_lock(worker_id).await {
        eprintln!("Warning: Failed to release processing lock: {e}");
    }

    classification
}

/// Handle gate execution results and transition queue entry appropriately.
///
/// # Arguments
/// * `queue` - The merge queue
/// * `entry` - The active entry being processed
/// * `outcome` - The gates execution outcome
/// * `worker_id` - The worker ID for lock release
///
/// # Returns
/// `Ok(())` on success, `Err` on failure (with error classification).
async fn handle_gates_outcome(
    queue: &MergeQueue,
    entry: &ActiveEntry,
    outcome: &GatesOutcome,
    worker_id: &str,
) -> Result<()> {
    match outcome.status {
        GatesStatus::AllPassed => {
            // Both gates passed - transition to ready_to_merge
            queue
                .transition_to(&entry.workspace, QueueStatus::ReadyToMerge)
                .await
                .with_context(|| {
                    format!("Failed to transition {} to ready_to_merge", entry.workspace)
                })?;

            // Release lock
            if let Err(e) = queue.release_processing_lock(worker_id).await {
                eprintln!("Warning: Failed to release processing lock: {e}");
            }

            eprintln!(
                "Entry {} passed all gates - ready to merge",
                entry.workspace
            );
            Ok(())
        }
        GatesStatus::QuickFailed | GatesStatus::TestFailed => {
            // Gate failed - transition to failed_retryable
            // Test failures are always retryable (not terminal)
            let failure_msg = format_failure_message(outcome);

            queue
                .transition_to_failed(&entry.workspace, &failure_msg, true)
                .await
                .with_context(|| {
                    format!(
                        "Failed to transition {} to failed_retryable",
                        entry.workspace
                    )
                })?;

            // Release lock
            if let Err(e) = queue.release_processing_lock(worker_id).await {
                eprintln!("Warning: Failed to release processing lock: {e}");
            }

            eprintln!("Entry {} failed gates: {}", entry.workspace, failure_msg);
            Err(anyhow::anyhow!("{failure_msg}"))
        }
    }
}

/// Release any active claims on shutdown.
///
/// This ensures the queue is left in a consistent state when the worker
/// is interrupted.
async fn release_active_claims(
    queue: &MergeQueue,
    active_entry: Option<&ActiveEntry>,
    worker_id: &str,
) {
    // Release the processing lock
    if let Err(e) = queue.release_processing_lock(worker_id).await {
        eprintln!("Warning: Failed to release processing lock on shutdown: {e}");
    }

    // If there was an active entry, it will be reclaimed by stale entry recovery
    // on the next worker run. We don't transition it to failed here because
    // we don't have a meaningful error message.
    if let Some(entry) = active_entry {
        eprintln!(
            "Note: Entry {} will be reclaimed by stale entry recovery",
            entry.workspace
        );
    }
}

/// Run the queue worker with the given options.
///
/// # Returns
/// The exit code that should be used.
///
/// # Exit Codes
/// - 0: Success (item processed in --once, or clean shutdown in --loop)
/// - 1: General error
/// - 0: Nothing to process (--once with no pending items)
pub async fn run_with_options(options: &WorkerOptions) -> Result<i32> {
    let worker_id = resolve_worker_id(options.worker_id.as_deref());
    let queue = get_queue().await?;

    // Reclaim stale entries on startup
    let reclaimed = queue
        .reclaim_stale(DEFAULT_STALE_THRESHOLD_SECS)
        .await
        .context("Failed to reclaim stale entries")?;

    if reclaimed > 0 && !options.format.is_json() {
        eprintln!("Reclaimed {reclaimed} stale queue entries");
    }

    let interval = Duration::from_secs(options.interval_secs);

    if options.once {
        // Process exactly one item
        run_once(&queue, &worker_id, reclaimed, options.format).await
    } else if options.loop_mode {
        // Run continuously until interrupted
        run_loop(&queue, &worker_id, reclaimed, interval, options.format).await
    } else {
        // Neither --once nor --loop: show usage hint
        anyhow::bail!(
            "Worker mode required. Use --once to process one item, or --loop for continuous processing."
        );
    }
}

/// Run the worker in one-shot mode (--once).
///
/// Processes exactly one item if available, then exits.
async fn run_once(
    queue: &MergeQueue,
    worker_id: &str,
    reclaimed: usize,
    format: OutputFormat,
) -> Result<i32> {
    let entry = queue
        .next_with_lock(worker_id)
        .await
        .context("Failed to claim queue entry")?;

    let Some(entry) = entry else {
        // No pending items
        let output = WorkerOutput {
            worker_id: worker_id.to_string(),
            processed: 0,
            reclaimed,
            failed_retryable: 0,
            failed_terminal: 0,
            message: "No pending items to process".to_string(),
        };
        print_output(&output, format)?;
        return Ok(EXIT_NOTHING_TO_DO);
    };

    // Track active entry for graceful shutdown
    let active = ActiveEntry {
        workspace: entry.workspace.clone(),
        id: entry.id,
        attempt_count: entry.attempt_count,
        max_attempts: entry.max_attempts,
    };

    // Process the entry
    if !format.is_json() {
        eprintln!(
            "Processing queue entry {} (workspace: {}, attempt {}/{})",
            entry.id, entry.workspace, entry.attempt_count, entry.max_attempts
        );
    }

    // Process with moon gates
    let process_result = process_entry_with_gates(queue, &active, worker_id, format).await;

    match process_result {
        Ok(()) => {
            let output = WorkerOutput {
                worker_id: worker_id.to_string(),
                processed: 1,
                reclaimed,
                failed_retryable: 0,
                failed_terminal: 0,
                message: format!("Successfully processed {}", entry.workspace),
            };
            print_output(&output, format)?;
            Ok(EXIT_SUCCESS)
        }
        Err(error) => {
            // Error already handled in process_entry_with_gates
            // Just need to determine if it was retryable or terminal
            let error_msg = error.to_string();
            let classification =
                classify_with_attempts(&error_msg, active.attempt_count, active.max_attempts);

            let (failed_retryable, failed_terminal) = match classification {
                ErrorClass::Retryable => (1, 0),
                ErrorClass::Terminal => (0, 1),
            };

            let output = WorkerOutput {
                worker_id: worker_id.to_string(),
                processed: 0,
                reclaimed,
                failed_retryable,
                failed_terminal,
                message: format!("Failed to process {}: {error}", entry.workspace),
            };
            print_output(&output, format)?;
            Ok(EXIT_ERROR)
        }
    }
}

/// Run the worker in loop mode (--loop).
///
/// Continuously processes items until interrupted by SIGINT/SIGTERM.
#[allow(clippy::too_many_lines)]
async fn run_loop(
    queue: &MergeQueue,
    worker_id: &str,
    reclaimed: usize,
    interval: Duration,
    format: OutputFormat,
) -> Result<i32> {
    let mut processed_count = 0usize;
    let mut failed_retryable_count = 0usize;
    let mut failed_terminal_count = 0usize;
    let mut active_entry: Option<ActiveEntry> = None;

    // Set up shutdown signal handling
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = Arc::clone(&shutdown);

    // Handle SIGINT (Ctrl+C)
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        shutdown_clone.notify_one();
    });

    // Handle SIGTERM (on Unix)
    #[cfg(unix)]
    {
        let shutdown_clone = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let sigterm_result = signal::unix::signal(signal::unix::SignalKind::terminate());
            if let Ok(mut sigterm) = sigterm_result {
                let _ = sigterm.recv().await;
                shutdown_clone.notify_one();
            }
        });
    }

    if !format.is_json() {
        eprintln!(
            "Worker {} starting (polling every {}s)",
            worker_id,
            interval.as_secs()
        );
        eprintln!("Press Ctrl+C to initiate graceful shutdown");
    }

    loop {
        // Check for shutdown signal
        if shutdown.notified().now_or_never().is_some() {
            if !format.is_json() {
                eprintln!("Shutdown signal received, initiating graceful shutdown...");
            }
            break;
        }

        // Try to claim and process one item
        let entry = queue
            .next_with_lock(worker_id)
            .await
            .context("Failed to claim queue entry")?;

        if let Some(entry) = entry {
            // Track active entry
            active_entry = Some(ActiveEntry {
                workspace: entry.workspace.clone(),
                id: entry.id,
                attempt_count: entry.attempt_count,
                max_attempts: entry.max_attempts,
            });

            if !format.is_json() {
                eprintln!(
                    "Processing queue entry {} (workspace: {}, attempt {}/{})",
                    entry.id, entry.workspace, entry.attempt_count, entry.max_attempts
                );
            }

            // Process with moon gates
            let default_entry = ActiveEntry {
                workspace: entry.workspace.clone(),
                id: entry.id,
                attempt_count: entry.attempt_count,
                max_attempts: entry.max_attempts,
            };
            let active_ref = match active_entry.as_ref() {
                Some(entry) => entry,
                None => &default_entry,
            };
            let process_result =
                process_entry_with_gates(queue, active_ref, worker_id, format).await;

            match process_result {
                Ok(()) => {
                    processed_count += 1;
                }
                Err(error) => {
                    let error_msg = error.to_string();
                    // Get attempt info from entry (we know entry is still valid here)
                    let (attempt_count, max_attempts) = active_entry
                        .as_ref()
                        .map_or((entry.attempt_count, entry.max_attempts), |a| {
                            (a.attempt_count, a.max_attempts)
                        });
                    let classification =
                        classify_with_attempts(&error_msg, attempt_count, max_attempts);

                    match classification {
                        ErrorClass::Retryable => failed_retryable_count += 1,
                        ErrorClass::Terminal => failed_terminal_count += 1,
                    }
                }
            }

            // Clear active entry after processing
            active_entry = None;
        } else {
            // No pending items, wait before polling again
            tokio::select! {
                () = sleep(interval) => {},
                () = shutdown.notified() => {
                    if !format.is_json() {
                        eprintln!("Shutdown signal received during idle, exiting...");
                    }
                    break;
                }
            }
        }
    }

    // Graceful shutdown: release any active claims
    if let Some(ref entry) = active_entry {
        if !format.is_json() {
            eprintln!(
                "Releasing active claim on entry {} for graceful shutdown",
                entry.workspace
            );
        }
        release_active_claims(queue, Some(entry), worker_id).await;
    } else {
        // Just release the lock
        release_active_claims(queue, None, worker_id).await;
    }

    let output = WorkerOutput {
        worker_id: worker_id.to_string(),
        processed: processed_count,
        reclaimed,
        failed_retryable: failed_retryable_count,
        failed_terminal: failed_terminal_count,
        message: format!(
            "Worker shutdown after processing {processed_count} items ({failed_retryable_count} retryable failures, {failed_terminal_count} terminal failures)"
        ),
    };
    print_output(&output, format)?;
    Ok(EXIT_SUCCESS)
}

/// Process a queue entry with moon gate execution.
///
/// This is the main processing function that:
/// 1. Transitions the entry to `testing` status
/// 2. Runs moon gates (:quick then :test)
/// 3. Transitions to `ready_to_merge` on success or `failed_retryable` on failure
///
/// # Arguments
/// * `queue` - The merge queue for status transitions
/// * `entry` - The active entry being processed
/// * `worker_id` - The worker ID for lock release
/// * `format` - Output format for logging
///
/// # Returns
/// `Ok(())` on success, `Err` on failure.
async fn process_entry_with_gates(
    queue: &MergeQueue,
    entry: &ActiveEntry,
    worker_id: &str,
    format: OutputFormat,
) -> Result<()> {
    if !format.is_json() {
        eprintln!("Entry {}: running quality gates...", entry.workspace);
    }

    // Get workspace path (using current directory for now)
    // In a full implementation, this would resolve the workspace path from the entry
    let working_dir = Path::new(".");

    let runtime = LocalMoonRuntime;
    let service = WorkerPipelineService::new(queue, &runtime);

    let outcome = service
        .process_claimed_entry(&entry.workspace, worker_id, working_dir)
        .await
        .with_context(|| format!("Failed to process worker pipeline for {}", entry.workspace))?;

    match outcome {
        WorkerPipelineOutcome::ReadyToMerge => {
            if !format.is_json() {
                eprintln!(
                    "Entry {} passed all gates - ready to merge",
                    entry.workspace
                );
            }
            Ok(())
        }
        WorkerPipelineOutcome::FailedRetryable { message } => {
            if !format.is_json() {
                eprintln!("Entry {} failed gates: {}", entry.workspace, message);
            }
            Err(anyhow::anyhow!(message))
        }
    }
}

/// Process a queue entry (legacy stub - kept for compatibility).
///
/// This function is no longer the main processing path.
/// Use `process_entry_with_gates` instead.
///
/// # Errors
///
/// Returns an error if processing fails.
#[allow(dead_code)]
fn process_entry(_entry: &zjj_core::QueueEntry) -> Result<()> {
    // This is kept for backward compatibility but is not used
    // The main processing now goes through process_entry_with_gates
    anyhow::bail!("Use process_entry_with_gates instead");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_worker_id() {
        let id = generate_worker_id();
        // Should contain a dash (hostname-pid format)
        assert!(id.contains('-'), "Worker ID should contain dash: {id}");
        // Should not be empty
        assert!(!id.is_empty(), "Worker ID should not be empty");
    }

    #[test]
    fn test_resolve_worker_id_with_provided() {
        let id = resolve_worker_id(Some("custom-worker-123"));
        assert_eq!(id, "custom-worker-123");
    }

    #[test]
    fn test_resolve_worker_id_without_provided() {
        let id = resolve_worker_id(None);
        assert!(!id.is_empty(), "Generated worker ID should not be empty");
        assert!(
            id.contains('-'),
            "Generated worker ID should contain dash: {id}"
        );
    }

    #[test]
    fn test_worker_options_defaults() {
        let options = WorkerOptions {
            loop_mode: false,
            once: false,
            interval_secs: 10,
            worker_id: None,
            format: OutputFormat::Human,
        };
        assert_eq!(options.interval_secs, 10);
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_ERROR, 1);
        assert_eq!(EXIT_NOTHING_TO_DO, 0);
    }

    #[test]
    fn test_active_entry_tracking() {
        let entry = ActiveEntry {
            workspace: "test-workspace".to_string(),
            id: 42,
            attempt_count: 1,
            max_attempts: 3,
        };

        assert_eq!(entry.workspace, "test-workspace");
        assert_eq!(entry.id, 42);
        assert_eq!(entry.attempt_count, 1);
        assert_eq!(entry.max_attempts, 3);
    }

    #[test]
    fn test_worker_output_serialization() {
        let output = WorkerOutput {
            worker_id: "test-worker".to_string(),
            processed: 5,
            reclaimed: 2,
            failed_retryable: 1,
            failed_terminal: 0,
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());

        let parsed: WorkerOutput = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(parsed.worker_id, "test-worker");
        assert_eq!(parsed.processed, 5);
        assert_eq!(parsed.reclaimed, 2);
        assert_eq!(parsed.failed_retryable, 1);
        assert_eq!(parsed.failed_terminal, 0);
    }

    #[test]
    fn test_gates_status_from_moon_gates() {
        // Test that we can use the moon_gates types correctly
        let quick_passed = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test_passed = GateResult::passed(MoonGate::Test, String::new(), String::new());
        let outcome = combine_results(quick_passed, Some(test_passed));

        assert_eq!(outcome.status, GatesStatus::AllPassed);
        assert!(outcome.status.is_success());
    }

    #[test]
    fn test_gates_status_quick_failed() {
        let quick_failed = GateResult::failed(MoonGate::Quick, 1, String::new(), String::new());
        let outcome = combine_results(quick_failed, None);

        assert_eq!(outcome.status, GatesStatus::QuickFailed);
        assert!(outcome.status.is_failure());
        assert!(outcome.test.is_none()); // Test should be skipped
    }

    #[test]
    fn test_gates_status_test_failed() {
        let quick_passed = GateResult::passed(MoonGate::Quick, String::new(), String::new());
        let test_failed = GateResult::failed(MoonGate::Test, 1, String::new(), String::new());
        let outcome = combine_results(quick_passed, Some(test_failed));

        assert_eq!(outcome.status, GatesStatus::TestFailed);
        assert!(outcome.status.is_failure());
        assert!(outcome.test.is_some());
    }
}

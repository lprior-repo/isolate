//! Queue worker daemon for processing queue entries.
//!
//! This module implements the worker command that processes entries from the
//! merge queue. It supports both one-shot processing (--once) and continuous
//! loop processing (--loop).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{path::Path, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{signal, time::sleep};
use zjj_core::{json::SchemaEnvelope, MergeQueue, OutputFormat};

/// Default stale threshold in seconds (5 minutes).
const DEFAULT_STALE_THRESHOLD_SECS: i64 = 300;

/// Exit code for successful processing.
const EXIT_SUCCESS: i32 = 0;

/// Exit code for general errors.
const EXIT_ERROR: i32 = 1;

/// Exit code for nothing to process (--once with no pending items).
const EXIT_NOTHING_TO_DO: i32 = 2;

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
    /// Final status message.
    pub message: String,
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
                    .map(String::from)
                    .map_or_else(|| "unknown".to_string(), |s| s)
            },
        );
    let pid = std::process::id();
    format!("{hostname}-{pid}")
}

/// Resolve the worker ID, generating one if not provided.
fn resolve_worker_id(provided: Option<&str>) -> String {
    provided
        .map(String::from)
        .map_or_else(generate_worker_id, |id| id)
}

/// Get or create the merge queue database.
async fn get_queue() -> Result<MergeQueue> {
    let queue_db = Path::new(".zjj/queue.db");
    MergeQueue::open(queue_db)
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
        println!("  {}", output.message);
    }
    Ok(())
}

/// Run the queue worker with the given options.
///
/// # Returns
/// The exit code that should be used.
///
/// # Exit Codes
/// - 0: Success (item processed in --once, or clean shutdown in --loop)
/// - 1: General error
/// - 2: Nothing to process (--once with no pending items)
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
            message: "No pending items to process".to_string(),
        };
        print_output(&output, format)?;
        return Ok(EXIT_NOTHING_TO_DO);
    };

    // Process the entry (stub: just log and mark as completed)
    if !format.is_json() {
        eprintln!(
            "Processing queue entry {} (workspace: {})",
            entry.id, entry.workspace
        );
    }

    // Mark as completed
    let marked = queue
        .mark_completed(&entry.workspace)
        .await
        .context("Failed to mark entry as completed")?;

    let release_result = queue.release_processing_lock(worker_id).await;

    if !marked {
        let output = WorkerOutput {
            worker_id: worker_id.to_string(),
            processed: 0,
            reclaimed,
            message: format!("Failed to mark {} as completed", entry.workspace),
        };
        print_output(&output, format)?;
        return Ok(EXIT_ERROR);
    }

    // Log if lock release failed (non-fatal)
    if let Err(e) = release_result {
        eprintln!("Warning: Failed to release processing lock: {e}");
    }

    let output = WorkerOutput {
        worker_id: worker_id.to_string(),
        processed: 1,
        reclaimed,
        message: format!("Successfully processed {}", entry.workspace),
    };
    print_output(&output, format)?;
    Ok(EXIT_SUCCESS)
}

/// Run the worker in loop mode (--loop).
///
/// Continuously processes items until interrupted by SIGINT/SIGTERM.
async fn run_loop(
    queue: &MergeQueue,
    worker_id: &str,
    reclaimed: usize,
    interval: Duration,
    format: OutputFormat,
) -> Result<i32> {
    let mut processed_count = 0usize;
    let mut shutdown = setup_shutdown_signal()?;

    if !format.is_json() {
        eprintln!(
            "Worker {} starting (polling every {}s)",
            worker_id,
            interval.as_secs()
        );
    }

    loop {
        // Check for shutdown signal
        if shutdown.is_shutdown() {
            break;
        }

        // Try to claim and process one item
        let entry = queue
            .next_with_lock(worker_id)
            .await
            .context("Failed to claim queue entry")?;

        if let Some(entry) = entry {
            if !format.is_json() {
                eprintln!(
                    "Processing queue entry {} (workspace: {})",
                    entry.id, entry.workspace
                );
            }

            // Process the entry (stub: just log and mark as completed)
            let marked = queue
                .mark_completed(&entry.workspace)
                .await
                .context("Failed to mark entry as completed")?;

            let release_result = queue.release_processing_lock(worker_id).await;

            if marked {
                processed_count += 1;
            } else {
                eprintln!("Warning: Failed to mark {} as completed", entry.workspace);
            }

            // Log if lock release failed (non-fatal)
            if let Err(e) = release_result {
                eprintln!("Warning: Failed to release processing lock: {e}");
            }
        } else {
            // No pending items, wait before polling again
            tokio::select! {
                _ = sleep(interval) => {},
                _ = shutdown.wait_for_shutdown() => {
                    break;
                }
            }
        }
    }

    let output = WorkerOutput {
        worker_id: worker_id.to_string(),
        processed: processed_count,
        reclaimed,
        message: format!("Worker shutdown after processing {} items", processed_count),
    };
    print_output(&output, format)?;
    Ok(EXIT_SUCCESS)
}

/// Shutdown signal handler.
struct ShutdownSignal {
    shutdown_rx: Option<tokio::sync::broadcast::Receiver<()>>,
}

impl ShutdownSignal {
    /// Check if shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.shutdown_rx.is_none()
    }

    /// Wait for shutdown signal.
    async fn wait_for_shutdown(&mut self) {
        if let Some(ref mut rx) = self.shutdown_rx {
            let _ = rx.recv().await;
            self.shutdown_rx = None;
        }
    }
}

/// Set up signal handlers for graceful shutdown.
fn setup_shutdown_signal() -> Result<ShutdownSignal> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    // Handle SIGINT (Ctrl+C)
    let tx_int = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        let _ = tx_int.send(());
    });

    // Handle SIGTERM (on Unix)
    #[cfg(unix)]
    {
        let tx_term = shutdown_tx;
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .map_or_else(|_| None, |s| Some(s));
            if let Some(ref mut sig) = sigterm {
                let _ = sig.recv().await;
                let _ = tx_term.send(());
            }
        });
    }

    Ok(ShutdownSignal {
        shutdown_rx: Some(shutdown_rx),
    })
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
        assert_eq!(EXIT_NOTHING_TO_DO, 2);
    }
}

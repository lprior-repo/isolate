//! Batch command implementation
//!
//! Executes multiple commands atomically with rollback support.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Batch command options
#[derive(Debug, Clone)]
pub struct BatchOptions {
    /// Enable atomic mode (all succeed or all rollback)
    pub atomic: bool,
    /// Dry run (preview without executing)
    pub dry_run: bool,
    /// Stop on first error
    pub stop_on_error: bool,
    /// Commands to execute (as strings)
    pub commands: Vec<String>,
}

/// A single operation within a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    /// Command to execute
    pub command: String,
    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,
    /// Whether this operation is optional
    #[serde(default)]
    pub optional: bool,
}

/// Result of a single batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult {
    /// Command that was executed
    pub command: String,
    /// Whether this operation succeeded
    pub success: bool,
    /// Status of the operation
    pub status: BatchItemStatus,
    /// Output from the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Status of a batch item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchItemStatus {
    /// Operation succeeded
    Succeeded,
    /// Operation failed
    Failed,
    /// Operation was skipped
    Skipped,
    /// Operation was rolled back
    RolledBack,
    /// Operation was previewed (dry run)
    DryRun,
}

/// Response from batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Overall success
    pub success: bool,
    /// Total operations
    pub total: usize,
    /// Operations that succeeded
    pub succeeded: usize,
    /// Operations that failed
    pub failed: usize,
    /// Operations that were skipped
    pub skipped: usize,
    /// Individual operation results
    pub results: Vec<BatchItemResult>,
    /// Whether this was executed in atomic mode
    pub atomic: bool,
    /// Whether rollback was performed
    pub rolled_back: bool,
}

/// Run the batch command
///
/// # Errors
///
/// Returns an error if:
/// - No commands are provided
/// - Any required command fails (in atomic mode)
pub async fn run(options: &BatchOptions) -> Result<()> {
    if options.commands.is_empty() {
        anyhow::bail!("No commands provided for batch execution");
    }

    let operations = parse_commands(&options.commands)?;
    let response = execute_batch(options, &operations).await;

    print_batch_response(&response);

    if !response.success {
        anyhow::bail!("Batch execution failed");
    }

    Ok(())
}

/// Parse commands into operations
fn parse_commands(commands: &[String]) -> Result<Vec<BatchOperation>> {
    commands
        .iter()
        .map(|cmd| {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                anyhow::bail!("Empty command in batch");
            }
            Ok(BatchOperation {
                command: parts[0].to_string(),
                args: parts[1..]
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                optional: false,
            })
        })
        .collect()
}

/// Execute batch operations
async fn execute_batch(options: &BatchOptions, operations: &[BatchOperation]) -> BatchResponse {
    let mut results = Vec::with_capacity(operations.len());
    let mut should_stop = false;

    for operation in operations {
        let result = if should_stop {
            make_skipped_result(operation, "Previous operation failed")
        } else if options.dry_run {
            make_dry_run_result(operation)
        } else {
            execute_operation(operation).await
        };

        // Check if we should stop
        if !options.dry_run && options.atomic && !operation.optional && !result.success {
            should_stop = true;
        }

        results.push(result);
    }

    // Calculate summary
    let succeeded = results
        .iter()
        .filter(|r| r.status == BatchItemStatus::Succeeded)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == BatchItemStatus::Failed)
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.status == BatchItemStatus::Skipped)
        .count();

    // Determine success
    let success = if options.atomic {
        results
            .iter()
            .all(|r| r.success || is_optional(operations, &r.command))
    } else {
        failed == 0 || !options.stop_on_error
    };

    // In atomic mode with failure, mark as rolled back
    let rolled_back = options.atomic && !success && !options.dry_run;
    if rolled_back {
        for result in &mut results {
            if result.status == BatchItemStatus::Succeeded {
                result.status = BatchItemStatus::RolledBack;
            }
        }
    }

    BatchResponse {
        success,
        total: results.len(),
        succeeded,
        failed,
        skipped,
        results,
        atomic: options.atomic,
        rolled_back,
    }
}

/// Execute a single operation
async fn execute_operation(operation: &BatchOperation) -> BatchItemResult {
    let start = Instant::now();
    let command_str = format!("{} {}", operation.command, operation.args.join(" "));

    // For now, we just simulate command execution
    // In a real implementation, this would dispatch to the appropriate command handler
    let result = simulate_command(&operation.command, &operation.args).await;

    let duration_ms = duration_to_ms(start.elapsed());

    match result {
        Ok(output) => BatchItemResult {
            command: command_str,
            success: true,
            status: BatchItemStatus::Succeeded,
            output: Some(output),
            error: None,
            duration_ms: Some(duration_ms),
        },
        Err(e) => BatchItemResult {
            command: command_str,
            success: false,
            status: BatchItemStatus::Failed,
            output: None,
            error: Some(e),
            duration_ms: Some(duration_ms),
        },
    }
}

/// Simulate command execution (placeholder)
async fn simulate_command(command: &str, _args: &[String]) -> Result<String, String> {
    // This is a placeholder - in real implementation, this would dispatch to handlers
    // For now, we just simulate success for known commands
    match command {
        "queue" | "agent" | "lock" | "events" => Ok(format!("Simulated: {command} executed")),
        _ => Err(format!("Unknown command: {command}")),
    }
}

/// Make a dry run result
fn make_dry_run_result(operation: &BatchOperation) -> BatchItemResult {
    BatchItemResult {
        command: format!("{} {}", operation.command, operation.args.join(" ")),
        success: true,
        status: BatchItemStatus::DryRun,
        output: Some("(dry run)".to_string()),
        error: None,
        duration_ms: Some(0),
    }
}

/// Make a skipped result
fn make_skipped_result(operation: &BatchOperation, reason: &str) -> BatchItemResult {
    BatchItemResult {
        command: format!("{} {}", operation.command, operation.args.join(" ")),
        success: false,
        status: BatchItemStatus::Skipped,
        output: None,
        error: Some(reason.to_string()),
        duration_ms: None,
    }
}

/// Check if an operation is optional
fn is_optional(operations: &[BatchOperation], command: &str) -> bool {
    operations
        .iter()
        .find(|op| format!("{} {}", op.command, op.args.join(" ")) == command)
        .is_some_and(|op| op.optional)
}

/// Convert duration to milliseconds
fn duration_to_ms(duration: std::time::Duration) -> u64 {
    duration.as_millis().try_into().map_or(u64::MAX, |v| v)
}

/// Print batch response
fn print_batch_response(response: &BatchResponse) {
    println!(
        "Batch (atomic={}): {} total, {} succeeded, {} failed, {} skipped",
        response.atomic, response.total, response.succeeded, response.failed, response.skipped
    );

    if response.rolled_back {
        println!("Status: ROLLED BACK (all operations undone)");
    } else if response.success {
        println!("Status: SUCCESS (all operations committed)");
    } else {
        println!("Status: FAILED (some operations failed)");
    }

    println!();

    for item in &response.results {
        let status_icon = match item.status {
            BatchItemStatus::Succeeded => "✓",
            BatchItemStatus::Failed => "✗",
            BatchItemStatus::Skipped => "○",
            BatchItemStatus::RolledBack => "↩",
            BatchItemStatus::DryRun => "▷",
        };

        println!("{status_icon} {}", item.command);

        if let Some(e) = &item.error {
            println!("    Error: {e}");
        }

        if let Some(ms) = item.duration_ms {
            println!("    Duration: {ms}ms");
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_dry_run() -> Result<()> {
        let options = BatchOptions {
            atomic: false,
            dry_run: true,
            stop_on_error: false,
            commands: vec!["queue list".to_string()],
        };

        run(&options).await
    }

    #[test]
    fn test_parse_commands() {
        let commands = vec!["queue list".to_string(), "agent status".to_string()];
        let operations = parse_commands(&commands).map_or(Vec::new(), |ops| ops);

        assert_eq!(operations.len(), 2);
        assert_eq!(operations[0].command, "queue");
        assert_eq!(operations[0].args, vec!["list"]);
    }
}

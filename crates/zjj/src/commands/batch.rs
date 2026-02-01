//! Batch command - Execute multiple commands in sequence
//!
//! Allows AI agents to batch multiple operations with transactional semantics.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

/// Options for the batch command
#[derive(Debug, Clone)]
pub struct BatchOptions {
    /// Commands to execute (JSON array or newline-separated)
    pub commands: Vec<BatchCommand>,
    /// Stop on first error
    pub stop_on_error: bool,
    /// Dry-run all commands
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// A single command in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCommand {
    /// Command name (without 'zjj' prefix)
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Optional ID for referencing in results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Whether this command is optional (continue on failure)
    #[serde(default)]
    pub optional: bool,
}

/// Result of batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Overall success (all required commands succeeded)
    pub success: bool,
    /// Total commands executed
    pub total: usize,
    /// Commands that succeeded
    pub succeeded: usize,
    /// Commands that failed
    pub failed: usize,
    /// Commands that were skipped
    pub skipped: usize,
    /// Individual command results
    pub results: Vec<CommandResult>,
    /// Whether this was a dry-run
    pub dry_run: bool,
}

/// Result of a single command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Command ID if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Full command string
    pub command: String,
    /// Whether this command succeeded
    pub success: bool,
    /// Status (succeeded, failed, skipped)
    pub status: CommandStatus,
    /// Output from the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Status of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    Succeeded,
    Failed,
    Skipped,
}

/// Run the batch command
pub fn run(options: &BatchOptions) -> Result<()> {
    let mut result = BatchResult {
        success: true,
        total: options.commands.len(),
        succeeded: 0,
        failed: 0,
        skipped: 0,
        results: vec![],
        dry_run: options.dry_run,
    };

    let mut stop = false;

    for cmd in &options.commands {
        if stop {
            result.results.push(CommandResult {
                id: cmd.id.clone(),
                command: format!("{} {}", cmd.command, cmd.args.join(" ")),
                success: false,
                status: CommandStatus::Skipped,
                output: None,
                error: Some("Skipped due to previous error".to_string()),
                duration_ms: None,
            });
            result.skipped += 1;
            continue;
        }

        let start = std::time::Instant::now();
        let exec_result = execute_command(&cmd.command, &cmd.args, options.dry_run);
        let duration = start.elapsed().as_millis() as u64;

        match exec_result {
            Ok(output) => {
                result.results.push(CommandResult {
                    id: cmd.id.clone(),
                    command: format!("{} {}", cmd.command, cmd.args.join(" ")),
                    success: true,
                    status: CommandStatus::Succeeded,
                    output: Some(output),
                    error: None,
                    duration_ms: Some(duration),
                });
                result.succeeded += 1;
            }
            Err(e) => {
                result.results.push(CommandResult {
                    id: cmd.id.clone(),
                    command: format!("{} {}", cmd.command, cmd.args.join(" ")),
                    success: false,
                    status: CommandStatus::Failed,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: Some(duration),
                });
                result.failed += 1;

                if !cmd.optional {
                    result.success = false;
                    if options.stop_on_error {
                        stop = true;
                    }
                }
            }
        }
    }

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("batch-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "Batch: {} total, {} succeeded, {} failed, {} skipped",
            result.total, result.succeeded, result.failed, result.skipped
        );
        println!();

        for cmd_result in &result.results {
            let status_icon = match cmd_result.status {
                CommandStatus::Succeeded => "✓",
                CommandStatus::Failed => "✗",
                CommandStatus::Skipped => "○",
            };

            let id_str = cmd_result.id.as_ref().map(|id| format!("[{id}] ")).unwrap_or_default();
            println!("{status_icon} {id_str}{}", cmd_result.command);

            if let Some(err) = &cmd_result.error {
                println!("    Error: {err}");
            }

            if let Some(ms) = cmd_result.duration_ms {
                println!("    Duration: {ms}ms");
            }
        }

        println!();
        if result.success {
            println!("Batch completed successfully");
        } else {
            println!("Batch failed");
        }
    }

    if result.success {
        Ok(())
    } else {
        anyhow::bail!("Batch execution failed")
    }
}

fn execute_command(command: &str, args: &[String], dry_run: bool) -> Result<String> {
    if dry_run {
        return Ok(format!("[dry-run] Would execute: zjj {} {}", command, args.join(" ")));
    }

    // Build the command
    let output = std::process::Command::new("zjj")
        .arg(command)
        .args(args)
        .arg("--json") // Always use JSON for parsing
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("{stderr}"))
    }
}

/// Parse batch commands from JSON input
pub fn parse_batch_commands(input: &str) -> Result<Vec<BatchCommand>> {
    // Try parsing as JSON array
    if let Ok(commands) = serde_json::from_str::<Vec<BatchCommand>>(input) {
        return Ok(commands);
    }

    // Try parsing as newline-separated commands
    let commands: Vec<BatchCommand> = input
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .enumerate()
        .map(|(i, line)| {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }

            // Remove 'zjj' prefix if present
            let (cmd, args) = if parts[0] == "zjj" && parts.len() > 1 {
                (parts[1].to_string(), parts[2..].iter().map(|s| (*s).to_string()).collect())
            } else {
                (parts[0].to_string(), parts[1..].iter().map(|s| (*s).to_string()).collect())
            };

            Some(BatchCommand {
                command: cmd,
                args,
                id: Some(format!("cmd-{}", i + 1)),
                optional: false,
            })
        })
        .flatten()
        .collect();

    if commands.is_empty() {
        anyhow::bail!("No valid commands found in input");
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_batch_commands_json() {
        let input = r#"[
            {"command": "add", "args": ["test-1"]},
            {"command": "list", "args": []}
        ]"#;

        let commands = parse_batch_commands(input).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[1].command, "list");
    }

    #[test]
    fn test_parse_batch_commands_newline() {
        let input = "add test-1\nlist\nstatus test-1";

        let commands = parse_batch_commands(input).unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[0].args, vec!["test-1"]);
    }

    #[test]
    fn test_parse_batch_commands_with_zjj_prefix() {
        let input = "zjj add test-1\nzjj list";

        let commands = parse_batch_commands(input).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[1].command, "list");
    }

    #[test]
    fn test_parse_batch_commands_ignores_comments() {
        let input = "# This is a comment\nadd test-1\n# Another comment\nlist";

        let commands = parse_batch_commands(input).unwrap();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_batch_result_serialization() {
        let result = BatchResult {
            success: true,
            total: 2,
            succeeded: 2,
            failed: 0,
            skipped: 0,
            results: vec![],
            dry_run: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total\":2"));
    }

    #[test]
    fn test_command_status_serialization() {
        let succeeded = serde_json::to_string(&CommandStatus::Succeeded).unwrap();
        assert_eq!(succeeded, "\"succeeded\"");

        let failed = serde_json::to_string(&CommandStatus::Failed).unwrap();
        assert_eq!(failed, "\"failed\"");
    }
}

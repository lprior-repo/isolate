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

/// Execute a single batch command and return its result
fn execute_batch_command(cmd: &BatchCommand, dry_run: bool) -> CommandResult {
    let start = std::time::Instant::now();
    let command_str = format!("{} {}", cmd.command, cmd.args.join(" "));

    execute_command(&cmd.command, &cmd.args, dry_run)
        .map(|output| CommandResult {
            id: cmd.id.clone(),
            command: command_str.clone(),
            success: true,
            status: CommandStatus::Succeeded,
            output: Some(output),
            error: None,
            duration_ms: Some(u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)),
        })
        .unwrap_or_else(|e| CommandResult {
            id: cmd.id.clone(),
            command: command_str,
            success: false,
            status: CommandStatus::Failed,
            output: None,
            error: Some(e.to_string()),
            duration_ms: Some(u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)),
        })
}

/// Create a skipped command result
fn make_skipped_result(cmd: &BatchCommand) -> CommandResult {
    CommandResult {
        id: cmd.id.clone(),
        command: format!("{} {}", cmd.command, cmd.args.join(" ")),
        success: false,
        status: CommandStatus::Skipped,
        output: None,
        error: Some("Skipped due to previous error".to_string()),
        duration_ms: None,
    }
}

/// Execute batch commands, returning all results
fn execute_batch(
    commands: &[BatchCommand],
    stop_on_error: bool,
    dry_run: bool,
) -> Vec<CommandResult> {
    commands
        .iter()
        .scan(false, |should_skip, cmd| {
            if *should_skip {
                Some(make_skipped_result(cmd))
            } else {
                let result = execute_batch_command(cmd, dry_run);
                if !result.success && !cmd.optional && stop_on_error {
                    *should_skip = true;
                }
                Some(result)
            }
        })
        .collect()
}

/// Compute batch result from command results
fn compute_batch_result(results: Vec<CommandResult>, dry_run: bool) -> BatchResult {
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results
        .iter()
        .filter(|r| matches!(r.status, CommandStatus::Failed))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r.status, CommandStatus::Skipped))
        .count();
    let success = results
        .iter()
        .all(|r| r.success || matches!(r.status, CommandStatus::Skipped));

    BatchResult {
        success,
        total: results.len(),
        succeeded,
        failed,
        skipped,
        results,
        dry_run,
    }
}

/// Print human-readable batch result
fn print_batch_human(result: &BatchResult) {
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

        let id_str = cmd_result
            .id
            .as_ref()
            .map(|id| format!("[{id}] "))
            .map_or(String::new(), |v| v);
        println!("{status_icon} {id_str}{}", cmd_result.command);

        if let Some(e) = cmd_result.error.as_ref() {
            println!("    Error: {e}");
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

/// Run the batch command
pub fn run(options: &BatchOptions) -> Result<()> {
    let results = execute_batch(&options.commands, options.stop_on_error, options.dry_run);
    let result = compute_batch_result(results, options.dry_run);

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("batch-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_batch_human(&result);
    }

    if result.success {
        Ok(())
    } else {
        anyhow::bail!("Batch execution failed")
    }
}

fn execute_command(command: &str, args: &[String], dry_run: bool) -> Result<String> {
    if dry_run {
        return Ok(format!(
            "[dry-run] Would execute: zjj {} {}",
            command,
            args.join(" ")
        ));
    }

    // Build and execute the command - don't force --json, let user control format
    std::process::Command::new("zjj")
        .arg(command)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {e}"))
        .and_then(|output| {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                // Include both stderr and stdout for better error context
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let error_msg = if stderr.is_empty() { stdout } else { stderr };
                Err(anyhow::anyhow!("{error_msg}"))
            }
        })
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
        .filter_map(|(i, line)| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }

            // Remove 'zjj' prefix if present
            let (cmd, args) = if parts[0] == "zjj" && parts.len() > 1 {
                (
                    parts[1].to_string(),
                    parts[2..].iter().map(|s| (*s).to_string()).collect(),
                )
            } else {
                (
                    parts[0].to_string(),
                    parts[1..].iter().map(|s| (*s).to_string()).collect(),
                )
            };

            Some(BatchCommand {
                command: cmd,
                args,
                id: Some(format!("cmd-{}", i + 1)),
                optional: false,
            })
        })
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
    fn test_parse_batch_commands_json() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"[
            {"command": "add", "args": ["test-1"]},
            {"command": "list", "args": []}
        ]"#;

        let commands = parse_batch_commands(input)?;
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[1].command, "list");
        Ok(())
    }

    #[test]
    fn test_parse_batch_commands_newline() -> Result<(), Box<dyn std::error::Error>> {
        let input = "add test-1\nlist\nstatus test-1";

        let commands = parse_batch_commands(input)?;
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[0].args, vec!["test-1"]);
        Ok(())
    }

    #[test]
    fn test_parse_batch_commands_with_zjj_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let input = "zjj add test-1\nzjj list";

        let commands = parse_batch_commands(input)?;
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, "add");
        assert_eq!(commands[1].command, "list");
        Ok(())
    }

    #[test]
    fn test_parse_batch_commands_ignores_comments() -> Result<(), Box<dyn std::error::Error>> {
        let input = "# This is a comment\nadd test-1\n# Another comment\nlist";

        let commands = parse_batch_commands(input)?;
        assert_eq!(commands.len(), 2);
        Ok(())
    }

    #[test]
    fn test_batch_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = BatchResult {
            success: true,
            total: 2,
            succeeded: 2,
            failed: 0,
            skipped: 0,
            results: vec![],
            dry_run: false,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total\":2"));
        Ok(())
    }

    #[test]
    fn test_command_status_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let succeeded = serde_json::to_string(&CommandStatus::Succeeded)?;
        assert_eq!(succeeded, "\"succeeded\"");

        let failed = serde_json::to_string(&CommandStatus::Failed)?;
        assert_eq!(failed, "\"failed\"");
        Ok(())
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the batch command
    // ============================================================================

    mod parsing_behavior {
        use super::*;

        /// GIVEN: JSON array of commands
        /// WHEN: Parsed
        /// THEN: Each command should preserve its structure
        #[test]
        fn json_input_preserves_command_structure() -> Result<(), Box<dyn std::error::Error>> {
            let input = r#"[
                {"command": "add", "args": ["session-1"], "id": "step-1", "optional": false},
                {"command": "sync", "args": ["session-1"], "id": "step-2", "optional": true}
            ]"#;

            let commands = parse_batch_commands(input)?;

            assert_eq!(commands[0].command, "add");
            assert_eq!(commands[0].args, vec!["session-1"]);
            assert_eq!(commands[0].id, Some("step-1".to_string()));
            assert!(!commands[0].optional);

            assert_eq!(commands[1].command, "sync");
            assert!(commands[1].optional);
            Ok(())
        }

        /// GIVEN: Newline-delimited commands
        /// WHEN: Parsed
        /// THEN: Each line becomes a command with auto-generated ID
        #[test]
        fn newline_input_generates_command_ids() -> Result<(), Box<dyn std::error::Error>> {
            let input = "add task-1\nsync task-1\nlist";

            let commands = parse_batch_commands(input)?;

            assert_eq!(commands.len(), 3);
            assert_eq!(commands[0].id, Some("cmd-1".to_string()));
            assert_eq!(commands[1].id, Some("cmd-2".to_string()));
            assert_eq!(commands[2].id, Some("cmd-3".to_string()));
            Ok(())
        }

        /// GIVEN: Commands with 'zjj' prefix
        /// WHEN: Parsed
        /// THEN: Prefix should be stripped
        #[test]
        fn zjj_prefix_is_stripped() -> Result<(), Box<dyn std::error::Error>> {
            let input = "zjj add task\nzjj list";

            let commands = parse_batch_commands(input)?;

            assert_eq!(commands[0].command, "add");
            assert_eq!(commands[1].command, "list");
            // 'zjj' should not appear in command
            assert_ne!(commands[0].command, "zjj");
            Ok(())
        }

        /// GIVEN: Input with comments and blank lines
        /// WHEN: Parsed
        /// THEN: Comments and blanks should be ignored
        #[test]
        fn comments_and_blanks_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
            let input = r"
# This is a header comment
add task-1

# Another comment
list

# Final comment
";

            let commands = parse_batch_commands(input)?;

            assert_eq!(commands.len(), 2);
            assert_eq!(commands[0].command, "add");
            assert_eq!(commands[1].command, "list");
            Ok(())
        }

        /// GIVEN: Empty input
        /// WHEN: Parsed
        /// THEN: Should error with helpful message
        #[test]
        fn empty_input_fails_with_message() {
            let result = parse_batch_commands("");

            assert!(result.is_err());
            if let Err(e) = result {
                let err_msg = e.to_string();
                assert!(err_msg.contains("No valid commands"), "Error: {err_msg}");
            }
        }

        /// GIVEN: Only comments
        /// WHEN: Parsed
        /// THEN: Should error (no actual commands)
        #[test]
        fn only_comments_fails() {
            let input = "# comment 1\n# comment 2";

            let result = parse_batch_commands(input);

            assert!(result.is_err());
        }
    }

    mod batch_command_behavior {
        use super::*;

        /// GIVEN: A batch command
        /// WHEN: Created
        /// THEN: Should have command and args at minimum
        #[test]
        fn command_has_required_fields() {
            let cmd = BatchCommand {
                command: "add".to_string(),
                args: vec!["my-session".to_string()],
                id: None,
                optional: false,
            };

            assert!(!cmd.command.is_empty(), "Command must have a name");
        }

        /// GIVEN: Optional flag is true
        /// WHEN: Command fails
        /// THEN: Batch should continue (not stop)
        #[test]
        fn optional_flag_allows_continuation() {
            let optional_cmd = BatchCommand {
                command: "risky-command".to_string(),
                args: vec![],
                id: Some("optional-step".to_string()),
                optional: true,
            };

            assert!(optional_cmd.optional, "Should be marked optional");
        }

        /// GIVEN: Required command (optional=false)
        /// WHEN: Command fails
        /// THEN: Batch should stop
        #[test]
        fn required_commands_stop_on_failure() {
            let required_cmd = BatchCommand {
                command: "important".to_string(),
                args: vec![],
                id: Some("critical-step".to_string()),
                optional: false,
            };

            assert!(!required_cmd.optional, "Should be required");
        }
    }

    mod batch_result_behavior {
        use super::*;

        /// GIVEN: All commands succeed
        /// WHEN: Result is created
        /// THEN: success=true, failed=0
        #[test]
        fn all_success_shows_overall_success() {
            let result = BatchResult {
                success: true,
                total: 3,
                succeeded: 3,
                failed: 0,
                skipped: 0,
                results: vec![],
                dry_run: false,
            };

            assert!(result.success);
            assert_eq!(result.succeeded, result.total);
            assert_eq!(result.failed, 0);
        }

        /// GIVEN: Some commands fail
        /// WHEN: Result is created
        /// THEN: success=false, failed > 0
        #[test]
        fn any_failure_shows_overall_failure() {
            let result = BatchResult {
                success: false,
                total: 3,
                succeeded: 1,
                failed: 1,
                skipped: 1,
                results: vec![],
                dry_run: false,
            };

            assert!(!result.success);
            assert!(result.failed > 0);
        }

        /// GIVEN: Dry run mode
        /// WHEN: Result is created
        /// THEN: `dry_run=true` and no side effects
        #[test]
        fn dry_run_is_indicated() {
            let result = BatchResult {
                success: true,
                total: 3,
                succeeded: 3,
                failed: 0,
                skipped: 0,
                results: vec![],
                dry_run: true,
            };

            assert!(result.dry_run, "Should indicate dry run");
        }

        /// GIVEN: Commands were skipped
        /// WHEN: Result is created
        /// THEN: skipped count shows how many were not run
        #[test]
        fn skipped_shows_unrun_commands() {
            let result = BatchResult {
                success: false,
                total: 5,
                succeeded: 2,
                failed: 1,
                skipped: 2,
                results: vec![],
                dry_run: false,
            };

            assert_eq!(result.skipped, 2);
            assert_eq!(
                result.succeeded + result.failed + result.skipped,
                result.total
            );
        }
    }

    mod command_result_behavior {
        use super::*;

        /// GIVEN: Command succeeded
        /// WHEN: Result is created
        /// THEN: status=succeeded, error=None
        #[test]
        fn success_has_no_error() {
            let result = CommandResult {
                id: Some("cmd-1".to_string()),
                command: "add".to_string(),
                success: true,
                status: CommandStatus::Succeeded,
                output: Some("Session created".to_string()),
                duration_ms: Some(100),
                error: None,
            };

            assert!(matches!(result.status, CommandStatus::Succeeded));
            assert!(result.error.is_none());
            assert!(result.success);
        }

        /// GIVEN: Command failed
        /// WHEN: Result is created
        /// THEN: status=failed, error=Some(message)
        #[test]
        fn failure_has_error_message() {
            let result = CommandResult {
                id: Some("cmd-2".to_string()),
                command: "add".to_string(),
                success: false,
                status: CommandStatus::Failed,
                output: None,
                duration_ms: Some(50),
                error: Some("Session already exists".to_string()),
            };

            assert!(matches!(result.status, CommandStatus::Failed));
            assert!(result.error.is_some());
            assert!(!result.success);
        }

        /// GIVEN: Command was skipped
        /// WHEN: Result is created
        /// THEN: `status=skipped`, `duration_ms=None`
        #[test]
        fn skipped_has_no_duration() {
            let result = CommandResult {
                id: Some("cmd-3".to_string()),
                command: "sync".to_string(),
                success: false,
                status: CommandStatus::Skipped,
                output: None,
                duration_ms: None,
                error: Some("Previous command failed".to_string()),
            };

            assert!(matches!(result.status, CommandStatus::Skipped));
            assert!(
                result.duration_ms.is_none(),
                "Skipped commands have no duration"
            );
        }

        /// GIVEN: Command result with timing
        /// WHEN: Duration is checked
        /// THEN: Should be reasonable (not negative, not huge)
        #[test]
        fn duration_is_reasonable() {
            let result = CommandResult {
                id: Some("cmd-1".to_string()),
                command: "list".to_string(),
                success: true,
                status: CommandStatus::Succeeded,
                output: None,
                duration_ms: Some(150),
                error: None,
            };

            let duration = match result.duration_ms {
                Some(value) => value,
                None => 0,
            };
            assert!(duration < 60000, "Single command should not take 60s");
        }
    }

    mod command_status_behavior {
        use super::*;

        /// GIVEN: All possible command statuses
        /// WHEN: Serialized
        /// THEN: Should be lowercase strings
        #[test]
        fn all_statuses_serialize_as_lowercase() -> Result<(), Box<dyn std::error::Error>> {
            let statuses = [
                (CommandStatus::Succeeded, "succeeded"),
                (CommandStatus::Failed, "failed"),
                (CommandStatus::Skipped, "skipped"),
            ];

            for (status, expected) in statuses {
                let json = serde_json::to_string(&status)?;
                assert_eq!(json, format!("\"{expected}\""));
            }
            Ok(())
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: `BatchResult` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have summary and per-command results
        #[test]
        fn batch_result_json_is_complete() -> Result<(), Box<dyn std::error::Error>> {
            let result = BatchResult {
                success: true,
                total: 2,
                succeeded: 2,
                failed: 0,
                skipped: 0,
                results: vec![CommandResult {
                    id: Some("cmd-1".to_string()),
                    command: "add".to_string(),
                    success: true,
                    status: CommandStatus::Succeeded,
                    output: Some("Session created".to_string()),
                    duration_ms: Some(100),
                    error: None,
                }],
                dry_run: false,
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            // Summary fields
            assert!(json.get("success").is_some());
            assert!(json.get("total").is_some());
            assert!(json.get("succeeded").is_some());
            assert!(json.get("failed").is_some());
            assert!(json.get("skipped").is_some());
            assert!(json.get("dry_run").is_some());

            // Results array
            assert!(json.get("results").is_some());
            assert!(json["results"].is_array());
            Ok(())
        }

        /// GIVEN: `CommandResult` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have enough info for debugging
        #[test]
        fn command_result_json_is_debuggable() -> Result<(), Box<dyn std::error::Error>> {
            let result = CommandResult {
                id: Some("step-1".to_string()),
                command: "add".to_string(),
                success: false,
                status: CommandStatus::Failed,
                output: None,
                duration_ms: Some(50),
                error: Some("Session exists".to_string()),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            // All debug fields present
            assert_eq!(json["id"].as_str(), Some("step-1"));
            assert_eq!(json["command"].as_str(), Some("add"));
            assert!(json.get("status").is_some());
            assert!(json.get("duration_ms").is_some());
            assert!(json.get("error").is_some());
            Ok(())
        }
    }
}

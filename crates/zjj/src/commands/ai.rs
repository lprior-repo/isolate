//! AI command - AI-first entry point
//!
//! This command is the "start here" for AI agents.
//! Provides status, workflows, and quick-start guidance.

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::{context, get_session_db, zjj_data_dir};
use crate::cli::is_inside_zellij;

/// AI Status output
#[derive(Debug, Clone, Serialize)]
pub struct AiStatusOutput {
    /// Current location (main or workspace)
    pub location: String,
    /// Current workspace name if in one
    pub workspace: Option<String>,
    /// Agent ID if registered
    pub agent_id: Option<String>,
    /// Whether zjj is initialized
    pub initialized: bool,
    /// Number of active sessions
    pub active_sessions: usize,
    /// Ready for work?
    pub ready: bool,
    /// Suggested next action
    pub suggestion: String,
    /// Command to run
    pub next_command: String,
}

/// Workflow information
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowInfo {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

/// Workflow step
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStep {
    pub step: usize,
    pub command: String,
    pub description: String,
}

/// Options for ai command
#[derive(Debug, Clone)]
pub struct AiOptions {
    pub subcommand: AiSubcommand,
    pub format: OutputFormat,
}

/// AI subcommands
#[derive(Debug, Clone)]
pub enum AiSubcommand {
    /// Show AI-optimized status
    Status,
    /// Show the parallel agent workflow
    Workflow,
    /// Show quick-start guide
    QuickStart,
    /// Default: show overview
    Default,
}

/// Run the ai command
pub fn run(options: &AiOptions) -> Result<()> {
    match options.subcommand {
        AiSubcommand::Status => run_status(options.format),
        AiSubcommand::Workflow => run_workflow(options.format),
        AiSubcommand::QuickStart => run_quick_start(options.format),
        AiSubcommand::Default => run_default(options.format),
    }
}

/// Run AI status - comprehensive state check with guidance
fn run_status(format: OutputFormat) -> Result<()> {
    let initialized = zjj_data_dir().is_ok();
    let agent_id = std::env::var("ZJJ_AGENT_ID").ok();
    let inside_zellij = is_inside_zellij();

    let (location, workspace) = if let Ok(root) = super::check_in_jj_repo() {
        match context::detect_location(&root) {
            Ok(context::Location::Main) => ("main".to_string(), None),
            Ok(context::Location::Workspace { name, .. }) => {
                ("workspace".to_string(), Some(name))
            }
            Err(_) => ("unknown".to_string(), None),
        }
    } else {
        ("not_in_repo".to_string(), None)
    };

    let active_sessions = get_session_db()
        .ok()
        .and_then(|db| db.list_blocking(None).ok())
        .map(|sessions| {
            sessions
                .iter()
                .filter(|s| s.status.to_string() == "active")
                .count()
        })
        .unwrap_or(0);

    // Determine readiness and suggestion
    let (ready, suggestion, next_command) = if !initialized {
        (
            false,
            "zjj not initialized".to_string(),
            "zjj init".to_string(),
        )
    } else if location == "not_in_repo" {
        (
            false,
            "Not in a JJ repository".to_string(),
            "cd <repo> && zjj init".to_string(),
        )
    } else if location == "workspace" {
        (
            true,
            "In workspace - continue working or complete".to_string(),
            "zjj done".to_string(),
        )
    } else if !inside_zellij {
        (
            true,
            "Ready to work (outside Zellij)".to_string(),
            "zjj work <task-name> --no-zellij".to_string(),
        )
    } else {
        (
            true,
            "Ready to start work".to_string(),
            "zjj work <task-name>".to_string(),
        )
    };

    let output = AiStatusOutput {
        location,
        workspace,
        agent_id,
        initialized,
        active_sessions,
        ready,
        suggestion,
        next_command,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("ai-status-response", "single", &output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("AI Agent Status");
        println!("===============");
        println!();
        println!("Location:      {}", output.location);
        if let Some(ref ws) = output.workspace {
            println!("Workspace:     {}", ws);
        }
        if let Some(ref agent) = output.agent_id {
            println!("Agent ID:      {}", agent);
        } else {
            println!("Agent ID:      (not registered)");
        }
        println!("Initialized:   {}", if output.initialized { "yes" } else { "no" });
        println!("Active work:   {} sessions", output.active_sessions);
        println!();
        println!("Status: {}", if output.ready { "READY" } else { "NOT READY" });
        println!("Suggestion: {}", output.suggestion);
        println!();
        println!("Next command:");
        println!("  {}", output.next_command);
    }

    Ok(())
}

/// Run AI workflow - show the parallel agent workflow
fn run_workflow(format: OutputFormat) -> Result<()> {
    let workflow = WorkflowInfo {
        name: "Parallel Agent Workflow".to_string(),
        steps: vec![
            WorkflowStep {
                step: 1,
                command: "zjj whereami".to_string(),
                description: "Orient: Check current location".to_string(),
            },
            WorkflowStep {
                step: 2,
                command: "zjj agent register".to_string(),
                description: "Register: Identify yourself".to_string(),
            },
            WorkflowStep {
                step: 3,
                command: "zjj work <task-name> --idempotent".to_string(),
                description: "Isolate: Create workspace".to_string(),
            },
            WorkflowStep {
                step: 4,
                command: "cd $(zjj context --field location.path)".to_string(),
                description: "Enter: Navigate to workspace".to_string(),
            },
            WorkflowStep {
                step: 5,
                command: "# implement changes".to_string(),
                description: "Implement: Do the work".to_string(),
            },
            WorkflowStep {
                step: 6,
                command: "zjj agent heartbeat".to_string(),
                description: "Heartbeat: Signal liveness".to_string(),
            },
            WorkflowStep {
                step: 7,
                command: "zjj done".to_string(),
                description: "Complete: Merge and cleanup".to_string(),
            },
        ],
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("ai-workflow-response", "single", &workflow);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("Parallel Agent Workflow");
        println!("=======================");
        println!();
        for step in &workflow.steps {
            println!("{}. {} ", step.step, step.description);
            println!("   $ {}", step.command);
            println!();
        }
        println!("Abandon work: zjj abort");
        println!("Check status: zjj ai status");
    }

    Ok(())
}

/// Run AI quick-start - minimum commands to be productive
fn run_quick_start(format: OutputFormat) -> Result<()> {
    #[derive(Serialize)]
    struct QuickStartOutput {
        essential_commands: Vec<QuickCommand>,
        orientation: Vec<QuickCommand>,
        workflow: Vec<QuickCommand>,
    }

    #[derive(Serialize)]
    struct QuickCommand {
        command: String,
        purpose: String,
    }

    let output = QuickStartOutput {
        essential_commands: vec![
            QuickCommand {
                command: "zjj whereami".to_string(),
                purpose: "Returns 'main' or 'workspace:<name>'".to_string(),
            },
            QuickCommand {
                command: "zjj work <name>".to_string(),
                purpose: "Create workspace and start working".to_string(),
            },
            QuickCommand {
                command: "zjj done".to_string(),
                purpose: "Complete work and merge".to_string(),
            },
            QuickCommand {
                command: "zjj abort".to_string(),
                purpose: "Abandon work without merging".to_string(),
            },
        ],
        orientation: vec![
            QuickCommand {
                command: "zjj whereami".to_string(),
                purpose: "Location check".to_string(),
            },
            QuickCommand {
                command: "zjj whoami".to_string(),
                purpose: "Agent identity".to_string(),
            },
            QuickCommand {
                command: "zjj ai status".to_string(),
                purpose: "Full status with guidance".to_string(),
            },
        ],
        workflow: vec![
            QuickCommand {
                command: "zjj work task-name --idempotent".to_string(),
                purpose: "Safe to retry".to_string(),
            },
            QuickCommand {
                command: "zjj done".to_string(),
                purpose: "Merge when done".to_string(),
            },
        ],
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("ai-quickstart-response", "single", &output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("AI Quick Start");
        println!("==============");
        println!();
        println!("ESSENTIAL COMMANDS:");
        for cmd in &output.essential_commands {
            println!("  {:30} {}", cmd.command, cmd.purpose);
        }
        println!();
        println!("MINIMAL WORKFLOW:");
        println!("  1. zjj work my-task      # Create workspace");
        println!("  2. # ... do work ...");
        println!("  3. zjj done              # Merge and cleanup");
        println!();
        println!("SAFE FLAGS:");
        println!("  --idempotent   Safe for retries");
        println!("  --dry-run      Preview without executing");
        println!("  --json         Machine-readable output");
    }

    Ok(())
}

/// Run AI default - overview and help
fn run_default(format: OutputFormat) -> Result<()> {
    #[derive(Serialize)]
    struct AiOverview {
        message: String,
        subcommands: Vec<SubcommandInfo>,
        quick_commands: Vec<String>,
    }

    #[derive(Serialize)]
    struct SubcommandInfo {
        command: String,
        description: String,
    }

    let output = AiOverview {
        message: "ZJJ AI Agent Interface - Start here for AI-driven workflows".to_string(),
        subcommands: vec![
            SubcommandInfo {
                command: "zjj ai status".to_string(),
                description: "Current state with guided next action".to_string(),
            },
            SubcommandInfo {
                command: "zjj ai workflow".to_string(),
                description: "7-step parallel agent workflow".to_string(),
            },
            SubcommandInfo {
                command: "zjj ai quick-start".to_string(),
                description: "Minimum commands to be productive".to_string(),
            },
        ],
        quick_commands: vec![
            "zjj whereami          # Location".to_string(),
            "zjj work <name>       # Start work".to_string(),
            "zjj done              # Finish work".to_string(),
        ],
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("ai-overview-response", "single", &output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("{}", output.message);
        println!();
        println!("SUBCOMMANDS:");
        for sub in &output.subcommands {
            println!("  {:30} {}", sub.command, sub.description);
        }
        println!();
        println!("QUICK COMMANDS:");
        for cmd in &output.quick_commands {
            println!("  {}", cmd);
        }
        println!();
        println!("Run 'zjj ai status' to get started.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_status_output_serializes() {
        let output = AiStatusOutput {
            location: "main".to_string(),
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready to work".to_string(),
            next_command: "zjj work <task>".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }

    #[test]
    fn test_workflow_info_serializes() {
        let workflow = WorkflowInfo {
            name: "Test".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "test".to_string(),
                description: "Test step".to_string(),
            }],
        };

        let json = serde_json::to_string(&workflow);
        assert!(json.is_ok());
    }
}

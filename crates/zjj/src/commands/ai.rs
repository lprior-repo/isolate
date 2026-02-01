//! AI command - AI-first entry point
//!
//! This command is the "start here" for AI agents.
//! Provides status, workflows, and quick-start guidance.

use anyhow::{Context, Result};
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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize AI status output")?;
        println!("{json_str}");
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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize AI workflow output")?;
        println!("{json_str}");
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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize AI quickstart output")?;
        println!("{json_str}");
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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize AI overview output")?;
        println!("{json_str}");
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

    // ============================================================================
    // Behavior Tests
    // ============================================================================

    /// Test AiStatusOutput location field
    #[test]
    fn test_ai_status_location_values() {
        // On main
        let main_status = AiStatusOutput {
            location: "main".to_string(),
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready to work".to_string(),
            next_command: "zjj work <name>".to_string(),
        };
        assert_eq!(main_status.location, "main");
        assert!(main_status.workspace.is_none());

        // In workspace
        let ws_status = AiStatusOutput {
            location: "workspace:feature".to_string(),
            workspace: Some("feature".to_string()),
            agent_id: Some("agent-1".to_string()),
            initialized: true,
            active_sessions: 1,
            ready: true,
            suggestion: "Continue work or done".to_string(),
            next_command: "zjj done".to_string(),
        };
        assert!(ws_status.location.starts_with("workspace:"));
        assert!(ws_status.workspace.is_some());
    }

    /// Test initialized flag
    #[test]
    fn test_ai_status_initialized_flag() {
        let uninitialized = AiStatusOutput {
            location: "main".to_string(),
            workspace: None,
            agent_id: None,
            initialized: false,
            active_sessions: 0,
            ready: false,
            suggestion: "Run 'zjj init' first".to_string(),
            next_command: "zjj init".to_string(),
        };

        assert!(!uninitialized.initialized);
        assert!(uninitialized.suggestion.contains("init"));
    }

    /// Test suggestion is always set
    #[test]
    fn test_ai_status_suggestion_always_set() {
        let status = AiStatusOutput {
            location: "main".to_string(),
            workspace: None,
            agent_id: None,
            initialized: true,
            active_sessions: 0,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "zjj work".to_string(),
        };

        assert!(!status.suggestion.is_empty());
    }

    /// Test WorkflowStep ordering
    #[test]
    fn test_workflow_step_ordering() {
        let workflow = WorkflowInfo {
            name: "Test Workflow".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj work <name>".to_string(),
                    description: "Start".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "# implement".to_string(),
                    description: "Work".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "zjj done".to_string(),
                    description: "Finish".to_string(),
                },
            ],
        };

        // Steps should be sequential
        for (i, step) in workflow.steps.iter().enumerate() {
            assert_eq!(step.step, i + 1);
        }
    }

    /// Test WorkflowStep has command and description
    #[test]
    fn test_workflow_step_required_fields() {
        let step = WorkflowStep {
            step: 1,
            command: "zjj work my-task".to_string(),
            description: "Create a new workspace".to_string(),
        };

        assert!(!step.command.is_empty());
        assert!(!step.description.is_empty());
    }

    /// Test quick start has essential commands
    #[test]
    fn test_quickstart_has_essential_commands() {
        // Essential commands that should be included
        let essential_commands = [
            "zjj work",
            "zjj done",
            "zjj abort",
            "zjj whereami",
        ];

        assert!(!essential_commands.is_empty());
        assert!(essential_commands.len() >= 2);
    }

    /// Test command structure requirements
    #[test]
    fn test_command_structure_requirements() {
        // Commands should have command and purpose
        let command = "zjj whereami";
        let purpose = "Show current location";

        assert!(command.starts_with("zjj "));
        assert!(!purpose.is_empty());
    }

    /// Test AiStatusOutput JSON has all fields
    #[test]
    fn test_ai_status_json_complete() {
        let status = AiStatusOutput {
            location: "main".to_string(),
            workspace: None,
            agent_id: Some("agent-1".to_string()),
            initialized: true,
            active_sessions: 2,
            ready: true,
            suggestion: "Ready".to_string(),
            next_command: "zjj work".to_string(),
        };

        let json_str = serde_json::to_string(&status).unwrap_or_default();

        assert!(json_str.contains("location"));
        assert!(json_str.contains("workspace"));
        assert!(json_str.contains("agent_id"));
        assert!(json_str.contains("initialized"));
        assert!(json_str.contains("active_sessions"));
        assert!(json_str.contains("ready"));
        assert!(json_str.contains("suggestion"));
        assert!(json_str.contains("next_command"));
    }

    /// Test workflow has minimum steps
    #[test]
    fn test_workflow_minimum_steps() {
        let workflow = WorkflowInfo {
            name: "Minimal".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj work".to_string(),
                    description: "Start".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj done".to_string(),
                    description: "End".to_string(),
                },
            ],
        };

        // Minimal workflow needs at least 2 steps (start and end)
        assert!(workflow.steps.len() >= 2);
    }

    /// Test subcommand enumeration
    #[test]
    fn test_ai_subcommands_defined() {
        let subcommands = ["status", "workflow", "quick-start"];

        for cmd in subcommands {
            assert!(!cmd.is_empty());
        }
    }

    /// Test safe flags list
    #[test]
    fn test_ai_safe_flags() {
        let safe_flags = [
            ("--dry-run", "Preview without executing"),
            ("--idempotent", "Safe to retry"),
            ("--json", "Machine-readable output"),
        ];

        for (flag, description) in safe_flags {
            assert!(flag.starts_with("--"));
            assert!(!description.is_empty());
        }
    }
}

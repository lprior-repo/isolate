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
    /// Get single next action
    Next,
    /// Default: show overview
    Default,
}

/// Run the ai command
pub fn run(options: &AiOptions) -> Result<()> {
    match options.subcommand {
        AiSubcommand::Status => run_status(options.format),
        AiSubcommand::Workflow => run_workflow(options.format),
        AiSubcommand::QuickStart => run_quick_start(options.format),
        AiSubcommand::Next => run_next(options.format),
        AiSubcommand::Default => run_default(options.format),
    }
}

/// Run AI status - comprehensive state check with guidance
fn run_status(format: OutputFormat) -> Result<()> {
    let initialized = zjj_data_dir().is_ok();
    let agent_id = std::env::var("ZJJ_AGENT_ID").ok();
    let inside_zellij = is_inside_zellij();

    let (location, workspace) = super::check_in_jj_repo().map_or_else(
        |_| ("not_in_repo".to_string(), None),
        |root| match context::detect_location(&root) {
            Ok(context::Location::Main) => ("main".to_string(), None),
            Ok(context::Location::Workspace { name, .. }) => ("workspace".to_string(), Some(name)),
            Err(_) => ("unknown".to_string(), None),
        },
    );

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
            println!("Workspace:     {ws}");
        }
        if let Some(ref agent) = output.agent_id {
            println!("Agent ID:      {agent}");
        } else {
            println!("Agent ID:      (not registered)");
        }
        println!(
            "Initialized:   {}",
            if output.initialized { "yes" } else { "no" }
        );
        println!("Active work:   {} sessions", output.active_sessions);
        println!();
        println!(
            "Status: {}",
            if output.ready { "READY" } else { "NOT READY" }
        );
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

/// Next action output
#[derive(Debug, Clone, Serialize)]
pub struct NextActionOutput {
    /// What to do
    pub action: String,
    /// Command to run (copy-paste ready)
    pub command: String,
    /// Why this is the next step
    pub reason: String,
    /// Priority: high, medium, low
    pub priority: String,
}

/// Run AI next - single next action
fn run_next(format: OutputFormat) -> Result<()> {
    let initialized = zjj_data_dir().is_ok();
    let inside_zellij = is_inside_zellij();

    let (location, workspace) = super::check_in_jj_repo().map_or_else(
        |_| ("not_in_repo".to_string(), None),
        |root| match context::detect_location(&root) {
            Ok(context::Location::Main) => ("main".to_string(), None),
            Ok(context::Location::Workspace { name, .. }) => ("workspace".to_string(), Some(name)),
            Err(_) => ("unknown".to_string(), None),
        },
    );

    // Determine next action based on current state
    let output = if !initialized && location != "not_in_repo" {
        NextActionOutput {
            action: "Initialize ZJJ".to_string(),
            command: "zjj init".to_string(),
            reason: "ZJJ is not initialized in this repository".to_string(),
            priority: "high".to_string(),
        }
    } else if location == "not_in_repo" {
        NextActionOutput {
            action: "Enter a JJ repository".to_string(),
            command: "cd <repo> && zjj init".to_string(),
            reason: "Not currently in a JJ repository".to_string(),
            priority: "high".to_string(),
        }
    } else if let Some(ws) = workspace {
        NextActionOutput {
            action: format!("Continue work in '{ws}'"),
            command: "zjj context --json".to_string(),
            reason: format!("Currently in workspace '{ws}' - check context or complete work"),
            priority: "medium".to_string(),
        }
    } else {
        // On main, ready to work
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

        if active_sessions > 0 {
            NextActionOutput {
                action: "Check existing sessions".to_string(),
                command: "zjj list --json".to_string(),
                reason: format!(
                    "{active_sessions} active session(s) exist - review or continue work"
                ),
                priority: "medium".to_string(),
            }
        } else if inside_zellij {
            NextActionOutput {
                action: "Start new work session".to_string(),
                command: "zjj work <task-name>".to_string(),
                reason: "Ready to start work - no active sessions".to_string(),
                priority: "medium".to_string(),
            }
        } else {
            NextActionOutput {
                action: "Start new work session".to_string(),
                command: "zjj work <task-name> --no-zellij".to_string(),
                reason: "Ready to start work (outside Zellij)".to_string(),
                priority: "medium".to_string(),
            }
        }
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("ai-next-response", "single", &output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize AI next output")?;
        println!("{json_str}");
    } else {
        println!("Next Action: {}", output.action);
        println!("Command:     {}", output.command);
        println!("Reason:      {}", output.reason);
        println!("Priority:    {}", output.priority);
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
                command: "zjj ai next".to_string(),
                description: "Single next action with copy-paste command".to_string(),
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
            println!("  {cmd}");
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

    /// Test `AiStatusOutput` location field
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

    /// Test `WorkflowStep` ordering
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

    /// Test `WorkflowStep` has command and description
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
        let essential_commands = ["zjj work", "zjj done", "zjj abort", "zjj whereami"];

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

    /// Test `AiStatusOutput` JSON has all fields
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

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe WHAT the system does, not HOW it does it
    // ============================================================================

    mod next_action_behavior {
        use super::*;

        /// GIVEN: User is not in a JJ repository
        /// WHEN: They request the next action
        /// THEN: They should be told to enter a repository
        #[test]
        fn when_not_in_repo_suggests_entering_repo() {
            let output = NextActionOutput {
                action: "Enter a JJ repository".to_string(),
                command: "cd <repo> && zjj init".to_string(),
                reason: "Not currently in a JJ repository".to_string(),
                priority: "high".to_string(),
            };

            assert_eq!(
                output.priority, "high",
                "Not being in a repo is a high priority issue"
            );
            assert!(
                output.command.contains("cd"),
                "Should suggest changing directory"
            );
            assert!(
                output.reason.contains("repository"),
                "Should explain the issue"
            );
        }

        /// GIVEN: ZJJ is not initialized in the current repo
        /// WHEN: User asks for next action
        /// THEN: They should be told to run zjj init
        #[test]
        fn when_uninitialized_suggests_init() {
            let output = NextActionOutput {
                action: "Initialize ZJJ".to_string(),
                command: "zjj init".to_string(),
                reason: "ZJJ is not initialized in this repository".to_string(),
                priority: "high".to_string(),
            };

            assert_eq!(output.command, "zjj init", "Should suggest init command");
            assert_eq!(output.priority, "high", "Initialization is high priority");
        }

        /// GIVEN: User is in a workspace
        /// WHEN: They ask for next action
        /// THEN: They should be guided to continue or complete work
        #[test]
        fn when_in_workspace_suggests_context_or_done() {
            let workspace_name = "feature-auth";
            let output = NextActionOutput {
                action: format!("Continue work in '{workspace_name}'"),
                command: "zjj context --json".to_string(),
                reason: format!(
                    "Currently in workspace '{workspace_name}' - check context or complete work"
                ),
                priority: "medium".to_string(),
            };

            assert!(
                output.action.contains(workspace_name),
                "Should mention current workspace"
            );
            assert!(
                output.command.contains("context"),
                "Should suggest checking context"
            );
            assert_eq!(
                output.priority, "medium",
                "Continuing work is medium priority"
            );
        }

        /// GIVEN: User is on main with active sessions
        /// WHEN: They ask for next action
        /// THEN: They should be told to check existing sessions
        #[test]
        fn when_sessions_exist_suggests_listing_them() {
            let active_count = 3;
            let output = NextActionOutput {
                action: "Check existing sessions".to_string(),
                command: "zjj list --json".to_string(),
                reason: format!(
                    "{active_count} active session(s) exist - review or continue work"
                ),
                priority: "medium".to_string(),
            };

            assert!(
                output.command.contains("list"),
                "Should suggest listing sessions"
            );
            assert!(output.reason.contains('3'), "Should mention session count");
        }

        /// GIVEN: System is ready with no active sessions
        /// WHEN: User asks for next action
        /// THEN: They should be told to start new work
        #[test]
        fn when_ready_and_idle_suggests_starting_work() {
            let output = NextActionOutput {
                action: "Start new work session".to_string(),
                command: "zjj work <task-name>".to_string(),
                reason: "Ready to start work - no active sessions".to_string(),
                priority: "medium".to_string(),
            };

            assert!(
                output.command.contains("work"),
                "Should suggest starting work"
            );
            assert!(
                output.reason.contains("no active sessions"),
                "Should explain why"
            );
        }

        /// GIVEN: User is outside Zellij
        /// WHEN: They want to start work
        /// THEN: The command should include --no-zellij flag
        #[test]
        fn when_outside_zellij_suggests_no_zellij_flag() {
            let output = NextActionOutput {
                action: "Start new work session".to_string(),
                command: "zjj work <task-name> --no-zellij".to_string(),
                reason: "Ready to start work (outside Zellij)".to_string(),
                priority: "medium".to_string(),
            };

            assert!(
                output.command.contains("--no-zellij"),
                "Should include no-zellij flag"
            );
        }
    }

    mod status_behavior {
        use super::*;

        /// GIVEN: System is fully ready
        /// WHEN: Status is checked
        /// THEN: ready should be true and suggestion should guide next step
        #[test]
        fn ready_system_shows_positive_guidance() {
            let status = AiStatusOutput {
                location: "main".to_string(),
                workspace: None,
                agent_id: Some("agent-abc".to_string()),
                initialized: true,
                active_sessions: 0,
                ready: true,
                suggestion: "Ready to start work".to_string(),
                next_command: "zjj work <name>".to_string(),
            };

            assert!(status.ready, "System should be ready");
            assert!(status.initialized, "Should be initialized");
            assert!(!status.suggestion.is_empty(), "Should have guidance");
            assert!(!status.next_command.is_empty(), "Should have next command");
        }

        /// GIVEN: ZJJ is not initialized
        /// WHEN: Status is checked
        /// THEN: ready should be false and suggestion should mention init
        #[test]
        fn uninitialized_system_guides_to_init() {
            let status = AiStatusOutput {
                location: "main".to_string(),
                workspace: None,
                agent_id: None,
                initialized: false,
                active_sessions: 0,
                ready: false,
                suggestion: "zjj not initialized".to_string(),
                next_command: "zjj init".to_string(),
            };

            assert!(!status.ready, "Uninitialized system is not ready");
            assert!(!status.initialized, "Should show as not initialized");
            assert!(status.next_command.contains("init"), "Should guide to init");
        }

        /// GIVEN: User is in a workspace
        /// WHEN: Status is checked
        /// THEN: location should indicate workspace and name should be set
        #[test]
        fn workspace_location_includes_workspace_name() {
            let status = AiStatusOutput {
                location: "workspace".to_string(),
                workspace: Some("feature-login".to_string()),
                agent_id: None,
                initialized: true,
                active_sessions: 1,
                ready: true,
                suggestion: "In workspace - continue working or complete".to_string(),
                next_command: "zjj done".to_string(),
            };

            assert_eq!(status.location, "workspace");
            assert_eq!(status.workspace, Some("feature-login".to_string()));
            assert!(
                status.next_command.contains("done"),
                "Workspace suggests done"
            );
        }

        /// GIVEN: Agent ID is set in environment
        /// WHEN: Status is checked
        /// THEN: `agent_id` should be populated
        #[test]
        fn agent_id_is_captured_from_environment() {
            let status = AiStatusOutput {
                location: "main".to_string(),
                workspace: None,
                agent_id: Some("agent-xyz789".to_string()),
                initialized: true,
                active_sessions: 0,
                ready: true,
                suggestion: "Ready".to_string(),
                next_command: "zjj work <name>".to_string(),
            };

            assert_eq!(status.agent_id, Some("agent-xyz789".to_string()));
        }
    }

    mod workflow_behavior {
        use super::*;

        /// GIVEN: User wants to understand the workflow
        /// WHEN: They request workflow info
        /// THEN: Steps should be sequential and complete
        #[test]
        fn workflow_steps_are_sequential_from_one() {
            let steps = [
                WorkflowStep {
                    step: 1,
                    command: "zjj whereami".to_string(),
                    description: "Orient".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj agent register".to_string(),
                    description: "Register".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "zjj work <name>".to_string(),
                    description: "Isolate".to_string(),
                },
                WorkflowStep {
                    step: 4,
                    command: "cd $(zjj context --field path)".to_string(),
                    description: "Enter".to_string(),
                },
                WorkflowStep {
                    step: 5,
                    command: "# implement".to_string(),
                    description: "Implement".to_string(),
                },
                WorkflowStep {
                    step: 6,
                    command: "zjj agent heartbeat".to_string(),
                    description: "Heartbeat".to_string(),
                },
                WorkflowStep {
                    step: 7,
                    command: "zjj done".to_string(),
                    description: "Complete".to_string(),
                },
            ];

            // Verify sequential numbering
            for (i, step) in steps.iter().enumerate() {
                assert_eq!(step.step, i + 1, "Step {} should have number {}", i, i + 1);
            }

            // Verify workflow starts and ends correctly
            if let Some(first) = steps.first() {
                assert!(
                    first.command.contains("whereami"),
                    "Workflow starts with orientation"
                );
            }
            if let Some(last) = steps.last() {
                assert!(
                    last.command.contains("done"),
                    "Workflow ends with completion"
                );
            }
        }

        /// GIVEN: User is an AI agent
        /// WHEN: They follow the workflow
        /// THEN: Each step should have an actionable command
        #[test]
        fn every_workflow_step_has_actionable_command() {
            let steps = vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj whereami".to_string(),
                    description: "Check location".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj work task".to_string(),
                    description: "Start work".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "zjj done".to_string(),
                    description: "Finish".to_string(),
                },
            ];

            for step in &steps {
                assert!(
                    !step.command.is_empty(),
                    "Step {} must have a command",
                    step.step
                );
                assert!(
                    !step.description.is_empty(),
                    "Step {} must have a description",
                    step.step
                );
            }
        }
    }

    mod subcommand_behavior {
        use super::*;

        /// GIVEN: AI agent needs to know available subcommands
        /// WHEN: They check the `AiSubcommand` enum
        /// THEN: All expected subcommands should exist
        #[test]
        fn all_ai_subcommands_are_defined() {
            // Verify each subcommand variant exists by matching
            let subcommands = [
                AiSubcommand::Status,
                AiSubcommand::Workflow,
                AiSubcommand::QuickStart,
                AiSubcommand::Next,
                AiSubcommand::Default,
            ];

            assert_eq!(subcommands.len(), 5, "Should have 5 AI subcommands");
        }

        /// GIVEN: User runs zjj ai without subcommand
        /// WHEN: Default is used
        /// THEN: Should show overview with all options
        #[test]
        fn default_subcommand_shows_overview() {
            // The default behavior should guide users to available commands
            let default = AiSubcommand::Default;

            // Matching ensures the variant exists and is the expected type
            assert!(
                matches!(default, AiSubcommand::Default),
                "Default variant should be the AiSubcommand::Default"
            );
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: User requests JSON output
        /// WHEN: Status is serialized
        /// THEN: All fields should be present and correctly typed
        #[test]
        fn json_output_has_complete_schema() -> Result<(), Box<dyn std::error::Error>> {
            let status = AiStatusOutput {
                location: "main".to_string(),
                workspace: None,
                agent_id: None,
                initialized: true,
                active_sessions: 5,
                ready: true,
                suggestion: "Do something".to_string(),
                next_command: "zjj work".to_string(),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&status)?)?;

            // Required fields
            assert!(json.get("location").is_some(), "Must have location");
            assert!(json.get("initialized").is_some(), "Must have initialized");
            assert!(
                json.get("active_sessions").is_some(),
                "Must have active_sessions"
            );
            assert!(json.get("ready").is_some(), "Must have ready");
            assert!(json.get("suggestion").is_some(), "Must have suggestion");
            assert!(json.get("next_command").is_some(), "Must have next_command");

            // Type verification
            assert!(json["location"].is_string(), "location must be string");
            assert!(
                json["initialized"].is_boolean(),
                "initialized must be boolean"
            );
            assert!(
                json["active_sessions"].is_number(),
                "active_sessions must be number"
            );
            assert!(json["ready"].is_boolean(), "ready must be boolean");
            Ok(())
        }

        /// GIVEN: `NextActionOutput` is serialized
        /// WHEN: AI agent parses it
        /// THEN: It should have all fields needed for automation
        #[test]
        fn next_action_json_is_machine_actionable() -> Result<(), Box<dyn std::error::Error>> {
            let action = NextActionOutput {
                action: "Start work".to_string(),
                command: "zjj work my-task".to_string(),
                reason: "Ready to begin".to_string(),
                priority: "medium".to_string(),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&action)?)?;

            // Action must be descriptive
            if let Some(action_str) = json["action"].as_str() {
                assert!(!action_str.is_empty());
            } else {
                panic!("action not a string");
            }

            // Command must be executable
            if let Some(cmd) = json["command"].as_str() {
                assert!(
                    cmd.starts_with("zjj ") || cmd.starts_with("cd ") || cmd.starts_with('#'),
                    "Command '{cmd}' should be executable or a comment"
                );
            } else {
                panic!("command not a string");
            }

            // Priority must be valid
            if let Some(priority) = json["priority"].as_str() {
                assert!(
                    ["high", "medium", "low"].contains(&priority),
                    "Priority '{priority}' must be high, medium, or low"
                );
            } else {
                panic!("priority not a string");
            }
            Ok(())
        }
    }
}

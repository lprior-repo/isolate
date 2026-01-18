//! Essentials command - curated subset of commands for humans (zjj-r1fk)
//!
//! This command provides a focused, human-friendly overview of the most
//! important commands for daily use, complementing the AI-focused commands
//! like context, introspect, and the full help system.

#![allow(dead_code)]

use anyhow::Result;
use serde::Serialize;

/// Essential command information
#[derive(Debug, Serialize)]
pub struct EssentialCommand {
    /// Command name
    pub name: String,
    /// Brief one-line description
    pub description: String,
    /// Simple usage example
    pub example: String,
}

/// Output structure for essentials
#[derive(Debug, Serialize)]
pub struct EssentialsOutput {
    pub success: bool,
    pub commands: Vec<EssentialCommand>,
}

/// Run the essentials command
pub async fn run(json: bool) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    let commands = get_essential_commands();
    let output = EssentialsOutput {
        success: true,
        commands,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human_readable(&output.commands);
    }

    Ok(())
}

/// Get the list of essential commands
fn get_essential_commands() -> Vec<EssentialCommand> {
    vec![
        EssentialCommand {
            name: "zjj init".to_string(),
            description: "Initialize zjj in a repository (first-time setup)".to_string(),
            example: "zjj init".to_string(),
        },
        EssentialCommand {
            name: "zjj add".to_string(),
            description: "Create a new session with workspace and Zellij tab".to_string(),
            example: "zjj add feature-auth".to_string(),
        },
        EssentialCommand {
            name: "zjj list".to_string(),
            description: "Show all active sessions".to_string(),
            example: "zjj list".to_string(),
        },
        EssentialCommand {
            name: "zjj focus".to_string(),
            description: "Switch to a session's Zellij tab".to_string(),
            example: "zjj focus feature-auth".to_string(),
        },
        EssentialCommand {
            name: "zjj status".to_string(),
            description: "Show detailed session information".to_string(),
            example: "zjj status".to_string(),
        },
        EssentialCommand {
            name: "zjj sync".to_string(),
            description: "Rebase current session on main branch".to_string(),
            example: "zjj sync".to_string(),
        },
        EssentialCommand {
            name: "zjj diff".to_string(),
            description: "Show changes between session and main".to_string(),
            example: "zjj diff feature-auth".to_string(),
        },
        EssentialCommand {
            name: "zjj remove".to_string(),
            description: "Clean up a session when done".to_string(),
            example: "zjj remove feature-auth".to_string(),
        },
        EssentialCommand {
            name: "zjj dashboard".to_string(),
            description: "Interactive TUI for managing sessions".to_string(),
            example: "zjj dashboard".to_string(),
        },
        EssentialCommand {
            name: "zjj doctor".to_string(),
            description: "Check system health and dependencies".to_string(),
            example: "zjj doctor".to_string(),
        },
    ]
}

/// Print human-readable essentials
fn print_human_readable(_commands: &[EssentialCommand]) {
    println!("Essential zjj Commands");
    println!("======================");
    println!();
    println!("These are the core commands you'll use every day.\n");
    println!("Getting Started:");
    println!("  zjj init              Initialize zjj (first-time setup)");
    println!();
    println!("Session Management:");
    println!("  zjj add <name>        Create new session");
    println!("  zjj list              Show all sessions");
    println!("  zjj focus <name>      Switch to session");
    println!("  zjj remove <name>     Clean up session");
    println!();
    println!("Working in Sessions:");
    println!("  zjj status            Check current session");
    println!("  zjj sync              Rebase on main");
    println!("  zjj diff <name>       See your changes");
    println!();
    println!("Tools:");
    println!("  zjj dashboard         Interactive dashboard");
    println!("  zjj doctor            System health check");
    println!();
    println!("Typical Workflow:");
    println!("  1. zjj add feature-x     # Create session");
    println!("  2. [work in session]     # Make changes");
    println!("  3. zjj sync              # Keep up to date");
    println!("  4. zjj remove feature-x  # Clean up");
    println!();
    println!("For detailed help: zjj <command> --help");
    println!("For AI agents: zjj context --json");
    println!("For all commands: zjj --help");
}

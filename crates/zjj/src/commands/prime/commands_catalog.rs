//! Command catalog and beads integration status
//!
//! This module manages command reference data and checks
//! beads integration availability.

use crate::cli::is_command_available;

use super::output_types::{BeadsStatus, CommandCategories, CommandRef};

/// Build command categories for output
///
/// Returns a complete set of command references organized by category
/// for use in prime context output.
pub fn build_command_categories() -> CommandCategories {
    CommandCategories {
        session_lifecycle: vec![
            CommandRef {
                name: "zjj add <name>".to_string(),
                description: "Create new session (workspace + Zellij tab)".to_string(),
            },
            CommandRef {
                name: "zjj list".to_string(),
                description: "Show all sessions".to_string(),
            },
            CommandRef {
                name: "zjj status <name>".to_string(),
                description: "Show session details".to_string(),
            },
            CommandRef {
                name: "zjj focus <name>".to_string(),
                description: "Switch to session's Zellij tab".to_string(),
            },
            CommandRef {
                name: "zjj remove <name>".to_string(),
                description: "Cleanup session and workspace".to_string(),
            },
        ],
        workspace_sync: vec![
            CommandRef {
                name: "zjj sync".to_string(),
                description: "Rebase current workspace on main".to_string(),
            },
            CommandRef {
                name: "zjj diff".to_string(),
                description: "Show changes in current workspace".to_string(),
            },
        ],
        system: vec![
            CommandRef {
                name: "zjj init".to_string(),
                description: "Initialize zjj in a JJ repository".to_string(),
            },
            CommandRef {
                name: "zjj config".to_string(),
                description: "View or modify configuration".to_string(),
            },
            CommandRef {
                name: "zjj doctor".to_string(),
                description: "Run health checks".to_string(),
            },
        ],
        introspection: vec![
            CommandRef {
                name: "zjj context --json".to_string(),
                description: "Get complete environment state".to_string(),
            },
            CommandRef {
                name: "zjj introspect --json".to_string(),
                description: "Get CLI metadata and command docs".to_string(),
            },
            CommandRef {
                name: "zjj dashboard".to_string(),
                description: "Interactive session dashboard".to_string(),
            },
            CommandRef {
                name: "zjj query <type>".to_string(),
                description: "Programmatic state queries".to_string(),
            },
        ],
        utilities: vec![
            CommandRef {
                name: "zjj backup".to_string(),
                description: "Backup session database".to_string(),
            },
            CommandRef {
                name: "zjj restore <file>".to_string(),
                description: "Restore from backup".to_string(),
            },
            CommandRef {
                name: "zjj completions <shell>".to_string(),
                description: "Generate shell completions".to_string(),
            },
        ],
    }
}

/// Check beads integration status
///
/// Determines if beads is available and properly configured
/// by checking for the command and `.beads` directory.
pub fn check_beads_status() -> BeadsStatus {
    let command_available = is_command_available("bd");

    let beads_dir = std::env::current_dir().ok().and_then(|mut dir| {
        loop {
            let beads_path = dir.join(".beads");
            if beads_path.exists() && beads_path.is_dir() {
                return Some(beads_path.display().to_string());
            }
            if !dir.pop() {
                break;
            }
        }
        None
    });

    let available = command_available && beads_dir.is_some();

    BeadsStatus {
        available,
        beads_dir,
        command_available,
    }
}

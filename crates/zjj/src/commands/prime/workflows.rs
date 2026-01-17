//! Workflow section building for prime output
//!
//! This module constructs workflow sections that guide users through
//! common ZJJ usage patterns and AI agent recovery procedures.

use super::output_types::WorkflowSection;

/// Build workflow sections for the prime output
///
/// Returns a curated set of workflow guides covering:
/// - Starting new work
/// - Syncing with main branch
/// - Completing work
/// - Switching between sessions
/// - AI agent recovery
pub fn build_workflow_sections() -> Vec<WorkflowSection> {
    vec![
        WorkflowSection {
            title: "Starting New Work".to_string(),
            steps: vec![
                "jjz list                    # Check existing sessions".to_string(),
                "jjz add <feature-name>      # Create new session".to_string(),
                "[automatically switches to new Zellij tab]".to_string(),
                "[work in isolated workspace]".to_string(),
            ],
        },
        WorkflowSection {
            title: "Syncing with Main Branch".to_string(),
            steps: vec![
                "jjz sync                    # Rebase on main".to_string(),
                "[resolve any conflicts]".to_string(),
                "jjz status                  # Verify sync succeeded".to_string(),
            ],
        },
        WorkflowSection {
            title: "Completing Work".to_string(),
            steps: vec![
                "jj commit -m '...'          # Commit changes".to_string(),
                "jj bookmark create <name>   # Create bookmark for PR".to_string(),
                "jj git push                 # Push to remote".to_string(),
                "jjz remove <session-name>   # Cleanup session".to_string(),
            ],
        },
        WorkflowSection {
            title: "Switching Between Sessions".to_string(),
            steps: vec![
                "jjz list                    # See all sessions".to_string(),
                "jjz focus <name>            # Switch to session tab".to_string(),
            ],
        },
        WorkflowSection {
            title: "AI Agent Recovery".to_string(),
            steps: vec![
                "jjz prime                   # Get this context".to_string(),
                "jjz context --json          # Full environment state".to_string(),
                "jjz introspect --json       # CLI documentation".to_string(),
            ],
        },
    ]
}

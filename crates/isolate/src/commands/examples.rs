//! Examples command - Show usage examples for commands
//!
//! Provides copy-pastable examples for AI agents and users.

use anyhow::Result;
use isolate_core::{OutputFormat, SchemaEnvelope};
use serde::{Deserialize, Serialize};

/// Options for the examples command
#[derive(Debug, Clone)]
pub struct ExamplesOptions {
    /// Specific command to show examples for
    pub command: Option<String>,
    /// Filter by use case (workflow, single-command, error-handling)
    pub use_case: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Example entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// Command or workflow name
    pub name: String,
    /// Description of what this example does
    pub description: String,
    /// The actual command(s) to run
    pub commands: Vec<String>,
    /// Expected output (truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<String>,
    /// Use case category
    pub use_case: String,
    /// Prerequisites
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<String>,
    /// Notes or warnings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Examples response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplesResponse {
    pub examples: Vec<Example>,
    pub use_cases: Vec<String>,
}

/// Run the examples command
pub fn run(options: &ExamplesOptions) -> Result<()> {
    let all_examples = build_examples();

    let filtered: Vec<Example> = all_examples
        .examples
        .into_iter()
        .filter(|ex| {
            if let Some(cmd) = &options.command {
                if !ex.commands.iter().any(|c| c.contains(cmd)) {
                    return false;
                }
            }
            if let Some(use_case) = &options.use_case {
                if ex.use_case != *use_case {
                    return false;
                }
            }
            true
        })
        .collect();

    let response = ExamplesResponse {
        examples: filtered,
        use_cases: all_examples.use_cases,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("examples-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        for example in &response.examples {
            println!("# {}", example.name);
            println!("# {}", example.description);
            if !example.prerequisites.is_empty() {
                println!("# Prerequisites: {}", example.prerequisites.join(", "));
            }
            println!();
            for cmd in &example.commands {
                println!("{cmd}");
            }
            if let Some(output) = &example.expected_output {
                println!();
                println!("# Expected output:");
                for line in output.lines() {
                    println!("# {line}");
                }
            }
            if let Some(note) = &example.notes {
                println!();
                println!("# Note: {note}");
            }
            println!();
            println!("---");
            println!();
        }
    }

    Ok(())
}

fn workflow_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Start working on a feature".to_string(),
            description: "Create a workspace and start coding".to_string(),
            commands: vec![
                "isolate work feature-auth".to_string(),
                "cd \"$(isolate context --field=current_session.workspace_path)\"".to_string(),
            ],
            expected_output: Some(
                "Created session 'feature-auth'\nRegistered as agent".to_string(),
            ),
            use_case: "workflow".to_string(),
            prerequisites: vec!["isolate init".to_string()],
            notes: None,
        },
        Example {
            name: "Complete work and merge".to_string(),
            description: "Finish work and merge to main".to_string(),
            commands: vec!["isolate done".to_string()],
            expected_output: Some("Merged 'feature-auth' to main".to_string()),
            use_case: "workflow".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: Some("Use --dry-run to preview first".to_string()),
        },
    ]
}

fn single_command_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Check current location".to_string(),
            description: "Quick orientation command for AI agents".to_string(),
            commands: vec!["isolate whereami".to_string()],
            expected_output: Some("workspace:feature-auth".to_string()),
            use_case: "single-command".to_string(),
            prerequisites: vec![],
            notes: None,
        },
        Example {
            name: "List all sessions".to_string(),
            description: "View all active sessions with status".to_string(),
            commands: vec![
                "isolate list".to_string(),
                "isolate list --json".to_string(),
            ],
            expected_output: None,
            use_case: "single-command".to_string(),
            prerequisites: vec!["isolate init".to_string()],
            notes: None,
        },
        Example {
            name: "Sync workspace with main".to_string(),
            description: "Rebase workspace onto latest main".to_string(),
            commands: vec!["isolate sync".to_string()],
            expected_output: Some("Synced 1 session".to_string()),
            use_case: "single-command".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: None,
        },
    ]
}

fn error_handling_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Undo a merge".to_string(),
            description: "Revert the last done operation".to_string(),
            commands: vec![
                "isolate undo --dry-run".to_string(),
                "isolate undo".to_string(),
            ],
            expected_output: Some("Reverted merge of 'feature-auth'".to_string()),
            use_case: "error-handling".to_string(),
            prerequisites: vec![
                "Must have undo history".to_string(),
                "Not pushed to remote".to_string(),
            ],
            notes: None,
        },
        Example {
            name: "Abort work without merging".to_string(),
            description: "Discard work and cleanup".to_string(),
            commands: vec![
                "isolate abort".to_string(),
                "isolate abort --keep-workspace".to_string(),
            ],
            expected_output: Some("Aborted 'feature-auth'".to_string()),
            use_case: "error-handling".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: None,
        },
    ]
}

fn automation_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Spawn automated agent".to_string(),
            description: "Run an AI agent on a bead".to_string(),
            commands: vec![
                "isolate spawn isolate-abc12".to_string(),
                "isolate spawn isolate-xyz34 --background".to_string(),
            ],
            expected_output: None,
            use_case: "automation".to_string(),
            prerequisites: vec!["Bead must exist".to_string()],
            notes: Some("Agent runs in background".to_string()),
        },
        Example {
            name: "Idempotent operations".to_string(),
            description: "Safe for retries".to_string(),
            commands: vec![
                "isolate work feature-auth --idempotent".to_string(),
                "isolate remove old-session --idempotent".to_string(),
            ],
            expected_output: None,
            use_case: "automation".to_string(),
            prerequisites: vec![],
            notes: Some("Returns success even if already done".to_string()),
        },
    ]
}

fn ai_agent_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Get full context (AI agent)".to_string(),
            description: "Get complete environment context for AI".to_string(),
            commands: vec![
                "isolate context --json".to_string(),
                "isolate context --field=repository.branch".to_string(),
            ],
            expected_output: None,
            use_case: "ai-agent".to_string(),
            prerequisites: vec![],
            notes: Some("Use --no-beads --no-health for faster response".to_string()),
        },
        Example {
            name: "AI agent quick start".to_string(),
            description: "Minimal workflow for AI agents".to_string(),
            commands: vec!["isolate ai quick-start".to_string()],
            expected_output: None,
            use_case: "ai-agent".to_string(),
            prerequisites: vec![],
            notes: None,
        },
    ]
}

fn multi_agent_examples() -> Vec<Example> {
    vec![Example {
        name: "Register as agent".to_string(),
        description: "Register for multi-agent coordination".to_string(),
        commands: vec![
            "isolate agent register".to_string(),
            "isolate agent heartbeat".to_string(),
        ],
        expected_output: None,
        use_case: "multi-agent".to_string(),
        prerequisites: vec![],
        notes: Some("Sets Isolate_AGENT_ID environment variable".to_string()),
    }]
}

fn safety_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Create checkpoint".to_string(),
            description: "Save current state for rollback".to_string(),
            commands: vec![
                "isolate checkpoint create -d \"Before refactor\"".to_string(),
                "isolate checkpoint list".to_string(),
                "isolate checkpoint restore <checkpoint_id>".to_string(),
            ],
            expected_output: None,
            use_case: "safety".to_string(),
            prerequisites: vec!["isolate init".to_string()],
            notes: None,
        },
        Example {
            name: "Dry-run preview".to_string(),
            description: "Preview operations without executing".to_string(),
            commands: vec![
                "isolate done --dry-run".to_string(),
                "isolate add test --dry-run".to_string(),
                "isolate undo --dry-run".to_string(),
            ],
            expected_output: None,
            use_case: "safety".to_string(),
            prerequisites: vec![],
            notes: Some("No side effects, just shows what would happen".to_string()),
        },
    ]
}

fn maintenance_examples() -> Vec<Example> {
    vec![Example {
        name: "Run health checks".to_string(),
        description: "Diagnose and fix issues".to_string(),
        commands: vec![
            "isolate doctor".to_string(),
            "isolate doctor --fix".to_string(),
        ],
        expected_output: None,
        use_case: "maintenance".to_string(),
        prerequisites: vec![],
        notes: None,
    }]
}

fn build_examples() -> ExamplesResponse {
    let mut examples = Vec::new();
    examples.extend(workflow_examples());
    examples.extend(single_command_examples());
    examples.extend(error_handling_examples());
    examples.extend(automation_examples());
    examples.extend(ai_agent_examples());
    examples.extend(multi_agent_examples());
    examples.extend(safety_examples());
    examples.extend(maintenance_examples());

    let use_cases = vec![
        "workflow".to_string(),
        "single-command".to_string(),
        "error-handling".to_string(),
        "maintenance".to_string(),
        "automation".to_string(),
        "ai-agent".to_string(),
        "multi-agent".to_string(),
        "safety".to_string(),
    ];

    ExamplesResponse {
        examples,
        use_cases,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_examples_not_empty() {
        let examples = build_examples();
        assert!(!examples.examples.is_empty());
        assert!(!examples.use_cases.is_empty());
    }

    #[test]
    fn test_examples_have_commands() {
        let examples = build_examples();
        for ex in &examples.examples {
            assert!(!ex.commands.is_empty());
            assert!(!ex.name.is_empty());
            assert!(!ex.description.is_empty());
        }
    }

    #[test]
    fn test_all_use_cases_covered() {
        let examples = build_examples();
        for use_case in &examples.use_cases {
            let count = examples
                .examples
                .iter()
                .filter(|e| e.use_case == *use_case)
                .count();
            assert!(count > 0, "Use case {use_case} has no examples");
        }
    }
}

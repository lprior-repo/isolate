//! Contract command - Show JSON Schema contracts for commands
//!
//! Provides machine-readable API contracts for AI agents to understand
//! command inputs, outputs, and side effects.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{json::schemas, OutputFormat, SchemaEnvelope};

/// Options for the contract command
#[derive(Debug, Clone)]
pub struct ContractOptions {
    /// Specific command to show contract for (or all if None)
    pub command: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Contract information for a single command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContract {
    /// Command name
    pub name: String,
    /// Short description
    pub description: String,
    /// Required arguments
    pub required_args: Vec<ArgContract>,
    /// Optional arguments
    pub optional_args: Vec<ArgContract>,
    /// Flags (boolean options)
    pub flags: Vec<FlagContract>,
    /// Output schema type
    pub output_schema: String,
    /// Side effects of this command
    pub side_effects: Vec<String>,
    /// Related commands
    pub related_commands: Vec<String>,
    /// Example usage
    pub examples: Vec<String>,
    /// Whether this command is reversible
    pub reversible: bool,
    /// Undo command if reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Required prerequisites
    pub prerequisites: Vec<String>,
}

/// Contract for a command argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgContract {
    /// Argument name
    pub name: String,
    /// Argument type
    pub arg_type: String,
    /// Description
    pub description: String,
    /// Validation pattern (regex) if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Default value if optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Example values
    pub examples: Vec<String>,
}

/// Contract for a flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagContract {
    /// Flag name (long form)
    pub name: String,
    /// Short form
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Description
    pub description: String,
    /// Whether flag is global (applies to all subcommands)
    pub global: bool,
}

/// Complete contracts response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractsResponse {
    /// All command contracts
    pub commands: Vec<CommandContract>,
    /// Global flags that apply to all commands
    pub global_flags: Vec<FlagContract>,
    /// Schema version
    pub version: String,
}

/// Run the contract command
pub fn run(options: &ContractOptions) -> Result<()> {
    let contracts = build_all_contracts();

    if let Some(cmd_name) = &options.command {
        let contract = contracts
            .commands
            .into_iter()
            .find(|c| c.name == *cmd_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {cmd_name}"))?;

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("contract-response", "single", contract);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            print_contract_human(&contract);
        }
        return Ok(());
    }

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("contracts-response", "single", contracts);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        for contract in &contracts.commands {
            print_contract_human(contract);
            println!();
        }
    }
    Ok(())
}

fn print_contract_human(contract: &CommandContract) {
    println!("Command: {}", contract.name);
    println!("  {}", contract.description);
    println!();

    if !contract.required_args.is_empty() {
        println!("  Required arguments:");
        for arg in &contract.required_args {
            println!("    {} ({}): {}", arg.name, arg.arg_type, arg.description);
            if let Some(pattern) = &arg.pattern {
                println!("      Pattern: {pattern}");
            }
            if !arg.examples.is_empty() {
                println!("      Examples: {}", arg.examples.join(", "));
            }
        }
        println!();
    }

    if !contract.optional_args.is_empty() {
        println!("  Optional arguments:");
        for arg in &contract.optional_args {
            print!("    {} ({}): {}", arg.name, arg.arg_type, arg.description);
            if let Some(default) = &arg.default {
                print!(" [default: {default}]");
            }
            println!();
        }
        println!();
    }

    if !contract.flags.is_empty() {
        println!("  Flags:");
        for flag in &contract.flags {
            let short = flag
                .short
                .as_ref()
                .map_or(String::new(), |s| format!("-{s}, "));
            println!("    {short}--{}: {}", flag.name, flag.description);
        }
        println!();
    }

    if !contract.side_effects.is_empty() {
        println!("  Side effects:");
        for effect in &contract.side_effects {
            println!("    - {effect}");
        }
        println!();
    }

    if contract.reversible {
        println!("  Reversible: yes");
        if let Some(undo) = &contract.undo_command {
            println!("  Undo command: {undo}");
        }
    }

    if !contract.examples.is_empty() {
        println!("  Examples:");
        for example in &contract.examples {
            println!("    {example}");
        }
    }
}

/// Build contracts for all commands
fn build_all_contracts() -> ContractsResponse {
    let global_flags = vec![
        FlagContract {
            name: "json".to_string(),
            short: Some("j".to_string()),
            description: "Output as JSON".to_string(),
            global: true,
        },
        FlagContract {
            name: "help".to_string(),
            short: Some("h".to_string()),
            description: "Show help information".to_string(),
            global: true,
        },
    ];

    let commands = vec![
        build_init_contract(),
        build_add_contract(),
        build_attach_contract(),
        build_list_contract(),
        build_remove_contract(),
        build_focus_contract(),
        build_status_contract(),
        build_sync_contract(),
        build_done_contract(),
        build_undo_contract(),
        build_revert_contract(),
        build_work_contract(),
        build_abort_contract(),
        build_spawn_contract(),
        build_whereami_contract(),
        build_whoami_contract(),
        build_doctor_contract(),
        build_clean_contract(),
        build_context_contract(),
        build_introspect_contract(),
        build_checkpoint_contract(),
        build_export_contract(),
        build_import_contract(),
    ];

    ContractsResponse {
        commands,
        global_flags,
        version: "1.0".to_string(),
    }
}

fn build_init_contract() -> CommandContract {
    CommandContract {
        name: "init".to_string(),
        description: "Initialize zjj in a JJ repository (or create one)".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![FlagContract {
            name: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            global: false,
        }],
        output_schema: "zjj://init-response/v1".to_string(),
        side_effects: vec![
            "Creates .zjj directory".to_string(),
            "Creates state.db database".to_string(),
            "Creates default layouts".to_string(),
        ],
        related_commands: vec!["doctor".to_string(), "add".to_string()],
        examples: vec!["zjj init".to_string(), "zjj init --json".to_string()],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["JJ must be installed".to_string()],
    }
}

fn build_add_contract() -> CommandContract {
    CommandContract {
        name: "add".to_string(),
        description: "Create session for manual work (JJ workspace + Zellij tab)".to_string(),
        required_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description:
                "Session name (required for normal execution; may be omitted only with --example-json, --contract, or --ai-hints)"
                    .to_string(),
            pattern: Some("^[a-zA-Z][a-zA-Z0-9_-]*$".to_string()),
            default: None,
            examples: vec!["feature-auth".to_string(), "bugfix-123".to_string()],
        }],
        optional_args: vec![
            ArgContract {
                name: "template".to_string(),
                arg_type: "string".to_string(),
                description: "Zellij layout template".to_string(),
                pattern: Some("^(minimal|standard|full)$".to_string()),
                default: None,
                examples: vec![
                    "minimal".to_string(),
                    "standard".to_string(),
                    "full".to_string(),
                ],
            },
            ArgContract {
                name: "bead".to_string(),
                arg_type: "string".to_string(),
                description: "Associate session with bead/issue ID".to_string(),
                pattern: None,
                default: None,
                examples: vec!["zjj-abc123".to_string()],
            },
        ],
        flags: vec![
            FlagContract {
                name: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-hooks".to_string(),
                short: None,
                description: "Skip executing post_create hooks".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-open".to_string(),
                short: None,
                description: "Create workspace without opening Zellij tab".to_string(),
                global: false,
            },
            FlagContract {
                name: "idempotent".to_string(),
                short: None,
                description: "Succeed if session already exists".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "Preview without creating".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-zellij".to_string(),
                short: None,
                description: "Skip Zellij integration".to_string(),
                global: false,
            },
            FlagContract {
                name: "example-json".to_string(),
                short: None,
                description: "Show example JSON output without executing".to_string(),
                global: false,
            },
            FlagContract {
                name: "contract".to_string(),
                short: None,
                description: "Show machine-readable command contract".to_string(),
                global: false,
            },
            FlagContract {
                name: "ai-hints".to_string(),
                short: None,
                description: "Show AI execution hints".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://add-response/v1".to_string(),
        side_effects: vec![
            "Creates JJ workspace".to_string(),
            "Creates Zellij tab".to_string(),
            "Updates session database".to_string(),
        ],
        related_commands: vec![
            "focus".to_string(),
            "remove".to_string(),
            "work".to_string(),
        ],
        examples: vec![
            "zjj add feature-auth".to_string(),
            "zjj add bugfix-123 --no-open".to_string(),
            "zjj add test --idempotent".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj remove <name>".to_string()),
        prerequisites: vec![
            "zjj must be initialized".to_string(),
            "Session name must not exist".to_string(),
        ],
    }
}

fn build_attach_contract() -> CommandContract {
    CommandContract {
        name: "attach".to_string(),
        description: "Enter Zellij session from outside (shell -> Zellij)".to_string(),
        required_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Name of the session to attach to".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string(), "work".to_string()],
        }],
        optional_args: vec![],
        flags: vec![FlagContract {
            name: "json".to_string(),
            short: None,
            description: "Output as JSON (errors only)".to_string(),
            global: false,
        }],
        output_schema: "zjj://attach-response/v1".to_string(),
        side_effects: vec!["Attaches current shell to Zellij session".to_string()],
        related_commands: vec!["focus".to_string(), "switch".to_string()],
        examples: vec![
            "zjj attach feature-auth".to_string(),
            "zjj attach work --json".to_string(),
        ],
        reversible: true,
        undo_command: Some("exit".to_string()),
        prerequisites: vec!["zellij must be installed".to_string()],
    }
}

fn build_list_contract() -> CommandContract {
    CommandContract {
        name: "list".to_string(),
        description: "List all sessions".to_string(),
        required_args: vec![],
        optional_args: vec![
            ArgContract {
                name: "bead".to_string(),
                arg_type: "string".to_string(),
                description: "Filter sessions by bead ID".to_string(),
                pattern: None,
                default: None,
                examples: vec!["zjj-1234".to_string()],
            },
            ArgContract {
                name: "agent".to_string(),
                arg_type: "string".to_string(),
                description: "Filter sessions by agent".to_string(),
                pattern: None,
                default: None,
                examples: vec!["agent-001".to_string()],
            },
        ],
        flags: vec![FlagContract {
            name: "all".to_string(),
            short: None,
            description: "Include completed and failed sessions".to_string(),
            global: false,
        }],
        output_schema: "zjj://list-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["status".to_string(), "add".to_string()],
        examples: vec![
            "zjj list".to_string(),
            "zjj list --all".to_string(),
            "zjj list --json".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_remove_contract() -> CommandContract {
    CommandContract {
        name: "remove".to_string(),
        description: "Remove a session and its workspace".to_string(),
        required_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Name of the session to remove".to_string(),
            pattern: None,
            default: None,
            examples: vec!["old-feature".to_string()],
        }],
        optional_args: vec![],
        flags: vec![
            FlagContract {
                name: "force".to_string(),
                short: Some("f".to_string()),
                description: "Skip confirmation prompt".to_string(),
                global: false,
            },
            FlagContract {
                name: "merge".to_string(),
                short: Some("m".to_string()),
                description: "Merge to main before removal".to_string(),
                global: false,
            },
            FlagContract {
                name: "keep-branch".to_string(),
                short: Some("k".to_string()),
                description: "Preserve branch after removal".to_string(),
                global: false,
            },
            FlagContract {
                name: "idempotent".to_string(),
                short: None,
                description: "Succeed if session doesn't exist".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://remove-response/v1".to_string(),
        side_effects: vec![
            "Removes JJ workspace".to_string(),
            "Closes Zellij tab".to_string(),
            "Removes from database".to_string(),
        ],
        related_commands: vec!["add".to_string(), "clean".to_string()],
        examples: vec![
            "zjj remove old-feature".to_string(),
            "zjj remove test -f".to_string(),
            "zjj remove feature-x --merge".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["Session must exist".to_string()],
    }
}

fn build_focus_contract() -> CommandContract {
    CommandContract {
        name: "focus".to_string(),
        description: "Switch to session's Zellij tab".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Name of session to focus".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![],
        output_schema: "zjj://focus-response/v1".to_string(),
        side_effects: vec!["Switches Zellij tab".to_string()],
        related_commands: vec!["attach".to_string(), "list".to_string()],
        examples: vec![
            "zjj focus feature-auth".to_string(),
            "zjj focus".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj focus main".to_string()),
        prerequisites: vec![
            "Must be inside Zellij".to_string(),
            "Session must exist".to_string(),
        ],
    }
}

fn build_status_contract() -> CommandContract {
    CommandContract {
        name: "status".to_string(),
        description: "Show detailed session status".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Session to show status for".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![FlagContract {
            name: "watch".to_string(),
            short: None,
            description: "Continuously update status".to_string(),
            global: false,
        }],
        output_schema: "zjj://status-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["list".to_string(), "context".to_string()],
        examples: vec![
            "zjj status".to_string(),
            "zjj status feature-auth".to_string(),
            "zjj status --watch".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_sync_contract() -> CommandContract {
    CommandContract {
        name: "sync".to_string(),
        description: "Sync session workspace with main (rebase)".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Session to sync".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![FlagContract {
            name: "all".to_string(),
            short: None,
            description: "Sync all active sessions".to_string(),
            global: false,
        }],
        output_schema: "zjj://sync-response/v1".to_string(),
        side_effects: vec!["Rebases workspace onto main".to_string()],
        related_commands: vec!["status".to_string(), "done".to_string()],
        examples: vec![
            "zjj sync".to_string(),
            "zjj sync feature-auth".to_string(),
            "zjj sync --all".to_string(),
        ],
        reversible: true,
        undo_command: Some("jj undo".to_string()),
        prerequisites: vec![
            "Session must exist".to_string(),
            "No uncommitted changes with conflicts".to_string(),
        ],
    }
}

fn build_done_contract() -> CommandContract {
    CommandContract {
        name: "done".to_string(),
        description: "Complete work and merge workspace to main".to_string(),
        required_args: vec![],
        optional_args: vec![
            ArgContract {
                name: "workspace".to_string(),
                arg_type: "string".to_string(),
                description: "Workspace to complete".to_string(),
                pattern: None,
                default: None,
                examples: vec!["feature-auth".to_string()],
            },
            ArgContract {
                name: "message".to_string(),
                arg_type: "string".to_string(),
                description: "Commit message".to_string(),
                pattern: None,
                default: None,
                examples: vec!["Fix authentication bug".to_string()],
            },
        ],
        flags: vec![
            FlagContract {
                name: "keep-workspace".to_string(),
                short: None,
                description: "Keep workspace after merge".to_string(),
                global: false,
            },
            FlagContract {
                name: "squash".to_string(),
                short: None,
                description: "Squash all commits into one".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "Preview without executing".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-bead-update".to_string(),
                short: None,
                description: "Skip bead status update".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://done-response/v1".to_string(),
        side_effects: vec![
            "Merges changes to main".to_string(),
            "Removes workspace (unless --keep-workspace)".to_string(),
            "Updates bead status".to_string(),
            "Records undo history".to_string(),
        ],
        related_commands: vec!["undo".to_string(), "abort".to_string()],
        examples: vec![
            "zjj done".to_string(),
            "zjj done -m \"Fix bug\"".to_string(),
            "zjj done --dry-run".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj undo".to_string()),
        prerequisites: vec!["Must be in a workspace or specify --workspace".to_string()],
    }
}

fn build_undo_contract() -> CommandContract {
    CommandContract {
        name: "undo".to_string(),
        description: "Revert last done operation".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![FlagContract {
            name: "dry-run".to_string(),
            short: None,
            description: "Preview without executing".to_string(),
            global: false,
        }],
        output_schema: "zjj://undo-response/v1".to_string(),
        side_effects: vec![
            "Reverts merge commit".to_string(),
            "Recreates workspace".to_string(),
        ],
        related_commands: vec!["done".to_string(), "revert".to_string()],
        examples: vec!["zjj undo".to_string(), "zjj undo --dry-run".to_string()],
        reversible: true,
        undo_command: Some("zjj done".to_string()),
        prerequisites: vec![
            "Must have undo history".to_string(),
            "Changes must not be pushed".to_string(),
        ],
    }
}

fn build_revert_contract() -> CommandContract {
    CommandContract {
        name: "revert".to_string(),
        description: "Revert specific session merge".to_string(),
        required_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Session name to revert".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        optional_args: vec![],
        flags: vec![FlagContract {
            name: "dry-run".to_string(),
            short: None,
            description: "Preview without executing".to_string(),
            global: false,
        }],
        output_schema: "zjj://revert-response/v1".to_string(),
        side_effects: vec!["Reverts specific merge".to_string()],
        related_commands: vec!["undo".to_string(), "done".to_string()],
        examples: vec![
            "zjj revert feature-auth".to_string(),
            "zjj revert old-feature --dry-run".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj done".to_string()),
        prerequisites: vec![
            "Session must be in undo history".to_string(),
            "Changes must not be pushed".to_string(),
        ],
    }
}

fn build_work_contract() -> CommandContract {
    CommandContract {
        name: "work".to_string(),
        description: "Start working on a task (create workspace + register agent)".to_string(),
        required_args: vec![ArgContract {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            description: "Session name to create/use".to_string(),
            pattern: Some("^[a-zA-Z][a-zA-Z0-9_-]*$".to_string()),
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        optional_args: vec![
            ArgContract {
                name: "bead".to_string(),
                arg_type: "string".to_string(),
                description: "Bead ID to associate".to_string(),
                pattern: None,
                default: None,
                examples: vec!["zjj-1234".to_string()],
            },
            ArgContract {
                name: "agent-id".to_string(),
                arg_type: "string".to_string(),
                description: "Agent ID to register".to_string(),
                pattern: None,
                default: None,
                examples: vec!["agent-001".to_string()],
            },
        ],
        flags: vec![
            FlagContract {
                name: "no-zellij".to_string(),
                short: None,
                description: "Don't create Zellij tab".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-agent".to_string(),
                short: None,
                description: "Don't register as agent".to_string(),
                global: false,
            },
            FlagContract {
                name: "idempotent".to_string(),
                short: None,
                description: "Succeed if session exists".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "Preview without creating".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://work-response/v1".to_string(),
        side_effects: vec![
            "Creates workspace".to_string(),
            "Registers agent".to_string(),
            "Sets environment variables".to_string(),
        ],
        related_commands: vec!["done".to_string(), "abort".to_string(), "add".to_string()],
        examples: vec![
            "zjj work feature-auth".to_string(),
            "zjj work bug-fix --bead zjj-123".to_string(),
            "zjj work test --idempotent".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj abort".to_string()),
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_abort_contract() -> CommandContract {
    CommandContract {
        name: "abort".to_string(),
        description: "Abandon workspace without merging".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "workspace".to_string(),
            arg_type: "string".to_string(),
            description: "Workspace to abort".to_string(),
            pattern: None,
            default: None,
            examples: vec!["feature-auth".to_string()],
        }],
        flags: vec![
            FlagContract {
                name: "keep-workspace".to_string(),
                short: None,
                description: "Keep workspace files".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-bead-update".to_string(),
                short: None,
                description: "Don't update bead status".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "Preview without executing".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://abort-response/v1".to_string(),
        side_effects: vec![
            "Removes workspace".to_string(),
            "Updates bead status to abandoned".to_string(),
        ],
        related_commands: vec!["done".to_string(), "work".to_string()],
        examples: vec![
            "zjj abort".to_string(),
            "zjj abort --workspace feature-x".to_string(),
            "zjj abort --keep-workspace".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["Must be in a workspace or specify --workspace".to_string()],
    }
}

fn build_spawn_contract() -> CommandContract {
    CommandContract {
        name: "spawn".to_string(),
        description: "Create session for automated agent work on a bead".to_string(),
        required_args: vec![ArgContract {
            name: "bead_id".to_string(),
            arg_type: "string".to_string(),
            description: "Bead ID to work on".to_string(),
            pattern: Some("^[a-z]+-[a-z0-9]+$".to_string()),
            default: None,
            examples: vec!["zjj-abc12".to_string()],
        }],
        optional_args: vec![
            ArgContract {
                name: "agent-command".to_string(),
                arg_type: "string".to_string(),
                description: "Agent command to run".to_string(),
                pattern: None,
                default: Some("claude".to_string()),
                examples: vec!["claude".to_string(), "llm-run".to_string()],
            },
            ArgContract {
                name: "timeout".to_string(),
                arg_type: "integer".to_string(),
                description: "Timeout in seconds".to_string(),
                pattern: None,
                default: Some("14400".to_string()),
                examples: vec!["3600".to_string(), "7200".to_string()],
            },
        ],
        flags: vec![
            FlagContract {
                name: "no-auto-merge".to_string(),
                short: None,
                description: "Don't merge on success".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-auto-cleanup".to_string(),
                short: None,
                description: "Don't cleanup on failure".to_string(),
                global: false,
            },
            FlagContract {
                name: "background".to_string(),
                short: Some("b".to_string()),
                description: "Run agent in background".to_string(),
                global: false,
            },
            FlagContract {
                name: "idempotent".to_string(),
                short: None,
                description: "Succeed if workspace already exists".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://spawn-response/v1".to_string(),
        side_effects: vec![
            "Creates workspace".to_string(),
            "Spawns agent process".to_string(),
            "Updates bead status".to_string(),
        ],
        related_commands: vec!["done".to_string(), "add".to_string()],
        examples: vec![
            "zjj spawn zjj-abc12".to_string(),
            "zjj spawn zjj-abc12 --idempotent".to_string(),
            "zjj spawn zjj-xyz34 -b".to_string(),
            "zjj spawn zjj-def56 --no-auto-merge".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec![
            "Bead must exist".to_string(),
            "Must be on main branch".to_string(),
        ],
    }
}

fn build_whereami_contract() -> CommandContract {
    CommandContract {
        name: "whereami".to_string(),
        description: "Quick location query - returns 'main' or 'workspace:<name>'".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![],
        output_schema: "zjj://whereami-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["whoami".to_string(), "context".to_string()],
        examples: vec![
            "zjj whereami".to_string(),
            "zjj whereami --json".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec![],
    }
}

fn build_whoami_contract() -> CommandContract {
    CommandContract {
        name: "whoami".to_string(),
        description: "Agent identity query - returns agent ID or 'unregistered'".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![],
        output_schema: "zjj://whoami-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["whereami".to_string(), "agents".to_string()],
        examples: vec!["zjj whoami".to_string(), "zjj whoami --json".to_string()],
        reversible: false,
        undo_command: None,
        prerequisites: vec![],
    }
}

fn build_doctor_contract() -> CommandContract {
    CommandContract {
        name: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![FlagContract {
            name: "fix".to_string(),
            short: None,
            description: "Auto-fix issues where possible".to_string(),
            global: false,
        }],
        output_schema: "zjj://doctor-response/v1".to_string(),
        side_effects: vec!["May fix database issues (with --fix)".to_string()],
        related_commands: vec!["init".to_string(), "clean".to_string()],
        examples: vec![
            "zjj doctor".to_string(),
            "zjj doctor --fix".to_string(),
            "zjj doctor --json".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec![],
    }
}

fn build_clean_contract() -> CommandContract {
    CommandContract {
        name: "clean".to_string(),
        description: "Remove stale sessions".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![
            FlagContract {
                name: "force".to_string(),
                short: Some("f".to_string()),
                description: "Skip confirmation".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "List stale sessions without removing".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://clean-response/v1".to_string(),
        side_effects: vec!["Removes stale session records".to_string()],
        related_commands: vec!["doctor".to_string(), "list".to_string()],
        examples: vec![
            "zjj clean".to_string(),
            "zjj clean --dry-run".to_string(),
            "zjj clean -f".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_context_contract() -> CommandContract {
    CommandContract {
        name: "context".to_string(),
        description: "Show complete environment context (AI agent query)".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "field".to_string(),
            arg_type: "string".to_string(),
            description: "Extract single field".to_string(),
            pattern: None,
            default: None,
            examples: vec![
                "repository.branch".to_string(),
                "sessions[0].name".to_string(),
            ],
        }],
        flags: vec![
            FlagContract {
                name: "no-beads".to_string(),
                short: None,
                description: "Skip beads query (faster)".to_string(),
                global: false,
            },
            FlagContract {
                name: "no-health".to_string(),
                short: None,
                description: "Skip health checks (faster)".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://context-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["status".to_string(), "introspect".to_string()],
        examples: vec![
            "zjj context".to_string(),
            "zjj context --field=repository.branch".to_string(),
            "zjj context --no-beads --no-health".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec![],
    }
}

fn build_introspect_contract() -> CommandContract {
    CommandContract {
        name: "introspect".to_string(),
        description: "Discover zjj capabilities and command details".to_string(),
        required_args: vec![],
        optional_args: vec![ArgContract {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            description: "Specific command to introspect".to_string(),
            pattern: None,
            default: None,
            examples: vec!["add".to_string(), "done".to_string()],
        }],
        flags: vec![
            FlagContract {
                name: "env-vars".to_string(),
                short: None,
                description: "Show environment variables".to_string(),
                global: false,
            },
            FlagContract {
                name: "workflows".to_string(),
                short: None,
                description: "Show workflow patterns".to_string(),
                global: false,
            },
            FlagContract {
                name: "session-states".to_string(),
                short: None,
                description: "Show state transitions".to_string(),
                global: false,
            },
        ],
        output_schema: "zjj://introspect-response/v1".to_string(),
        side_effects: vec![],
        related_commands: vec!["contract".to_string(), "context".to_string()],
        examples: vec![
            "zjj introspect".to_string(),
            "zjj introspect add".to_string(),
            "zjj introspect --workflows".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec![],
    }
}

fn build_checkpoint_contract() -> CommandContract {
    CommandContract {
        name: "checkpoint".to_string(),
        description: "Save and restore session state snapshots".to_string(),
        required_args: vec![],
        optional_args: vec![],
        flags: vec![],
        output_schema: "zjj://checkpoint-response/v1".to_string(),
        side_effects: vec!["Creates or restores checkpoints".to_string()],
        related_commands: vec!["status".to_string()],
        examples: vec![
            "zjj checkpoint create".to_string(),
            "zjj checkpoint create -d \"Before refactor\"".to_string(),
            "zjj checkpoint list".to_string(),
            "zjj checkpoint restore <id>".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj checkpoint restore <prev_id>".to_string()),
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_export_contract() -> CommandContract {
    CommandContract {
        name: "export".to_string(),
        description: "Export session state to stdout or file".to_string(),
        required_args: vec![],
        optional_args: vec![
            ArgContract {
                name: "session".to_string(),
                arg_type: "string".to_string(),
                description: "Optional session name to export (all sessions if omitted)"
                    .to_string(),
                pattern: Some("^[a-zA-Z][a-zA-Z0-9_-]*$".to_string()),
                default: None,
                examples: vec!["feature-auth".to_string(), "bugfix-123".to_string()],
            },
            ArgContract {
                name: "output".to_string(),
                arg_type: "path".to_string(),
                description: "Output file path (must be used with -o|--output)".to_string(),
                pattern: None,
                default: None,
                examples: vec!["state.json".to_string(), "./exports/all.json".to_string()],
            },
        ],
        flags: vec![FlagContract {
            name: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            global: false,
        }],
        output_schema: schemas::uri(schemas::EXPORT_RESPONSE),
        side_effects: vec!["Writes export payload when --output is provided".to_string()],
        related_commands: vec!["import".to_string(), "backup".to_string()],
        examples: vec![
            "zjj export --json".to_string(),
            "zjj export feature-auth -o state.json".to_string(),
            "zjj export -o all-sessions.json".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

fn build_import_contract() -> CommandContract {
    CommandContract {
        name: "import".to_string(),
        description: "Import session state from file".to_string(),
        required_args: vec![ArgContract {
            name: "file".to_string(),
            arg_type: "path".to_string(),
            description: "Input file containing exported session data".to_string(),
            pattern: None,
            default: None,
            examples: vec![
                "state.json".to_string(),
                "./exports/sessions.json".to_string(),
            ],
        }],
        optional_args: vec![],
        flags: vec![
            FlagContract {
                name: "force".to_string(),
                short: Some("f".to_string()),
                description: "Overwrite existing sessions".to_string(),
                global: false,
            },
            FlagContract {
                name: "skip-existing".to_string(),
                short: None,
                description: "Skip sessions that already exist".to_string(),
                global: false,
            },
            FlagContract {
                name: "dry-run".to_string(),
                short: None,
                description: "Preview import without changes".to_string(),
                global: false,
            },
            FlagContract {
                name: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                global: false,
            },
        ],
        output_schema: schemas::uri(schemas::IMPORT_RESPONSE),
        side_effects: vec!["Creates or updates session records from file".to_string()],
        related_commands: vec!["export".to_string(), "status".to_string()],
        examples: vec![
            "zjj import state.json".to_string(),
            "zjj import state.json --skip-existing".to_string(),
            "zjj import state.json --force --json".to_string(),
        ],
        reversible: false,
        undo_command: None,
        prerequisites: vec!["zjj must be initialized".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_all_contracts_has_commands() {
        let contracts = build_all_contracts();
        assert!(!contracts.commands.is_empty());
        assert!(!contracts.global_flags.is_empty());
    }

    #[test]
    fn test_contracts_have_required_fields() {
        let contracts = build_all_contracts();
        for cmd in &contracts.commands {
            assert!(!cmd.name.is_empty());
            assert!(!cmd.description.is_empty());
            assert!(!cmd.output_schema.is_empty());
        }
    }

    #[test]
    fn test_add_contract_has_name_validation() {
        let add = build_add_contract();
        assert!(!add.required_args.is_empty());
        let name_arg = &add.required_args[0];
        assert_eq!(name_arg.name, "name");
        assert!(name_arg.pattern.is_some());
    }

    #[test]
    fn test_done_is_reversible() {
        let done = build_done_contract();
        assert!(done.reversible);
        assert!(done.undo_command.is_some());
    }

    #[test]
    fn test_list_has_no_side_effects() {
        let list = build_list_contract();
        assert!(list.side_effects.is_empty());
    }

    #[test]
    fn test_contract_serialization() {
        let contracts = build_all_contracts();
        let json = serde_json::to_string_pretty(&contracts);
        assert!(json.is_ok());
    }
}

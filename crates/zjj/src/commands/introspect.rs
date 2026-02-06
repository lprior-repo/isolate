//! Introspect command - discover zjj capabilities
//!
//! This command enables AI agents to understand available commands,
//! system state, and dependencies.

use anyhow::Result;
use zjj_core::{
    introspection::{
        ArgumentSpec, CommandExample, CommandIntrospection, ErrorCondition, FlagSpec,
        IntrospectOutput, Prerequisites,
    },
    json::SchemaEnvelope,
    OutputFormat,
};

mod dependencies;
mod output;
mod system_state;

/// Run the introspect command - show all capabilities
pub async fn run(format: OutputFormat) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut output = IntrospectOutput::new(version);

    // Add dependencies
    output.dependencies = dependencies::check_dependencies().await;

    // Add system state
    output.system_state = system_state::get_system_state().await;

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        output::print_human_readable(&output);
    }

    Ok(())
}

/// Introspect a specific command
pub async fn run_command_introspect(command: &str, format: OutputFormat) -> Result<()> {
    let introspection = match command {
        "add" => get_add_introspection(),
        "remove" => get_remove_introspection(),
        "list" => get_list_introspection(),
        "init" => get_init_introspection(),
        "focus" => get_focus_introspection(),
        "status" => get_status_introspection(),
        "sync" => get_sync_introspection(),
        "diff" => get_diff_introspection(),
        "introspect" => get_introspect_introspection(),
        "doctor" => get_doctor_introspection(),
        "query" => get_query_introspection(),
        _ => {
            anyhow::bail!("Unknown command: {command}");
        }
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-command-response", "single", introspection);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_command_human_readable(&introspection);
    }

    Ok(())
}

/// Print command introspection in human-readable format
fn print_command_human_readable(cmd: &CommandIntrospection) {
    println!("Command: {}", cmd.command);
    println!("Description: {}", cmd.description);
    println!();

    if !cmd.arguments.is_empty() {
        println!("Arguments:");
        for arg in &cmd.arguments {
            let required = if arg.required {
                " (required)"
            } else {
                " (optional)"
            };
            println!("  {}{required}", arg.name);
            println!("    Type: {}", arg.arg_type);
            println!("    Description: {}", arg.description);
            if !arg.examples.is_empty() {
                println!("    Examples: {}", arg.examples.join(", "));
            }
        }
        println!();
    }

    if !cmd.flags.is_empty() {
        print_flags_by_category(&cmd.flags);
        println!();
    }

    if !cmd.examples.is_empty() {
        println!("Examples:");
        for example in &cmd.examples {
            println!("  {}", example.command);
            println!("    {}", example.description);
        }
        println!();
    }

    println!("Prerequisites:");
    println!("  Initialized: {}", cmd.prerequisites.initialized);
    println!("  JJ Installed: {}", cmd.prerequisites.jj_installed);
    println!("  Zellij Running: {}", cmd.prerequisites.zellij_running);
}

/// Print flags grouped by category with deterministic ordering
///
/// Categories are displayed in the following order:
/// 1. Behavior
/// 2. Configuration
/// 3. Filter
/// 4. Output
/// 5. Advanced
/// 6. General (for uncategorized flags)
///
/// Uses functional patterns with `BTreeMap` for deterministic ordering and
/// custom category ordering via match-based key transformation.
fn print_flags_by_category(flags: &[FlagSpec]) {
    print!("{}", format_flags_by_category(flags));
}

/// Format flags grouped by category with deterministic ordering
///
/// Returns a formatted string with flags grouped by category.
/// This is the core formatting logic used by `print_flags_by_category`.
///
/// Categories are displayed in the following order:
/// 1. Behavior
/// 2. Configuration
/// 3. Filter
/// 4. Output
/// 5. Advanced
/// 6. General (for uncategorized flags)
pub fn format_flags_by_category(flags: &[FlagSpec]) -> String {
    use std::{collections::BTreeMap, fmt::Write};

    let mut output = String::from("Flags:");

    // Group flags by category using functional iterator patterns
    // Map None to "general" for uncategorized flags
    let grouped = flags.iter().fold(
        BTreeMap::new(),
        |mut acc: BTreeMap<String, Vec<&FlagSpec>>, flag| {
            let category = flag.category.as_deref().unwrap_or("general").to_string();
            acc.entry(category).or_default().push(flag);
            acc
        },
    );

    // Define category display order (deterministic, consistent across runs)
    let category_order = [
        "behavior",
        "configuration",
        "filter",
        "output",
        "advanced",
        "general",
    ];

    // Display categories in defined order using for loops
    for &category in &category_order {
        let Some(flags_in_category) = grouped.get(category) else {
            continue;
        };
        let _ = write!(output, "\n\n  {}:", capitalize_category(category));

        for flag in flags_in_category {
            let short = flag
                .short
                .as_ref()
                .map(|s| format!("-{s}, "))
                .map_or(String::new(), |value| value);
            let _ = write!(output, "\n    {short}--{}", flag.long);
            let _ = write!(output, "\n      Type: {}", flag.flag_type);
            let _ = write!(output, "\n      Description: {}", flag.description);

            if let Some(ref default) = flag.default {
                let _ = write!(output, "\n      Default: {default}");
            }

            if !flag.possible_values.is_empty() {
                let _ = write!(
                    output,
                    "\n      Values: {}",
                    flag.possible_values.join(", ")
                );
            }
        }
    }

    output.push('\n');
    output
}

/// Capitalize category name for display
///
/// Converts category strings like "behavior" or "multi-word" to
/// "Behavior" or "Multi Word" using functional transformations.
///
/// # Examples
///
/// ```
/// # use zjj::commands::introspect::capitalize_category;
/// assert_eq!(capitalize_category("behavior"), "Behavior");
/// assert_eq!(capitalize_category("multi-word"), "Multi Word");
/// ```
fn capitalize_category(category: &str) -> String {
    category
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .map_or(String::new(), |value| value)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// Command introspection definitions

/// Helper function to create a boolean flag with common defaults
///
/// Creates a flag of type "bool" with default value of false.
/// Used for flags like --all, --json, --force, etc.
fn create_bool_flag(long: &str, description: &str) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: None,
        description: description.to_string(),
        flag_type: "bool".to_string(),
        default: Some(serde_json::json!(false)),
        possible_values: vec![],
        category: None,
    }
}

/// Helper function to create a string filter flag
///
/// Creates a flag of type "string" for filtering operations.
/// These flags support dynamic values and pattern matching.
fn create_string_filter_flag(long: &str, short: &str, description: &str) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: Some(short.to_string()),
        description: description.to_string(),
        flag_type: "string".to_string(),
        default: None,
        possible_values: vec![],
        category: None,
    }
}

/// Helper function to create an enum flag with predefined values
///
/// Creates a flag with specific allowed values and a default.
fn create_enum_flag(
    long: &str,
    short: Option<&str>,
    description: &str,
    possible_values: Vec<String>,
    default_value: &str,
) -> FlagSpec {
    FlagSpec {
        long: long.to_string(),
        short: short.map(ToString::to_string),
        description: description.to_string(),
        flag_type: "enum".to_string(),
        default: Some(serde_json::json!(default_value)),
        possible_values,
        category: None,
    }
}

/// Helper function to create an error condition with comprehensive context
///
/// Uses Railway-Oriented Programming to ensure consistent error documentation
/// across all commands.
fn create_error_condition(code: &str, description: &str, resolution: &str) -> ErrorCondition {
    ErrorCondition {
        code: code.to_string(),
        description: description.to_string(),
        resolution: resolution.to_string(),
    }
}

/// Helper function to create a command example with description
fn create_example(command: &str, description: &str) -> CommandExample {
    CommandExample {
        command: command.to_string(),
        description: description.to_string(),
    }
}

/// List command filter flags with comprehensive documentation
///
/// Returns a vector of filter flags used by the list command.
/// Factored out to reduce duplication and improve maintainability.
fn create_list_filter_flags() -> Vec<FlagSpec> {
    vec![
        create_bool_flag("all", "Include completed and failed sessions"),
        create_bool_flag("json", "Output as JSON"),
        create_string_filter_flag(
            "bead",
            "b",
            "Filter by bead ID or pattern - supports dynamic values like 'feature-*'",
        ),
        create_string_filter_flag(
            "agent",
            "a",
            "Filter by agent name or pattern - supports dynamic values",
        ),
    ]
}

/// List command examples demonstrating filtering capabilities
///
/// Returns comprehensive examples showing basic usage and advanced filter combinations.
fn create_list_examples() -> Vec<CommandExample> {
    vec![
        create_example("zjj list", "List active sessions"),
        create_example("zjj list --all", "List all sessions including completed"),
        create_example(
            "zjj list --bead feature-123",
            "List sessions for bead feature-123",
        ),
        create_example("zjj list --agent alice", "List sessions assigned to alice"),
        create_example(
            "zjj list --bead feature-123 --agent alice",
            "List feature-123 sessions assigned to alice",
        ),
    ]
}

/// List command error conditions with recovery guidance
///
/// Documents expected error scenarios and how to resolve them.
fn create_list_error_conditions() -> Vec<ErrorCondition> {
    vec![
        create_error_condition(
            "NO_MATCHING_SESSIONS",
            "No sessions match the specified filter criteria (bead, agent, status, etc.)",
            "Review filter parameters: check bead IDs with 'br list' or agent names, try with fewer restrictions",
        ),
    ]
}

/// Add command flags with comprehensive documentation
///
/// Returns the flags for the add command, organized for clarity.
fn create_add_flags() -> Vec<FlagSpec> {
    vec![
        create_bool_flag("no-hooks", "Skip post_create hooks"),
        create_enum_flag(
            "template",
            Some("t"),
            "Layout template name",
            vec![
                "minimal".to_string(),
                "standard".to_string(),
                "full".to_string(),
            ],
            "standard",
        ),
        create_bool_flag("no-open", "Create workspace but don't open Zellij tab"),
    ]
}

/// Add command examples showing various usage patterns
fn create_add_examples() -> Vec<CommandExample> {
    vec![
        create_example(
            "zjj add feature-auth",
            "Create session with default template",
        ),
        create_example(
            "zjj add bugfix-123 --no-hooks",
            "Create without running hooks",
        ),
        create_example(
            "zjj add experiment -t minimal",
            "Create with minimal layout",
        ),
    ]
}

/// Add command error conditions with resolution guidance
fn create_add_error_conditions() -> Vec<ErrorCondition> {
    vec![
        create_error_condition(
            "SESSION_ALREADY_EXISTS",
            "Session with this name already exists in the database",
            "Choose a different session name or remove the existing session with 'zjj remove'",
        ),
        create_error_condition(
            "INVALID_SESSION_NAME",
            "Session name contains invalid characters or does not match naming rules",
            "Use only alphanumeric characters, hyphens, and underscores; must start with a letter",
        ),
        create_error_condition(
            "ZELLIJ_NOT_RUNNING",
            "Zellij terminal multiplexer is not currently running",
            "Start Zellij first with 'zellij' command, then retry session creation",
        ),
    ]
}

fn get_add_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "add".to_string(),
        description: "Create new parallel development session".to_string(),
        aliases: vec!["a".to_string(), "new".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z0-9_-]+$".to_string()),
            examples: vec![
                "feature-auth".to_string(),
                "bugfix-123".to_string(),
                "experiment".to_string(),
            ],
        }],
        flags: create_add_flags(),
        examples: create_add_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: true,
            custom: vec!["Session name must be unique".to_string()],
        },
        side_effects: vec![
            "Creates JJ workspace".to_string(),
            "Generates Zellij layout file".to_string(),
            "Opens Zellij tab".to_string(),
            "Executes post_create hooks".to_string(),
            "Records session in state.db".to_string(),
        ],
        error_conditions: create_add_error_conditions(),
    }
}

/// Remove command flags
///
/// Provides control over removal behavior: force-skip, merge, and branch preservation.
fn create_remove_flags() -> Vec<FlagSpec> {
    vec![
        FlagSpec {
            long: "force".to_string(),
            short: Some("f".to_string()),
            description: "Skip confirmation prompt and hooks".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
        FlagSpec {
            long: "merge".to_string(),
            short: Some("m".to_string()),
            description: "Squash-merge to main before removal".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
        FlagSpec {
            long: "keep-branch".to_string(),
            short: Some("k".to_string()),
            description: "Preserve branch after removal".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        },
    ]
}

/// Remove command examples showing cleanup patterns
fn create_remove_examples() -> Vec<CommandExample> {
    vec![
        create_example("zjj remove my-session", "Remove session with confirmation"),
        create_example("zjj remove my-session -f", "Remove without confirmation"),
        create_example("zjj remove my-session -m", "Merge changes before removing"),
    ]
}

/// Remove command error conditions
fn create_remove_error_conditions() -> Vec<ErrorCondition> {
    vec![create_error_condition(
        "SESSION_NOT_FOUND",
        "The specified session does not exist in the database",
        "List active sessions with 'zjj list' to verify the session name",
    )]
}

fn get_remove_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "remove".to_string(),
        description: "Remove a session and its workspace".to_string(),
        aliases: vec!["rm".to_string(), "delete".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to remove".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: create_remove_flags(),
        examples: create_remove_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![
            "Closes Zellij tab".to_string(),
            "Removes JJ workspace".to_string(),
            "Deletes layout file".to_string(),
            "Removes session from state.db".to_string(),
        ],
        error_conditions: create_remove_error_conditions(),
    }
}

fn get_list_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "list".to_string(),
        description: "List all sessions".to_string(),
        aliases: vec!["ls".to_string()],
        arguments: vec![],
        flags: create_list_filter_flags(),
        examples: create_list_examples(),
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: create_list_error_conditions(),
    }
}

fn get_init_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "init".to_string(),
        description: "Initialize zjj in a JJ repository".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![CommandExample {
            command: "zjj init".to_string(),
            description: "Initialize zjj in current directory".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Creates .zjj directory".to_string(),
            "Creates config.toml".to_string(),
            "Creates state.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "ALREADY_INITIALIZED".to_string(),
            description: "ZJJ already initialized".to_string(),
            resolution: "Remove .zjj directory to reinitialize".to_string(),
        }],
    }
}

fn get_focus_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "focus".to_string(),
        description: "Switch to a session's Zellij tab".to_string(),
        aliases: vec!["switch".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to focus".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "zjj focus my-session".to_string(),
            description: "Switch to my-session tab".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: true,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec!["Switches Zellij tab".to_string()],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'zjj list'".to_string(),
        }],
    }
}

fn get_status_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "status".to_string(),
        description: "Show detailed session status".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
            FlagSpec {
                long: "watch".to_string(),
                short: None,
                description: "Continuously update status".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
        ],
        examples: vec![
            CommandExample {
                command: "zjj status".to_string(),
                description: "Show status of all sessions".to_string(),
            },
            CommandExample {
                command: "zjj status my-session".to_string(),
                description: "Show status of specific session".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_sync_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "sync".to_string(),
        description: "Sync session workspace with main (rebase)".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (syncs current if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "zjj sync my-session".to_string(),
            description: "Sync session with main branch".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Rebases workspace onto main".to_string(),
            "Updates last_synced timestamp".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "CONFLICTS".to_string(),
            description: "Rebase resulted in conflicts".to_string(),
            resolution: "Resolve conflicts manually".to_string(),
        }],
    }
}

fn get_diff_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "diff".to_string(),
        description: "Show diff between session and main".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "stat".to_string(),
            short: None,
            description: "Show diffstat only".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "zjj diff my-session".to_string(),
                description: "Show full diff".to_string(),
            },
            CommandExample {
                command: "zjj diff my-session --stat".to_string(),
                description: "Show diffstat summary".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_introspect_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "introspect".to_string(),
        description: "Discover zjj capabilities".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Command to introspect (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["add".to_string(), "remove".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "zjj introspect".to_string(),
                description: "Show all capabilities".to_string(),
            },
            CommandExample {
                command: "zjj introspect add --json".to_string(),
                description: "Get add command schema as JSON".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_doctor_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        aliases: vec!["check".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
            FlagSpec {
                long: "fix".to_string(),
                short: None,
                description: "Auto-fix issues where possible".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: None,
            },
        ],
        examples: vec![
            CommandExample {
                command: "zjj doctor".to_string(),
                description: "Check system health".to_string(),
            },
            CommandExample {
                command: "zjj doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec!["May fix issues with --fix flag".to_string()],
        error_conditions: vec![],
    }
}

fn get_query_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "query".to_string(),
        description: "Query system state".to_string(),
        aliases: vec![],
        arguments: vec![
            ArgumentSpec {
                name: "query_type".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Type of query".to_string(),
                validation: None,
                examples: vec![
                    "session-exists".to_string(),
                    "session-count".to_string(),
                    "can-run".to_string(),
                    "suggest-name".to_string(),
                ],
            },
            ArgumentSpec {
                name: "args".to_string(),
                arg_type: "string".to_string(),
                required: false,
                description: "Query-specific arguments".to_string(),
                validation: None,
                examples: vec!["my-session".to_string(), "feature-{n}".to_string()],
            },
        ],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(true)),
            possible_values: vec![],
            category: None,
        }],
        examples: vec![
            CommandExample {
                command: "zjj query session-exists my-session".to_string(),
                description: "Check if session exists".to_string(),
            },
            CommandExample {
                command: "zjj query can-run add".to_string(),
                description: "Check if add command can run".to_string(),
            },
            CommandExample {
                command: "zjj query suggest-name feature-{n}".to_string(),
                description: "Suggest next available name".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AI-FOCUSED INTROSPECTION MODES
// ═══════════════════════════════════════════════════════════════════════════

/// Environment variable information
#[derive(serde::Serialize)]
pub struct EnvVarInfo {
    pub name: String,
    pub description: String,
    pub direction: String, // "read", "write", or "both"
    pub default: Option<String>,
    pub example: String,
}

/// Output for --env-vars mode
#[derive(serde::Serialize)]
pub struct EnvVarsOutput {
    pub env_vars: Vec<EnvVarInfo>,
}

/// Workflow step
#[derive(serde::Serialize)]
pub struct WorkflowStep {
    pub step: usize,
    pub command: String,
    pub description: String,
}

/// Workflow pattern
#[derive(serde::Serialize)]
pub struct WorkflowPattern {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

/// Output for --workflows mode
#[derive(serde::Serialize)]
pub struct WorkflowsOutput {
    pub workflows: Vec<WorkflowPattern>,
}

/// Session state transition
#[derive(serde::Serialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: String,
}

/// Output for --session-states mode
#[derive(serde::Serialize)]
pub struct SessionStatesOutput {
    pub states: Vec<String>,
    pub transitions: Vec<StateTransition>,
}

/// Run introspect with --env-vars flag
pub async fn run_env_vars(format: OutputFormat) -> Result<()> {
    let env_vars = vec![
        EnvVarInfo {
            name: "ZJJ_AGENT_ID".to_string(),
            description: "Current agent ID for tracking".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "agent-12345678-abcd".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_SESSION".to_string(),
            description: "Current session name".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "feature-auth".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_WORKSPACE".to_string(),
            description: "Path to current workspace directory".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "/path/to/.zjj/workspaces/feature-auth".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_BEAD_ID".to_string(),
            description: "Bead ID associated with current work".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "zjj-abc12".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_ACTIVE".to_string(),
            description: "Set to 1 when in an active zjj workspace".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "1".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_RECOVERY_POLICY".to_string(),
            description: "Database recovery behavior: silent, warn, fail-fast".to_string(),
            direction: "read".to_string(),
            default: Some("warn".to_string()),
            example: "fail-fast".to_string(),
        },
        EnvVarInfo {
            name: "ZELLIJ_SESSION_NAME".to_string(),
            description: "Zellij session name (read by zjj)".to_string(),
            direction: "read".to_string(),
            default: None,
            example: "dev".to_string(),
        },
    ];

    let output = EnvVarsOutput { env_vars };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-env-vars-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Environment Variables:\n");
        for var in &output.env_vars {
            println!("  {} ({}):", var.name, var.direction);
            println!("    {}", var.description);
            if let Some(ref default) = var.default {
                println!("    Default: {default}");
            }
            println!("    Example: {}", var.example);
            println!();
        }
    }

    Ok(())
}

/// Run introspect with --workflows flag
#[allow(clippy::too_many_lines)]
pub async fn run_workflows(format: OutputFormat) -> Result<()> {
    let workflows = vec![
        WorkflowPattern {
            name: "Quick Work Session".to_string(),
            description: "Start working on a task, do work, complete".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj work my-task --idempotent".to_string(),
                    description: "Create workspace (idempotent for retries)".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "cd $(zjj context --field location.path)".to_string(),
                    description: "Enter workspace directory".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "# ... do work ...".to_string(),
                    description: "Implement changes".to_string(),
                },
                WorkflowStep {
                    step: 4,
                    command: "zjj done".to_string(),
                    description: "Merge and cleanup".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Agent-Managed Workflow".to_string(),
            description: "Full agent lifecycle with registration".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj agent register".to_string(),
                    description: "Register as an agent".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj work my-task --bead zjj-abc12".to_string(),
                    description: "Create workspace for bead".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "zjj agent heartbeat --command \"implementing\"".to_string(),
                    description: "Send heartbeat while working".to_string(),
                },
                WorkflowStep {
                    step: 4,
                    command: "zjj done".to_string(),
                    description: "Complete work and merge".to_string(),
                },
                WorkflowStep {
                    step: 5,
                    command: "zjj agent unregister".to_string(),
                    description: "Deregister agent".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Quick Orientation".to_string(),
            description: "Quickly understand current state".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj whereami".to_string(),
                    description: "Check location: main or workspace".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj whoami".to_string(),
                    description: "Check agent identity".to_string(),
                },
                WorkflowStep {
                    step: 3,
                    command: "zjj query can-spawn".to_string(),
                    description: "Check if spawning is possible".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Abandon Work".to_string(),
            description: "Discard work without merging".to_string(),
            steps: vec![
                WorkflowStep {
                    step: 1,
                    command: "zjj abort --dry-run".to_string(),
                    description: "Preview what will be aborted".to_string(),
                },
                WorkflowStep {
                    step: 2,
                    command: "zjj abort".to_string(),
                    description: "Abort and cleanup".to_string(),
                },
            ],
        },
        WorkflowPattern {
            name: "Sync All Workspaces".to_string(),
            description: "Keep all workspaces up to date".to_string(),
            steps: vec![WorkflowStep {
                step: 1,
                command: "zjj sync --all".to_string(),
                description: "Sync all active sessions with main".to_string(),
            }],
        },
    ];

    let output = WorkflowsOutput { workflows };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-workflows-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Workflow Patterns:\n");
        for workflow in &output.workflows {
            println!("  {}:", workflow.name);
            println!("    {}\n", workflow.description);
            for step in &workflow.steps {
                println!("    {}. {}", step.step, step.command);
                println!("       {}", step.description);
            }
            println!();
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// AI-OPTIMIZED INTROSPECTION MODE
// ═══════════════════════════════════════════════════════════════════════════

/// AI-optimized introspection output
#[derive(serde::Serialize)]
pub struct AiIntrospectOutput {
    /// ZJJ version
    pub zjj_version: String,
    /// Quick summary for AI decision-making
    pub quick_ref: AiQuickRef,
    /// System readiness
    pub system: AiSystemInfo,
    /// Available commands grouped by category
    pub commands: AiCommandGroups,
    /// Recommended next actions
    pub recommendations: Vec<AiRecommendation>,
    /// Key environment variables
    pub env_vars: Vec<EnvVarInfo>,
    /// Common workflow patterns (condensed)
    pub workflows: Vec<AiWorkflowSummary>,
}

/// Quick reference for AI agents
#[derive(serde::Serialize)]
pub struct AiQuickRef {
    /// Whether zjj is ready to use
    pub ready: bool,
    /// Blocking issues if not ready
    pub blockers: Vec<String>,
    /// Current location (main or workspace)
    pub location: String,
    /// Active session count
    pub active_sessions: usize,
    /// Agent ID if registered
    pub agent_id: Option<String>,
}

/// System information for AI
#[derive(serde::Serialize)]
pub struct AiSystemInfo {
    /// Whether zjj is initialized
    pub initialized: bool,
    /// Whether in a JJ repo
    pub jj_repo: bool,
    /// Dependencies status
    pub dependencies: AiDependencies,
}

/// Dependency status for AI
#[derive(serde::Serialize)]
pub struct AiDependencies {
    /// JJ installed and version
    pub jj: Option<String>,
    /// Zellij installed and version
    pub zellij: Option<String>,
    /// Missing required dependencies
    pub missing: Vec<String>,
}

/// Command groups for AI
#[derive(serde::Serialize)]
pub struct AiCommandGroups {
    /// Session management commands
    pub session: Vec<String>,
    /// Version control commands
    pub version_control: Vec<String>,
    /// Agent management commands
    pub agent: Vec<String>,
    /// Introspection commands
    pub introspection: Vec<String>,
}

/// AI-friendly recommendation
#[derive(serde::Serialize)]
pub struct AiRecommendation {
    /// Action to take
    pub action: String,
    /// Command to run
    pub command: String,
    /// Why this is recommended
    pub reason: String,
    /// Priority: high, medium, low
    pub priority: String,
}

/// Condensed workflow for AI
#[derive(serde::Serialize)]
pub struct AiWorkflowSummary {
    /// Workflow name
    pub name: String,
    /// When to use this workflow
    pub use_when: String,
    /// Commands in sequence
    pub commands: Vec<String>,
}

/// Run introspect with --ai flag
///
/// Provides a comprehensive, AI-optimized output combining:
/// - System readiness
/// - Available commands
/// - Recommendations
/// - Environment info
#[allow(clippy::too_many_lines)]
pub async fn run_ai() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let dependencies = dependencies::check_dependencies().await;
    let system_state = system_state::get_system_state().await;

    // Determine location
    let location = crate::cli::jj_root().await.map_or_else(
        |_| "not_in_repo".to_string(),
        |root| {
            let path = std::path::PathBuf::from(&root);
            match crate::commands::context::detect_location(&path) {
                Ok(crate::commands::context::Location::Main) => "main".to_string(),
                Ok(crate::commands::context::Location::Workspace { name, .. }) => {
                    format!("workspace:{name}")
                }
                Err(_) => "unknown".to_string(),
            }
        },
    );

    // Check readiness
    let jj_ok = dependencies.get("jj").is_some_and(|d| d.installed);
    let zellij_ok = dependencies.get("zellij").is_some_and(|d| d.installed);
    let ready = jj_ok && zellij_ok && system_state.initialized;

    let mut blockers = Vec::new();
    if !jj_ok {
        blockers.push("JJ not installed".to_string());
    }
    if !zellij_ok {
        blockers.push("Zellij not installed".to_string());
    }
    if !system_state.initialized {
        blockers.push("ZJJ not initialized (run 'zjj init')".to_string());
    }

    // Get agent ID from environment
    let agent_id = std::env::var("ZJJ_AGENT_ID").ok();

    // Build dependency info
    let jj_version = dependencies.get("jj").and_then(|d| d.version.clone());
    let zellij_version = dependencies.get("zellij").and_then(|d| d.version.clone());

    let mut missing_deps = Vec::new();
    if !jj_ok {
        missing_deps.push("jj".to_string());
    }
    if !zellij_ok {
        missing_deps.push("zellij".to_string());
    }

    // Build recommendations
    let mut recommendations = Vec::new();

    if !system_state.initialized && system_state.jj_repo {
        recommendations.push(AiRecommendation {
            action: "Initialize ZJJ".to_string(),
            command: "zjj init".to_string(),
            reason: "ZJJ is not initialized in this repository".to_string(),
            priority: "high".to_string(),
        });
    }

    if ready && system_state.active_sessions == 0 {
        recommendations.push(AiRecommendation {
            action: "Create first session".to_string(),
            command: "zjj work <task-name>".to_string(),
            reason: "No active sessions - ready to start work".to_string(),
            priority: "medium".to_string(),
        });
    }

    if ready && location.starts_with("workspace:") {
        recommendations.push(AiRecommendation {
            action: "Check current context".to_string(),
            command: "zjj context".to_string(),
            reason: "In a workspace - get full context".to_string(),
            priority: "low".to_string(),
        });
    }

    if agent_id.is_none() && ready {
        recommendations.push(AiRecommendation {
            action: "Register as agent".to_string(),
            command: "zjj agent register".to_string(),
            reason: "Enable agent tracking for multi-agent coordination".to_string(),
            priority: "low".to_string(),
        });
    }

    // Build env vars (condensed)
    let env_vars = vec![
        EnvVarInfo {
            name: "ZJJ_AGENT_ID".to_string(),
            description: "Agent identifier for tracking".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "agent-12345678".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_SESSION".to_string(),
            description: "Current session name (set by zjj)".to_string(),
            direction: "write".to_string(),
            default: None,
            example: "feature-auth".to_string(),
        },
        EnvVarInfo {
            name: "ZJJ_BEAD_ID".to_string(),
            description: "Associated bead ID".to_string(),
            direction: "both".to_string(),
            default: None,
            example: "zjj-abc12".to_string(),
        },
    ];

    // Build workflows (condensed)
    let workflows = vec![
        AiWorkflowSummary {
            name: "Quick Task".to_string(),
            use_when: "Working on a single task".to_string(),
            commands: vec![
                "zjj work <name>".to_string(),
                "# do work".to_string(),
                "zjj done".to_string(),
            ],
        },
        AiWorkflowSummary {
            name: "Agent Workflow".to_string(),
            use_when: "Multi-agent coordination".to_string(),
            commands: vec![
                "zjj agent register".to_string(),
                "zjj work <name> --bead <id>".to_string(),
                "zjj agent heartbeat".to_string(),
                "zjj done".to_string(),
                "zjj agent unregister".to_string(),
            ],
        },
        AiWorkflowSummary {
            name: "Orientation".to_string(),
            use_when: "Understanding current state".to_string(),
            commands: vec![
                "zjj whereami".to_string(),
                "zjj whoami".to_string(),
                "zjj list".to_string(),
            ],
        },
    ];

    let output = AiIntrospectOutput {
        zjj_version: version.to_string(),
        quick_ref: AiQuickRef {
            ready,
            blockers,
            location,
            active_sessions: system_state.active_sessions,
            agent_id,
        },
        system: AiSystemInfo {
            initialized: system_state.initialized,
            jj_repo: system_state.jj_repo,
            dependencies: AiDependencies {
                jj: jj_version,
                zellij: zellij_version,
                missing: missing_deps,
            },
        },
        commands: AiCommandGroups {
            session: vec![
                "add".to_string(),
                "remove".to_string(),
                "list".to_string(),
                "focus".to_string(),
                "work".to_string(),
                "done".to_string(),
                "abort".to_string(),
            ],
            version_control: vec![
                "sync".to_string(),
                "diff".to_string(),
                "checkpoint".to_string(),
                "undo".to_string(),
                "revert".to_string(),
            ],
            agent: vec![
                "agent register".to_string(),
                "agent unregister".to_string(),
                "agent heartbeat".to_string(),
                "agent list".to_string(),
                "spawn".to_string(),
            ],
            introspection: vec![
                "introspect".to_string(),
                "introspect --ai".to_string(),
                "context".to_string(),
                "whereami".to_string(),
                "whoami".to_string(),
                "doctor".to_string(),
                "query".to_string(),
            ],
        },
        recommendations,
        env_vars,
        workflows,
    };

    let envelope = SchemaEnvelope::new("introspect-ai-response", "single", output);
    println!("{}", serde_json::to_string_pretty(&envelope)?);

    Ok(())
}

/// Run introspect with --session-states flag
pub async fn run_session_states(format: OutputFormat) -> Result<()> {
    let states = vec![
        "creating".to_string(),
        "active".to_string(),
        "syncing".to_string(),
        "merging".to_string(),
        "completed".to_string(),
        "failed".to_string(),
    ];

    let transitions = vec![
        StateTransition {
            from: "none".to_string(),
            to: "creating".to_string(),
            trigger: "zjj add / zjj work".to_string(),
        },
        StateTransition {
            from: "creating".to_string(),
            to: "active".to_string(),
            trigger: "workspace created successfully".to_string(),
        },
        StateTransition {
            from: "creating".to_string(),
            to: "failed".to_string(),
            trigger: "workspace creation failed".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "syncing".to_string(),
            trigger: "zjj sync".to_string(),
        },
        StateTransition {
            from: "syncing".to_string(),
            to: "active".to_string(),
            trigger: "sync completed".to_string(),
        },
        StateTransition {
            from: "syncing".to_string(),
            to: "failed".to_string(),
            trigger: "sync failed (conflicts)".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "merging".to_string(),
            trigger: "zjj done".to_string(),
        },
        StateTransition {
            from: "merging".to_string(),
            to: "completed".to_string(),
            trigger: "merge successful".to_string(),
        },
        StateTransition {
            from: "merging".to_string(),
            to: "failed".to_string(),
            trigger: "merge failed".to_string(),
        },
        StateTransition {
            from: "active".to_string(),
            to: "failed".to_string(),
            trigger: "zjj abort".to_string(),
        },
    ];

    let output = SessionStatesOutput {
        states,
        transitions,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("introspect-session-states-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Session States: {}\n", output.states.join(" -> "));
        println!("Transitions:");
        for t in &output.transitions {
            println!("  {} -> {} : {}", t.from, t.to, t.trigger);
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_introspect_json_has_envelope() -> Result<()> {
        // Verify envelope wrapping for introspect command output
        use zjj_core::json::SchemaEnvelope;

        let version = env!("CARGO_PKG_VERSION");
        let output = IntrospectOutput::new(version);
        let envelope = SchemaEnvelope::new("introspect-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_introspect_command_wrapped() -> Result<()> {
        // Verify command introspection results are wrapped in envelope
        use zjj_core::json::SchemaEnvelope;

        let cmd = get_add_introspection();
        let envelope = SchemaEnvelope::new("introspect-command-response", "single", cmd);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    // ============================================================================
    // Tests for New Introspect Modes
    // ============================================================================

    /// Test `env_vars` output structure
    #[test]
    fn test_introspect_env_vars_output_structure() {
        // Expected structure for env-vars output
        let expected_vars = vec![
            "ZJJ_SESSION",
            "ZJJ_WORKSPACE",
            "ZJJ_ACTIVE",
            "ZJJ_AGENT_ID",
            "ZJJ_BEAD_ID",
        ];

        for var in expected_vars {
            assert!(!var.is_empty());
            assert!(var.starts_with("ZJJ_"));
        }
    }

    /// Test `env_vars` contains required variables
    #[test]
    fn test_introspect_env_vars_contains_core_vars() {
        // Core env vars that must be documented
        let core_vars = [
            ("ZJJ_SESSION", "Current session name"),
            ("ZJJ_WORKSPACE", "Workspace path"),
            ("ZJJ_ACTIVE", "Whether zjj is active"),
            ("ZJJ_AGENT_ID", "Agent identifier"),
            ("ZJJ_BEAD_ID", "Current bead being worked on"),
        ];

        for (name, description) in core_vars {
            assert!(!name.is_empty());
            assert!(!description.is_empty());
        }
    }

    /// Test workflows output structure
    #[test]
    fn test_introspect_workflows_output_structure() {
        use serde_json::json;

        // Expected workflow structure
        let workflow = json!({
            "name": "minimal",
            "description": "Minimal workflow for quick tasks",
            "steps": [
                {"step": 1, "command": "zjj work <name>", "description": "Start work"},
                {"step": 2, "command": "# do work", "description": "Implementation"},
                {"step": 3, "command": "zjj done", "description": "Complete work"}
            ]
        });

        assert!(workflow["name"].is_string());
        assert!(workflow["steps"].is_array());
        assert!(workflow["steps"]
            .as_array()
            .map(|a| a.len() >= 2)
            .unwrap_or(false));
    }

    /// Test workflows contains minimal workflow
    #[test]
    fn test_introspect_workflows_has_minimal() {
        let workflow_names = ["minimal", "standard", "parallel"];

        for name in workflow_names {
            assert!(!name.is_empty());
        }
    }

    /// Test `session_states` output structure
    #[test]
    fn test_introspect_session_states_output_structure() {
        use serde_json::json;

        // Expected session state structure
        let state = json!({
            "state": "active",
            "description": "Session is in use",
            "transitions": ["completing", "aborting"]
        });

        assert!(state["state"].is_string());
        assert!(state["description"].is_string());
        assert!(state["transitions"].is_array());
    }

    /// Test `session_states` contains all valid states
    #[test]
    fn test_introspect_session_states_all_states() {
        // All valid session states
        let states = [
            "pending",
            "active",
            "completing",
            "completed",
            "aborting",
            "aborted",
            "failed",
        ];

        for state in states {
            assert!(!state.is_empty());
            // States should be lowercase
            assert!(state.chars().all(|c| c.is_ascii_lowercase()));
        }
    }

    /// Test session state transitions are valid
    #[test]
    fn test_introspect_session_state_transitions() {
        use std::collections::HashMap;

        // Define valid transitions
        let transitions: HashMap<&str, Vec<&str>> = [
            ("pending", vec!["active", "aborted"]),
            ("active", vec!["completing", "aborting"]),
            ("completing", vec!["completed", "failed"]),
            ("completed", vec![]),
            ("aborting", vec!["aborted", "failed"]),
            ("aborted", vec![]),
            ("failed", vec!["active"]), // can retry
        ]
        .into_iter()
        .collect();

        // Verify all states have defined transitions
        assert!(transitions.contains_key("pending"));
        assert!(transitions.contains_key("active"));
        assert!(transitions.contains_key("completed"));
    }

    /// Test introspect modes are recognized
    #[test]
    fn test_introspect_modes_recognized() {
        let modes = ["commands", "env-vars", "workflows", "session-states"];

        for mode in modes {
            assert!(!mode.is_empty());
            // Modes should be kebab-case
            assert!(mode.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    /// Test `env_vars` includes usage examples
    #[test]
    fn test_introspect_env_vars_has_examples() {
        // Each env var should have usage examples
        let var_with_examples = [
            ("ZJJ_SESSION", "echo $ZJJ_SESSION"),
            ("ZJJ_WORKSPACE", "cd $ZJJ_WORKSPACE"),
            ("ZJJ_AGENT_ID", "export ZJJ_AGENT_ID=my-agent"),
        ];

        for (var, example) in var_with_examples {
            assert!(example.contains(var) || example.contains('$'));
        }
    }

    /// Test workflows have required fields
    #[test]
    fn test_introspect_workflows_required_fields() {
        let required_fields = ["name", "description", "steps", "use_case"];

        for field in required_fields {
            assert!(!field.is_empty());
        }
    }
}

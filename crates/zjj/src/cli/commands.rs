//! CLI command definitions using `clap`

use clap::{Arg, Command as ClapCommand};

use crate::cli::json_docs;

pub fn after_help_text(examples: &[&str], json_output: Option<&'static str>) -> String {
    let mut text = String::from("EXAMPLES:\n");
    for example in examples {
        text.push_str("  ");
        text.push_str(example);
        text.push('\n');
    }
    if let Some(json) = json_output {
        text.push('\n');
        text.push_str(json);
        if !json.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}

pub fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize zjj in a JJ repository (or create one)")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .after_help(after_help_text(
            &[
                "zjj init                        Initialize ZJJ in the current JJ repository",
                "zjj init --json                 Output JSON metadata for automation",
                "zjj init                        Reinitialize after deleting .zjj to refresh helpers",
            ],
            Some(json_docs::init()),
        ))
}

pub fn cmd_attach() -> ClapCommand {
    ClapCommand::new("attach")
        .about("Enter Zellij session from outside (shell → Zellij)")
        .long_about(
            "Use this when you are in a regular shell and want to enter the Zellij session.

            This replaces your current process with Zellij.


            If already inside Zellij, use 'zjj focus' to switch tabs instead.",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .help("Name of the session to attach to"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (only for errors)"),
        )
}

pub fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create session for manual work (JJ workspace + Zellij tab)")
        .long_about(
            "Creates a JJ workspace and Zellij tab for interactive development.

            Use this when YOU will work in the session.


            For automated agent workflows, use 'zjj spawn' instead.",
        )
        .after_help(after_help_text(
            &[
                "zjj add feature-auth              Create session with standard layout",
                "zjj add bugfix-123 --no-open       Create without opening Zellij tab",
                "zjj add experiment -t minimal      Use minimal layout template",
                "zjj add quick-test --no-hooks      Skip post-create hooks",
                "zjj add work --bead zjj-abc123     Associate with bead zjj-abc123",
                "zjj add --example-json            Show example JSON output",
            ],
            Some(json_docs::add()),
        ))
        .arg(
            Arg::new("name")
                .required_unless_present("example-json")
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name for the new session (must start with a letter)"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID")
                .help("Associate this session with a bead/issue ID"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .help("Skip executing post_create hooks"),
        )
        .arg(
            Arg::new("template")
                .short('t')
                .long("template")
                .value_name("TEMPLATE")
                .help("Zellij layout template to use (minimal, standard, full)"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .help("Create workspace without opening Zellij tab"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("example-json")
                .long("example-json")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("name")
                .help("Show example JSON output without executing"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without creating"),
        )
        .arg(
            Arg::new("no-zellij")
                .long("no-zellij")
                .action(clap::ArgAction::SetTrue)
                .help("Skip Zellij integration (for non-TTY environments)"),
        )
}

pub fn cmd_agents() -> ClapCommand {
    ClapCommand::new("agents")
        .alias("agent")
        .about("List and manage agents")
        .long_about(
            "Shows all agents that have recently sent heartbeats, along with their current sessions and any locks they hold.


            Agents are considered active if they've sent a heartbeat within the last 60 seconds.


            Subcommands allow self-management for AI agents.",
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include stale agents (not seen within heartbeat timeout)"),
        )
        .arg(
            Arg::new("session")
                .long("session")
                .value_name("SESSION")
                .help("Filter by session"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
        .subcommand(
            ClapCommand::new("register")
                .about("Register as an agent")
                .long_about(
                    "Register this process as an agent for zjj tracking.


                    Sets ZJJ_AGENT_ID environment variable.

                    Agent ID is auto-generated if not provided.",
                )
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("AGENT_ID")
                        .help("Agent ID to register (auto-generated if not provided)"),
                )
                .arg(
                    Arg::new("session")
                        .long("session")
                        .short('s')
                        .value_name("SESSION")
                        .help("Session to associate with this agent"),
                ),
        )
        .subcommand(
            ClapCommand::new("heartbeat")
                .about("Send a heartbeat to indicate agent is alive")
                .long_about(
                    "Updates the agent's last_seen timestamp.


                    Requires ZJJ_AGENT_ID to be set (run 'zjj agent register' first).",
                )
                .arg(
                    Arg::new("command")
                        .long("command")
                        .short('c')
                        .value_name("COMMAND")
                        .help("Current command being executed"),
                ),
        )
        .subcommand(
            ClapCommand::new("status")
                .about("Show current agent status")
                .long_about(
                    "Shows the status of the currently registered agent.


                    Uses ZJJ_AGENT_ID environment variable.",
                ),
        )
        .subcommand(
            ClapCommand::new("unregister")
                .about("Unregister as an agent")
                .long_about(
                    "Remove this agent from zjj tracking.


                    Clears ZJJ_AGENT_ID environment variable.",
                )
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("AGENT_ID")
                        .help("Agent ID to unregister (uses ZJJ_AGENT_ID if not provided)"),
                ),
        )
}

pub fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .after_help(after_help_text(
            &[
                "zjj list                        Show all active sessions",
                "zjj list --verbose              Include workspace paths and bead titles",
                "zjj list --all --json           Dump every session in JSON",
            ],
            Some(json_docs::list()),
        ))
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(clap::ArgAction::SetTrue)
                .help("Show verbose output with workspace paths and bead titles"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .value_name("BEAD_ID")
                .help("Filter sessions by bead ID"),
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .value_name("NAME")
                .action(clap::ArgAction::Set)
                .help("Filter sessions by agent owner"),
        )
        .arg(
            Arg::new("state")
                .long("state")
                .value_name("STATE")
                .action(clap::ArgAction::Set)
                .help("Filter sessions by workspace state (created, working, ready, merged, abandoned, conflict, active, complete, terminal, non-terminal)"),
        )
}

#[allow(clippy::too_many_lines)]
pub fn cmd_bookmark() -> ClapCommand {
    ClapCommand::new("bookmark")
        .about("Manage JJ bookmarks/branches")
        .long_about(
            "Manage bookmarks (branches) in JJ workspaces.


            zjj wraps JJ completely - use 'zjj bookmark' not 'jj bookmark'.

            Provides: list, create, delete, move operations.",
        )
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("list")
                .about("List bookmarks in a session workspace")
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show all bookmarks including remote"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("create")
                .about("Create a new bookmark at current revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new bookmark"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("push")
                        .long("push")
                        .short('p')
                        .action(clap::ArgAction::SetTrue)
                        .help("Push bookmark to remote after creation"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("delete")
                .about("Delete a bookmark")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to delete"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("move")
                .about("Move a bookmark to a different revision")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the bookmark to move"),
                )
                .arg(
                    Arg::new("to")
                        .long("to")
                        .required(true)
                        .value_name("REVISION")
                        .help("Target revision (commit hash or revset)"),
                )
                .arg(
                    Arg::new("session")
                        .value_name("SESSION")
                        .help("Session name (uses current workspace if omitted)"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
}

pub fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .after_help(after_help_text(
            &[
                "zjj remove old-feature            Remove with confirmation prompt",
                "zjj remove test-session -f        Force removal without prompt",
                "zjj remove feature-x --merge       Merge changes to main first",
                "zjj remove experiment -k -f       Keep branch, force removal",
            ],
            Some(json_docs::remove()),
        ))
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to remove"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt and hooks"),
        )
        .arg(
            Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue)
                .help("Squash-merge to main before removal"),
        )
        .arg(
            Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue)
                .help("Preserve branch after removal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session doesn't exist (safe for retries)"),
        )
}

pub fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to session's Zellij tab (inside Zellij)")
        .long_about(
            "Use this when you are already inside Zellij and want to switch tabs.


            If you are outside Zellij, use 'zjj attach' to enter the session instead.",
        )
        .after_help(after_help_text(
            &[
                "zjj focus feature-auth            Switch to session's Zellij tab",
                "zjj focus                         Interactive session selection",
                "zjj focus bugfix-123 --json       Get JSON output of focus operation",
            ],
            Some(json_docs::focus()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to focus (interactive if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("no-zellij")
                .long("no-zellij")
                .action(clap::ArgAction::SetTrue)
                .help("Skip Zellij integration (for non-TTY environments)"),
        )
}

pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
        .after_help(after_help_text(
            &[
                "zjj status                      Show status for all sessions",
                "zjj status feature-auth         Inspect a specific workspace",
                "zjj status --watch              Watch live updates (JSON available with --json)",
            ],
            Some(json_docs::status()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
}

pub fn cmd_switch() -> ClapCommand {
    ClapCommand::new("switch")
        .about("Switch to a different workspace")
        .long_about(
            "Navigate between workspaces when inside Zellij.


            Use this for quick workspace switching. Similar to 'zjj focus' but 
            emphasizes navigation between existing sessions.",
        )
        .after_help(after_help_text(
            &[
                "zjj switch feature-auth           Switch to named session",
                "zjj switch                        Interactive session selection",
                "zjj switch test --show-context    Switch and show session details",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Name of the session to switch to (interactive if omitted)"),
        )
        .arg(
            Arg::new("show-context")
                .long("show-context")
                .action(clap::ArgAction::SetTrue)
                .help("Show session details after switching"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync a session's workspace with main (rebase)")
        .after_help(after_help_text(
            &[
                "zjj sync feature-auth             Sync named session with main",
                "zjj sync                          Sync current workspace",
                "zjj sync --all                    Sync all active sessions",
                "zjj sync --json                   Get JSON output of sync operation",
            ],
            Some(json_docs::sync()),
        ))
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to sync (syncs current workspace if omitted)"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("name")
                .help("Sync all active sessions"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
        .after_help(after_help_text(
            &[
                "zjj diff feature-auth           Show diff between feature workspace and main",
                "zjj diff feature-auth --stat    Show diffstat summary",
                "zjj diff feature-auth --json    Output diff metadata in JSON",
            ],
            Some(json_docs::diff()),
        ))
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Session name to show diff for"),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .action(clap::ArgAction::SetTrue)
                .help("Show diffstat only (summary of changes)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .after_help(after_help_text(
            &[
                "zjj config                      Show current project config",
                "zjj config workspace_dir        Display the workspace_dir setting",
                "zjj config workspace_dir /new/path --json  Update key and emit JSON",
            ],
            Some(json_docs::config()),
        ))
        .arg(Arg::new("key").help("Config key to view/set (dot notation: 'zellij.use_tabs')"))
        .arg(Arg::new("value").help("Value to set (omit to view)"))
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help("Operate on global config instead of project"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_clean() -> ClapCommand {
    ClapCommand::new("clean")
        .about("Remove stale sessions (where workspace no longer exists)")
        .after_help(after_help_text(
            &[
                "zjj clean                       Remove stale sessions",
                "zjj clean --dry-run             List stale sessions without deleting",
                "zjj clean --force --json        Force clean and emit JSON summary",
            ],
            Some(json_docs::clean()),
        ))
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("List stale sessions without removing"),
        )
        .arg(
            Arg::new("periodic")
                .long("periodic")
                .action(clap::ArgAction::SetTrue)
                .help("Run as periodic cleanup daemon (1hr interval)"),
        )
        .arg(
            Arg::new("age-threshold")
                .long("age-threshold")
                .value_name("SECONDS")
                .help("Age threshold for periodic cleanup (default: 7200 = 2hr)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_template() -> ClapCommand {
    ClapCommand::new("template")
        .about("Manage Zellij layout templates")
        .subcommand(
            ClapCommand::new("list")
                .about("List all available templates")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("create")
                .about("Create a new template")
                .arg(Arg::new("name").required(true).help("Template name"))
                .arg(
                    Arg::new("description")
                        .long("description")
                        .short('d')
                        .help("Template description"),
                )
                .arg(
                    Arg::new("from-file")
                        .long("from-file")
                        .short('f')
                        .help("Import from KDL file"),
                )
                .arg(
                    Arg::new("builtin")
                        .long("builtin")
                        .short('b')
                        .value_parser(["minimal", "standard", "full", "split", "review"])
                        .help("Use builtin template as base"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show template details")
                .arg(Arg::new("name").required(true).help("Template name"))
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("delete")
                .about("Delete a template")
                .arg(Arg::new("name").required(true).help("Template name"))
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
}

pub fn cmd_dashboard() -> ClapCommand {
    ClapCommand::new("dashboard")
        .about("Launch interactive TUI dashboard with kanban view")
        .alias("dash")
        .after_help(after_help_text(
            &[
                "zjj dashboard                  Launch the kanban-style dashboard",
                "zjj dashboard --json           Export dashboard snapshot for agents",
                "zjj dash                       Use the alias to open the TUI quickly",
            ],
            None,
        ))
}

pub fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover zjj capabilities and command details")
        .long_about(
            "AI-optimized capability discovery.


            Use this to understand:
  
            - Available commands and their arguments
  
            - System state and dependencies
  
            - Environment variables zjj uses
  
            - Common workflow patterns",
        )
        .after_help(after_help_text(
            &[
                "zjj introspect                Show commands and their arguments",
                "zjj introspect focus          Inspect focus command contract",
                "zjj introspect --json         Emit machine-readable capability data",
            ],
            Some(json_docs::introspect()),
        ))
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to introspect (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("ai")
                .long("ai")
                .action(clap::ArgAction::SetTrue)
                .help("AI-optimized output: combines capabilities, state, and recommendations"),
        )
        .arg(
            Arg::new("env-vars")
                .long("env-vars")
                .action(clap::ArgAction::SetTrue)
                .help("Show environment variables zjj reads and sets"),
        )
        .arg(
            Arg::new("workflows")
                .long("workflows")
                .action(clap::ArgAction::SetTrue)
                .help("Show common workflow patterns for AI agents"),
        )
        .arg(
            Arg::new("session-states")
                .long("session-states")
                .action(clap::ArgAction::SetTrue)
                .help("Show valid session state transitions"),
        )
}

pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run system health checks")
        .alias("check")
        .after_help(after_help_text(
            &[
                "zjj doctor                    Run all system health checks",
                "zjj doctor --fix              Auto-fix issues where possible",
                "zjj doctor --json             Export check results to JSON",
            ],
            Some(json_docs::doctor()),
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible"),
        )
}

pub fn cmd_integrity() -> ClapCommand {
    ClapCommand::new("integrity")
        .about("Manage workspace integrity and corruption recovery")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("validate")
                .about("Validate workspace integrity")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("repair")
                .about("Repair corrupted workspace")
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name or path"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("backup")
                .about("Manage workspace backups")
                .subcommand_required(true)
                .subcommand(
                    ClapCommand::new("list")
                        .about("List available backups")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                )
                .subcommand(
                    ClapCommand::new("restore")
                        .about("Restore from a backup")
                        .arg(
                            Arg::new("backup_id")
                                .required(true)
                                .help("Backup ID to restore"),
                        )
                        .arg(
                            Arg::new("force")
                                .long("force")
                                .short('f')
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .action(clap::ArgAction::SetTrue)
                                .help("Output as JSON"),
                        ),
                ),
        )
}

pub fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .after_help(after_help_text(
            &[
                "zjj query session-exists feature   Check if session exists",
                "zjj query session-count             Count active sessions",
                "zjj query can-run                   Check if zjj can run",
                "zjj query suggest-name \"feat{n}\"   Get name suggestion",
            ],
            Some(json_docs::query()),
        ))
        .arg(
            Arg::new("query_type")
                .required(true)
                .help("Type of query (session-exists, session-count, can-run, suggest-name)"),
        )
        .arg(
            Arg::new("args")
                .required(false)
                .help("Query-specific arguments"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (default for query)"),
        )
}

pub fn cmd_queue() -> ClapCommand {
    ClapCommand::new("queue")
        .about("Manage merge queue for multi-agent coordination")
        .long_about(
            "Manage the merge queue that coordinates sequential processing of workspaces.


            The merge queue ensures that multiple agents process workspaces in order,

            preventing merge conflicts and ensuring deterministic ordering.",
        )
        .after_help(after_help_text(
            &[
                "zjj queue --list                          Show all queue entries",
                "zjj queue --add <workspace> --bead <id>  Add workspace to queue",
                "zjj queue --next                          Get next pending entry",
                "zjj queue --status <workspace>            Check workspace queue status",
                "zjj queue --remove <workspace>            Remove workspace from queue",
                "zjj queue --stats                         Show queue statistics",
                "zjj queue --list --json                   Show queue as JSON",
            ],
            None,
        ))
        .arg(
            Arg::new("add")
                .long("add")
                .value_name("WORKSPACE")
                .help("Add workspace to queue"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .value_name("BEAD_ID")
                .help("Associate with bead/issue ID (used with --add)"),
        )
        .arg(
            Arg::new("priority")
                .long("priority")
                .value_name("PRIORITY")
                .value_parser(clap::value_parser!(i32))
                .default_value("5")
                .help("Queue priority (lower = higher priority, 1-10)"),
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .value_name("AGENT_ID")
                .help("Agent ID that will process this entry"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .action(clap::ArgAction::SetTrue)
                .help("List all queue entries"),
        )
        .arg(
            Arg::new("next")
                .long("next")
                .action(clap::ArgAction::SetTrue)
                .help("Get next pending entry"),
        )
        .arg(
            Arg::new("remove")
                .long("remove")
                .value_name("WORKSPACE")
                .help("Remove workspace from queue"),
        )
        .arg(
            Arg::new("status")
                .long("status")
                .value_name("WORKSPACE")
                .help("Check queue status of workspace"),
        )
        .arg(
            Arg::new("stats")
                .long("stats")
                .action(clap::ArgAction::SetTrue)
                .help("Show queue statistics"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_context() -> ClapCommand {
    ClapCommand::new("context")
        .about("Show complete environment context (AI agent query)")
        .after_help(after_help_text(
            &[
                "zjj context                     Show environment context summary",
                "zjj context --field=repository.branch  Extract a single field",
                "zjj context --json               Emit JSON (default when not TTY)",
            ],
            Some(json_docs::context()),
        ))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (default when not TTY)"),
        )
        .arg(
            Arg::new("field")
                .long("field")
                .value_name("PATH")
                .help("Extract single field (e.g., --field=repository.branch)"),
        )
        .arg(
            Arg::new("no-beads")
                .long("no-beads")
                .action(clap::ArgAction::SetTrue)
                .help("Skip beads database query (faster)"),
        )
        .arg(
            Arg::new("no-health")
                .long("no-health")
                .action(clap::ArgAction::SetTrue)
                .help("Skip health checks (faster)"),
        )
}

pub fn cmd_spawn() -> ClapCommand {
    ClapCommand::new("spawn")
        .about("Create session for automated agent work on a bead (issue)")
        .long_about(
            "Creates a JJ workspace, runs an agent (default: claude), and auto-merges on success.

            Use this when an AI AGENT should work autonomously on a bead.


            For manual interactive work, use 'zjj add' instead.",
        )
        .after_help(after_help_text(
            &[
                "zjj spawn zjj-abc12               Spawn workspace for bead with Claude",
                "zjj spawn zjj-xyz34 -b            Run agent in background",
                "zjj spawn zjj-def56 --agent-command=llm-run  Use custom agent",
                "zjj spawn zjj-ghi78 --no-auto-merge  Don't auto-merge on success",
            ],
            Some(json_docs::spawn()),
        ))
        .arg(
            Arg::new("bead_id")
                .required(true)
                .help("Bead ID to work on (e.g., zjj-xxxx)"),
        )
        .arg(
            Arg::new("agent-command")
                .long("agent-command")
                .value_name("COMMAND")
                .default_value("claude")
                .help("Agent command to run"),
        )
        .arg(
            Arg::new("agent-args")
                .long("agent-args")
                .value_name("ARGS")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Additional agent arguments"),
        )
        .arg(
            Arg::new("no-auto-merge")
                .long("no-auto-merge")
                .action(clap::ArgAction::SetTrue)
                .help("Don't merge on success"),
        )
        .arg(
            Arg::new("no-auto-cleanup")
                .long("no-auto-cleanup")
                .action(clap::ArgAction::SetTrue)
                .help("Don't cleanup on failure"),
        )
        .arg(
            Arg::new("background")
                .long("background")
                .short('b')
                .action(clap::ArgAction::SetTrue)
                .help("Run agent in background"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .default_value("14400")
                .help("Timeout in seconds (default: 14400 = 4 hours)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_checkpoint() -> ClapCommand {
    ClapCommand::new("checkpoint")
        .about("Save and restore session state snapshots")
        .alias("ckpt")
        .subcommand_required(true)
        .after_help(after_help_text(
            &[
                "zjj checkpoint create --description=\"before lunch\"  Snapshot current sessions",
                "zjj checkpoint list                 Show all available checkpoints",
                "zjj checkpoint restore ckpt-123     Restore workspace state from checkpoint",
            ],
            Some(json_docs::checkpoint()),
        ))
        .subcommand(
            ClapCommand::new("create")
                .about("Create a checkpoint of all current sessions")
                .arg(
                    Arg::new("description")
                        .short('d')
                        .long("description")
                        .value_name("DESC")
                        .help("Description for this checkpoint"),
                ),
        )
        .subcommand(
            ClapCommand::new("restore")
                .about("Restore sessions to a checkpoint state")
                .arg(
                    Arg::new("checkpoint_id")
                        .required(true)
                        .help("Checkpoint ID to restore"),
                ),
        )
        .subcommand(ClapCommand::new("list").about("List all available checkpoints"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
}

pub fn cmd_done() -> ClapCommand {
    ClapCommand::new("done")
        .about("Complete work and merge workspace to main")
        .after_help(after_help_text(
            &[
                "zjj done                            Complete work and merge to main",
                "zjj done -m \"Fix auth bug\"         Use custom commit message",
                "zjj done --workspace feature-x      Complete specific workspace from main",
                "zjj done --dry-run                  Preview without executing",
                "zjj done --keep-workspace           Keep workspace after merge",
                "zjj done --detect-conflicts         Check for conflicts before merging",
                "zjj done --json                     Get JSON output",
            ],
            Some(json_docs::done()),
        ))
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .value_name("NAME")
                .help("Workspace to complete (uses current if not specified)"),
        )
        .arg(
            Arg::new("message")
                .short('m')
                .long("message")
                .value_name("MSG")
                .help("Commit message (auto-generated if not provided)"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue)
                .help("Keep workspace after merge"),
        )
        .arg(
            Arg::new("squash")
                .long("squash")
                .action(clap::ArgAction::SetTrue)
                .help("Squash all commits into one"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("detect-conflicts")
                .long("detect-conflicts")
                .action(clap::ArgAction::SetTrue)
                .help("Check for conflicts before merging"),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue)
                .help("Skip bead status update"),
        )
        .arg(
            Arg::new("no-keep")
                .long("no-keep")
                .action(clap::ArgAction::SetTrue)
                .help("Skip workspace retention (cleanup immediately)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_undo() -> ClapCommand {
    ClapCommand::new("undo")
        .about("Revert last done operation")
        .long_about(
            "Reverts the most recent 'zjj done' operation, rolling back to the state before the merge.

            Works only if changes haven't been pushed to remote.

            Undo history is kept for 24 hours.",
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .action(clap::ArgAction::SetTrue)
                .help("List undo history without reverting"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_revert() -> ClapCommand {
    ClapCommand::new("revert")
        .about("Revert specific session merge")
        .long_about(
            "Reverts a specific session's merge operation, identified by session name.

            Works only if changes haven't been pushed to remote.

            Undo history is kept for 24 hours.",
        )
        .arg(
            Arg::new("name")
                .required(true)
                .help("Name of session to revert"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .short('j')
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_whereami() -> ClapCommand {
    ClapCommand::new("whereami")
        .about("Quick location query - returns 'main' or 'workspace:<name>'")
        .long_about(
            "AI-optimized command for quick orientation.


            Returns a simple, parseable string:
  
            - 'main' if on main branch
  
            - 'workspace:<name>' if in a workspace


            Use this before operations that depend on location.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_whoami() -> ClapCommand {
    ClapCommand::new("whoami")
        .about("Agent identity query - returns agent ID or 'unregistered'")
        .long_about(
            "AI-optimized command for identity verification.


            Returns:
  
            - Agent ID if registered (from ZJJ_AGENT_ID env var)
  
            - 'unregistered' if no agent registered


            Also shows current session and bead from environment.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_work() -> ClapCommand {
    ClapCommand::new("work")
        .about("Start working on a task (create workspace + register agent)")
        .long_about(
            "Unified workflow start command for AI agents.


            Combines multiple steps:
  
            1. Create workspace (or reuse if --idempotent)
  
            2. Register as agent (unless --no-agent)
  
            3. Set environment variables
  
            4. Output workspace info


            This is the AI-friendly entry point for starting work.",
        )
        .after_help(after_help_text(
            &[
                "zjj work feature-auth              Start working on feature-auth",
                "zjj work bug-fix --bead zjj-123    Start work on bead",
                "zjj work test --idempotent         Reuse existing session if exists",
                "zjj work quick --no-zellij         Create workspace without Zellij tab",
                "zjj work --dry-run feature         Preview what would be created",
            ],
            None,
        ))
        .arg(
            Arg::new("name")
                .required(true)
                .help("Session name to create/use"),
        )
        .arg(
            Arg::new("bead")
                .long("bead")
                .short('b')
                .value_name("BEAD_ID")
                .help("Bead ID to associate with this work"),
        )
        .arg(
            Arg::new("agent-id")
                .long("agent-id")
                .value_name("ID")
                .help("Agent ID to register (auto-generated if not provided)"),
        )
        .arg(
            Arg::new("no-zellij")
                .long("no-zellij")
                .action(clap::ArgAction::SetTrue)
                .help("Don't create Zellij tab"),
        )
        .arg(
            Arg::new("no-agent")
                .long("no-agent")
                .action(clap::ArgAction::SetTrue)
                .help("Don't register as agent"),
        )
        .arg(
            Arg::new("idempotent")
                .long("idempotent")
                .action(clap::ArgAction::SetTrue)
                .help("Succeed if session already exists (safe for retries)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without creating"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_ai() -> ClapCommand {
    ClapCommand::new("ai")
        .about("AI-first entry point - start here for AI agents")
        .long_about(
            "ZJJ AI Agent Interface


            This is the 'start here' command for AI agents.

            Provides status, workflows, and guidance for AI-driven work.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .global(true)
                .help("Output as JSON"),
        )
        .subcommand(
            ClapCommand::new("status")
                .about("AI-optimized status with guided next action")
                .long_about(
                    "Shows current state and suggests the next command.


                    Use this to orient yourself before starting work.",
                ),
        )
        .subcommand(
            ClapCommand::new("workflow")
                .about("Show the 7-step parallel agent workflow")
                .long_about(
                    "Displays the recommended workflow for AI agents:


                    1. Orient (whereami)

                    2. Register (agent register)

                    3. Isolate (work <name>)

                    4. Enter (cd to workspace)

                    5. Implement (do work)

                    6. Heartbeat (signal liveness)

                    7. Complete (done)",
                ),
        )
        .subcommand(
            ClapCommand::new("quick-start")
                .about("Minimum commands to be productive")
                .long_about(
                    "Shows the essential commands for quick productivity:


                    - whereami: Check location

                    - work: Start working

                    - done: Finish work",
                ),
        )
        .subcommand(
            ClapCommand::new("next")
                .about("Get single next action with copy-paste command")
                .long_about(
                    "Returns the single most important next action.


                    Output includes:

                    - action: What to do

                    - command: Copy-paste ready command

                    - reason: Why this is the next step

                    - priority: high, medium, or low",
                ),
        )
}

pub fn cmd_can_i() -> ClapCommand {
    ClapCommand::new("can-i")
        .about("Check if an action is permitted")
        .long_about(
            "Checks preconditions before attempting operations.


            Returns whether an action is allowed, and if not, what prerequisites are missing.

            Useful for AI agents to check before executing commands.",
        )
        .arg(
            Arg::new("action")
                .required(true)
                .help("Action to check (add, remove, done, undo, sync, spawn, claim, merge)"),
        )
        .arg(
            Arg::new("resource")
                .required(false)
                .help("Resource to check (session name, bead ID, etc.)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_contract() -> ClapCommand {
    ClapCommand::new("contract")
        .about("Show command contracts for AI integration")
        .long_about(
            "Displays structured contracts for commands, including:
  
            - Input/output schemas
  
            - Argument types and constraints
  
            - Flags and their effects
  
            - Side effects and rollback information


            Useful for AI agents to understand command capabilities.",
        )
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to show contract for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_examples() -> ClapCommand {
    ClapCommand::new("examples")
        .about("Show usage examples for commands")
        .long_about(
            "Provides copy-pastable examples for AI agents and users.


            Filter by command or use case to find relevant examples.",
        )
        .arg(
            Arg::new("command")
                .required(false)
                .help("Filter examples for specific command"),
        )
        .arg(
            Arg::new("use-case")
                .long("use-case")
                .value_name("CASE")
                .help("Filter by use case (workflow, single-command, error-handling, etc.)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_help() -> ClapCommand {
    ClapCommand::new("help")
        .about("Print help for a command")
        .arg(
            Arg::new("command")
                .required(false)
                .allow_hyphen_values(true)
                .help("Command to show help for (omit for top-level help)"),
        )
}

pub fn cmd_validate() -> ClapCommand {
    ClapCommand::new("validate")
        .about("Pre-validate inputs before execution")
        .long_about(
            "Validates inputs without executing commands.


            Use this to check:
  
            - Session name format
  
            - Bead ID format
  
            - Required arguments
  
            - Reserved names


            Returns structured validation results for AI agents.",
        )
        .arg(
            Arg::new("command")
                .required(true)
                .help("Command to validate inputs for"),
        )
        .arg(
            Arg::new("args")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Arguments to validate"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_whatif() -> ClapCommand {
    ClapCommand::new("whatif")
        .about("Preview command effects without executing")
        .long_about(
            "Shows what a command would do without actually doing it.


            More detailed than --dry-run, includes:
  
            - Steps that would be executed
  
            - Resource changes (files, sessions)
  
            - Prerequisite checks
  
            - Reversibility information",
        )
        .arg(
            Arg::new("command")
                .required(true)
                .help("Command to preview"),
        )
        .arg(
            Arg::new("args")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Command arguments"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_claim() -> ClapCommand {
    ClapCommand::new("claim")
        .about("Acquire exclusive lock on a resource")
        .long_about(
            "Claims exclusive access to a resource for multi-agent coordination.


            Resources can be:
  
            - Sessions
  
            - Files
  
            - Beads


            Use 'zjj yield' to release the lock when done.",
        )
        .arg(
            Arg::new("resource")
                .required(true)
                .help("Resource to claim (e.g., session:name, file:path, bead:id)"),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .value_name("SECONDS")
                .default_value("60")
                .help("Lock timeout in seconds"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_yield() -> ClapCommand {
    ClapCommand::new("yield")
        .about("Release exclusive lock on a resource")
        .long_about(
            "Releases a previously claimed resource.


            Use this when done with exclusive access to allow other agents to proceed.",
        )
        .arg(
            Arg::new("resource")
                .required(true)
                .help("Resource to release (e.g., session:name, file:path, bead:id)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_batch() -> ClapCommand {
    ClapCommand::new("batch")
        .about("Execute multiple commands in a batch")
        .long_about(
            "Runs multiple commands in sequence or from a file.


            Features:
  
            - Atomic transactional mode (all succeed or all roll back)
  
            - Stop-on-error control
  
            - Combined results output",
        )
        .arg(
            Arg::new("file")
                .long("file")
                .short('f')
                .value_name("FILE")
                .help("File containing commands (one per line)"),
        )
        .arg(
            Arg::new("atomic")
                .long("atomic")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .help("Execute all or none (requires checkpoint support)"),
        )
        .arg(
            Arg::new("stop-on-error")
                .long("stop-on-error")
                .action(clap::ArgAction::SetTrue)
                .help("Stop execution if a command fails"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("commands")
                .action(clap::ArgAction::Append)
                .num_args(0..)
                .help("Commands to execute"),
        )
}

pub fn cmd_events() -> ClapCommand {
    ClapCommand::new("events")
        .about("Listen for or query system events")
        .long_about(
            "Provides access to the zjj event log.


            Use this to track session lifecycle, agent heartbeats, and resource claims.",
        )
        .arg(
            Arg::new("session")
                .long("session")
                .value_name("NAME")
                .help("Filter by session"),
        )
        .arg(
            Arg::new("type")
                .long("type")
                .value_name("TYPE")
                .help("Filter by event type"),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .short('l')
                .value_name("COUNT")
                .help("Limit number of events returned"),
        )
        .arg(
            Arg::new("follow")
                .long("follow")
                .short('f')
                .action(clap::ArgAction::SetTrue)
                .help("Stream new events as they occur"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_lock() -> ClapCommand {
    ClapCommand::new("lock")
        .about("Acquire exclusive lock on a session")
        .arg(
            Arg::new("session")
                .required(true)
                .help("Session name to lock"),
        )
        .arg(
            Arg::new("agent-id")
                .long("agent-id")
                .value_name("ID")
                .help("Agent ID (uses ZJJ_AGENT_ID if not provided)"),
        )
        .arg(
            Arg::new("ttl")
                .long("ttl")
                .value_name("SECONDS")
                .value_parser(clap::value_parser!(u64))
                .default_value("300")
                .help("Lock TTL in seconds"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_unlock() -> ClapCommand {
    ClapCommand::new("unlock")
        .about("Release exclusive lock on a session")
        .arg(
            Arg::new("session")
                .required(true)
                .help("Session name to unlock"),
        )
        .arg(
            Arg::new("agent-id")
                .long("agent-id")
                .value_name("ID")
                .help("Agent ID (uses ZJJ_AGENT_ID if not provided)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_completions() -> ClapCommand {
    ClapCommand::new("completions")
        .about("Generate shell completions")
        .arg(
            Arg::new("shell")
                .required(true)
                .value_parser(["bash", "zsh", "fish", "powershell", "elvish"])
                .help("Shell to generate completions for"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (only for errors)"),
        )
}

pub fn cmd_rename() -> ClapCommand {
    ClapCommand::new("rename")
        .about("Rename an existing session")
        .arg(
            Arg::new("old_name")
                .required(true)
                .help("Current session name"),
        )
        .arg(Arg::new("new_name").required(true).help("New session name"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_pause() -> ClapCommand {
    ClapCommand::new("pause")
        .about("Pause an active session (suspend agent work)")
        .arg(Arg::new("name").help("Session name to pause"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_resume() -> ClapCommand {
    ClapCommand::new("resume")
        .about("Resume a paused session")
        .arg(Arg::new("name").help("Session name to resume"))
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_clone() -> ClapCommand {
    ClapCommand::new("clone")
        .about("Clone a session into a new one")
        .arg(
            Arg::new("source")
                .required(true)
                .help("Source session name"),
        )
        .arg(
            Arg::new("dest")
                .required(true)
                .help("Destination session name"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_export() -> ClapCommand {
    ClapCommand::new("export")
        .about("Export session state to a file")
        .arg(Arg::new("session").help("Session name to export (all if omitted)"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path"),
        )
        .arg(
            Arg::new("include-files")
                .long("include-files")
                .action(clap::ArgAction::SetTrue)
                .help("Include workspace files in export (creates tarball)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_import() -> ClapCommand {
    ClapCommand::new("import")
        .about("Import session state from a file")
        .arg(Arg::new("file").required(true).help("Input file path"))
        .arg(
            Arg::new("skip-existing")
                .long("skip-existing")
                .action(clap::ArgAction::SetTrue)
                .help("Skip sessions that already exist"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview import without changes"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_wait() -> ClapCommand {
    ClapCommand::new("wait")
        .about("Wait for a condition to be met")
        .arg(
            Arg::new("condition")
                .required(true)
                .value_parser([
                    "session-exists",
                    "session-unlocked",
                    "healthy",
                    "session-status",
                ])
                .help("Condition to wait for"),
        )
        .arg(Arg::new("name").help("Session name (for session conditions)"))
        .arg(
            Arg::new("status")
                .long("status")
                .help("Expected status (for session-status condition)"),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .default_value("30")
                .help("Timeout in seconds"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .default_value("1")
                .help("Polling interval in seconds"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_schema() -> ClapCommand {
    ClapCommand::new("schema")
        .about("Show JSON schemas for zjj protocol")
        .arg(Arg::new("name").help("Schema name (e.g., add-response)"))
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .action(clap::ArgAction::SetTrue)
                .help("List all available schemas"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .short('a')
                .action(clap::ArgAction::SetTrue)
                .help("Show all schemas"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_recover() -> ClapCommand {
    ClapCommand::new("recover")
        .about("Recover from inconsistent state")
        .arg(
            Arg::new("diagnose")
                .short('d')
                .long("diagnose")
                .action(clap::ArgAction::SetTrue)
                .help("Only diagnose issues without fixing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_retry() -> ClapCommand {
    ClapCommand::new("retry")
        .about("Retry the last failed operation")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_rollback() -> ClapCommand {
    ClapCommand::new("rollback")
        .about("Rollback session to a specific checkpoint")
        .arg(Arg::new("session").required(true).help("Session name"))
        .arg(
            Arg::new("to")
                .long("to")
                .required(true)
                .help("Checkpoint ID to rollback to"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview rollback without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn cmd_pane() -> ClapCommand {
    ClapCommand::new("pane")
        .about("Manage Zellij panes")
        .subcommand(
            ClapCommand::new("focus")
                .about("Focus a specific pane")
                .arg(Arg::new("session").required(true).help("Session name"))
                .arg(Arg::new("pane").help("Pane identifier"))
                .arg(
                    Arg::new("direction")
                        .long("direction")
                        .short('d')
                        .value_parser(["up", "down", "left", "right"])
                        .help("Focus pane in direction"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("list")
                .about("List panes in a session")
                .arg(Arg::new("session").required(true).help("Session name"))
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            ClapCommand::new("next")
                .about("Focus next pane")
                .arg(Arg::new("session").required(true).help("Session name"))
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
}

pub fn cmd_abort() -> ClapCommand {
    ClapCommand::new("abort")
        .about("Abort work and abandon workspace changes")
        .arg(
            Arg::new("workspace")
                .short('w')
                .long("workspace")
                .value_name("NAME")
                .help("Workspace to abort (uses current if not specified)"),
        )
        .arg(
            Arg::new("no-bead-update")
                .long("no-bead-update")
                .action(clap::ArgAction::SetTrue)
                .help("Don't update bead status"),
        )
        .arg(
            Arg::new("keep-workspace")
                .long("keep-workspace")
                .action(clap::ArgAction::SetTrue)
                .help("Keep workspace files (just remove from zjj tracking)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

pub fn build_cli() -> ClapCommand {
    ClapCommand::new("zjj")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Isolated workspace manager combining JJ workspaces with Zellij sessions")
        .long_about(
            "ZJJ creates isolated JJ workspaces paired with Zellij tabs for parallel work.\n\n\
            Core workflow:\n  \
              zjj init          Initialize zjj in a JJ repo\n  \
              zjj add <name>    Create session for manual work (you control tab)\n  \
              zjj spawn <bead>  Create session for automated agent work\n  \
            zjj focus <name>  Switch Zellij tab (inside Zellij)\n  \
            zjj done          Merge workspace to main and clean up",
        )
        .disable_help_subcommand(true)
        // Global hook arguments
        .arg(
            Arg::new("on-success")
                .long("on-success")
                .global(true)
                .value_name("CMD")
                .help("Command to run after successful execution"),
        )
        .arg(
            Arg::new("on-failure")
                .long("on-failure")
                .global(true)
                .value_name("CMD")
                .help("Command to run after failed execution"),
        )
        .subcommand_required(true)
        .subcommand(cmd_init())
        .subcommand(cmd_add())
        .subcommand(cmd_agents())
        .subcommand(cmd_attach())
        .subcommand(cmd_list())
        .subcommand(cmd_bookmark())
        .subcommand(cmd_remove())
        .subcommand(cmd_focus())
        .subcommand(cmd_switch())
        .subcommand(cmd_status())
        .subcommand(cmd_sync())
        .subcommand(cmd_diff())
        .subcommand(cmd_config())
        .subcommand(cmd_clean())
        .subcommand(cmd_template())
        .subcommand(cmd_dashboard())
        .subcommand(cmd_introspect())
        .subcommand(cmd_doctor())
        .subcommand(cmd_integrity())
        .subcommand(cmd_query())
        .subcommand(cmd_context())
        .subcommand(cmd_done())
        .subcommand(cmd_spawn())
        .subcommand(cmd_checkpoint())
        .subcommand(cmd_undo())
        .subcommand(cmd_revert())
        .subcommand(cmd_whereami())
        .subcommand(cmd_whoami())
        .subcommand(cmd_work())
        .subcommand(cmd_abort())
        .subcommand(cmd_ai())
        // AI-first commands
        .subcommand(cmd_help())
        .subcommand(cmd_can_i())
        .subcommand(cmd_contract())
        .subcommand(cmd_examples())
        .subcommand(cmd_validate())
        .subcommand(cmd_whatif())
        .subcommand(cmd_claim())
        .subcommand(cmd_yield())
        .subcommand(cmd_batch())
        .subcommand(cmd_events())
        .subcommand(cmd_lock())
        .subcommand(cmd_unlock())
        .subcommand(cmd_completions())
        // Session management
        .subcommand(cmd_rename())
         .subcommand(cmd_pause())
        .subcommand(cmd_resume())
        .subcommand(cmd_clone())
        // Pane management
        .subcommand(cmd_pane())
        // Export/Import
        .subcommand(cmd_export())
        .subcommand(cmd_import())
        // Wait/Poll commands
        .subcommand(cmd_wait())
        // Schema command
        .subcommand(cmd_schema())
        // Recovery commands
        .subcommand(cmd_recover())
        .subcommand(cmd_retry())
        .subcommand(cmd_rollback())
        // Merge queue coordination
        .subcommand(cmd_queue())
}

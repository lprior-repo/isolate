//! Object-based CLI command type system
//!
//! This module defines the new object-based command structure following
//! the pattern: `zjj <object> <action>`
//!
//! Objects are nouns (Task, Session, Queue, etc.) and actions are verbs.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use clap::{Arg, Command as ClapCommand};

/// Top-level objects in the zjj CLI
///
/// Each object represents a domain of related operations following
/// the `zjj <object> <action>` pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZjjObject {
    /// Task management (beads, work items)
    Task,
    /// Session management (workspaces)
    Session,
    /// Merge queue operations
    Queue,
    /// Stack operations (parent-child session relationships)
    Stack,
    /// Agent coordination and tracking
    Agent,
    /// Status and introspection queries
    Status,
    /// Configuration management
    Config,
    /// Diagnostics and health checks
    Doctor,
}

impl ZjjObject {
    /// Returns all object variants
    pub const fn all() -> &'static [Self] {
        &[
            Self::Task,
            Self::Session,
            Self::Queue,
            Self::Stack,
            Self::Agent,
            Self::Status,
            Self::Config,
            Self::Doctor,
        ]
    }

    /// Returns the CLI name for this object
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Session => "session",
            Self::Queue => "queue",
            Self::Stack => "stack",
            Self::Agent => "agent",
            Self::Status => "status",
            Self::Config => "config",
            Self::Doctor => "doctor",
        }
    }

    /// Returns a short description for this object
    pub const fn about(&self) -> &'static str {
        match self {
            Self::Task => "Manage tasks and work items (beads)",
            Self::Session => "Manage workspaces and sessions",
            Self::Queue => "Manage merge queue operations",
            Self::Stack => "Manage stacked session relationships",
            Self::Agent => "Manage agent coordination and tracking",
            Self::Status => "Query system and session status",
            Self::Config => "Manage zjj configuration",
            Self::Doctor => "Run diagnostics and health checks",
        }
    }
}

/// Subcommands for the Task object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAction {
    /// List all tasks
    List,
    /// Show task details
    Show,
    /// Claim a task for work
    Claim,
    /// Yield a claimed task
    Yield,
    /// Start work on a task (creates session)
    Start,
    /// Complete a task
    Done,
}

/// Subcommands for the Session object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAction {
    /// List all sessions
    List,
    /// Create a new session
    Add,
    /// Remove a session
    Remove,
    /// Switch to a session
    Focus,
    /// Pause a session
    Pause,
    /// Resume a session
    Resume,
    /// Clone a session
    Clone,
    /// Rename a session
    Rename,
    /// Attach to session from shell
    Attach,
    /// Spawn session for agent work
    Spawn,
    /// Sync session with remote
    Sync,
    /// Initialize zjj in repository
    Init,
}

/// Subcommands for the Queue object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueAction {
    /// List queue entries
    List,
    /// Add entry to queue
    Enqueue,
    /// Remove entry from queue
    Dequeue,
    /// Show queue status
    Status,
    /// Process queue entries
    Process,
}

/// Subcommands for the Stack object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackAction {
    /// Show stack status
    Status,
    /// List all stacks
    List,
    /// Create a new stack
    Create,
    /// Push to stack
    Push,
    /// Pop from stack
    Pop,
}

/// Subcommands for the Agent object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAction {
    /// List all agents
    List,
    /// Register as an agent
    Register,
    /// Unregister as an agent
    Unregister,
    /// Send heartbeat
    Heartbeat,
    /// Show agent status
    Status,
    /// Broadcast message to agents
    Broadcast,
}

/// Subcommands for the Status object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusAction {
    /// Show current status
    Show,
    /// Show where you are
    Whereami,
    /// Show who you are
    Whoami,
    /// Show context information
    Context,
}

/// Subcommands for the Config object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    /// List configuration
    List,
    /// Get a config value
    Get,
    /// Set a config value
    Set,
    /// Show configuration schema
    Schema,
}

/// Subcommands for the Doctor object
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorAction {
    /// Run diagnostics
    Check,
    /// Fix issues
    Fix,
    /// Show system integrity
    Integrity,
    /// Clean up invalid sessions
    Clean,
}

/// Global flags available on all commands
#[derive(Debug, Clone, Default)]
pub struct GlobalFlags {
    /// Output as JSON
    pub json: bool,
    /// Verbose output
    pub verbose: bool,
    /// Dry run (preview without executing)
    pub dry_run: bool,
}

// ============================================================================
// Command Builders
// ============================================================================

/// Create the JSON argument (common to all commands)
fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(clap::ArgAction::SetTrue)
        .help("Output as JSON (machine-parseable format)")
}

/// Create the verbose argument
fn verbose_arg() -> Arg {
    Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(clap::ArgAction::SetTrue)
        .help("Enable verbose output")
}

/// Create the dry-run argument
fn dry_run_arg() -> Arg {
    Arg::new("dry-run")
        .long("dry-run")
        .action(clap::ArgAction::SetTrue)
        .help("Preview without executing")
}

/// Build the Task object command with all subcommands
pub fn cmd_task() -> ClapCommand {
    ClapCommand::new("task")
        .about("Manage tasks and work items (beads)")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all tasks")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include completed tasks"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by task state"),
                ),
        )
        .subcommand(
            ClapCommand::new("show")
                .about("Show task details")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to show")),
        )
        .subcommand(
            ClapCommand::new("claim")
                .visible_alias("take")
                .about("Claim a task for work")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to claim")),
        )
        .subcommand(
            ClapCommand::new("yield")
                .visible_alias("release")
                .about("Yield a claimed task")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to yield")),
        )
        .subcommand(
            ClapCommand::new("start")
                .about("Start work on a task (creates session)")
                .arg(json_arg())
                .arg(Arg::new("id").required(true).help("Task/bead ID to start"))
                .arg(
                    Arg::new("template")
                        .long("template")
                        .short('t')
                        .value_name("TEMPLATE")
                        .help("Zellij layout template"),
                ),
        )
        .subcommand(
            ClapCommand::new("done")
                .visible_alias("complete")
                .about("Complete a task")
                .arg(json_arg())
                .arg(Arg::new("id").help("Task/bead ID (uses current session if omitted)")),
        )
}

/// Build the Session object command with all subcommands
pub fn cmd_session() -> ClapCommand {
    ClapCommand::new("session")
        .about("Manage workspaces and sessions")
        .subcommand_required(true)
        .arg(json_arg())
        .arg(verbose_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include closed sessions"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed information"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .value_name("BEAD_ID")
                        .help("Filter by bead ID"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Filter by agent owner"),
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("Filter by session state"),
                ),
        )
        .subcommand(
            ClapCommand::new("add")
                .visible_alias("create")
                .about("Create a new session for manual work")
                .arg(json_arg())
                .arg(dry_run_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new session"),
                )
                .arg(
                    Arg::new("bead")
                        .long("bead")
                        .short('b')
                        .value_name("BEAD_ID")
                        .help("Associate with a bead ID"),
                )
                .arg(
                    Arg::new("parent")
                        .long("parent")
                        .short('p')
                        .value_name("PARENT")
                        .help("Create as stacked session under parent"),
                )
                .arg(
                    Arg::new("template")
                        .long("template")
                        .short('t')
                        .value_name("TEMPLATE")
                        .help("Zellij layout template (minimal, standard, full)"),
                )
                .arg(
                    Arg::new("no-open")
                        .long("no-open")
                        .action(clap::ArgAction::SetTrue)
                        .help("Create without opening Zellij tab"),
                )
                .arg(
                    Arg::new("no-hooks")
                        .long("no-hooks")
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip post-create hooks"),
                ),
        )
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove a session")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to remove"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force removal without confirmation"),
                ),
        )
        .subcommand(
            ClapCommand::new("focus")
                .about("Switch to a session (inside Zellij)")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to focus"),
                ),
        )
        .subcommand(
            ClapCommand::new("pause")
                .about("Pause a session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("resume")
                .about("Resume a paused session")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("clone")
                .about("Clone a session")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to clone"),
                )
                .arg(
                    Arg::new("new-name")
                        .long("new-name")
                        .value_name("NAME")
                        .help("Name for cloned session"),
                ),
        )
        .subcommand(
            ClapCommand::new("rename")
                .about("Rename a session")
                .arg(json_arg())
                .arg(
                    Arg::new("old-name")
                        .required(true)
                        .help("Current session name"),
                )
                .arg(Arg::new("new-name").required(true).help("New session name")),
        )
        .subcommand(
            ClapCommand::new("attach")
                .about("Attach to session from shell")
                .arg(json_arg())
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to attach to"),
                ),
        )
        .subcommand(
            ClapCommand::new("spawn")
                .about("Spawn session for automated agent work")
                .arg(json_arg())
                .arg(dry_run_arg())
                .arg(
                    Arg::new("bead")
                        .required(true)
                        .help("Bead ID for the spawned session"),
                )
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("AGENT")
                        .help("Agent to assign"),
                ),
        )
        .subcommand(
            ClapCommand::new("sync")
                .visible_alias("rebase")
                .about("Sync session with remote")
                .arg(json_arg())
                .arg(Arg::new("name").help("Session name (uses current if omitted)"))
                .arg(
                    Arg::new("push")
                        .long("push")
                        .action(clap::ArgAction::SetTrue)
                        .help("Push changes to remote"),
                )
                .arg(
                    Arg::new("pull")
                        .long("pull")
                        .action(clap::ArgAction::SetTrue)
                        .help("Pull changes from remote"),
                ),
        )
        .subcommand(
            ClapCommand::new("init")
                .about("Initialize zjj in a JJ repository")
                .arg(json_arg())
                .arg(dry_run_arg()),
        )
}

/// Build the Queue object command with all subcommands
pub fn cmd_queue() -> ClapCommand {
    ClapCommand::new("queue")
        .about("Manage merge queue operations")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List queue entries")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include completed entries"),
                ),
        )
        .subcommand(
            ClapCommand::new("enqueue")
                .about("Add session to queue")
                .arg(json_arg())
                .arg(
                    Arg::new("session")
                        .required(true)
                        .help("Session name to enqueue"),
                ),
        )
        .subcommand(
            ClapCommand::new("dequeue")
                .about("Remove session from queue")
                .arg(json_arg())
                .arg(
                    Arg::new("session")
                        .required(true)
                        .help("Session name to dequeue"),
                ),
        )
        .subcommand(
            ClapCommand::new("status")
                .about("Show queue status")
                .arg(json_arg())
                .arg(
                    Arg::new("session")
                        .help("Session name to show status for (shows queue stats if omitted)"),
                ),
        )
        .subcommand(
            ClapCommand::new("process")
                .about("Process queue entries")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                ),
        )
}

/// Build the Stack object command with all subcommands
pub fn cmd_stack() -> ClapCommand {
    ClapCommand::new("stack")
        .about("Manage stacked session relationships")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("status")
                .about("Show stack status")
                .arg(json_arg())
                .arg(
                    Arg::new("workspace")
                        .required(true)
                        .help("Workspace name to query stack status for"),
                ),
        )
        .subcommand(
            ClapCommand::new("list")
                .about("List all stacks")
                .arg(json_arg())
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Show detailed information"),
                ),
        )
        .subcommand(
            ClapCommand::new("create")
                .about("Create a new stack")
                .arg(json_arg())
                .arg(Arg::new("name").required(true).help("Name for the stack"))
                .arg(
                    Arg::new("base")
                        .long("base")
                        .value_name("SESSION")
                        .help("Base session for the stack"),
                ),
        )
        .subcommand(
            ClapCommand::new("push")
                .about("Push session onto stack")
                .arg(json_arg())
                .arg(Arg::new("session").required(true).help("Session to push"))
                .arg(
                    Arg::new("parent")
                        .long("parent")
                        .value_name("PARENT")
                        .help("Parent session in stack"),
                ),
        )
        .subcommand(
            ClapCommand::new("pop")
                .about("Pop session from stack")
                .arg(json_arg())
                .arg(Arg::new("session").help("Session to pop (uses current if omitted)")),
        )
}

/// Build the Agent object command with all subcommands
pub fn cmd_agent() -> ClapCommand {
    ClapCommand::new("agent")
        .about("Manage agent coordination and tracking")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List all agents")
                .arg(json_arg())
                .arg(
                    Arg::new("all")
                        .long("all")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include stale agents"),
                )
                .arg(
                    Arg::new("session")
                        .long("session")
                        .value_name("SESSION")
                        .help("Filter by session"),
                ),
        )
        .subcommand(
            ClapCommand::new("register")
                .about("Register as an agent")
                .arg(json_arg())
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("ID")
                        .help("Agent ID (auto-generated if not provided)"),
                )
                .arg(
                    Arg::new("session")
                        .long("session")
                        .short('s')
                        .value_name("SESSION")
                        .help("Session to associate"),
                ),
        )
        .subcommand(
            ClapCommand::new("unregister")
                .about("Unregister as an agent")
                .arg(json_arg())
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("ID")
                        .help("Agent ID (uses ZJJ_AGENT_ID if not provided)"),
                ),
        )
        .subcommand(
            ClapCommand::new("heartbeat")
                .about("Send a heartbeat")
                .arg(json_arg())
                .arg(
                    Arg::new("command")
                        .long("command")
                        .short('c')
                        .value_name("CMD")
                        .help("Current command being executed"),
                ),
        )
        .subcommand(
            ClapCommand::new("status")
                .about("Show agent status")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("broadcast")
                .about("Broadcast message to all agents")
                .arg(json_arg())
                .arg(
                    Arg::new("message")
                        .required(true)
                        .help("Message to broadcast"),
                ),
        )
}

/// Build the Status object command with all subcommands
pub fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Query system and session status")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("show")
                .about("Show current status")
                .arg(json_arg())
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
        .subcommand(
            ClapCommand::new("whereami")
                .about("Show current location")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("whoami")
                .about("Show current identity")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("context")
                .about("Show context information")
                .arg(json_arg())
                .arg(Arg::new("session").help("Session name (uses current if omitted)")),
        )
}

/// Build the Config object command with all subcommands
pub fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .about("Manage zjj configuration")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("list")
                .about("List configuration values")
                .arg(json_arg()),
        )
        .subcommand(
            ClapCommand::new("get")
                .about("Get a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to get"),
                ),
        )
        .subcommand(
            ClapCommand::new("set")
                .about("Set a config value")
                .arg(json_arg())
                .arg(
                    Arg::new("key")
                        .required(true)
                        .help("Configuration key to set"),
                )
                .arg(Arg::new("value").required(true).help("Value to set")),
        )
        .subcommand(
            ClapCommand::new("schema")
                .about("Show configuration schema")
                .arg(json_arg()),
        )
}

/// Build the Doctor object command with all subcommands
pub fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run diagnostics and health checks")
        .subcommand_required(true)
        .arg(json_arg())
        .subcommand(
            ClapCommand::new("check")
                .about("Run diagnostics")
                .arg(json_arg())
                .arg(
                    Arg::new("fix")
                        .long("fix")
                        .action(clap::ArgAction::SetTrue)
                        .help("Attempt to fix issues"),
                ),
        )
        .subcommand(
            ClapCommand::new("fix")
                .about("Fix detected issues")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                ),
        )
        .subcommand(
            ClapCommand::new("integrity")
                .about("Check system integrity")
                .arg(json_arg())
                .arg(
                    Arg::new("fix")
                        .long("fix")
                        .action(clap::ArgAction::SetTrue)
                        .help("Attempt to fix issues"),
                ),
        )
        .subcommand(
            ClapCommand::new("clean")
                .about("Clean up invalid sessions")
                .arg(json_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(clap::ArgAction::SetTrue)
                        .help("Preview without executing"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .action(clap::ArgAction::SetTrue)
                        .help("Force cleanup without confirmation"),
                ),
        )
}

/// Build the complete object-based CLI
///
/// This creates the new `zjj <object> <action>` command structure
/// while maintaining compatibility with existing handlers.
pub fn build_object_cli() -> ClapCommand {
    ClapCommand::new("zjj")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Isolated workspace manager (object-based CLI)")
        .long_about(
            "ZJJ creates isolated JJ workspaces paired with Zellij sessions.\n\n\
             Object-based command structure:\n\
             \n  zjj task <action>     Manage tasks and work items\n\
             \n  zjj session <action>  Manage workspaces and sessions\n\
             \n  zjj queue <action>    Manage merge queue\n\
             \n  zjj stack <action>    Manage session stacks\n\
             \n  zjj agent <action>    Manage agent coordination\n\
             \n  zjj status <action>   Query system status\n\
             \n  zjj config <action>   Manage configuration\n\
             \n  zjj doctor <action>   Run diagnostics\n",
        )
        .subcommand_required(true)
        .arg(json_arg().global(true))
        .arg(verbose_arg().global(true))
        .subcommand(cmd_task())
        .subcommand(cmd_session())
        .subcommand(cmd_queue())
        .subcommand(cmd_stack())
        .subcommand(cmd_agent())
        .subcommand(cmd_status())
        .subcommand(cmd_config())
        .subcommand(cmd_doctor())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zjj_object_names() {
        assert_eq!(ZjjObject::Task.name(), "task");
        assert_eq!(ZjjObject::Session.name(), "session");
        assert_eq!(ZjjObject::Queue.name(), "queue");
        assert_eq!(ZjjObject::Stack.name(), "stack");
        assert_eq!(ZjjObject::Agent.name(), "agent");
        assert_eq!(ZjjObject::Status.name(), "status");
        assert_eq!(ZjjObject::Config.name(), "config");
        assert_eq!(ZjjObject::Doctor.name(), "doctor");
    }

    #[test]
    fn test_zjj_object_all_count() {
        assert_eq!(ZjjObject::all().len(), 8);
    }

    #[test]
    fn test_build_object_cli_has_all_subcommands() {
        let cli = build_object_cli();
        let subcommands: Vec<&str> = cli.get_subcommands().map(|cmd| cmd.get_name()).collect();

        assert!(subcommands.contains(&"task"));
        assert!(subcommands.contains(&"session"));
        assert!(subcommands.contains(&"queue"));
        assert!(subcommands.contains(&"stack"));
        assert!(subcommands.contains(&"agent"));
        assert!(subcommands.contains(&"status"));
        assert!(subcommands.contains(&"config"));
        assert!(subcommands.contains(&"doctor"));
    }

    #[test]
    fn test_all_commands_have_json_flag() {
        let cli = build_object_cli();

        for object_cmd in cli.get_subcommands() {
            // Check object-level has json flag
            let has_json = object_cmd
                .get_arguments()
                .any(|arg| arg.get_id().as_str() == "json");
            assert!(
                has_json,
                "Object {} should have --json flag",
                object_cmd.get_name()
            );

            // Check all subcommands have json flag
            for action_cmd in object_cmd.get_subcommands() {
                let action_has_json = action_cmd
                    .get_arguments()
                    .any(|arg| arg.get_id().as_str() == "json");
                assert!(
                    action_has_json,
                    "Action {} {} should have --json flag",
                    object_cmd.get_name(),
                    action_cmd.get_name()
                );
            }
        }
    }

    #[test]
    fn test_task_subcommands() {
        let cmd = cmd_task();
        let subcommands: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();

        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"show"));
        assert!(subcommands.contains(&"claim"));
        assert!(subcommands.contains(&"yield"));
        assert!(subcommands.contains(&"start"));
        assert!(subcommands.contains(&"done"));
    }

    #[test]
    fn test_session_subcommands() {
        let cmd = cmd_session();
        let subcommands: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();

        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"add"));
        assert!(subcommands.contains(&"remove"));
        assert!(subcommands.contains(&"focus"));
        assert!(subcommands.contains(&"pause"));
        assert!(subcommands.contains(&"resume"));
        assert!(subcommands.contains(&"clone"));
        assert!(subcommands.contains(&"rename"));
        assert!(subcommands.contains(&"attach"));
        assert!(subcommands.contains(&"spawn"));
        assert!(subcommands.contains(&"sync"));
        assert!(subcommands.contains(&"init"));
    }
}

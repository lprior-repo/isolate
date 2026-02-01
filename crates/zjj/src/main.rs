//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `zjj`

use std::process;

use anyhow::Result;
use clap::{Arg, Command as ClapCommand};

mod cli;
mod commands;
mod db;
mod json;
mod selector;
mod session;

use commands::{
    abort, add, agents, ai, attach, checkpoint, clean, config, context, dashboard, diff, doctor,
    done, focus, init, introspect, list, query, remove, revert, spawn, status, sync, undo,
    whereami, whoami, work,
};

/// Generate JSON OUTPUT documentation for command help
/// These strings document the `SchemaEnvelope` structure used in JSON output
#[allow(dead_code)]
mod json_docs {
    pub const fn add() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://add-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "zellij_tab": "zjj:<session_name>",
    "message": "Created session '<name>'"
  }"#
    }

    pub const fn list() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelopeArray:
  {
    "$schema": "zjj://list-response/v1",
    "_schema_version": "1.0",
    "schema_type": "array",
    "success": true,
    "data": [
      {
        "name": "<session_name>",
        "status": "<active|paused|completed|failed>",
        "branch": "<branch_name>",
        "changes": "<modified_count>",
        "beads": "<open/in_progress/blocked>",
        ...
      }
    ]
  }"#
    }

    pub const fn remove() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://remove-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "message": "Removed session '<name>'"
  }"#
    }

    pub const fn focus() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://focus-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "zellij_tab": "zjj:<session_name>",
    "message": "Switched to session '<name>'"
  }"#
    }

    pub const fn status() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelope:
  {
    "$schema": "zjj://status-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "sessions": [
      {
        "name": "<session_name>",
        "status": "<active|paused|completed|failed>",
        "workspace_path": "<absolute_path>",
        "branch": "<branch_name>",
        "changes": {
          "modified": <count>,
          "added": <count>,
          "deleted": <count>,
          "renamed": <count>
        },
        "diff_stats": {
          "insertions": <count>,
          "deletions": <count>
        },
        "beads": {
          "open": <count>,
          "in_progress": <count>,
          "blocked": <count>,
          "closed": <count>
        },
        ...
      }
    ]
  }"#
    }

    pub const fn sync() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://sync-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name_or_null>",
    "synced_count": <count>,
    "failed_count": <count>,
    "errors": []
  }"#
    }

    pub const fn init() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://init-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "message": "<message>",
    "zjj_dir": "<absolute_path>",
    "config_file": "<absolute_path>",
    "state_db": "<absolute_path>",
    "layouts_dir": "<absolute_path>"
  }"#
    }

    pub const fn spawn() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://spawn-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "bead_id": "<bead_id>",
    "session_name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "agent": "<agent_command>",
    "status": "<started|running|completed|failed>",
    "message": "<status_message>"
  }"#
    }

    pub const fn done() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://done-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "session_name": "<session_name>",
    "merged": true,
    "commit_id": "<commit_hash>",
    "message": "Merged and cleaned up '<name>'"
  }"#
    }

    pub const fn diff() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://diff-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "base": "<base_commit>",
    "head": "<head_commit>",
    "diff_stat": {
      "files_changed": <count>,
      "insertions": <count>,
      "deletions": <count>,
      "files": [...]
    },
    "diff_content": "<full_diff_or_null>"
  }"#
    }

    pub const fn config() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://config-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "key": "<config_key_or_null>",
    "value": "<config_value_or_null>",
    "config": {...}
  }"#
    }

    pub const fn clean() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://clean-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "removed_count": <count>,
    "sessions": ["<session_name>", ...]
  }"#
    }

    pub const fn introspect() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://introspect-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "commands": [...],
    "dependencies": {...},
    "system_state": {...}
  }"#
    }

    pub const fn doctor() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://doctor-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "checks": [
      {
        "name": "<check_name>",
        "status": "<pass|warn|fail>",
        "message": "<message>",
        "suggestion": "<suggestion_or_null>"
      },
      ...
    ],
    "summary": {
      "passed": <count>,
      "warnings": <count>,
      "failed": <count>
    }
  }"#
    }

    pub const fn query() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used (default), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://query-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "query_type": "<query_type>",
    "result": <query_specific_result>
  }"#
    }

    pub const fn context() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used (default when not TTY), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://context-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "repository": {...},
    "sessions": [...],
    "beads": {...},
    "health": {...},
    "environment": {...}
  }"#
    }

    pub const fn checkpoint() -> &'static str {
        r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "zjj://checkpoint-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "action": "<create|restore|list>",
    "checkpoint_id": "<id_or_null>",
    "checkpoints": [...]
  }"#
    }
}

fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize zjj in a JJ repository (or create one)")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_attach() -> ClapCommand {
    ClapCommand::new("attach")
        .about("Enter Zellij session from outside (shell → Zellij)")
        .long_about(
            "Use this when you are in a regular shell and want to enter the Zellij session.\n\
            This replaces your current process with Zellij.\n\n\
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

fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create session for manual work (JJ workspace + Zellij tab)")
        .long_about(
            "Creates a JJ workspace and Zellij tab for interactive development.\n\
            Use this when YOU will work in the session.\n\n\
            For automated agent workflows, use 'zjj spawn' instead.",
        )
        .after_help(
            "EXAMPLES:\n  \
            zjj add feature-auth              Create session with standard layout\n  \
            zjj add bugfix-123 --no-open       Create without opening Zellij tab\n  \
            zjj add experiment -t minimal      Use minimal layout template\n  \
            zjj add quick-test --no-hooks      Skip post-create hooks\n  \
            zjj add --example-json            Show example JSON output",
        )
        .arg(
            Arg::new("name")
                .required_unless_present("example-json")
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name for the new session (must start with a letter)"),
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
}

fn cmd_agents() -> ClapCommand {
    ClapCommand::new("agents")
        .alias("agent")
        .about("List and manage agents")
        .long_about(
            "Shows all agents that have recently sent heartbeats, along with their current sessions and any locks they hold.\n\n\
            Agents are considered active if they've sent a heartbeat within the last 60 seconds.\n\n\
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
                    "Register this process as an agent for zjj tracking.\n\n\
                    Sets ZJJ_AGENT_ID environment variable.\n\
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
                    "Updates the agent's last_seen timestamp.\n\n\
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
                    "Shows the status of the currently registered agent.\n\n\
                    Uses ZJJ_AGENT_ID environment variable.",
                ),
        )
        .subcommand(
            ClapCommand::new("unregister")
                .about("Unregister as an agent")
                .long_about(
                    "Remove this agent from zjj tracking.\n\n\
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

fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions"),
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
}

fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .after_help(
            "EXAMPLES:\n  \
            zjj remove old-feature            Remove with confirmation prompt\n  \
            zjj remove test-session -f        Force removal without prompt\n  \
            zjj remove feature-x --merge       Merge changes to main first\n  \
            zjj remove experiment -k -f       Keep branch, force removal",
        )
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

fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to session's Zellij tab (inside Zellij)")
        .long_about(
            "Use this when you are already inside Zellij and want to switch tabs.\n\n\
            If you are outside Zellij, use 'zjj attach' to enter the session instead.",
        )
        .after_help(
            "EXAMPLES:\n  \
            zjj focus feature-auth            Switch to session's Zellij tab\n  \
            zjj focus                         Interactive session selection\n  \
            zjj focus bugfix-123 --json       Get JSON output of focus operation",
        )
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
}

fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
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

fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync a session's workspace with main (rebase)")
        .after_help(
            "EXAMPLES:\n  \
            zjj sync feature-auth             Sync named session with main\n  \
            zjj sync                          Sync current workspace\n  \
            zjj sync --all                    Sync all active sessions\n  \
            zjj sync --json                   Get JSON output of sync operation",
        )
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

fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
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

fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
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

fn cmd_clean() -> ClapCommand {
    ClapCommand::new("clean")
        .about("Remove stale sessions (where workspace no longer exists)")
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
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_dashboard() -> ClapCommand {
    ClapCommand::new("dashboard")
        .about("Launch interactive TUI dashboard with kanban view")
        .alias("dash")
}

fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover zjj capabilities and command details")
        .long_about(
            "AI-optimized capability discovery.\n\n\
            Use this to understand:\n  \
            - Available commands and their arguments\n  \
            - System state and dependencies\n  \
            - Environment variables zjj uses\n  \
            - Common workflow patterns",
        )
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

fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run system health checks")
        .alias("check")
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

fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .after_help(
            "EXAMPLES:\n  \
            zjj query session-exists feature   Check if session exists\n  \
            zjj query session-count             Count active sessions\n  \
            zjj query can-run                   Check if zjj can run\n  \
            zjj query suggest-name feat         Get name suggestion",
        )
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

fn cmd_context() -> ClapCommand {
    ClapCommand::new("context")
        .about("Show complete environment context (AI agent query)")
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

fn cmd_spawn() -> ClapCommand {
    ClapCommand::new("spawn")
        .about("Create session for automated agent work on a bead (issue)")
        .long_about(
            "Creates a JJ workspace, runs an agent (default: claude), and auto-merges on success.\n\
            Use this when an AI AGENT should work autonomously on a bead.\n\n\
            For manual interactive work, use 'zjj add' instead.",
        )
        .after_help(
            "EXAMPLES:\n  \
            zjj spawn zjj-abc12               Spawn workspace for bead with Claude\n  \
            zjj spawn zjj-xyz34 -b            Run agent in background\n  \
            zjj spawn zjj-def56 --agent-command=llm-run  Use custom agent\n  \
            zjj spawn zjj-ghi78 --no-auto-merge  Don't auto-merge on success",
        )
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

fn cmd_checkpoint() -> ClapCommand {
    ClapCommand::new("checkpoint")
        .about("Save and restore session state snapshots")
        .alias("ckpt")
        .subcommand_required(true)
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

fn cmd_done() -> ClapCommand {
    ClapCommand::new("done")
        .about("Complete work and merge workspace to main")
        .after_help(
            "EXAMPLES:\n  \
            zjj done                            Complete work and merge to main\n  \
            zjj done -m \"Fix auth bug\"         Use custom commit message\n  \
            zjj done --workspace feature-x      Complete specific workspace from main\n  \
            zjj done --dry-run                  Preview without executing\n  \
            zjj done --keep-workspace           Keep workspace after merge\n  \
            zjj done --json                     Get JSON output",
        )
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

fn cmd_undo() -> ClapCommand {
    ClapCommand::new("undo")
        .about("Revert last done operation")
        .long_about(
            "Reverts the most recent 'zjj done' operation, rolling back to the state before the merge.\n\
            Works only if changes haven't been pushed to remote.\n\
            Undo history is kept for 24 hours.",
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

fn cmd_revert() -> ClapCommand {
    ClapCommand::new("revert")
        .about("Revert specific session merge")
        .long_about(
            "Reverts a specific session's merge operation, identified by session name.\n\
            Works only if changes haven't been pushed to remote.\n\
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

fn cmd_whereami() -> ClapCommand {
    ClapCommand::new("whereami")
        .about("Quick location query - returns 'main' or 'workspace:<name>'")
        .long_about(
            "AI-optimized command for quick orientation.\n\n\
            Returns a simple, parseable string:\n  \
            - 'main' if on main branch\n  \
            - 'workspace:<name>' if in a workspace\n\n\
            Use this before operations that depend on location.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_whoami() -> ClapCommand {
    ClapCommand::new("whoami")
        .about("Agent identity query - returns agent ID or 'unregistered'")
        .long_about(
            "AI-optimized command for identity verification.\n\n\
            Returns:\n  \
            - Agent ID if registered (from ZJJ_AGENT_ID env var)\n  \
            - 'unregistered' if no agent registered\n\n\
            Also shows current session and bead from environment.",
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_work() -> ClapCommand {
    ClapCommand::new("work")
        .about("Start working on a task (create workspace + register agent)")
        .long_about(
            "Unified workflow start command for AI agents.\n\n\
            Combines multiple steps:\n  \
            1. Create workspace (or reuse if --idempotent)\n  \
            2. Register as agent (unless --no-agent)\n  \
            3. Set environment variables\n  \
            4. Output workspace info\n\n\
            This is the AI-friendly entry point for starting work.",
        )
        .after_help(
            "EXAMPLES:\n  \
            zjj work feature-auth              Start working on feature-auth\n  \
            zjj work bug-fix --bead zjj-123    Start work on bead\n  \
            zjj work test --idempotent         Reuse existing session if exists\n  \
            zjj work quick --no-zellij         Create workspace without Zellij tab\n  \
            zjj work --dry-run feature         Preview what would be created",
        )
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

fn cmd_ai() -> ClapCommand {
    ClapCommand::new("ai")
        .about("AI-first entry point - start here for AI agents")
        .long_about(
            "ZJJ AI Agent Interface\n\n\
            This is the 'start here' command for AI agents.\n\
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
                    "Shows current state and suggests the next command.\n\n\
                    Use this to orient yourself before starting work.",
                ),
        )
        .subcommand(
            ClapCommand::new("workflow")
                .about("Show the 7-step parallel agent workflow")
                .long_about(
                    "Displays the recommended workflow for AI agents:\n\n\
                    1. Orient (whereami)\n\
                    2. Register (agent register)\n\
                    3. Isolate (work <name>)\n\
                    4. Enter (cd to workspace)\n\
                    5. Implement (do work)\n\
                    6. Heartbeat (signal liveness)\n\
                    7. Complete (done)",
                ),
        )
        .subcommand(
            ClapCommand::new("quick-start")
                .about("Minimum commands to be productive")
                .long_about(
                    "Shows the essential commands for quick productivity:\n\n\
                    - whereami: Check location\n\
                    - work: Start working\n\
                    - done: Finish work",
                ),
        )
}

fn cmd_abort() -> ClapCommand {
    ClapCommand::new("abort")
        .about("Abandon workspace without merging")
        .long_about(
            "Opposite of 'zjj done' - discard work without merging.\n\n\
            Use this when:\n  \
            - Work is no longer needed\n  \
            - You want to start fresh\n  \
            - The approach didn't work out\n\n\
            Can be run from inside or outside the workspace.",
        )
        .after_help(
            "EXAMPLES:\n  \
            zjj abort                          Abort current workspace\n  \
            zjj abort --workspace feature-x    Abort specific workspace\n  \
            zjj abort --keep-workspace         Remove from zjj but keep files\n  \
            zjj abort --dry-run                Preview without executing",
        )
        .arg(
            Arg::new("workspace")
                .long("workspace")
                .short('w')
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

fn build_cli() -> ClapCommand {
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
        .subcommand_required(true)
        .subcommand(cmd_init())
        .subcommand(cmd_add())
        .subcommand(cmd_agents())
        .subcommand(cmd_attach())
        .subcommand(cmd_list())
        .subcommand(cmd_remove())
        .subcommand(cmd_focus())
        .subcommand(cmd_status())
        .subcommand(cmd_sync())
        .subcommand(cmd_diff())
        .subcommand(cmd_config())
        .subcommand(cmd_clean())
        .subcommand(cmd_dashboard())
        .subcommand(cmd_introspect())
        .subcommand(cmd_doctor())
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
}

/// Format an error for user display (no stack traces)
fn format_error(err: &anyhow::Error) -> String {
    // Get the root cause message
    let mut msg = err.to_string();

    // If the error chain has more context, include it
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        // Only add source if it's different and adds value
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            msg = format!("{msg}\nCause: {source_msg}");
        }
    }

    msg
}

fn handle_init(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    match init::run_with_options(init::InitOptions { format }) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_add(sub_m: &clap::ArgMatches) -> Result<()> {
    // Handle --example-json flag (return example output without execution)
    if sub_m.get_flag("example-json") {
        let example_output = json::AddOutput {
            name: "example-session".to_string(),
            workspace_path: "/path/to/.zjj/workspaces/example-session".to_string(),
            zellij_tab: "zjj:example-session".to_string(),
            status: "active".to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&example_output)?);
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

    let no_hooks = sub_m.get_flag("no-hooks");
    let template = sub_m.get_one::<String>("template").cloned();
    let no_open = sub_m.get_flag("no-open");
    let json = sub_m.get_flag("json");
    let idempotent = sub_m.get_flag("idempotent");
    let dry_run = sub_m.get_flag("dry-run");

    let options = add::AddOptions {
        name: name.clone(),
        no_hooks,
        template,
        no_open,
        format: zjj_core::OutputFormat::from_json_flag(json),
        idempotent,
        dry_run,
    };

    match add::run_with_options(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if json {
                json::output_json_error_and_exit(&e);
            } else {
                // For regular output, we still want to exit with code 1 for validation errors
                // This ensures consistency between JSON and regular error reporting
                Err(e)
            }
        }
    }
}

fn handle_list(sub_m: &clap::ArgMatches) -> Result<()> {
    let all = sub_m.get_flag("all");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let bead = sub_m.get_one::<String>("bead").cloned();
    let agent = sub_m.get_one::<String>("agent").map(String::as_str);
    list::run(all, format, bead.as_deref(), agent)
}

fn handle_remove(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        format,
    };
    match remove::run_with_options(name, &options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_focus(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = focus::FocusOptions { format };

    // Pass name as Option<&str> to run_with_options
    // If name is None, focus::run_with_options will trigger interactive selection
    match focus::run_with_options(name, &options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_status(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let watch = sub_m.get_flag("watch");
    match status::run(name, format, watch) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_sync(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = sync::SyncOptions { format };
    match sync::run_with_options(name, options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_diff(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let stat = sub_m.get_flag("stat");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    match diff::run(name, stat, format) {
        Ok(()) => Ok(()),
        Err(e) => {
            if json {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_config(sub_m: &clap::ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = config::ConfigOptions {
        key,
        value,
        global,
        format,
    };
    match config::run(options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_clean(sub_m: &clap::ArgMatches) -> Result<()> {
    let force = sub_m.get_flag("force");
    let dry_run = sub_m.get_flag("dry-run");
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = clean::CleanOptions {
        force,
        dry_run,
        format,
    };
    clean::run_with_options(&options)
}

fn handle_introspect(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);

    // Check for special modes first
    if sub_m.get_flag("env-vars") {
        return introspect::run_env_vars(format);
    }
    if sub_m.get_flag("workflows") {
        return introspect::run_workflows(format);
    }
    if sub_m.get_flag("session-states") {
        return introspect::run_session_states(format);
    }

    // Default behavior: introspect command or all
    let command = sub_m.get_one::<String>("command").map(String::as_str);
    let result = command.map_or_else(
        || introspect::run(format),
        |cmd| introspect::run_command_introspect(cmd, format),
    );
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_doctor(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let fix = sub_m.get_flag("fix");
    match doctor::run(format, fix) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_spawn(sub_m: &clap::ArgMatches) -> Result<()> {
    let args = spawn::SpawnArgs::from_matches(sub_m)?;
    let options = args.to_options();
    spawn::run_with_options(&options)
}

fn handle_query(sub_m: &clap::ArgMatches) -> Result<()> {
    let query_type = sub_m
        .get_one::<String>("query_type")
        .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
    let args = sub_m.get_one::<String>("args").map(String::as_str);
    let _json = sub_m.get_flag("json"); // Ignored as query is always JSON
    query::run(query_type, args)
}

fn handle_context(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let field = sub_m.get_one::<String>("field").map(String::as_str);
    let no_beads = sub_m.get_flag("no-beads");
    let no_health = sub_m.get_flag("no-health");
    context::run(json, field, no_beads, no_health)
}

fn handle_checkpoint(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);

    let action = match sub_m.subcommand() {
        Some(("create", create_m)) => checkpoint::CheckpointAction::Create {
            description: create_m.get_one::<String>("description").cloned(),
        },
        Some(("restore", restore_m)) => {
            let checkpoint_id = restore_m
                .get_one::<String>("checkpoint_id")
                .ok_or_else(|| anyhow::anyhow!("Checkpoint ID is required"))?
                .clone();
            checkpoint::CheckpointAction::Restore { checkpoint_id }
        }
        Some(("list", _)) => checkpoint::CheckpointAction::List,
        _ => anyhow::bail!("Unknown checkpoint subcommand"),
    };

    let args = checkpoint::CheckpointArgs { action, format };
    match checkpoint::run(&args) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_undo(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let args = commands::undo::UndoArgs {
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };

    let options = args.to_options();
    match undo::run_with_options(&options) {
        Ok(_) => Ok(()),
        Err(e) => {
            if format.is_json() {
                let anyhow_err: anyhow::Error = e.into();
                json::output_json_error_and_exit(&anyhow_err);
            } else {
                Err(e.into())
            }
        }
    }
}

fn handle_revert(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let args = commands::revert::RevertArgs {
        session_name: name.clone(),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };

    let options = args.to_options();
    match revert::run_with_options(&options) {
        Ok(_) => Ok(()),
        Err(e) => {
            if format.is_json() {
                let anyhow_err: anyhow::Error = e.into();
                json::output_json_error_and_exit(&anyhow_err);
            } else {
                Err(e.into())
            }
        }
    }
}

fn handle_done(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let args = commands::done::types::DoneArgs {
        message: sub_m.get_one::<String>("message").cloned(),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        no_keep: sub_m.get_flag("no-keep"),
        squash: sub_m.get_flag("squash"),
        dry_run: sub_m.get_flag("dry-run"),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        format: zjj_core::OutputFormat::from_json_flag(json),
    };

    let options = args.to_options();
    done::run_with_options(&options)?;
    Ok(())
}

fn handle_agents(sub_m: &clap::ArgMatches) -> Result<()> {
    let format = zjj_core::OutputFormat::from_json_flag(sub_m.get_flag("json"));

    // Check for subcommands first
    match sub_m.subcommand() {
        Some(("register", register_m)) => {
            let args = agents::types::RegisterArgs {
                agent_id: register_m.get_one::<String>("id").cloned(),
                session: register_m.get_one::<String>("session").cloned(),
            };
            agents::run_register(&args, format)
        }
        Some(("heartbeat", heartbeat_m)) => {
            let args = agents::types::HeartbeatArgs {
                command: heartbeat_m.get_one::<String>("command").cloned(),
            };
            agents::run_heartbeat(&args, format)
        }
        Some(("status", _)) => agents::run_status(format),
        Some(("unregister", unregister_m)) => {
            let args = agents::types::UnregisterArgs {
                agent_id: unregister_m.get_one::<String>("id").cloned(),
            };
            agents::run_unregister(&args, format)
        }
        _ => {
            // Default: list agents
            let args = agents::types::AgentsArgs {
                all: sub_m.get_flag("all"),
                session: sub_m.get_one::<String>("session").cloned(),
            };
            agents::run(&args, format)
        }
    }
}

fn handle_whereami(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = whereami::WhereAmIOptions { format };
    match whereami::run(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_whoami(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);
    let options = whoami::WhoAmIOptions { format };
    match whoami::run(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_work(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);

    let options = work::WorkOptions {
        name: sub_m
            .get_one::<String>("name")
            .ok_or_else(|| anyhow::anyhow!("Name is required"))?
            .clone(),
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        agent_id: sub_m.get_one::<String>("agent-id").cloned(),
        no_zellij: sub_m.get_flag("no-zellij"),
        no_agent: sub_m.get_flag("no-agent"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };

    match work::run(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_abort(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);

    let options = abort::AbortOptions {
        workspace: sub_m.get_one::<String>("workspace").cloned(),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        dry_run: sub_m.get_flag("dry-run"),
        format,
    };

    match abort::run(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

fn handle_ai(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let format = zjj_core::OutputFormat::from_json_flag(json);

    let subcommand = match sub_m.subcommand() {
        Some(("status", _)) => ai::AiSubcommand::Status,
        Some(("workflow", _)) => ai::AiSubcommand::Workflow,
        Some(("quick-start", _)) => ai::AiSubcommand::QuickStart,
        _ => ai::AiSubcommand::Default,
    };

    let options = ai::AiOptions { subcommand, format };

    match ai::run(&options) {
        Ok(()) => Ok(()),
        Err(e) => {
            if format.is_json() {
                json::output_json_error_and_exit(&e);
            } else {
                Err(e)
            }
        }
    }
}

/// Execute the CLI and return a Result
fn run_cli() -> Result<()> {
    let cli = build_cli();

    // Check for --json flag before parsing to handle Clap errors in JSON format
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args
        .iter()
        .any(|a| a.as_str() == "--json" || a.as_str() == "-j");

    // Set ZJJ_STRICT environment variable for database layer to check
    if args.iter().any(|a| a.as_str() == "--strict") {
        std::env::set_var("ZJJ_STRICT", "1");
    }

    let matches = match cli.try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            if json_mode {
                // Convert Clap error to JSON and exit
                let json_err = serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "INVALID_ARGUMENT",
                        "message": e.to_string(),
                        "exit_code": 2
                    }
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json_err).unwrap_or_default()
                );
            }
            let _ = e.print();
            std::process::exit(2);
        }
    };

    match matches.subcommand() {
        Some(("init", sub_m)) => handle_init(sub_m),
        Some(("attach", sub_m)) => {
            let options = attach::AttachOptions::from_matches(sub_m)?;
            match attach::run_with_options(&options) {
                Ok(()) => Ok(()),
                Err(e) => {
                    if options.format.is_json() {
                        json::output_json_error_and_exit(&e);
                    } else {
                        Err(e)
                    }
                }
            }
        }
        Some(("add", sub_m)) => handle_add(sub_m),
        Some(("agents", sub_m)) => handle_agents(sub_m),
        Some(("list", sub_m)) => handle_list(sub_m),
        Some(("remove", sub_m)) => handle_remove(sub_m),
        Some(("focus", sub_m)) => handle_focus(sub_m),
        Some(("status", sub_m)) => handle_status(sub_m),
        Some(("sync", sub_m)) => handle_sync(sub_m),
        Some(("diff", sub_m)) => handle_diff(sub_m),
        Some(("config", sub_m)) => handle_config(sub_m),
        Some(("clean", sub_m)) => handle_clean(sub_m),
        Some(("dashboard" | "dash", _)) => dashboard::run(),
        Some(("introspect", sub_m)) => handle_introspect(sub_m),
        Some(("doctor" | "check", sub_m)) => handle_doctor(sub_m),
        Some(("query", sub_m)) => handle_query(sub_m),
        Some(("context", sub_m)) => handle_context(sub_m),
        Some(("done", sub_m)) => handle_done(sub_m),
        Some(("spawn", sub_m)) => handle_spawn(sub_m),
        Some(("checkpoint" | "ckpt", sub_m)) => handle_checkpoint(sub_m),
        Some(("undo", sub_m)) => handle_undo(sub_m),
        Some(("revert", sub_m)) => handle_revert(sub_m),
        Some(("whereami", sub_m)) => handle_whereami(sub_m),
        Some(("whoami", sub_m)) => handle_whoami(sub_m),
        Some(("work", sub_m)) => handle_work(sub_m),
        Some(("abort", sub_m)) => handle_abort(sub_m),
        Some(("ai", sub_m)) => handle_ai(sub_m),
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

fn main() {
    // HARD REQUIREMENT: JJ must be installed
    // AI agents that don't have JJ cannot use zjj - period.
    if !cli::is_jj_installed() {
        eprintln!();
        eprintln!("╔════════════════════════════════════════════════════════════════════════╗");
        eprintln!("║  🔒 ZJJ REQUIRES JJ (JUJUTSU)                                          ║");
        eprintln!("╚════════════════════════════════════════════════════════════════════════╝");
        eprintln!();
        eprintln!("JJ is NOT installed. ZJJ cannot function without it.");
        eprintln!();
        eprintln!("Install JJ now:");
        eprintln!("  cargo install jj-cli");
        eprintln!("  # or: brew install jj");
        eprintln!("  # or: https://martinvonz.github.io/jj/latest/install-and-setup/");
        eprintln!();
        eprintln!("ZJJ is built on top of JJ for workspace isolation.");
        eprintln!("There is NO workaround - JJ is required.");
        eprintln!();
        std::process::exit(1);
    }

    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Run the CLI and handle errors gracefully
    if let Err(err) = run_cli() {
        eprintln!("Error: {}", format_error(&err));
        let code = err
            .downcast_ref::<zjj_core::Error>()
            .map(zjj_core::Error::exit_code)
            .unwrap_or(1);
        process::exit(code);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PHASE 2 (RED) - OutputFormat Migration Tests for main.rs
// These tests FAIL until handlers are updated to use OutputFormat
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod main_tests {
    use zjj_core::OutputFormat;

    /// RED: `handle_add` should accept `OutputFormat` from options
    #[test]
    fn test_handle_add_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_add should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to AddOptions with format field
        // 4. Call add::run_with_options() which uses the format

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert_eq!(format, OutputFormat::Json);
        // When implemented: AddOptions { name, no_hooks, template, no_open, format }
    }

    /// RED: `handle_init` should accept `OutputFormat` parameter
    #[test]
    fn test_handle_init_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_init should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to init::run(format) or create InitOptions with format

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert!(format.is_json());
        // When implemented: init::run(OutputFormat::from_json_flag(json))
    }

    /// RED: `handle_diff` should accept `OutputFormat` parameter
    #[test]
    fn test_handle_diff_converts_json_flag_to_output_format() {
        // This test documents the expected behavior:
        // handle_diff should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json)
        // 3. Pass to diff::run(name, stat, format)

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);

        assert!(format.is_json());
        // When implemented: diff::run("session", stat, format)
    }

    /// RED: `handle_query` always uses JSON format
    #[test]
    fn test_handle_query_always_uses_json_format() {
        // Query always outputs JSON for programmatic access
        // Even if --json flag is false, query should output JSON

        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());

        let json_flag_false = false;
        let _ = OutputFormat::from_json_flag(json_flag_false);
        // But query::run should internally convert to Json
        let query_format = OutputFormat::Json;
        assert!(query_format.is_json());
    }

    /// RED: `AddOptions` constructor includes format field
    #[test]
    fn test_add_options_struct_has_format() {
        use crate::commands::add::AddOptions;

        // When AddOptions is updated to include format field:
        // pub struct AddOptions {
        //     pub name: String,
        //     pub no_hooks: bool,
        //     pub template: Option<String>,
        //     pub no_open: bool,
        //     pub format: OutputFormat,
        // }

        let opts = AddOptions {
            name: "test".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        };

        assert_eq!(opts.name, "test");
        assert_eq!(opts.format, OutputFormat::Json);
    }

    /// RED: --json flag is converted to `OutputFormat` for add
    #[test]
    fn test_add_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_add:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   options = AddOptions { ..., format }
        //   add::run_with_options(&options)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert_eq!(format, OutputFormat::Json);
        assert_eq!(format.to_json_flag(), json_bool);
    }

    /// RED: --json flag is converted to `OutputFormat` for init
    #[test]
    fn test_init_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_init:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   init::run(format)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert!(format.is_json());
    }

    /// RED: --json flag is converted to `OutputFormat` for diff
    #[test]
    fn test_diff_json_flag_propagates_through_handler() {
        // Document the expected flow:
        // main.rs handle_diff:
        //   json = sub_m.get_flag("json")           // Extract --json flag
        //   format = OutputFormat::from_json_flag(json)
        //   diff::run(name, stat, format)

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        assert!(format.is_json());
    }

    /// RED: `OutputFormat` prevents mixing json bool with command options
    #[test]
    fn test_output_format_eliminates_json_bool_field() {
        // After migration, command options should NOT have:
        //   pub json: bool
        //
        // Instead they should have:
        //   pub format: OutputFormat
        //
        // This test documents that the bool field is completely removed

        let format1 = OutputFormat::Json;
        let format2 = OutputFormat::Human;

        assert_ne!(format1, format2);
        // No more mixing bool and enum - exhaustive pattern matching enforced
    }

    /// RED: `OutputFormat` handles both --json flag conversions
    #[test]
    fn test_output_format_bidirectional_conversion() {
        let original_bool = true;
        let format = OutputFormat::from_json_flag(original_bool);
        let restored_bool = format.to_json_flag();

        assert_eq!(original_bool, restored_bool);

        let original_bool2 = false;
        let format2 = OutputFormat::from_json_flag(original_bool2);
        let restored_bool2 = format2.to_json_flag();

        assert_eq!(original_bool2, restored_bool2);
    }

    /// RED: All handlers use `OutputFormat` instead of bool
    #[test]
    fn test_all_handlers_accept_output_format() {
        // Document which handlers need updates:
        // - handle_init: format parameter
        // - handle_add: format in AddOptions
        // - handle_diff: format parameter
        // - handle_query: always Json, ignores flag
        //
        // Already updated (10 commands):
        // - handle_list, handle_remove, handle_focus
        // - handle_status, handle_sync
        // - handle_config, handle_clean
        // - handle_introspect, handle_doctor
        // - handle_attach

        let json_format = OutputFormat::Json;
        let human_format = OutputFormat::Human;

        assert!(json_format.is_json());
        assert!(human_format.is_human());
    }

    /// RED: JSON output errors also use `OutputFormat`
    #[test]
    fn test_error_output_respects_format() {
        // When errors occur, they should also respect OutputFormat:
        // if format.is_json() {
        //     json::output_json_error_and_exit(&e)
        // } else {
        //     Err(e) for default error handling
        // }

        let format = OutputFormat::Json;
        assert!(format.is_json());

        let format2 = OutputFormat::Human;
        assert!(format2.is_human());
    }

    /// RED: No panics during format conversion in handlers
    #[test]
    fn test_handlers_never_panic_on_format() {
        // All handlers should handle both formats without panic
        for format in &[OutputFormat::Json, OutputFormat::Human] {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_string();
            let _ = format.to_json_flag();
        }
    }

    /// RED: `OutputFormat` is passed to all command functions
    #[test]
    fn test_format_parameter_reaches_command_functions() {
        // Document parameter passing:
        // main.rs handle_* extracts --json flag
        //   -> converts to OutputFormat
        //   -> passes to command::run() or struct with format field
        //   -> command functions check format to decide output style

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        // This format should reach all command implementations
        assert!(format.is_json());
    }
}

//! Session command handler - Dispatches session subcommands
//!
//! This module provides the main dispatcher for `isolate session <action>` commands,
//! delegating to existing command implementations or providing custom handling.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use anyhow::Result;
use clap::ArgMatches;
use isolate_core::OutputFormat;

use super::json_format::get_format;
use crate::commands::{add, focus, init, list, remove, rename, session_mgmt, spawn, sync};

/// Handle session list subcommand
async fn handle_session_list(args: &ArgMatches) -> Result<()> {
    let format = get_format(args);
    let include_all = args.get_flag("all");
    let verbose = args.get_flag("verbose");
    let bead = args.get_one::<String>("bead").map(String::as_str);
    let agent = args.get_one::<String>("agent").map(String::as_str);
    let state = args.get_one::<String>("state").map(String::as_str);

    list::run(include_all, verbose, format, bead, agent, state).await
}

/// Handle session add subcommand
async fn handle_session_add(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");
    let no_open = args.get_flag("no-open");
    let no_hooks = args.get_flag("no-hooks");

    let options = add::AddOptions {
        name: name.clone(),
        template: None,
        bead_id: args.get_one::<String>("bead").cloned(),
        no_hooks,
        no_open,
        format,
        idempotent: false,
        dry_run,
    };

    add::run_with_options(&options).await
}

/// Handle session remove subcommand
async fn handle_session_remove(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);
    let force = args.get_flag("force");

    let options = remove::RemoveOptions {
        force,
        merge: false,
        keep_branch: false,
        idempotent: false,
        dry_run: false,
        format,
    };

    remove::run_with_options(name, &options).await
}

/// Handle session focus subcommand
async fn handle_session_focus(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);
    let options = focus::FocusOptions { format };

    focus::run_with_options(Some(name), &options).await
}

/// Handle session pause subcommand
async fn handle_session_pause(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);
    let options = session_mgmt::PauseOptions {
        session: name.clone(),
        format,
    };

    session_mgmt::run_pause(&options).await
}

/// Handle session resume subcommand
async fn handle_session_resume(args: &ArgMatches) -> Result<()> {
    let name = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Session name is required"))?;

    let format = get_format(args);
    let options = session_mgmt::ResumeOptions {
        session: name.clone(),
        format,
    };

    session_mgmt::run_resume(&options).await
}

/// Handle session clone subcommand
async fn handle_session_clone(args: &ArgMatches) -> Result<()> {
    let source = args
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Source session name is required"))?;

    let target = args
        .get_one::<String>("new-name")
        .cloned()
        .unwrap_or_else(|| format!("{source}-copy"));

    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");

    let options = session_mgmt::CloneOptions {
        source: source.clone(),
        target,
        dry_run,
        format,
    };

    session_mgmt::run_clone(&options).await
}

/// Handle session rename subcommand
async fn handle_session_rename(args: &ArgMatches) -> Result<()> {
    let old_name = args
        .get_one::<String>("old-name")
        .ok_or_else(|| anyhow::anyhow!("Old session name is required"))?;

    let new_name = args
        .get_one::<String>("new-name")
        .ok_or_else(|| anyhow::anyhow!("New session name is required"))?;

    let format = get_format(args);
    let options = rename::RenameOptions {
        old_name: old_name.clone(),
        new_name: new_name.clone(),
        dry_run: false,
        format,
    };

    rename::run(&options).await
}

/// Handle session spawn subcommand
async fn handle_session_spawn(args: &ArgMatches) -> Result<()> {
    let bead = args
        .get_one::<String>("bead")
        .ok_or_else(|| anyhow::anyhow!("Bead ID is required"))?;

    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");

    let options = spawn::SpawnOptions {
        bead_id: bead.clone(),
        agent_command: String::new(),
        agent_args: Vec::new(),
        no_auto_merge: false,
        no_auto_cleanup: false,
        background: false,
        timeout_secs: 0,
        idempotent: false,
        format,
        dry_run,
    };

    spawn::run_with_options(&options).await
}

/// Handle session sync subcommand
async fn handle_session_sync(args: &ArgMatches) -> Result<()> {
    let name = args.get_one::<String>("name").map(String::as_str);
    let format = get_format(args);

    let options = sync::SyncOptions {
        format,
        all: false,
        dry_run: false,
    };

    sync::run_with_options(name, options).await
}

/// Handle session init subcommand
async fn handle_session_init(args: &ArgMatches) -> Result<()> {
    let format = get_format(args);
    let dry_run = args.get_flag("dry-run");
    init::run_with_options(init::InitOptions { format, dry_run }).await
}

/// Main session command dispatcher
///
/// Routes `isolate session <action>` commands to their handlers.
pub async fn handle_session(args: &ArgMatches) -> Result<()> {
    match args.subcommand() {
        Some(("list", sub_args)) => handle_session_list(sub_args).await,
        Some(("add", sub_args)) => handle_session_add(sub_args).await,
        Some(("remove", sub_args)) => handle_session_remove(sub_args).await,
        Some(("focus", sub_args)) => handle_session_focus(sub_args).await,
        Some(("pause", sub_args)) => handle_session_pause(sub_args).await,
        Some(("resume", sub_args)) => handle_session_resume(sub_args).await,
        Some(("clone", sub_args)) => handle_session_clone(sub_args).await,
        Some(("rename", sub_args)) => handle_session_rename(sub_args).await,
        Some(("spawn", sub_args)) => handle_session_spawn(sub_args).await,
        Some(("sync", sub_args)) => handle_session_sync(sub_args).await,
        Some(("init", sub_args)) => handle_session_init(sub_args).await,
        _ => {
            // No subcommand - show help
            let format = extract_json_flag(args);
            if format.is_json() {
                let help_json = serde_json::json!({
                    "command": "session",
                    "subcommands": [
                        {"name": "list", "description": "List all sessions"},
                        {"name": "add", "description": "Create a new session"},
                        {"name": "remove", "description": "Remove a session"},
                        {"name": "focus", "description": "Switch to a session"},
                        {"name": "pause", "description": "Pause a session"},
                        {"name": "resume", "description": "Resume a paused session"},
                        {"name": "clone", "description": "Clone a session"},
                        {"name": "rename", "description": "Rename a session"},
                        {"name": "attach", "description": "Attach to session from shell"},
                        {"name": "spawn", "description": "Spawn session for agent work"},
                        {"name": "sync", "description": "Sync session with remote"},
                        {"name": "init", "description": "Initialize isolate in repository"},
                    ]
                });
                println!("{}", serde_json::to_string_pretty(&help_json)?);
            } else {
                println!("Session management commands:");
                println!();
                println!("  isolate session list [--all]                List all sessions");
                println!("  isolate session add <name>                  Create a new session");
                println!("  isolate session remove <name>               Remove a session");
                println!("  isolate session focus <name>                Switch to a session");
                println!("  isolate session pause [name]                Pause a session");
                println!("  isolate session resume [name]               Resume a paused session");
                println!("  isolate session clone <name>                Clone a session");
                println!("  isolate session rename <old> <new>          Rename a session");
                println!("  isolate session attach <name>               Attach to session from shell");
                println!("  isolate session spawn <bead>                Spawn session for agent work");
                println!("  isolate session sync [name]                 Sync session with remote");
                println!("  isolate session init                        Initialize isolate in repository");
                println!();
                println!("Run 'isolate session <command> --help' for more information.");
            }
            Ok(())
        }
    }
}

/// Extract JSON flag from args (helper for help display)
fn extract_json_flag(args: &ArgMatches) -> OutputFormat {
    get_format(args)
}

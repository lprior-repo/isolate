//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `jjz`

use anyhow::Result;
use clap::{Arg, Command as ClapCommand};

mod cli;
mod commands;
mod db;
mod session;

use commands::{add, diff, focus, init, list, remove, status, sync};

fn build_cli() -> ClapCommand {
    ClapCommand::new("jjz")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Manage JJ workspaces with Zellij sessions")
        .subcommand_required(true)
        .subcommand(
            ClapCommand::new("init").about("Initialize jjz in a JJ repository (or create one)"),
        )
        .subcommand(
            ClapCommand::new("add")
                .about("Create a new session with JJ workspace + Zellij tab")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name for the new session"),
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
                ),
        )
        .subcommand(
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
                ),
        )
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove a session and its workspace")
                .arg(
                    Arg::new("name")
                        .required(true)
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
                ),
        )
        .subcommand(
            ClapCommand::new("focus")
                .about("Switch to a session's Zellij tab")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the session to focus"),
                ),
        )
        .subcommand(
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
                ),
        )
        .subcommand(
            ClapCommand::new("sync")
                .about("Sync a session's workspace with main (rebase)")
                .arg(
                    Arg::new("name")
                        .required(false)
                        .help("Session name to sync (syncs current workspace if omitted)"),
                ),
        )
        .subcommand(
            ClapCommand::new("diff")
                .about("Show diff between session and main branch")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Session name to show diff for"),
                )
                .arg(
                    Arg::new("stat")
                        .long("stat")
                        .action(clap::ArgAction::SetTrue)
                        .help("Show diffstat only (summary of changes)"),
                ),
        )
}

fn main() -> Result<()> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("init", _)) => init::run(),
        Some(("add", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

            let no_hooks = sub_m.get_flag("no-hooks");
            let template = sub_m.get_one::<String>("template").cloned();
            let no_open = sub_m.get_flag("no-open");

            let options = add::AddOptions {
                name: name.clone(),
                no_hooks,
                template,
                no_open,
            };

            add::run_with_options(options)
        }
        Some(("list", sub_m)) => {
            let all = sub_m.get_flag("all");
            let json = sub_m.get_flag("json");
            list::run(all, json)
        }
        Some(("remove", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            let options = remove::RemoveOptions {
                force: sub_m.get_flag("force"),
                merge: sub_m.get_flag("merge"),
                keep_branch: sub_m.get_flag("keep-branch"),
            };
            remove::run_with_options(name, options)
        }
        Some(("focus", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            focus::run(name)
        }
        Some(("status", sub_m)) => {
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            let json = sub_m.get_flag("json");
            let watch = sub_m.get_flag("watch");
            status::run(name, json, watch)
        }
        Some(("sync", sub_m)) => {
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            sync::run(name)
        }
        Some(("diff", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            let stat = sub_m.get_flag("stat");
            diff::run(name, stat)
        }
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

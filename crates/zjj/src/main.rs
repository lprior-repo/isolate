//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `jjz`

use anyhow::Result;
use clap::{Arg, Command as ClapCommand};

mod cli;
mod commands;
mod db;
mod session;

use commands::{add, focus, init, list, remove, status, sync};

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
                ),
        )
        .subcommand(ClapCommand::new("list").about("List all sessions"))
        .subcommand(
            ClapCommand::new("remove")
                .about("Remove a session and its workspace")
                .arg(
                    Arg::new("name")
                        .required(true)
                        .help("Name of the session to remove"),
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
        .subcommand(ClapCommand::new("status").about("Show current ZJJ status and context"))
        .subcommand(
            ClapCommand::new("sync")
                .about("Sync a session's workspace with main (rebase)")
                .arg(
                    Arg::new("name")
                        .required(false)
                        .help("Session name to sync (syncs current workspace if omitted)"),
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
            add::run(name)
        }
        Some(("list", _)) => list::run(),
        Some(("remove", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            remove::run(name)
        }
        Some(("focus", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            focus::run(name)
        }
        Some(("status", _)) => status::run(),
        Some(("sync", sub_m)) => {
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            sync::run(name)
        }
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

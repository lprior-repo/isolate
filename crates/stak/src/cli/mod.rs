pub mod handlers;

use clap::{Arg, Command};

pub fn build_cli() -> Command {
    Command::new("stak")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Merge queue for stacking PRs - local Graphite")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .subcommand(cmd_list())
        .subcommand(cmd_status())
        .subcommand(cmd_enqueue())
        .subcommand(cmd_dequeue())
        .subcommand(cmd_process())
}

fn cmd_list() -> Command {
    Command::new("list").about("List queue entries").arg(
        Arg::new("all")
            .long("all")
            .action(clap::ArgAction::SetTrue)
            .help("Include completed entries"),
    )
}

fn cmd_status() -> Command {
    Command::new("status")
        .about("Show queue status")
        .arg(Arg::new("session").help("Session name to show status for"))
}

fn cmd_enqueue() -> Command {
    Command::new("enqueue")
        .about("Add session to queue")
        .arg(
            Arg::new("session")
                .required(true)
                .help("Session name to enqueue"),
        )
        .arg(
            Arg::new("priority")
                .long("priority")
                .short('p')
                .value_name("N")
                .help("Priority 1-10, lower = higher (default: 5)"),
        )
}

fn cmd_dequeue() -> Command {
    Command::new("dequeue")
        .about("Remove session from queue")
        .arg(
            Arg::new("session")
                .required(true)
                .help("Session name to dequeue"),
        )
}

fn cmd_process() -> Command {
    Command::new("process")
        .about("Process queue entries (merge in order)")
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview without executing"),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .value_name("N")
                .help("Max entries to process (default: all)"),
        )
}

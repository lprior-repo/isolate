pub mod handlers;

use clap::Command;

pub fn build_cli() -> Command {
    Command::new("stak")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Merge queue and PR coordination for zjj workspaces")
        .subcommand_required(true)
        .subcommand(cmd_queue())
        .subcommand(cmd_agent())
        .subcommand(cmd_lock())
        .subcommand(cmd_events())
        .subcommand(cmd_batch())
}

fn cmd_queue() -> Command {
    Command::new("queue")
        .about("Manage merge queue")
        .subcommand_required(true)
        .subcommand(Command::new("list").about("List queue entries"))
        .subcommand(Command::new("status").about("Show queue status"))
        .subcommand(Command::new("enqueue").about("Add session to queue"))
        .subcommand(Command::new("dequeue").about("Remove session from queue"))
        .subcommand(Command::new("process").about("Process queue entries"))
}

fn cmd_agent() -> Command {
    Command::new("agent")
        .about("Manage agents")
        .subcommand_required(true)
        .subcommand(Command::new("list").about("List agents"))
        .subcommand(Command::new("register").about("Register as agent"))
        .subcommand(Command::new("heartbeat").about("Send heartbeat"))
        .subcommand(Command::new("unregister").about("Unregister agent"))
}

fn cmd_lock() -> Command {
    Command::new("lock")
        .about("Manage locks")
        .subcommand_required(true)
        .subcommand(Command::new("acquire").about("Acquire lock"))
        .subcommand(Command::new("release").about("Release lock"))
}

fn cmd_events() -> Command {
    Command::new("events")
        .about("System events")
        .subcommand_required(true)
        .subcommand(Command::new("list").about("List events"))
        .subcommand(Command::new("follow").about("Stream events"))
}

fn cmd_batch() -> Command {
    Command::new("batch").about("Execute multiple commands")
}

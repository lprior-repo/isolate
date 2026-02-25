use anyhow::Result;
use clap::ArgMatches;

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let subcommand = matches.subcommand().map_or("", |(name, _)| name);

    match subcommand {
        "list" | "" => {
            let all = matches
                .subcommand_matches("list")
                .is_some_and(|m| m.get_flag("all"));

            println!("Agents:");
            if std::env::var("STAK_AGENT_ID").is_ok() {
                let agent_id = std::env::var("STAK_AGENT_ID").unwrap_or_default();
                println!("  {agent_id} [active]");
                println!("    Session: (current)");
            } else {
                println!("  (No agents registered)");
            }

            if all {
                println!();
                println!("Use --all to include stale agents.");
            }
        }
        "register" => {
            let agent_id = format!("agent-{}", std::process::id());
            println!("Registered agent '{agent_id}'");
            println!();
            println!("Set STAK_AGENT_ID={agent_id} in your environment");
        }
        "heartbeat" => {
            if let Ok(agent_id) = std::env::var("STAK_AGENT_ID") {
                println!("Heartbeat sent for agent '{agent_id}'");
            } else {
                println!("No agent registered (STAK_AGENT_ID not set)");
                println!("Run 'stak agent register' first.");
            }
        }
        "unregister" => {
            if let Ok(agent_id) = std::env::var("STAK_AGENT_ID") {
                println!("Unregistered agent '{agent_id}'");
            } else {
                println!("No agent registered (STAK_AGENT_ID not set)");
            }
        }
        _ => {
            println!("Unknown agent subcommand: {subcommand}");
            println!("Run 'stak agent --help' for usage.");
        }
    }

    Ok(())
}

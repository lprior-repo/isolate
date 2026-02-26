//! Pane command handler - Dispatches pane subcommands

use anyhow::Result;
use clap::ArgMatches;

pub async fn handle_pane(sub_m: &ArgMatches) -> Result<()> {
    if let Some(("focus", sm)) = sub_m.subcommand() {
        handle_focus(sm)
    } else {
        println!("Use 'isolate pane --help' for more information.");
        Ok(())
    }
}

fn handle_focus(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") || sub_m.get_flag("ai-hints") {
        println!("AI COMMAND FLOW: Focus pane");
        return Ok(());
    }

    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("session is required"))?;
    let direction = sub_m.get_one::<String>("direction").map(String::as_str);

    println!("Focusing session '{session}' (direction: {:?})", direction);
    Ok(())
}

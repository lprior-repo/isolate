//! AI-first command handlers

use anyhow::Result;
use clap::ArgMatches;

pub async fn handle_ai(sub_m: &ArgMatches) -> Result<()> {
    // AI work subcommand was removed - use `zjj work` instead
    match sub_m.subcommand() {
        Some((_, _)) => {
            anyhow::bail!("AI subcommands have been removed. Use `zjj work` instead.")
        }
        _ => unreachable!(),
    }
}

//! AI-first command handlers

use anyhow::Result;
use clap::ArgMatches;

/// Handle AI commands (deprecated - redirects to work command)
#[allow(clippy::unused_async, dead_code)]
pub async fn handle_ai(sub_m: &ArgMatches) -> Result<()> {
    // AI work subcommand was removed - use `isolate work` instead
    match sub_m.subcommand() {
        Some((_, _)) => {
            anyhow::bail!("AI subcommands have been removed. Use `isolate work` instead.")
        }
        _ => unreachable!(),
    }
}

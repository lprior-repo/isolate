use anyhow::Result;
use clap::ArgMatches;

pub mod agent;
pub mod batch;
pub mod events;
pub mod lock;
pub mod queue;

pub async fn dispatch(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("queue", sub_m)) => queue::handle(sub_m).await,
        Some(("agent", sub_m)) => agent::handle(sub_m).await,
        Some(("lock", sub_m)) => lock::handle(sub_m).await,
        Some(("events", sub_m)) => events::handle(sub_m).await,
        Some(("batch", sub_m)) => batch::handle(sub_m).await,
        _ => anyhow::bail!("Unknown command. Run 'stak --help' for usage."),
    }
}

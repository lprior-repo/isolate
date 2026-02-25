use anyhow::Result;
use clap::ArgMatches;

pub async fn dispatch(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub_m)) => handle_list(sub_m).await,
        Some(("status", sub_m)) => handle_status(sub_m).await,
        Some(("enqueue", sub_m)) => handle_enqueue(sub_m).await,
        Some(("dequeue", sub_m)) => handle_dequeue(sub_m).await,
        Some(("process", sub_m)) => handle_process(sub_m).await,
        _ => anyhow::bail!("Unknown command. Run 'stak --help' for usage.");
    }
}

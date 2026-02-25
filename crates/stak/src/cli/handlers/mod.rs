use anyhow::Result;
use clap::ArgMatches;

pub async fn dispatch(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => handle_list().await,
        Some(("status", _)) => handle_status().await,
        Some(("enqueue", sub_m)) => handle_enqueue(sub_m).await,
        Some(("dequeue", sub_m)) => handle_dequeue(sub_m).await,
        Some(("process", _)) => handle_process().await,
        _ => anyhow::bail!("Unknown command. Run 'stak --help' for usage."),
    }
}

async fn handle_list() -> Result<()> {
    println!("Queue Entries:");
    println!("  (No entries - queue is empty)");
    println!();
    println!("Use 'stak enqueue <session>' to add entries.");
    Ok(())
}

async fn handle_status() -> Result<()> {
    println!("Queue Status:");
    println!("  Pending:    0");
    println!("  Processing: 0");
    println!("  Completed:  0");
    println!("  Failed:     0");
    Ok(())
}

async fn handle_enqueue(matches: &ArgMatches) -> Result<()> {
    let session = matches
        .get_one::<String>("session")
        .map(|s| s.as_str())
        .unwrap_or("<session>");
    let priority = matches
        .get_one::<String>("priority")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(5);

    println!("Enqueued session '{session}'");
    println!("  Priority: {priority}");
    println!("  Position: 1");
    Ok(())
}

async fn handle_dequeue(matches: &ArgMatches) -> Result<()> {
    let session = matches
        .get_one::<String>("session")
        .map(|s| s.as_str())
        .unwrap_or("<session>");
    println!("Dequeued session '{session}'");
    Ok(())
}

async fn handle_process() -> Result<()> {
    println!("Processing queue...");
    println!("  No entries to process");
    Ok(())
}

//! Queue command handler - Dispatches queue subcommands

use anyhow::Result;
use clap::ArgMatches;

pub async fn handle_queue(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", _)) => handle_list().await,
        Some(("status", _)) => handle_status().await,
        Some(("enqueue", sm)) => handle_enqueue(sm).await,
        Some(("dequeue", sm)) => handle_dequeue(sm).await,
        Some(("process", _)) => handle_process().await,
        _ => {
            println!("Use 'isolate queue --help' for more information.");
            Ok(())
        }
    }
}

async fn handle_list() -> Result<()> {
    println!("Merge Queue:");
    println!("  (Empty)");
    Ok(())
}

async fn handle_status() -> Result<()> {
    println!("Queue Status: Active");
    Ok(())
}

async fn handle_enqueue(sub_m: &ArgMatches) -> Result<()> {
    let session = sub_m.get_one::<String>("session").expect("required");
    let _priority = sub_m.get_one::<String>("priority")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(5);
    
    println!("Enqueued session '{session}'");
    Ok(())
}

async fn handle_dequeue(sub_m: &ArgMatches) -> Result<()> {
    let session = sub_m.get_one::<String>("session").expect("required");
    println!("Dequeued session '{session}'");
    Ok(())
}

async fn handle_process() -> Result<()> {
    println!("Processing queue...");
    Ok(())
}

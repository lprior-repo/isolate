//! Queue command handler - Dispatches queue subcommands

use anyhow::Result;
use clap::ArgMatches;

pub async fn handle_queue(sub_m: &ArgMatches) -> Result<()> {
    match sub_m.subcommand() {
        Some(("list", _)) => {
            handle_list();
            Ok(())
        }
        Some(("status", _)) => {
            handle_status();
            Ok(())
        }
        Some(("enqueue", sm)) => handle_enqueue(sm),
        Some(("dequeue", sm)) => handle_dequeue(sm),
        Some(("process", _)) => {
            handle_process();
            Ok(())
        }
        _ => {
            println!("Use 'isolate queue --help' for more information.");
            Ok(())
        }
    }
}

fn handle_list() {
    println!("Merge Queue:");
    println!("  (Empty)");
}

fn handle_status() {
    println!("Queue Status: Active");
}

fn handle_enqueue(sub_m: &ArgMatches) -> Result<()> {
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("session is required"))?;
    let _priority = sub_m
        .get_one::<String>("priority")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(5);

    println!("Enqueued session '{session}'");
    Ok(())
}

fn handle_dequeue(sub_m: &ArgMatches) -> Result<()> {
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("session is required"))?;
    println!("Dequeued session '{session}'");
    Ok(())
}

fn handle_process() {
    println!("Processing queue...");
}

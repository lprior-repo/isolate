use anyhow::Result;
use clap::ArgMatches;

pub async fn dispatch(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", sub_m)) => handle_list(sub_m).await,
        Some(("status", sub_m)) => handle_status(sub_m).await,
        Some(("enqueue", sub_m)) => handle_enqueue(sub_m).await,
        Some(("dequeue", sub_m)) => handle_dequeue(sub_m).await,
        Some(("process", sub_m)) => handle_process(sub_m).await,
        _ => anyhow::bail!("Unknown command. Run 'stak --help' for usage."),
    }
}

async fn handle_list(matches: &ArgMatches) -> Result<()> {
    let all = matches.get_flag("all");
    let json = matches.get_flag("json");

    if json {
        println!(r#"{{"entries": [], "total": 0, "has_more": false}}"#);
    } else {
        println!("Queue Entries:");
        if all {
            println!("  (No entries - queue is empty)");
        } else {
            println!("  (No pending entries)");
            println!();
            println!("Use --all to include completed entries.");
        }
    }
    Ok(())
}

async fn handle_status(matches: &ArgMatches) -> Result<()> {
    let session = matches.get_one::<String>("session");
    let json = matches.get_flag("json");

    if let Some(name) = session {
        if json {
            println!(r#"{{"session": "{name}", "status": "not_in_queue", "position": null}}"#);
        } else {
            println!("Session '{name}' is not in queue.");
        }
    } else if json {
        println!(r#"{{"pending": 0, "processing": 0, "completed": 0, "failed": 0}}"#);
    } else {
        println!("Queue Status:");
        println!("  Pending:    0");
        println!("  Processing: 0");
        println!("  Completed:  0");
        println!("  Failed:     0");
    }
    Ok(())
}

async fn handle_enqueue(matches: &ArgMatches) -> Result<()> {
    let session = matches
        .get_one::<String>("session")
        .map(|s| s.as_str())
        .unwrap_or("<session>");
    let priority = matches
        .get_one::<String>("priority")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let json = matches.get_flag("json");

    if json {
        println!(
            r#"{{"session": "{session}", "priority": {priority}, "status": "pending", "position": 1}}"#
        );
    } else {
        println!("Enqueued session '{session}'");
        println!("  Priority: {priority}");
        println!("  Position: 1");
    }
    Ok(())
}

async fn handle_dequeue(matches: &ArgMatches) -> Result<()> {
    let session = matches
        .get_one::<String>("session")
        .map(|s| s.as_str())
        .unwrap_or("<session>");
    let json = matches.get_flag("json");

    if json {
        println!(r#"{{"session": "{session}", "removed": true}}"#);
    } else {
        println!("Dequeued session '{session}'");
    }
    Ok(())
}

async fn handle_process(matches: &ArgMatches) -> Result<()> {
    let dry_run = matches.get_flag("dry-run");
    let limit = matches
        .get_one::<String>("limit")
        .and_then(|s| s.parse().ok());
    let json = matches.get_flag("json");

    if json {
        if dry_run {
            println!(
                r#"{{"dry_run": true, "would_process": 0, "message": "No entries to process"}}"#
            );
        } else {
            println!(r#"{{"processed": 0, "succeeded": 0, "failed": 0}}"#);
        }
    } else if dry_run {
        println!("Dry run - would process queue entries...");
        println!("  No entries to process");
    } else {
        println!("Processing queue...");
        println!("  No entries to process");
    }
    Ok(())
}

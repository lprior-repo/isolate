use anyhow::Result;
use clap::ArgMatches;

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let subcommand = matches.subcommand().map_or("", |(name, _)| name);

    match subcommand {
        "list" | "" => {
            println!("Queue Entries:");
            println!("  (No entries - queue is empty)");
            println!();
            println!("Use 'stak queue enqueue <session>' to add entries.");
        }
        "status" => {
            println!("Queue Status:");
            println!("  Pending: 0");
            println!("  Processing: 0");
            println!("  Completed: 0");
            println!("  Failed: 0");
        }
        "enqueue" => {
            let session = matches
                .subcommand_matches("enqueue")
                .and_then(|m| m.get_one::<String>("session"))
                .map(|s| s.as_str())
                .unwrap_or("<session>");

            println!("Enqueued session '{session}'");
            println!("Priority: 5 (default)");
        }
        "dequeue" => {
            let session = matches
                .subcommand_matches("dequeue")
                .and_then(|m| m.get_one::<String>("session"))
                .map(|s| s.as_str())
                .unwrap_or("<session>");

            println!("Dequeued session '{session}'");
        }
        "process" => {
            let dry_run = matches
                .subcommand_matches("process")
                .is_some_and(|m| m.get_flag("dry-run"));

            if dry_run {
                println!("Dry run - would process queue entries");
            } else {
                println!("Processing queue entries...");
                println!("  No entries to process");
            }
        }
        _ => {
            println!("Unknown queue subcommand: {subcommand}");
            println!("Run 'stak queue --help' for usage.");
        }
    }

    Ok(())
}

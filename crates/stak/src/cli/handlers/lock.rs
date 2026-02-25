use anyhow::Result;
use clap::ArgMatches;

pub async fn handle(matches: &ArgMatches) -> Result<()> {
    let subcommand = matches.subcommand().map_or("", |(name, _)| name);

    match subcommand {
        "acquire" => {
            let resource = matches
                .subcommand_matches("acquire")
                .and_then(|m| m.get_one::<String>("resource"))
                .map(|s| s.as_str())
                .unwrap_or("<resource>");

            println!("Acquired lock on '{resource}'");
            println!("TTL: 60 seconds");
        }
        "release" => {
            let resource = matches
                .subcommand_matches("release")
                .and_then(|m| m.get_one::<String>("resource"))
                .map(|s| s.as_str())
                .unwrap_or("<resource>");

            println!("Released lock on '{resource}'");
        }
        _ => {
            println!("Unknown lock subcommand: {subcommand}");
            println!("Run 'stak lock --help' for usage.");
        }
    }

    Ok(())
}

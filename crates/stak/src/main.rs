use anyhow::Result;
use stak::cli::build_cli;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = build_cli().get_matches();
    stak::cli::handlers::dispatch(&matches).await
}

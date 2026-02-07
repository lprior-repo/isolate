//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `zjj`

use std::{process, time::Duration};

mod beads;
mod cli;
mod commands;
mod db;
mod hooks;
mod json;
mod progress;
mod selector;
mod session;

use cli::handlers::{format_error, run_cli};
use zjj_core::ShutdownCoordinator;

#[tokio::main]

async fn main() {
    // HARD REQUIREMENT: JJ must be installed

    // AI agents that don't have JJ cannot use zjj - period.

    if !cli::is_jj_installed().await {
        #[allow(clippy::print_stderr)]
        {
            eprintln!();

            eprintln!("╔════════════════════════════════════════════════════════════════════════╗");

            eprintln!("║  🔒 ZJJ REQUIRES JJ (JUJUTSU)                                          ║");

            eprintln!("╚════════════════════════════════════════════════════════════════════════╝");

            eprintln!();

            eprintln!("JJ is NOT installed. ZJJ cannot function without it.");

            eprintln!();

            eprintln!("Install JJ now:");

            eprintln!("  cargo install jj-cli");

            eprintln!("  # or: brew install jj");

            eprintln!("  # or: https://martinvonz.github.io/jj/latest/install-and-setup/");

            eprintln!();

            eprintln!("ZJJ is built on top of JJ for workspace isolation.");

            eprintln!("There is NO workaround - JJ is required.");

            eprintln!();
        }

        #[allow(clippy::exit)]
        std::process::exit(1);
    }

    // Initialize tracing subscriber for logging

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Setup shutdown coordinator
    let shutdown_coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

    // Setup signal channels
    let (mut sigint, mut sigterm) = match zjj_core::signal_channels().await {
        Ok(channels) => channels,
        Err(e) => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("Error: Failed to setup signal handlers: {e}");
            }
            #[allow(clippy::exit)]
            process::exit(1);
        }
    };

    // Run the CLI with signal handling
    let cli_result = tokio::select! {
        result = run_cli() => result,
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT, initiating shutdown...");
            let _ = shutdown_coordinator.shutdown().await;
            Err(anyhow::anyhow!("Shutdown requested"))
        }
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, initiating shutdown...");
            let _ = shutdown_coordinator.shutdown().await;
            Err(anyhow::anyhow!("Shutdown requested"))
        }
    };

    // Handle errors gracefully
    if let Err(err) = cli_result {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("Error: {}", format_error(&err));
        }

        let code = err
            .downcast_ref::<zjj_core::Error>()
            .map(zjj_core::Error::exit_code)
            .unwrap_or_else(|| 1);

        #[allow(clippy::exit)]
        process::exit(code);
    }
}

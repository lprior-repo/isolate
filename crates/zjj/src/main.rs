//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `zjj`

use std::process;

mod cli;
mod commands;
mod db;
mod hooks;
mod json;
mod selector;
mod session;

use cli::handlers::{format_error, run_cli};



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



    // Run the CLI and handle errors gracefully

    if let Err(err) = run_cli().await {

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

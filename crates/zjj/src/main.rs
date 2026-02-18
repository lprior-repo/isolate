//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `zjj`

// Pragmatic allowances for existing code patterns
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::redundant_locals)]
#![allow(dead_code)]

//! Pragmatic test allowances - brutal test scenarios may use unwrap/expect/panic
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::ignored_unit_patterns,
        clippy::option_if_let_else,
        clippy::manual_let_else,
        clippy::needless_collect,
        clippy::await_holding_lock,
        clippy::significant_drop_tightening,
        clippy::redundant_clone,
        clippy::no_effect_underscore_binding,
        clippy::unnecessary_semicolon,
        clippy::needless_borrows_for_generic_args,
        clippy::items_after_statements,
        unused_must_use
    )
)]

use std::{panic, process, time::Duration};

mod beads;
mod cli;
mod command_context;
mod commands;
mod db;
mod hooks;
mod json;
mod session;

use cli::handlers::{format_error, run_cli};
use zjj_core::ShutdownCoordinator;

/// Install panic hook to handle broken pipe gracefully
///
/// When piping output (e.g., `zjj list | head -n 5`), if the receiving end
/// closes early, println!/print! macros panic with "Broken pipe (os error 32)".
///
/// Unix convention: exit with code 0 for SIGPIPE (reader closed pipe).
/// This hook catches broken pipe panics and exits cleanly, propagating
/// all other panics normally.
fn install_broken_pipe_handler() {
    let original_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        // Match on panic payload to detect broken pipe
        let is_broken_pipe = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(|msg| msg.contains("Broken pipe") || msg.contains("os error 32"))
            .or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<&str>()
                    .map(|msg| msg.contains("Broken pipe") || msg.contains("os error 32"))
            })
            .unwrap_or(false);

        if is_broken_pipe {
            // Broken pipe is not an error - exit silently with code 0
            // This follows Unix convention for SIGPIPE
            #[allow(clippy::exit)]
            process::exit(0);
        }

        // Propagate other panics to original handler
        original_hook(panic_info);
    }));
}

#[tokio::main]

async fn main() {
    // Install broken pipe handler BEFORE any output
    install_broken_pipe_handler();
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

        let code = json::semantic_exit_code(&err);

        #[allow(clippy::exit)]
        process::exit(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_broken_pipe_handler_does_not_crash() {
        // This test verifies the panic hook can be installed without crashing
        install_broken_pipe_handler();
        // If we reach here, the hook was installed successfully
    }

    #[test]
    fn test_broken_pipe_detection_in_message() {
        // Test various broken pipe message formats
        let test_cases = vec![
            "Broken pipe (os error 32)",
            "failed printing to stdout: Broken pipe (os error 32)",
            "os error 32",
            "some prefix Broken pipe some suffix",
        ];

        for msg in test_cases {
            let msg_string = msg.to_string();
            let contains_broken_pipe =
                msg_string.contains("Broken pipe") || msg_string.contains("os error 32");
            assert!(
                contains_broken_pipe,
                "Message should be detected as broken pipe: {msg}"
            );
        }
    }

    #[test]
    fn test_non_broken_pipe_messages_not_detected() {
        // Test that other error messages are not mistaken for broken pipe
        let test_cases = vec![
            "Connection reset",
            "Permission denied",
            "os error 13",
            "Some other error",
        ];

        for msg in test_cases {
            let msg_string = msg.to_string();
            let contains_broken_pipe =
                msg_string.contains("Broken pipe") || msg_string.contains("os error 32");
            assert!(
                !contains_broken_pipe,
                "Message should NOT be detected as broken pipe: {msg}"
            );
        }
    }
}

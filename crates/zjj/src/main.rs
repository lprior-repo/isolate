//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `jjz`
//!
//! ## Functional Rust Compiler Enforcements
//!
//! The following lints are denied:
//! - `unwrap_used` - prevents runtime panics from `.unwrap()`
//! - `expect_used` - prevents runtime panics from `.expect()`
//! - `panic` - prevents explicit panics

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

mod app;
mod cli;
mod commands;
mod database;
mod json_output;
mod session;

/// Main entry point for the jjz CLI
///
/// This is a minimal entry point that:
/// 1. Parses early flags
/// 2. Delegates to app module for all logic
fn main() {
    let config = cli::setup::parse_early_flags();
    app::run(&config);
}

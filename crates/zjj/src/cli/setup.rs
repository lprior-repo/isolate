//! Global setup utilities for CLI initialization
//!
//! Provides functional setup routines for:
//! - Early flag parsing (--help-json, --json)
//! - Logging initialization with tracing
//! - Tokio runtime creation

use std::io::Write;

use anyhow::{Context, Result};

/// Configuration for CLI setup
#[derive(Debug, Clone)]
pub struct SetupConfig {
    /// Whether JSON mode is enabled
    pub json_mode: bool,
    /// Whether help JSON was requested
    pub help_json_requested: bool,
}

/// Parse early CLI flags before full argument parsing
///
/// This function checks for:
/// - `--help-json`: Request JSON help output
/// - `--json`: Enable JSON output mode
///
/// Returns a `SetupConfig` with the parsed flags.
pub fn parse_early_flags() -> SetupConfig {
    let args: Vec<String> = std::env::args().collect();

    SetupConfig {
        help_json_requested: args.iter().any(|arg| arg == "--help-json"),
        json_mode: args.iter().any(|arg| arg == "--json"),
    }
}

/// Initialize tracing subscriber for logging
///
/// Configures the tracing subscriber with:
/// - Environment filter (defaults to INFO level)
/// - Stderr output (to avoid mixing with stdout)
///
/// # Errors
/// Returns an error if the subscriber initialization fails
pub fn init_tracing() -> Result<()> {
    let result = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .try_init();
    result.map_err(|e| anyhow::anyhow!("Failed to initialize tracing subscriber: {e}"))
}

/// Create a tokio runtime for async operations
///
/// # Errors
/// Returns an error if runtime creation fails
pub fn create_runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new().context("Failed to create tokio runtime")
}

/// Output an error message, respecting JSON mode
///
/// # Arguments
/// * `json_mode` - Whether to output in JSON format
/// * `error_code` - Error code for JSON output
/// * `message` - Error message
pub fn output_error(json_mode: bool, error_code: &str, message: &str) {
    if json_mode {
        super::output_json_error(error_code, message, None);
    } else {
        let _ = writeln!(std::io::stderr(), "{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_early_flags_empty() {
        // Note: Can't easily test without manipulating env::args
        // This is a placeholder for potential future improvements
        let _config = parse_early_flags();
        // In real test environment, these will be based on actual args
    }

    #[test]
    fn test_create_runtime() {
        let result = create_runtime();
        assert!(result.is_ok());
    }
}

//! Context command - full environment state for AI agents (zjj-k1w)
//!
//! This command provides a single JSON endpoint for AI agents to understand
//! the complete environment context in one API call.
//!
//! Module structure:
//! - `types` - Data structures for context output
//! - `env` - Environment gathering functions using functional patterns
//! - `format` - Output formatting functions

pub mod env;
pub mod format;
pub mod types;

use anyhow::Result;

pub use types::ContextOutput;

/// Run the context command
///
/// # Arguments
/// * `json` - Whether to output JSON format (true) or human-readable (false)
///
/// # Errors
/// Returns error if context gathering or serialization fails
pub async fn run(json: bool) -> Result<()> {
    // Gather all context using functional composition
    let context = env::gather_context().await;

    // Create output structure
    let output = ContextOutput {
        success: true,
        context,
    };

    if json {
        // Serialize to JSON using functional pattern
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Format as human-readable text
        format::format_human_readable(&output.context);
    }

    Ok(())
}

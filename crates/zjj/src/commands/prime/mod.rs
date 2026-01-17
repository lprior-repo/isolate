//! Prime command - AI workflow context for session recovery (zjj-9l09)
//!
//! This command provides curated context for AI agents to recover from context loss.
//! Similar to `bd prime`, it outputs essential workflow information in a format
//! optimized for AI agent consumption.
//!
//! Output includes:
//! - JJ repository status
//! - Active ZJJ sessions
//! - Command reference by category
//! - Beads integration status
//! - Common workflows

pub mod commands_catalog;
pub mod formatting;
pub mod jj_status;
pub mod output_types;
pub mod workflows;
pub mod zjj_status;

use anyhow::Result;

use self::{
    commands_catalog::{build_command_categories, check_beads_status},
    formatting::print_markdown_context,
    jj_status::gather_jj_status,
    output_types::PrimeOutput,
    workflows::build_workflow_sections,
    zjj_status::{gather_active_sessions, gather_zjj_status},
};

/// Run the prime command
pub async fn run(json: bool) -> Result<()> {
    let output = gather_prime_context().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_markdown_context(&output);
    }

    Ok(())
}

/// Run the prime command with quiet mode support
pub async fn run_with_quiet(json: bool, quiet: bool) -> Result<()> {
    if quiet {
        // Suppress all output in quiet mode (for hooks)
        return Ok(());
    }
    run(json).await
}

/// Gather all context for prime output
///
/// Orchestrates gathering of all the different context components:
/// - JJ repository status
/// - ZJJ initialization and session status
/// - Active sessions
/// - Command categories
/// - Beads integration status
/// - Workflow sections
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
async fn gather_prime_context() -> Result<PrimeOutput> {
    let jj_status = gather_jj_status();
    let zjj_status = gather_zjj_status().await;
    let sessions = gather_active_sessions().await;
    let commands = build_command_categories();
    let beads_status = check_beads_status();
    let workflows = build_workflow_sections();

    Ok(PrimeOutput {
        jj_status,
        zjj_status,
        sessions,
        commands,
        beads_status,
        workflows,
    })
}

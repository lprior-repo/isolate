//! Output formatting for sync operations
//!
//! Handles both JSON and text output formatting for sync results.

use anyhow::Result;

use super::{operations::SessionSyncResult, rebase::RebaseStats, SyncOptions};
use crate::json_output::{SyncError, SyncOutput};

/// Output success message for single session sync
pub fn output_sync_success(name: &str, stats: &RebaseStats, options: SyncOptions) -> Result<()> {
    if options.json {
        let output = SyncOutput {
            success: true,
            session_name: Some(name.to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
            rebased_commits: Some(stats.rebased_commits),
            conflicts: Some(stats.conflicts),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("Synced session '{name}' with main");
    }
    Ok(())
}

/// Output failure message for single session sync
pub fn output_sync_failure(name: &str, error: anyhow::Error, options: SyncOptions) -> Result<()> {
    if options.json {
        let output = SyncOutput {
            success: false,
            session_name: Some(name.to_string()),
            synced_count: 0,
            failed_count: 1,
            errors: vec![SyncError {
                session_name: name.to_string(),
                error: error.to_string(),
            }],
            rebased_commits: Some(0),
            conflicts: Some(0),
        };
        if let Ok(json_str) = serde_json::to_string(&output) {
            println!("{json_str}");
        }
    }
    Err(error)
}

/// Output message when no sessions exist
pub fn output_no_sessions(options: SyncOptions) -> Result<()> {
    if options.json {
        let output = SyncOutput {
            success: true,
            session_name: None,
            synced_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            rebased_commits: None,
            conflicts: None,
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("No sessions to sync");
    }
    Ok(())
}

/// Output results for all sessions sync
pub fn output_all_sessions_results(results: &[SessionSyncResult], options: SyncOptions) {
    let (success_count, failure_count, errors) = super::operations::aggregate_results(results);

    if options.json {
        output_json_results(success_count, failure_count, errors);
    } else {
        output_text_results(results.len(), success_count, failure_count, &errors);
    }

    if failure_count > 0 {
        std::process::exit(2); // Exit code 2: System error (one or more syncs failed)
    }
}

/// Output JSON results for all sessions
fn output_json_results(success_count: usize, failure_count: usize, errors: Vec<SyncError>) {
    let output = SyncOutput {
        success: failure_count == 0,
        session_name: None,
        synced_count: success_count,
        failed_count: failure_count,
        errors,
        rebased_commits: None,
        conflicts: None,
    };
    if let Ok(json_str) = serde_json::to_string(&output) {
        println!("{json_str}");
    }
}

/// Output text results for all sessions
fn output_text_results(
    total: usize,
    success_count: usize,
    failure_count: usize,
    errors: &[SyncError],
) {
    println!("Syncing {total} session(s)...");

    // Show individual session results
    for error in errors {
        println!(
            "Syncing '{}' ... FAILED: {}",
            error.session_name, error.error
        );
    }

    // Show successful syncs
    let successful = total.saturating_sub(errors.len());
    for i in 0..successful {
        println!("Session {} ... OK", i.saturating_add(1));
    }

    println!();
    println!("Summary: {success_count} succeeded, {failure_count} failed");

    if !errors.is_empty() {
        println!("\nErrors:");
        for error in errors {
            println!("  {}: {}", error.session_name, error.error);
        }
    }
}

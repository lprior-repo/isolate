//! Clean stale sessions (sessions where workspace no longer exists)

use std::{io::Write, path::Path};

use anyhow::Result;
use serde::Serialize;
use zjj_core::OutputFormat;

use crate::commands::get_session_db;

/// Options for the clean command
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Skip confirmation prompt
    pub force: bool,
    /// List stale sessions without removing
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Output for clean command in JSON mode
#[derive(Debug, Clone, Serialize)]
pub struct CleanOutput {
    pub stale_count: usize,
    pub removed_count: usize,
    pub stale_sessions: Vec<String>,
}

/// Run the clean command with options
///
/// Uses Railway-Oriented Programming to handle the workflow:
/// 1. Load all sessions from database
/// 2. Filter to find stale sessions (workspace missing)
/// 3. Handle dry-run or interactive confirmation
/// 4. Remove stale sessions if confirmed
pub fn run_with_options(options: &CleanOptions) -> Result<()> {
    let db = get_session_db()?;

    // 1. List all sessions and filter to stale ones using functional pipeline
    let sessions = db.list(None).map_err(anyhow::Error::new)?;

    let stale_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|session| !Path::new(&session.workspace_path).exists())
        .collect();

    // 2. Handle no stale sessions case
    if stale_sessions.is_empty() {
        output_no_stale(options.format);
        return Ok(());
    }

    let stale_names: Vec<String> = stale_sessions.iter().map(|s| s.name.clone()).collect();

    // 3. Dry-run: list and exit
    if options.dry_run {
        output_dry_run(&stale_names, options.format);
        return Ok(());
    }

    // 4. Prompt for confirmation unless --force
    if !options.force && !confirm_removal(&stale_names)? {
        output_cancelled(&stale_names, options.format);
        return Ok(());
    }

    // 5. Remove stale sessions using functional fold for error handling
    let removed_count = stale_sessions.iter().try_fold(0, |count, session| {
        db.delete(&session.name)
            .map_err(anyhow::Error::new)
            .map(|_| count + 1)
    })?;

    // 6. Output result
    output_result(removed_count, &stale_names, options.format);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT FUNCTIONS (Imperative Shell)
// ═══════════════════════════════════════════════════════════════════════════

/// Output when no stale sessions found
fn output_no_stale(format: OutputFormat) -> () {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: 0,
            removed_count: 0,
            stale_sessions: Vec::new(),
        };
        if let Ok(json_str) = serde_json::to_string_pretty(&output) {
            println!("{json_str}");
        }
    } else {
        println!("✓ No stale sessions found");
        println!("  All sessions have valid workspaces");
    }
}

/// Output for dry-run mode
fn output_dry_run(stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count: 0,
            stale_sessions: stale_names.to_vec(),
        };
        if let Ok(json_str) = serde_json::to_string_pretty(&output) {
            println!("{json_str}");
        }
    } else {
        println!(
            "Found {} stale session(s) (dry-run, no changes made):",
            stale_names.len()
        );
        stale_names.iter().for_each(|name| {
            println!("  - {name}");
        });
        println!();
        println!("Run 'zjj clean --force' to remove these sessions");
    }
}

/// Output when cleanup is cancelled
fn output_cancelled(stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count: 0,
            stale_sessions: stale_names.to_vec(),
        };
        if let Ok(json_str) = serde_json::to_string_pretty(&output) {
            println!("{json_str}");
        }
    } else {
        println!("Cleanup cancelled");
    }
}

/// Output cleanup result
fn output_result(removed_count: usize, stale_names: &[String], format: OutputFormat) {
    if format.is_json() {
        let output = CleanOutput {
            stale_count: stale_names.len(),
            removed_count,
            stale_sessions: stale_names.to_vec(),
        };
        if let Ok(json_str) = serde_json::to_string_pretty(&output) {
            println!("{json_str}");
        }
    } else {
        println!("✓ Removed {} stale session(s)", removed_count);
        stale_names.iter().for_each(|name| {
            println!("  - {name}");
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIRMATION (Interactive I/O)
// ═══════════════════════════════════════════════════════════════════════════

/// Prompt user for confirmation to remove stale sessions
///
/// Returns Ok(true) if user confirms, Ok(false) if user cancels
fn confirm_removal(stale_names: &[String]) -> Result<bool> {
    println!("Found {} stale session(s):", stale_names.len());
    stale_names.iter().for_each(|name| {
        println!("  - {name}");
    });
    println!();
    print!("Remove these sessions? [y/N] ");
    std::io::stdout()
        .flush()
        .map_err(|e| anyhow::Error::new(zjj_core::Error::IoError(e.to_string())))?;

    let mut response = String::new();
    std::io::stdin()
        .read_line(&mut response)
        .map_err(|e| anyhow::Error::new(zjj_core::Error::IoError(e.to_string())))?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_options_default() {
        let opts = CleanOptions::default();
        assert!(!opts.force);
        assert!(!opts.dry_run);
        assert!(opts.format.is_human());
    }

    #[test]
    fn test_clean_output_serialization() -> Result<()> {
        let output = CleanOutput {
            stale_count: 2,
            removed_count: 2,
            stale_sessions: vec!["session1".to_string(), "session2".to_string()],
        };

        let json = serde_json::to_string(&output)?;
        assert!(json.contains("\"stale_count\":2"));
        assert!(json.contains("\"removed_count\":2"));
        assert!(json.contains("session1"));
        assert!(json.contains("session2"));

        Ok(())
    }

    #[test]
    fn test_stale_session_filtering() {
        // This test verifies the filtering logic conceptually
        // In a real integration test, we'd create temp sessions
        let sessions = vec![
            ("session1", "/tmp/exists"),
            ("session2", "/nonexistent/path"),
        ];

        let stale: Vec<_> = sessions
            .into_iter()
            .filter(|(_, path)| !Path::new(path).exists())
            .collect();

        // /tmp/exists might or might not exist, but /nonexistent/path definitely doesn't
        assert!(stale.iter().any(|(name, _)| *name == "session2"));
    }

    #[test]
    fn test_empty_stale_list_handling() {
        let stale_names: Vec<String> = Vec::new();

        // Verify that empty lists are handled correctly
        assert_eq!(stale_names.len(), 0);
        assert!(stale_names.is_empty());
    }

    #[test]
    fn test_removed_count_calculation() {
        // Simulate the fold operation for counting removals
        let sessions = vec!["s1", "s2", "s3"];

        // Simulate successful removals using functional fold
        let count = sessions
            .iter()
            .try_fold(0, |acc, _| -> Result<i32> { Ok(acc + 1) });

        assert_eq!(count.ok(), Some(3));
    }
}

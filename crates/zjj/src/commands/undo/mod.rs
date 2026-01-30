//! Undo command - Revert last done operation
//!
//! This command:
//! 1. Reads undo history from .zjj/undo.log
//! 2. Validates undo is possible (not pushed to remote)
//! 3. Reverts the last merge operation
//! 4. Restores workspace state
//! 5. Updates undo history

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zjj_core::{log_recovery, OutputFormat, RecoveryPolicy};

use crate::{
    cli::jj_root,
    commands::context::{detect_location, Location},
};

pub mod types;

pub use types::{UndoArgs, UndoError, UndoExitCode, UndoOptions, UndoOutput};

const UNDO_LOG_PATH: &str = ".zjj/undo.log";
const WORKSPACE_RETENTION_HOURS: u64 = 24;

/// Run the undo command with options
pub fn run_with_options(options: &UndoOptions) -> Result<UndoExitCode, UndoError> {
    let result = execute_undo(options);

    match &result {
        Ok(output) => {
            output_result(output, options.format)?;
            Ok(UndoExitCode::Success)
        }
        Err(e) => {
            output_error(e, options.format)?;
            Ok(match e {
                UndoError::AlreadyPushedToRemote => UndoExitCode::AlreadyPushed,
                UndoError::NoUndoHistory => UndoExitCode::NoHistory,
                UndoError::InvalidState { .. } => UndoExitCode::InvalidState,
                _ => UndoExitCode::OtherError,
            })
        }
    }
}

/// Core undo logic using Railway-Oriented Programming
fn execute_undo(options: &UndoOptions) -> Result<UndoOutput, UndoError> {
    let root = jj_root().map_err(|e| UndoError::JjCommandFailed {
        command: "jj root".to_string(),
        reason: e.to_string(),
    })?;

    validate_location(&root)?;

    let history = read_undo_history(&root)?;
    let last_entry = get_last_undo_entry(&history)?;

    validate_undo_possible(&root, &last_entry)?;

    if options.dry_run {
        return Ok(UndoOutput {
            session_name: last_entry.session_name.clone(),
            dry_run: true,
            commit_id: last_entry.commit_id.clone(),
            pushed_to_remote: false,
            error: None,
        });
    }

    let revert_result = revert_merge(&root, &last_entry)?;

    update_undo_history(&root, &history, &last_entry, "undone")?;

    Ok(UndoOutput {
        session_name: last_entry.session_name.clone(),
        dry_run: false,
        commit_id: last_entry.commit_id.clone(),
        pushed_to_remote: false,
        error: None,
    })
}

/// Validate we're in a valid location
fn validate_location(root: &str) -> Result<(), UndoError> {
    let location = detect_location(&PathBuf::from(root)).map_err(|e| UndoError::InvalidState {
        reason: e.to_string(),
    })?;

    match location {
        Location::Main => Ok(()),
        Location::Workspace { name, .. } => Err(UndoError::NotInMain { workspace: name }),
    }
}

/// Read undo history from log file
fn read_undo_history(root: &str) -> Result<Vec<UndoEntry>, UndoError> {
    let undo_log_path = Path::new(root).join(UNDO_LOG_PATH);

    if !undo_log_path.exists() {
        return Err(UndoError::NoUndoHistory);
    }

    let content = fs::read_to_string(&undo_log_path).map_err(|e| UndoError::ReadUndoLogFailed {
        reason: e.to_string(),
    })?;

    content
        .lines()
        .filter_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                serde_json::from_str::<UndoEntry>(line).ok()
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .ok_or_else(|| UndoError::NoUndoHistory)
}

/// Get the last (most recent) undo entry
fn get_last_undo_entry(history: &[UndoEntry]) -> Result<UndoEntry, UndoError> {
    history
        .first()
        .cloned()
        .ok_or_else(|| UndoError::NoUndoHistory)
}

/// Validate that undo is possible (not pushed to remote)
fn validate_undo_possible(root: &str, entry: &UndoEntry) -> Result<(), UndoError> {
    if entry.pushed_to_remote {
        return Err(UndoError::AlreadyPushedToRemote {
            commit_id: entry.commit_id.clone(),
        });
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| UndoError::SystemTimeError {
            reason: e.to_string(),
        })?
        .as_secs();

    let retention_seconds = WORKSPACE_RETENTION_HOURS * 3600;

    if now - entry.timestamp > retention_seconds {
        return Err(UndoError::WorkspaceExpired {
            session_name: entry.session_name.clone(),
            hours: WORKSPACE_RETENTION_HOURS,
        });
    }

    Ok(())
}

/// Revert the merge operation
fn revert_merge(root: &str, entry: &UndoEntry) -> Result<(), UndoError> {
    let output = std::process::Command::new("jj")
        .current_dir(root)
        .args(["rebase", "-d", &entry.pre_merge_commit_id])
        .output()
        .map_err(|e| UndoError::JjCommandFailed {
            command: "jj rebase".to_string(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UndoError::RebaseFailed {
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

/// Update undo history after successful undo
fn update_undo_history(
    root: &str,
    history: &[UndoEntry],
    entry: &UndoEntry,
    status: &str,
) -> Result<(), UndoError> {
    let undo_log_path = Path::new(root).join(UNDO_LOG_PATH);

    let mut new_content = String::new();

    for hist_entry in history.iter().skip(1) {
        let json =
            serde_json::to_string(hist_entry).map_err(|e| UndoError::SerializationError {
                reason: e.to_string(),
            })?;
        new_content.push_str(&json);
        new_content.push('\n');
    }

    let mut updated_entry = entry.clone();
    updated_entry.status = status.to_string();

    let json =
        serde_json::to_string(&updated_entry).map_err(|e| UndoError::SerializationError {
            reason: e.to_string(),
        })?;
    new_content.push_str(&json);
    new_content.push('\n');

    fs::write(&undo_log_path, new_content).map_err(|e| UndoError::WriteUndoLogFailed {
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Output the result in the appropriate format
fn output_result(result: &UndoOutput, format: OutputFormat) -> Result<(), UndoError> {
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else if result.dry_run {
        println!("🔍 Dry-run undo for session: {}", result.session_name);
        println!("  Commit: {}", result.commit_id);
    } else {
        println!("✅ Undone merge from session: {}", result.session_name);
        println!("  Commit: {}", result.commit_id);
        println!();
        println!("NEXT: Verify changes and re-commit if needed:");
        println!("  jj status");
        println!("  jj commit -m 'Revert: {}'", result.session_name);
    }
    Ok(())
}

/// Output error in the appropriate format
fn output_error(error: &UndoError, format: OutputFormat) -> Result<(), UndoError> {
    if format.is_json() {
        let error_json = serde_json::json!({
            "error": error.to_string(),
            "error_code": error.error_code(),
        });
        println!("{}", serde_json::to_string_pretty(&error_json)?);
    } else {
        eprintln!("❌ {error}");
        if matches!(error, UndoError::AlreadyPushedToRemote { .. }) {
            eprintln!("   Changes have been pushed to remote and cannot be undone.");
            eprintln!("   Use 'jj revert' to manually revert the commit.");
        }
    }
    Ok(())
}

/// Undo entry in history log
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UndoEntry {
    session_name: String,
    commit_id: String,
    pre_merge_commit_id: String,
    timestamp: u64,
    pushed_to_remote: bool,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_output_default() {
        let output = UndoOutput {
            session_name: "test-session".to_string(),
            dry_run: false,
            commit_id: "abc123".to_string(),
            pushed_to_remote: false,
            error: None,
        };
        assert_eq!(output.session_name, "test-session");
        assert!(!output.dry_run);
    }
}

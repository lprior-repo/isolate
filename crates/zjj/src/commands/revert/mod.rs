//! Revert command - Revert specific session merge
//!
//! This command:
//! 1. Reads undo history from .zjj/undo.log
//! 2. Finds specific session entry
//! 3. Validates revert is possible (not pushed to remote)
//! 4. Reverts that specific merge
//! 5. Updates undo history

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use tokio::process::Command;
use zjj_core::{
    json::{ErrorDetail, SchemaEnvelope},
    OutputFormat,
};

use crate::{
    cli::jj_root,
    commands::context::{detect_location, Location},
};

pub mod types;

pub use types::{RevertArgs, RevertError, RevertExitCode, RevertOptions, RevertOutput};

const UNDO_LOG_PATH: &str = ".zjj/undo.log";

/// Run revert command with options
pub async fn run_with_options(options: &RevertOptions) -> Result<RevertExitCode, RevertError> {
    let result = execute_revert(options).await;

    match &result {
        Ok(output) => {
            output_result(output, options.format)?;
            Ok(RevertExitCode::Success)
        }
        Err(e) => {
            output_error(e, options.format)?;
            Ok(match e {
                RevertError::SessionNotFound { .. } => RevertExitCode::SessionNotFound,
                RevertError::AlreadyPushedToRemote { .. } => RevertExitCode::AlreadyPushed,
                RevertError::InvalidState { .. } => RevertExitCode::InvalidState,
                _ => RevertExitCode::OtherError,
            })
        }
    }
}

/// Core revert logic using Railway-Oriented Programming
async fn execute_revert(options: &RevertOptions) -> Result<RevertOutput, RevertError> {
    let root = jj_root().await.map_err(|e: anyhow::Error| RevertError::JjCommandFailed {
        command: "jj root".to_string(),
        reason: e.to_string(),
    })?;

    validate_location(&root)?;

    let history = read_undo_history(&root)?;
    let entry = find_session_entry(&history, &options.session_name)?;

    validate_revert_possible(&entry)?;

    if options.dry_run {
        return Ok(RevertOutput {
            session_name: options.session_name.clone(),
            dry_run: true,
            commit_id: entry.commit_id,
            pushed_to_remote: false,
            error: None,
        });
    }

    revert_merge(&root, &entry).await?;

    update_undo_history(&root, &history, &entry, "reverted")?;

    Ok(RevertOutput {
        session_name: options.session_name.clone(),
        dry_run: false,
        commit_id: entry.commit_id,
        pushed_to_remote: false,
        error: None,
    })
}

/// Validate we're in a valid location
fn validate_location(root: &str) -> Result<(), RevertError> {
    let location =
        detect_location(&PathBuf::from(root)).map_err(|e: anyhow::Error| RevertError::InvalidState {
            reason: e.to_string(),
        })?;

    match location {
        Location::Main => Ok(()),
        Location::Workspace { name, .. } => Err(RevertError::NotInMain { workspace: name }),
    }
}

/// Read undo history from log file
fn read_undo_history(root: &str) -> Result<Vec<UndoEntry>, RevertError> {
    let undo_log_path = Path::new(root).join(UNDO_LOG_PATH);

    if !undo_log_path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(&undo_log_path).map_err(|e: std::io::Error| RevertError::ReadUndoLogFailed {
            reason: e.to_string(),
        })?;

    let entries: Vec<UndoEntry> = content
        .lines()
        .filter_map(|line: &str| {
            if line.trim().is_empty() {
                None
            } else {
                serde_json::from_str::<UndoEntry>(line).ok()
            }
        })
        .collect();

    Ok(entries)
}

/// Find specific session entry in history
fn find_session_entry(history: &[UndoEntry], session_name: &str) -> Result<UndoEntry, RevertError> {
    history
        .iter()
        .find(|entry: &&UndoEntry| entry.session_name == session_name && entry.status == "completed")
        .cloned()
        .ok_or_else(|| RevertError::SessionNotFound {
            session_name: session_name.to_string(),
        })
}

/// Validate that revert is possible
fn validate_revert_possible(entry: &UndoEntry) -> Result<(), RevertError> {
    if entry.pushed_to_remote {
        return Err(RevertError::AlreadyPushedToRemote {
            commit_id: entry.commit_id.clone(),
        });
    }

    Ok(())
}

/// Revert specific merge operation
async fn revert_merge(root: &str, entry: &UndoEntry) -> Result<(), RevertError> {
    let output = Command::new("jj")
        .current_dir(root)
        .args(["rebase", "-d", &entry.pre_merge_commit_id])
        .output()
        .await
        .map_err(|e: std::io::Error| RevertError::JjCommandFailed {
            command: "jj rebase".to_string(),
            reason: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RevertError::RebaseFailed {
            reason: stderr.to_string(),
        });
    }

    Ok(())
}

/// Update undo history after successful revert
fn update_undo_history(
    root: &str,
    history: &[UndoEntry],
    entry: &UndoEntry,
    status: &str,
) -> Result<(), RevertError> {
    let undo_log_path = Path::new(root).join(UNDO_LOG_PATH);

    let new_content = history
        .iter()
        .map(|hist_entry: &UndoEntry| {
            if hist_entry.session_name == entry.session_name {
                let mut updated = hist_entry.clone();
                updated.status = status.to_string();
                serde_json::to_string(&updated)
            } else {
                serde_json::to_string(hist_entry)
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: serde_json::Error| RevertError::SerializationError {
            reason: e.to_string(),
        })?
        .join("\n");

    fs::write(&undo_log_path, new_content).map_err(|e: std::io::Error| RevertError::WriteUndoLogFailed {
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Output result in appropriate format
fn output_result(result: &RevertOutput, format: OutputFormat) -> Result<(), RevertError> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("revert-response", "single", result);
        let json_output = serde_json::to_string_pretty(&envelope).map_err(|e: serde_json::Error| {
            RevertError::SerializationError {
                reason: e.to_string(),
            }
        })?;
        println!("{json_output}");
    } else if result.dry_run {
        println!("Dry-run revert for session: {}", result.session_name);
        println!("  Commit: {}", result.commit_id);
    } else {
        println!("Reverted merge from session: {}", result.session_name);
        println!("  Commit: {}", result.commit_id);
        println!();
        println!("NEXT: Verify changes and re-commit if needed:");
        println!("  jj status");
        println!("  jj commit -m 'Revert: {}'", result.session_name);
    }
    Ok(())
}

/// Output error in appropriate format
fn output_error(error: &RevertError, format: OutputFormat) -> Result<(), RevertError> {
    if format.is_json() {
        let error_detail = ErrorDetail {
            code: error.error_code().to_string(),
            message: error.to_string(),
            exit_code: revert_error_exit_code(error),
            details: None,
            suggestion: None,
        };
        let payload = RevertErrorPayload {
            error: error_detail,
        };
        let envelope = SchemaEnvelope::new("error-response", "single", payload).as_error();
        let json_output = serde_json::to_string_pretty(&envelope).map_err(|e: serde_json::Error| {
            RevertError::SerializationError {
                reason: e.to_string(),
            }
        })?;
        println!("{json_output}");
    } else {
        eprintln!("Error: {error}");
        if matches!(error, RevertError::AlreadyPushedToRemote { .. }) {
            eprintln!("   Changes have been pushed to remote and cannot be reverted.");
            eprintln!("   Use 'jj revert' to manually revert the commit.");
        }
    }
    Ok(())
}

const fn revert_error_exit_code(error: &RevertError) -> i32 {
    match error {
        RevertError::SessionNotFound { .. } => RevertExitCode::SessionNotFound as i32,
        RevertError::AlreadyPushedToRemote { .. } => RevertExitCode::AlreadyPushed as i32,
        RevertError::InvalidState { .. } => RevertExitCode::InvalidState as i32,
        _ => RevertExitCode::OtherError as i32,
    }
}

#[derive(serde::Serialize)]
struct RevertErrorPayload {
    error: ErrorDetail,
}

/// Undo entry in history log
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UndoEntry {
    session_name: String,
    commit_id: String,
    pre_merge_commit_id: String,
    timestamp: u64,
    pushed_to_remote: bool,
    status: String,
}

impl From<serde_json::Error> for RevertError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError {
            reason: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revert_output_default() {
        let output = RevertOutput {
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
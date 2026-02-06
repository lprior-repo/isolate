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

use tokio::process::Command;
use num_traits::ToPrimitive;
use zjj_core::{
    json::{ErrorDetail, SchemaEnvelope},
    OutputFormat,
};

use crate::{
    cli::jj_root,
    commands::context::{detect_location, Location},
};

pub mod types;

pub use types::{UndoArgs, UndoError, UndoExitCode, UndoOptions, UndoOutput};

const UNDO_LOG_PATH: &str = ".zjj/undo.log";
const WORKSPACE_RETENTION_HOURS: u64 = 24;

/// Run the undo command with options
pub async fn run_with_options(options: &UndoOptions) -> Result<UndoExitCode, UndoError> {
    // Handle list mode
    if options.list {
        return run_list(options).await;
    }

    let result = execute_undo(options).await;

    match &result {
        Ok(output) => {
            output_result(output, options.format)?;
            Ok(UndoExitCode::Success)
        }
        Err(e) => {
            output_error(e, options.format)?;
            Ok(match e {
                UndoError::AlreadyPushedToRemote { .. } => UndoExitCode::AlreadyPushed,
                UndoError::NoUndoHistory => UndoExitCode::NoHistory,
                UndoError::InvalidState { .. } => UndoExitCode::InvalidState,
                _ => UndoExitCode::OtherError,
            })
        }
    }
}

/// List undo history
async fn run_list(options: &UndoOptions) -> Result<UndoExitCode, UndoError> {
    let root = jj_root().await.map_err(|e| UndoError::JjCommandFailed {
        command: "jj root".to_string(),
        reason: e.to_string(),
    })?;

    let history = match read_undo_history(&root) {
        Ok(h) => h,
        Err(UndoError::NoUndoHistory) => {
            if options.format.is_json() {
                let output = UndoHistoryOutput {
                    entries: vec![],
                    total: 0,
                    can_undo: false,
                };
                let json = serde_json::to_string_pretty(&output).map_err(|e| {
                    UndoError::SerializationError {
                        reason: e.to_string(),
                    }
                })?;
                println!("{json}");
            } else {
                println!("No undo history available.");
            }
            return Ok(UndoExitCode::Success);
        }
        Err(e) => return Err(e),
    };

    output_history(&history, options.format)?;
    Ok(UndoExitCode::Success)
}

/// Output for undo history listing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UndoHistoryOutput {
    entries: Vec<UndoHistoryEntry>,
    total: usize,
    can_undo: bool,
}

/// A single entry in the undo history for display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UndoHistoryEntry {
    session_name: String,
    commit_id: String,
    timestamp: String,
    status: String,
    pushed_to_remote: bool,
    can_undo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_cannot_undo: Option<String>,
}

/// Output history in the appropriate format
fn output_history(history: &[UndoEntry], format: OutputFormat) -> Result<(), UndoError> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e: std::time::SystemTimeError| UndoError::SystemTimeError {
            reason: e.to_string(),
        })?
        .as_secs();

    let retention_seconds = WORKSPACE_RETENTION_HOURS * 3600;

    let entries: Vec<UndoHistoryEntry> = history
        .iter()
        .map(|entry| {
            let (can_undo, reason) = if entry.pushed_to_remote {
                (false, Some("Already pushed to remote".to_string()))
            } else if now - entry.timestamp > retention_seconds {
                (
                    false,
                    Some(format!("Expired after {WORKSPACE_RETENTION_HOURS} hours")),
                )
            } else if entry.status == "undone" {
                (false, Some("Already undone".to_string()))
            } else {
                (true, None)
            };

            // Convert timestamp to human-readable format
            let datetime = chrono::DateTime::from_timestamp(entry.timestamp.to_i64().unwrap_or_default(), 0)
                .map_or_else(
                    || entry.timestamp.to_string(),
                    |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                );

            UndoHistoryEntry {
                session_name: entry.session_name.clone(),
                commit_id: entry.commit_id.clone(),
                timestamp: datetime,
                status: entry.status.clone(),
                pushed_to_remote: entry.pushed_to_remote,
                can_undo,
                reason_cannot_undo: reason,
            }
        })
        .collect();

    let can_undo_any = entries.iter().any(|e| e.can_undo);

    if format.is_json() {
        let output = UndoHistoryOutput {
            total: entries.len(),
            can_undo: can_undo_any,
            entries,
        };
        let envelope = SchemaEnvelope::new("undo-response", "single", output);
        let json =
            serde_json::to_string_pretty(&envelope).map_err(|e| UndoError::SerializationError {
                reason: e.to_string(),
            })?;
        println!("{json}");
    } else {
        println!("Undo History ({} entries):", entries.len());
        println!();

        for (i, entry) in entries.iter().enumerate() {
            let status_indicator = if entry.can_undo { "✓" } else { "✗" };
            let index = i + 1;

            println!(
                "{index}. [{status_indicator}] {} ({})",
                entry.session_name, entry.status
            );
            println!("      Commit: {}", entry.commit_id);
            println!("      Time:   {}", entry.timestamp);

            if let Some(reason) = &entry.reason_cannot_undo {
                println!("      Cannot undo: {reason}");
            }
            println!();
        }

        if can_undo_any {
            println!("Run 'zjj undo' to revert the most recent undoable entry.");
        } else {
            println!("No entries can be undone.");
        }
    }

    Ok(())
}

/// Core undo logic using Railway-Oriented Programming
async fn execute_undo(options: &UndoOptions) -> Result<UndoOutput, UndoError> {
    let root = jj_root().await.map_err(|e| UndoError::JjCommandFailed {
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
            commit_id: last_entry.commit_id,
            pushed_to_remote: false,
            error: None,
        });
    }

    revert_merge(&root, &last_entry).await?;

    update_undo_history(&root, &history, &last_entry, "undone")?;

    Ok(UndoOutput {
        session_name: last_entry.session_name.clone(),
        dry_run: false,
        commit_id: last_entry.commit_id,
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

    let entries: Vec<UndoEntry> = content
        .lines()
        .filter_map(|line: &str| {
            if line.trim().is_empty() {
                None
            } else {
                serde_json::from_str::<UndoEntry>(line).ok()
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(entries)
}

/// Get the last (most recent) undo entry
fn get_last_undo_entry(history: &[UndoEntry]) -> Result<UndoEntry, UndoError> {
    history.first().cloned().ok_or(UndoError::NoUndoHistory)
}

/// Validate that undo is possible (not pushed to remote)
fn validate_undo_possible(_root: &str, entry: &UndoEntry) -> Result<(), UndoError> {
    if entry.pushed_to_remote {
        return Err(UndoError::AlreadyPushedToRemote {
            commit_id: entry.commit_id.clone(),
        });
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e: std::time::SystemTimeError| UndoError::SystemTimeError {
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
async fn revert_merge(root: &str, entry: &UndoEntry) -> Result<(), UndoError> {
    let output = Command::new("jj")
        .current_dir(root)
        .args(["rebase", "-d", &entry.pre_merge_commit_id])
        .output()
        .await
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

    fs::write(&undo_log_path, new_content).map_err(|e: std::io::Error| UndoError::WriteUndoLogFailed {
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Output the result in the appropriate format
fn output_result(result: &UndoOutput, format: OutputFormat) -> Result<(), UndoError> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("undo-response", "single", result);
        let json_output =
            serde_json::to_string_pretty(&envelope).map_err(|e| UndoError::SerializationError {
                reason: e.to_string(),
            })?;
        println!("{json_output}");
    } else if result.dry_run {
        println!("Dry-run undo for session: {}", result.session_name);
        println!("  Commit: {}", result.commit_id);
    } else {
        println!("Undone merge from session: {}", result.session_name);
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
        let error_detail = ErrorDetail {
            code: error.error_code().to_string(),
            message: error.to_string(),
            exit_code: undo_error_exit_code(error),
            details: None,
            suggestion: None,
        };
        let payload = UndoErrorPayload {
            error: error_detail,
        };
        let envelope = SchemaEnvelope::new("error-response", "single", payload).as_error();
        let json_output =
            serde_json::to_string_pretty(&envelope).map_err(|e| UndoError::SerializationError {
                reason: e.to_string(),
            })?;
        println!("{json_output}");
    } else {
        eprintln!("Error: {error}");
        if matches!(error, UndoError::AlreadyPushedToRemote { .. }) {
            eprintln!("   Changes have been pushed to remote and cannot be undone.");
            eprintln!("   Use 'jj revert' to manually revert the commit.");
        }
    }
    Ok(())
}

const fn undo_error_exit_code(error: &UndoError) -> i32 {
    match error {
        UndoError::AlreadyPushedToRemote { .. } => UndoExitCode::AlreadyPushed as i32,
        UndoError::NoUndoHistory => UndoExitCode::NoHistory as i32,
        UndoError::InvalidState { .. } => UndoExitCode::InvalidState as i32,
        _ => UndoExitCode::OtherError as i32,
    }
}

#[derive(serde::Serialize)]
struct UndoErrorPayload {
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

impl From<serde_json::Error> for UndoError {
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

    #[test]
    fn test_undo_history_entry_can_undo() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc123".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: "pending".to_string(),
            pushed_to_remote: false,
            can_undo: true,
            reason_cannot_undo: None,
        };

        assert!(entry.can_undo);
        assert!(entry.reason_cannot_undo.is_none());
    }

    #[test]
    fn test_undo_history_entry_cannot_undo_pushed() {
        let entry = UndoHistoryEntry {
            session_name: "test".to_string(),
            commit_id: "abc123".to_string(),
            timestamp: "2025-01-01 00:00:00 UTC".to_string(),
            status: "pending".to_string(),
            pushed_to_remote: true,
            can_undo: false,
            reason_cannot_undo: Some("Already pushed to remote".to_string()),
        };

        assert!(!entry.can_undo);
        assert!(entry.reason_cannot_undo.is_some());
    }

    #[test]
    fn test_undo_history_output_serialization() {
        let output = UndoHistoryOutput {
            entries: vec![],
            total: 0,
            can_undo: false,
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"total\":0"));
        assert!(json_str.contains("\"can_undo\":false"));
    }

    #[test]
    fn test_undo_history_entry_serialization() {
        let entry = UndoHistoryEntry {
            session_name: "feature-auth".to_string(),
            commit_id: "xyz789".to_string(),
            timestamp: "2025-01-15 12:00:00 UTC".to_string(),
            status: "pending".to_string(),
            pushed_to_remote: false,
            can_undo: true,
            reason_cannot_undo: None,
        };

        let json = serde_json::to_string(&entry);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"session_name\":\"feature-auth\""));
        assert!(json_str.contains("\"can_undo\":true"));
        // reason_cannot_undo should be skipped when None
        assert!(!json_str.contains("reason_cannot_undo"));
    }
}

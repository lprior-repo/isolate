//! JSON output structures for zjj commands

use anyhow::Error;
use serde::Serialize;
use zjj_core::json::{ErrorCode, JsonError};

/// Init command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InitOutput {
    pub success: bool,
    pub message: String,
    pub jjz_dir: String,
    pub config_file: String,
    pub state_db: String,
    pub layouts_dir: String,
}

/// Add command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AddOutput {
    pub success: bool,
    pub session_name: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub status: String,
}

/// Remove command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOutput {
    pub success: bool,
    pub session_name: String,
    pub message: String,
}

/// Focus command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FocusOutput {
    pub success: bool,
    pub session_name: String,
    pub zellij_tab: String,
    pub message: String,
}

/// Sync command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncOutput {
    pub success: bool,
    pub session_name: Option<String>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub session_name: String,
    pub error: String,
}

/// Diff command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    pub session_name: String,
    pub base: String,
    pub head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<FileDiffStat>,
}

#[derive(Debug, Serialize)]
pub struct FileDiffStat {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
    pub status: String,
}

/// Output a JSON error and exit with code 1
///
/// Converts an `anyhow::Error` to a JSON error structure and outputs it to stdout.
/// Then exits the process with exit code 1.
pub fn output_json_error_and_exit(error: &Error) -> ! {
    let json_error = error_to_json_error(error);

    if let Ok(json_str) = serde_json::to_string_pretty(&json_error) {
        println!("{json_str}");
    } else {
        eprintln!("Error: {error}");
    }

    std::process::exit(1);
}

/// Convert an `anyhow::Error` to a `JsonError`
fn error_to_json_error(error: &Error) -> JsonError {
    let error_str = error.to_string();

    // Try to classify the error by its message
    let code = if error_str.contains("database") || error_str.contains("Database") {
        ErrorCode::StateDbCorrupted
    } else if error_str.contains("not found") || error_str.contains("Not found") {
        ErrorCode::SessionNotFound
    } else if error_str.contains("Invalid session name") {
        ErrorCode::SessionNameInvalid
    } else if error_str.contains("already exists") {
        ErrorCode::SessionAlreadyExists
    } else if error_str.contains("JJ is not installed") || error_str.contains("jj not found") {
        ErrorCode::JjNotInstalled
    } else if error_str.contains("Not a JJ repository") || error_str.contains("not in a jj repo") {
        ErrorCode::NotJjRepository
    } else {
        ErrorCode::Unknown
    };

    let mut json_error = JsonError::new(code, error_str);

    // Add suggestions based on error type
    let suggestion = match code {
        ErrorCode::StateDbCorrupted => {
            Some("Try running 'jjz doctor --fix' to repair the database")
        }
        ErrorCode::SessionNotFound => Some("Use 'jjz list' to see available sessions"),
        ErrorCode::JjNotInstalled => Some("Install JJ: cargo install jj-cli or brew install jj"),
        ErrorCode::NotJjRepository => Some("Run 'jjz init' to initialize a JJ repository"),
        _ => None,
    };

    if let Some(sugg) = suggestion {
        json_error = json_error.with_suggestion(sugg);
    }

    json_error
}

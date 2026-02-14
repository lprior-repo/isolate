//! JSON error conversion and handling

use anyhow::Error;
use serde::Serialize;
use zjj_core::{
    json::{ErrorCode, ErrorDetail, JsonError, SchemaEnvelope},
    Error as ZjjError,
};

use crate::commands::spawn::types::SpawnError;

/// Sync command error details
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub name: String,
    pub error: ErrorDetail,
}

/// Output a JSON error and return the appropriate semantic exit code
///
/// Converts an `anyhow::Error` to a JSON error structure and outputs it to stdout.
/// Returns the semantic exit code from the error:
/// - 1: Validation errors (user input issues)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors
pub fn output_json_error(error: &Error) -> i32 {
    let json_error = error_to_json_error(error);
    let exit_code = json_error.error.exit_code;

    let payload = ErrorEnvelopePayload {
        error: json_error.error,
    };
    let envelope = SchemaEnvelope::new("error-response", "single", payload).as_error();

    if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
        println!("{json_str}");
    } else {
        // If JSON serialization fails, output a minimal JSON error to stdout
        println!(r#"{{"error":{{"message":"{error}","exit_code":{exit_code}}}}}"#);
    }

    exit_code
}

#[derive(Debug, Serialize)]
struct ErrorEnvelopePayload {
    error: ErrorDetail,
}

/// Convert an `anyhow::Error` to a `JsonError`
///
/// Uses Railway-Oriented Programming to extract the underlying error type
/// and convert it to a standardized JSON error format using `ErrorDetail::from_error()`.
fn error_to_json_error(error: &Error) -> JsonError {
    // Try to downcast to SpawnError first (Railway left track 1 - spawn errors)
    if let Some(spawn_error) = error.downcast_ref::<SpawnError>() {
        return convert_spawn_error(spawn_error);
    }

    // Try to downcast to zjj_core::Error (Railway left track 2 - core errors)
    error
        .downcast_ref::<ZjjError>()
        .map(JsonError::from)
        .unwrap_or_else(|| {
            // Railway right track - fallback for other error types
            let error_str = error.to_string();

            // Classify error by message pattern (fallback heuristic)
            let code = classify_error_by_message(&error_str);
            let mut json_error = JsonError::new(code, error_str.clone());

            // Add suggestions based on error type
            if let Some(sugg) = suggest_resolution(code) {
                json_error = json_error.with_suggestion(sugg);
            }

            // Determine exit code based on error classification
            json_error.error.exit_code = classify_exit_code_by_message(&error_str);

            json_error
        })
}

/// Convert a `SpawnError` to a `JsonError` preserving structured context
///
/// AI agents use `error_code()` and `phase()` to distinguish error types:
/// - `NOT_ON_MAIN` vs `INVALID_BEAD_STATUS` vs `BEAD_NOT_FOUND`
/// - Know which phase of spawn failed (validating, creating, etc.)
fn convert_spawn_error(error: &SpawnError) -> JsonError {
    use zjj_core::ErrorCode;

    // Map SpawnError variants to ErrorCode and exit codes
    let (code, exit_code) = match error {
        SpawnError::NotOnMain { .. } => (ErrorCode::SpawnNotOnMain, 1),
        SpawnError::InvalidBeadStatus { .. } => (ErrorCode::SpawnInvalidBeadStatus, 1),
        SpawnError::BeadNotFound { .. } => (ErrorCode::SpawnBeadNotFound, 2),
        SpawnError::WorkspaceCreationFailed { .. } => (ErrorCode::SpawnWorkspaceCreationFailed, 3),
        SpawnError::AgentSpawnFailed { .. } => (ErrorCode::SpawnAgentSpawnFailed, 4),
        SpawnError::Timeout { .. } => (ErrorCode::SpawnTimeout, 4),
        SpawnError::MergeFailed { .. } => (ErrorCode::SpawnMergeFailed, 4),
        SpawnError::CleanupFailed { .. } => (ErrorCode::SpawnCleanupFailed, 3),
        SpawnError::DatabaseError { .. } => (ErrorCode::SpawnDatabaseError, 3),
        SpawnError::JjCommandFailed { .. } => (ErrorCode::SpawnJjCommandFailed, 4),
    };

    // Build details JSON with error_code and phase
    let details = serde_json::json!({
        "error_code": error.error_code(),
        "phase": error.phase().name(),
    });

    JsonError::new(code, error.to_string())
        .with_details(details)
        .with_exit_code(exit_code)
}

/// Classify exit code based on error message pattern
fn classify_exit_code_by_message(error_str: &str) -> i32 {
    // Validation errors: exit code 1
    if error_str.contains("Session name")
        || error_str.contains("Invalid session name")
        || error_str.contains("Validation error")
        || error_str.contains("already exists")
    {
        return 1;
    }

    // Not found errors: exit code 2
    if error_str.contains("not found") || error_str.contains("Not found") {
        return 2;
    }

    // System errors: exit code 3
    if error_str.contains("database") || error_str.contains("IO error") {
        return 3;
    }

    // External command errors: exit code 4
    4
}

/// Classify an error by its message text (fallback heuristic)
fn classify_error_by_message(error_str: &str) -> ErrorCode {
    if error_str.contains("database") || error_str.contains("Database") {
        ErrorCode::StateDbCorrupted
    } else if error_str.contains("not found") || error_str.contains("Not found") {
        ErrorCode::SessionNotFound
    } else if error_str.contains("Invalid session name")
        || error_str.contains("Session name")
        || error_str.contains("must start with a letter")
    {
        ErrorCode::SessionNameInvalid
    } else if error_str.contains("already exists") {
        ErrorCode::SessionAlreadyExists
    } else if error_str.contains("JJ is not installed") || error_str.contains("jj not found") {
        ErrorCode::JjNotInstalled
    } else if error_str.contains("Not a JJ repository") || error_str.contains("not in a jj repo") {
        ErrorCode::NotJjRepository
    } else if error_str.contains("Zellij") || error_str.contains("zellij") {
        ErrorCode::ZellijCommandFailed
    } else if error_str.contains("workspace") && error_str.contains("not found") {
        ErrorCode::WorkspaceNotFound
    } else if error_str.contains("Not in workspace")
        || error_str.contains("Not in a workspace")
        || error_str.contains("not in a workspace")
    {
        ErrorCode::InvalidArgument
    } else if error_str.contains("conflict") || error_str.contains("Conflicting") {
        ErrorCode::JjCommandFailed
    } else {
        ErrorCode::Unknown
    }
}

/// Suggest resolution for an error code
const fn suggest_resolution(code: ErrorCode) -> Option<&'static str> {
    match code {
        ErrorCode::StateDbCorrupted => Some(
            "Try running 'zjj doctor --fix' to repair the database, or delete .zjj/state.db to reset",
        ),
        ErrorCode::SessionNotFound => Some("Use 'zjj list' to see available sessions"),
        ErrorCode::SessionNameInvalid => {
            Some("Session names must be 1-64 chars, start with a letter, and contain only alphanumeric, dash, underscore")
        }
        ErrorCode::SessionAlreadyExists => {
            Some("Use 'zjj focus <name>' to switch to existing session, or choose a different name")
        }
        ErrorCode::JjNotInstalled => {
            Some("Install JJ: cargo install jj-cli or brew install jj or visit https://martinvonz.github.io/jj/latest/install-and-setup/")
        }
        ErrorCode::NotJjRepository => Some("Run 'zjj init' to initialize a JJ repository in this directory"),
        ErrorCode::ZellijCommandFailed => {
            Some("Check that Zellij is installed: zellij --version, or start with: zellij attach -c new-session")
        }
        ErrorCode::WorkspaceNotFound => Some("Run 'zjj list' to see available workspaces, or 'zjj doctor' to check system health"),
        ErrorCode::WorkspaceCreationFailed => {
            Some("Check JJ is working: jj status, or try: zjj doctor")
        }
        ErrorCode::InvalidArgument => {
            Some("Use 'zjj context' to see current state, or check command help: zjj <command> --help")
        }
        ErrorCode::JjCommandFailed => {
            Some("Check JJ status: jj status, resolve conflicts with: jj resolve, or see: zjj doctor")
        }
        ErrorCode::ConfigNotFound => Some("Run 'zjj init' to create default configuration"),
        ErrorCode::HookFailed => Some("Check hook scripts in .zjj/hooks/, or use --no-hooks to skip"),
        ErrorCode::SpawnNotOnMain => Some("Switch to main branch: jj checkout main"),
        ErrorCode::SpawnInvalidBeadStatus => Some("Check bead status with: br show <bead-id>"),
        ErrorCode::SpawnBeadNotFound => Some("List available beads with: br ready"),
        ErrorCode::SpawnWorkspaceCreationFailed => {
            Some("Check disk space and permissions, or run: zjj doctor")
        }
        ErrorCode::SpawnAgentSpawnFailed => {
            Some("Check agent command is valid, or use --agent-command flag")
        }
        ErrorCode::SpawnTimeout => {
            Some("Increase timeout with --timeout flag, or check for infinite loops")
        }
        ErrorCode::SpawnMergeFailed => {
            Some("Resolve conflicts manually in workspace, or use: jj abandon")
        }
        ErrorCode::SpawnCleanupFailed => Some("Manually clean workspace: rm -rf .zjj/workspaces/<bead-id>"),
        ErrorCode::SpawnDatabaseError => Some("Run: br sync or zjj doctor --fix"),
        ErrorCode::SpawnJjCommandFailed => {
            Some("Check JJ is working: jj status, or run: zjj doctor")
        }
        ErrorCode::Unknown => Some("Run 'zjj doctor' to check system health and configuration"),
        ErrorCode::StateDbLocked
        | ErrorCode::ConfigParseError
        | ErrorCode::ConfigKeyNotFound
        | ErrorCode::ZellijNotRunning
        | ErrorCode::HookExecutionError => None,
    }
}

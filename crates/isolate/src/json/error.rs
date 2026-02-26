//! JSON error conversion and handling

use anyhow::Error;
use serde::Serialize;
use isolate_core::{
    json::{ErrorCode, ErrorDetail, JsonError, SchemaEnvelope},
    Error as ZjjError,
};

use crate::commands::{spawn::types::SpawnError, undo::types::UndoError};

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
    let error_str = error.to_string();

    // If the error message is already a JSON object (e.g. from doctor command),
    // output it as-is instead of wrapping it in another error envelope.
    // This prevents double-enveloping of JSON responses.
    if error_str.trim().starts_with('{') {
        println!("{error_str}");
        return semantic_exit_code(error);
    }

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
        println!(r#"{{"error":{{"message":"{error_str}","exit_code":{exit_code}}}}}"#);
    }

    exit_code
}

/// Output a CLI parse error as JSON and return clap-compatible exit code 2.
pub fn output_json_parse_error(message: impl Into<String>) -> i32 {
    let error = JsonError::new(ErrorCode::InvalidArgument, message.into())
        .with_suggestion("Use --help to view valid flags and arguments")
        .with_exit_code(2);

    let payload = ErrorEnvelopePayload { error: error.error };
    let envelope = SchemaEnvelope::new("error-response", "single", payload).as_error();

    if let Ok(json_str) = serde_json::to_string_pretty(&envelope) {
        println!("{json_str}");
    } else {
        println!(r#"{{"error":{{"message":"Failed to serialize parse error","exit_code":2}}}}"#);
    }

    2
}

/// Return the semantic exit code for an error.
///
/// This is shared by JSON and non-JSON output modes so both paths
/// return the same process status for the same failure.
pub fn semantic_exit_code(error: &Error) -> i32 {
    error_to_json_error(error).error.exit_code
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

    // Try to downcast to UndoError
    if let Some(undo_error) = error.downcast_ref::<UndoError>() {
        return convert_undo_error(undo_error);
    }

    // Try to downcast to isolate_core::Error (Railway left track 2 - core errors)
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
    use isolate_core::ErrorCode;

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

/// Convert an `UndoError` to a `JsonError`
fn convert_undo_error(error: &UndoError) -> JsonError {
    let exit_code = match error {
        UndoError::AlreadyPushedToRemote { .. } | UndoError::NotInMain { .. } => 1,
        UndoError::NoUndoHistory => 2,
        _ => 4,
    };

    JsonError::new(error.error_code(), error.to_string()).with_exit_code(exit_code)
}

/// Classify exit code based on error message pattern
fn classify_exit_code_by_message(error_str: &str) -> i32 {
    let lower = error_str.to_ascii_lowercase();

    // Missing resources: exit code 2
    if lower.contains("not found") || lower.contains("no backup found") {
        return 2;
    }

    // Validation errors: exit code 1
    if lower.contains("invalid")
        || lower.contains("validation")
        || lower.contains("already exists")
        || lower.contains("unknown database")
        || lower.contains("unknown backup action")
        || lower.contains("timestamp")
        || lower.contains("not in a jj repository")
        || lower.contains("not in a jj repo")
        || lower.contains("session name")
        || lower.contains("invalid session name")
        || lower.contains("isolate not initialized")
        || lower.contains("already pushed to remote")
        || lower.contains("expired")
    {
        return 1;
    }

    // System errors: exit code 3
    if lower.contains("database") || lower.contains("io error") {
        return 3;
    }

    // External command errors: exit code 4
    4
}

/// Classify an error by its message text (fallback heuristic)
fn classify_error_by_message(error_str: &str) -> ErrorCode {
    let lower = error_str.to_lowercase();

    if lower.contains("no backup found") || lower.contains("not found") {
        ErrorCode::SessionNotFound
    } else if lower.contains("invalid session name")
        || lower.contains("session name")
        || lower.contains("must start with a letter")
    {
        ErrorCode::SessionNameInvalid
    } else if lower.contains("unknown database")
        || lower.contains("unknown backup action")
        || lower.contains("invalid --timestamp")
        || lower.contains("invalid")
        || lower.contains("validation")
    {
        ErrorCode::InvalidArgument
    } else if lower.contains("already exists") {
        ErrorCode::SessionAlreadyExists
    } else if lower.contains("jj is not installed") || lower.contains("jj not found") {
        ErrorCode::JjNotInstalled
    } else if lower.contains("not a jj repository")
        || lower.contains("not in a jj repository")
        || lower.contains("not in a jj repo")
    {
        ErrorCode::NotJjRepository
    } else if lower.contains("workspace") && lower.contains("not found") {
        ErrorCode::WorkspaceNotFound
    } else if lower.contains("not in workspace") || lower.contains("not in a workspace") {
        ErrorCode::InvalidArgument
    } else if lower.contains("conflict") {
        ErrorCode::JjCommandFailed
    } else if lower.contains("database") {
        ErrorCode::StateDbCorrupted
    } else {
        ErrorCode::Unknown
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use isolate_core::ErrorCode;

    use super::{classify_error_by_message, classify_exit_code_by_message};

    #[test]
    fn given_unknown_database_when_classified_then_invalid_argument() {
        let code = classify_error_by_message("Unknown database: notadb.db");
        assert!(matches!(code, ErrorCode::InvalidArgument));
        assert_eq!(
            classify_exit_code_by_message("Unknown database: notadb.db"),
            1
        );
    }

    #[test]
    fn given_not_jj_repo_when_classified_then_not_jj_repository_exit_one() {
        let code = classify_error_by_message("Not in a JJ repository. Run 'isolate init' first.");
        assert!(matches!(code, ErrorCode::NotJjRepository));
        assert_eq!(
            classify_exit_code_by_message("Not in a JJ repository. Run 'isolate init' first."),
            1
        );
    }

    #[test]
    fn given_not_initialized_when_classifying_then_exit_code_is_validation() {
        let code = classify_exit_code_by_message("Isolate not initialized. Run 'isolate init' first.");
        assert_eq!(code, 1);
    }

    #[test]
    fn given_no_backup_found_when_classified_then_not_found_exit_two() {
        let code = classify_error_by_message("No backup found with timestamp: 20250101-010101");
        assert!(matches!(code, ErrorCode::SessionNotFound));
        assert_eq!(
            classify_exit_code_by_message("No backup found with timestamp: 20250101-010101"),
            2
        );
    }

    #[test]
    fn given_not_in_jj_repository_when_mapping_error_code_then_not_jj_repository() {
        let code = classify_error_by_message("Not in a JJ repository. Run 'isolate init' first.");
        assert_eq!(code, ErrorCode::NotJjRepository);
    }
}

/// Suggest resolution for an error code
const fn suggest_resolution(code: ErrorCode) -> Option<&'static str> {
    match code {
        ErrorCode::StateDbCorrupted => Some(
            "Try running 'isolate doctor --fix' to repair the database, or delete .isolate/state.db to reset",
        ),
        ErrorCode::SessionNotFound => Some("Use 'isolate list' to see available sessions"),
        ErrorCode::SessionNameInvalid => {
            Some("Session names must be 1-64 chars, start with a letter, and contain only alphanumeric, dash, underscore")
        }
        ErrorCode::SessionAlreadyExists => {
            Some("Use 'isolate focus <name>' to switch to existing session, or choose a different name")
        }
        ErrorCode::JjNotInstalled => {
            Some("Install JJ: cargo install jj-cli or brew install jj or visit https://martinvonz.github.io/jj/latest/install-and-setup/")
        }
        ErrorCode::NotJjRepository => Some("Run 'isolate init' to initialize a JJ repository in this directory"),
        ErrorCode::WorkspaceNotFound => Some("Run 'isolate list' to see available workspaces, or 'isolate doctor' to check system health"),
        ErrorCode::WorkspaceCreationFailed => {
            Some("Check JJ is working: jj status, or try: isolate doctor")
        }
        ErrorCode::InvalidArgument => {
            Some("Use 'isolate context' to see current state, or check command help: isolate <command> --help")
        }
        ErrorCode::JjCommandFailed => {
            Some("Check JJ status: jj status, resolve conflicts with: jj resolve, or see: isolate doctor")
        }
        ErrorCode::ConfigNotFound => Some("Run 'isolate init' to create default configuration"),
        ErrorCode::HookFailed => Some("Check hook scripts in .isolate/hooks/, or use --no-hooks to skip"),
        ErrorCode::SpawnNotOnMain => Some("Switch to main branch: jj checkout main"),
        ErrorCode::SpawnInvalidBeadStatus => Some("Check bead status with: br show <bead-id>"),
        ErrorCode::SpawnBeadNotFound => Some("List available beads with: br ready"),
        ErrorCode::SpawnWorkspaceCreationFailed => {
            Some("Check disk space and permissions, or run: isolate doctor")
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
        ErrorCode::SpawnCleanupFailed => Some("Manually clean workspace: rm -rf .isolate/workspaces/<bead-id>"),
        ErrorCode::SpawnDatabaseError => Some("Run: br sync or isolate doctor --fix"),
        ErrorCode::SpawnJjCommandFailed => {
            Some("Check JJ is working: jj status, or run: isolate doctor")
        }
        ErrorCode::Unknown => Some("Run 'isolate doctor' to check system health and configuration"),
        ErrorCode::StateDbLocked
        | ErrorCode::ConfigParseError
        | ErrorCode::ConfigKeyNotFound
        | ErrorCode::ZellijNotRunning
        | ErrorCode::ZellijCommandFailed
        | ErrorCode::ReadUndoLogFailed
        | ErrorCode::WriteUndoLogFailed
        | ErrorCode::HookExecutionError => None,
    }
}

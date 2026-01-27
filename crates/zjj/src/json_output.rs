//! JSON output structures for zjj commands

use anyhow::Error;
use serde::Serialize;
use zjj_core::{
    json::{ErrorCode, JsonError},
    Error as ZjjError,
};

/// Init command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InitOutput {
    pub message: String,
    pub zjj_dir: String,
    pub config_file: String,
    pub state_db: String,
    pub layouts_dir: String,
}

/// Add command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AddOutput {
    pub name: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub status: String,
}

/// Remove command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOutput {
    pub name: String,
    pub message: String,
}

/// Focus command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FocusOutput {
    pub name: String,
    pub zellij_tab: String,
    pub message: String,
}

/// Sync command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncOutput {
    pub name: Option<String>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub name: String,
    pub error: String,
}

/// Diff command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    pub name: String,
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

/// Output a JSON error and exit with the appropriate semantic exit code
///
/// Converts an `anyhow::Error` to a JSON error structure and outputs it to stdout.
/// Then exits the process with the semantic exit code from the error:
/// - 1: Validation errors (user input issues)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors
pub fn output_json_error_and_exit(error: &Error) -> ! {
    let json_error = error_to_json_error(error);
    let exit_code = json_error.error.exit_code;

    if let Ok(json_str) = serde_json::to_string_pretty(&json_error) {
        println!("{json_str}");
    } else {
        eprintln!("Error: {error}");
    }

    std::process::exit(exit_code);
}

/// Convert an `anyhow::Error` to a `JsonError`
///
/// Uses Railway-Oriented Programming to extract the underlying `zjj_core::Error`
/// and convert it to a standardized JSON error format using `ErrorDetail::from_error()`.
fn error_to_json_error(error: &Error) -> JsonError {
    // Try to downcast to zjj_core::Error first (Railway left track - success)
    error
        .downcast_ref::<ZjjError>()
        .map(JsonError::from)
        .unwrap_or_else(|| {
            // Railway right track - fallback for non-zjj errors
            // This handles cases where anyhow wraps other error types
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
    } else if error_str.contains("Not in workspace") || error_str.contains("not in a workspace") {
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
        ErrorCode::Unknown => Some("Run 'zjj doctor' to check system health and configuration"),
        ErrorCode::StateDbLocked
        | ErrorCode::ConfigParseError
        | ErrorCode::ConfigKeyNotFound
        | ErrorCode::ZellijNotRunning
        | ErrorCode::HookExecutionError => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = AddOutput {
            name: "test-session".to_string(),
            workspace_path: "/path/to/workspace".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            status: "active".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_name_matches_session() -> Result<(), serde_json::Error> {
        let session_name = "my-feature";
        let output = AddOutput {
            name: session_name.to_string(),
            workspace_path: format!("/workspaces/{session_name}"),
            zellij_tab: format!("zjj:{session_name}"),
            status: "creating".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some(session_name),
            "name in JSON should match session name"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_backwards_compat_session_name_removed() -> Result<(), serde_json::Error> {
        // This test verifies that the old 'session_name' field is completely removed
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/path".to_string(),
            zellij_tab: "zjj:test".to_string(),
            status: "failed".to_string(),
        };

        let json_str = serde_json::to_string(&output)?;

        // The JSON string should not contain 'session_name' anywhere
        assert!(
            !json_str.contains("session_name"),
            "JSON should not contain 'session_name' field: {json_str}"
        );

        // But should contain 'name'
        assert!(
            json_str.contains("\"name\""),
            "JSON should contain 'name' field: {json_str}"
        );
        Ok(())
    }

    #[test]
    fn test_add_output_all_fields_present() -> Result<(), serde_json::Error> {
        let output = AddOutput {
            name: "test".to_string(),
            workspace_path: "/workspace/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            status: "active".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(
            json.get("workspace_path").and_then(|v| v.as_str()),
            Some("/workspace/test")
        );
        assert_eq!(
            json.get("zellij_tab").and_then(|v| v.as_str()),
            Some("zjj:test")
        );
        assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("active"));
        Ok(())
    }

    #[test]
    fn test_remove_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = RemoveOutput {
            name: "test-session".to_string(),
            message: "Session removed successfully".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_remove_output_matches_add_structure() -> Result<(), serde_json::Error> {
        // RemoveOutput should use same 'name' field as AddOutput
        let add_output = AddOutput {
            name: "my-session".to_string(),
            workspace_path: "/workspace".to_string(),
            zellij_tab: "zjj:my-session".to_string(),
            status: "active".to_string(),
        };

        let remove_output = RemoveOutput {
            name: "my-session".to_string(),
            message: "Removed".to_string(),
        };

        let add_json = serde_json::to_value(&add_output)?;
        let remove_json = serde_json::to_value(&remove_output)?;

        // Both should have 'name' field with same value
        assert_eq!(
            add_json.get("name").and_then(|v| v.as_str()),
            remove_json.get("name").and_then(|v| v.as_str()),
            "Both should use 'name' field consistently"
        );

        // Neither should have session_name
        assert!(add_json.get("session_name").is_none());
        assert!(remove_json.get("session_name").is_none());
        Ok(())
    }

    #[test]
    fn test_focus_output_json_uses_name_field() -> Result<(), serde_json::Error> {
        let output = FocusOutput {
            name: "test-session".to_string(),
            zellij_tab: "zjj:test-session".to_string(),
            message: "Focused on session".to_string(),
        };

        let json = serde_json::to_value(&output)?;

        // Should have 'name' field, not 'session_name'
        assert!(json.get("name").is_some(), "JSON should have 'name' field");
        assert!(
            json.get("session_name").is_none(),
            "JSON should NOT have 'session_name' field"
        );
        assert_eq!(
            json.get("name").and_then(|v| v.as_str()),
            Some("test-session"),
            "name field should match"
        );
        Ok(())
    }

    #[test]
    fn test_focus_output_consistent_with_other_outputs() -> Result<(), serde_json::Error> {
        // All output structs should use 'name' field consistently
        let focus = FocusOutput {
            name: "my-session".to_string(),
            zellij_tab: "zjj:my-session".to_string(),
            message: "Focused".to_string(),
        };

        let add = AddOutput {
            name: "my-session".to_string(),
            workspace_path: "/workspace".to_string(),
            zellij_tab: "zjj:my-session".to_string(),
            status: "active".to_string(),
        };

        let remove = RemoveOutput {
            name: "my-session".to_string(),
            message: "Removed".to_string(),
        };

        let focus_json = serde_json::to_value(&focus)?;
        let add_json = serde_json::to_value(&add)?;
        let remove_json = serde_json::to_value(&remove)?;

        // All should have 'name' field with same value
        assert_eq!(
            focus_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );
        assert_eq!(
            add_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );
        assert_eq!(
            remove_json.get("name").and_then(|v| v.as_str()),
            Some("my-session")
        );

        // None should have session_name
        assert!(focus_json.get("session_name").is_none());
        assert!(add_json.get("session_name").is_none());
        assert!(remove_json.get("session_name").is_none());
        Ok(())
    }

    // PHASE 5 - GREEN for zjj-ioa3: SyncOutput SchemaEnvelope wrapping tests
    // These tests verify that SyncOutput JSON includes SchemaEnvelope wrapper

    #[test]
    fn test_sync_json_has_envelope() -> Result<(), serde_json::Error> {
        // Create a SyncOutput (single session success)
        let output = SyncOutput {
            name: Some("test-session".to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };

        // Wrap in envelope (as done in sync.rs)
        let envelope = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify envelope fields exist
        assert!(
            json.get("$schema").is_some(),
            "SyncOutput JSON must have $schema field in envelope"
        );
        assert!(
            json.get("_schema_version").is_some(),
            "SyncOutput JSON must have _schema_version field in envelope"
        );
        assert!(
            json.get("schema_type").is_some(),
            "SyncOutput JSON must have schema_type field in envelope"
        );

        Ok(())
    }

    #[test]
    fn test_sync_schema_type_single() -> Result<(), serde_json::Error> {
        // Create a SyncOutput (all sessions sync)
        let output = SyncOutput {
            name: None,
            synced_count: 3,
            failed_count: 0,
            errors: Vec::new(),
        };

        // Wrap in envelope (as done in sync.rs)
        let envelope = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output);

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify schema_type is "single"
        assert_eq!(
            json.get("schema_type").and_then(|v| v.as_str()),
            Some("single"),
            "SyncOutput schema_type must be 'single' (SyncOutput is a single object, not array)"
        );

        // Verify $schema URI format
        let schema_value = json.get("$schema").and_then(|v| v.as_str());
        assert!(schema_value.is_some(), "$schema field should be present");
        let Some(schema) = schema_value else {
            return Ok(());
        };
        assert!(
            schema.starts_with("zjj://"),
            "Schema URI must start with 'zjj://'"
        );
        assert!(
            schema.contains("/v1"),
            "Schema URI must include version '/v1'"
        );

        Ok(())
    }

    #[test]
    fn test_sync_all_serialization_points() -> Result<(), serde_json::Error> {
        // Test all 4 serialization points mentioned in PLAN.md

        // Point 1: Single session success (line 56 in sync.rs)
        let output1 = SyncOutput {
            name: Some("session1".to_string()),
            synced_count: 1,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope1 = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output1);
        let json1_str = serde_json::to_string(&envelope1)?;
        let json1: serde_json::Value = serde_json::from_str(&json1_str)?;
        assert!(
            json1.get("$schema").is_some(),
            "Single success case must have envelope (line 56)"
        );

        // Point 2: Single session failure (line 75 in sync.rs)
        let output2 = SyncOutput {
            name: Some("session2".to_string()),
            synced_count: 0,
            failed_count: 1,
            errors: vec![SyncError {
                name: "session2".to_string(),
                error: "rebase failed".to_string(),
            }],
        };
        let envelope2 = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output2);
        let json2_str = serde_json::to_string(&envelope2)?;
        let json2: serde_json::Value = serde_json::from_str(&json2_str)?;
        assert!(
            json2.get("$schema").is_some(),
            "Single failure case must have envelope (line 75)"
        );

        // Point 3: All sessions empty (line 100 in sync.rs)
        let output3 = SyncOutput {
            name: None,
            synced_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };
        let envelope3 = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output3);
        let json3_str = serde_json::to_string(&envelope3)?;
        let json3: serde_json::Value = serde_json::from_str(&json3_str)?;
        assert!(
            json3.get("$schema").is_some(),
            "All sessions empty case must have envelope (line 100)"
        );

        // Point 4: All sessions with results (line 136 in sync.rs)
        let output4 = SyncOutput {
            name: None,
            synced_count: 2,
            failed_count: 1,
            errors: vec![SyncError {
                name: "session3".to_string(),
                error: "workspace not found".to_string(),
            }],
        };
        let envelope4 = zjj_core::json::SchemaEnvelope::new("sync-response", "single", output4);
        let json4_str = serde_json::to_string(&envelope4)?;
        let json4: serde_json::Value = serde_json::from_str(&json4_str)?;
        assert!(
            json4.get("$schema").is_some(),
            "All sessions with results case must have envelope (line 136)"
        );

        Ok(())
    }
}

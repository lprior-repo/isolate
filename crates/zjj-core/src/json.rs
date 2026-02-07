//! JSON output structures for AI-first CLI design
//!
//! This module provides consistent JSON output formats across all commands.

use serde::{Deserialize, Serialize};

use crate::{fix::Fix, hints::NextAction};

/// Standard JSON success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSuccess<T> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T> JsonSuccess<T> {
    /// Create a new success response
    pub const fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Standard JSON error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub success: bool,
    pub error: ErrorDetail,
}

impl Default for JsonError {
    fn default() -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "UNKNOWN".to_string(),
                message: "An unknown error occurred".to_string(),
                exit_code: 4,
                details: None,
                suggestion: None,
            },
        }
    }
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Machine-readable error code (`SCREAMING_SNAKE_CASE`)
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Semantic exit code (1-4)
    pub exit_code: i32,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggestion for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl JsonError {
    /// Create a new JSON error with just a code and message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                exit_code: 4, // Default to unknown/external error
                details: None,
                suggestion: None,
            },
        }
    }

    /// Add details to the error
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }

    /// Add a suggestion to the error
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error.suggestion = Some(suggestion.into());
        self
    }

    /// Set exit code for this error
    #[must_use]
    pub const fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.error.exit_code = exit_code;
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize error: {e}")))
    }
}

/// Error codes for machine-readable errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Session errors
    SessionNotFound,
    SessionAlreadyExists,
    SessionNameInvalid,

    // Workspace errors
    WorkspaceCreationFailed,
    WorkspaceNotFound,

    // JJ errors
    JjNotInstalled,
    JjCommandFailed,
    NotJjRepository,

    // Zellij errors
    ZellijNotRunning,
    ZellijCommandFailed,

    // Config errors
    ConfigNotFound,
    ConfigParseError,
    ConfigKeyNotFound,

    // Hook errors
    HookFailed,
    HookExecutionError,

    // State errors
    StateDbCorrupted,
    StateDbLocked,

    // Spawn errors
    SpawnNotOnMain,
    SpawnInvalidBeadStatus,
    SpawnBeadNotFound,
    SpawnWorkspaceCreationFailed,
    SpawnAgentSpawnFailed,
    SpawnTimeout,
    SpawnMergeFailed,
    SpawnCleanupFailed,
    SpawnDatabaseError,
    SpawnJjCommandFailed,

    // Generic errors
    InvalidArgument,
    Unknown,
}

impl ErrorCode {
    /// Get the string representation of the error code
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionAlreadyExists => "SESSION_ALREADY_EXISTS",
            Self::SessionNameInvalid => "SESSION_NAME_INVALID",
            Self::WorkspaceCreationFailed => "WORKSPACE_CREATION_FAILED",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::JjNotInstalled => "JJ_NOT_INSTALLED",
            Self::JjCommandFailed => "JJ_COMMAND_FAILED",
            Self::NotJjRepository => "NOT_JJ_REPOSITORY",
            Self::ZellijNotRunning => "ZELLIJ_NOT_RUNNING",
            Self::ZellijCommandFailed => "ZELLIJ_COMMAND_FAILED",
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::ConfigParseError => "CONFIG_PARSE_ERROR",
            Self::ConfigKeyNotFound => "CONFIG_KEY_NOT_FOUND",
            Self::HookFailed => "HOOK_FAILED",
            Self::HookExecutionError => "HOOK_EXECUTION_ERROR",
            Self::StateDbCorrupted => "STATE_DB_CORRUPTED",
            Self::StateDbLocked => "STATE_DB_LOCKED",
            Self::SpawnNotOnMain => "SPAWN_NOT_ON_MAIN",
            Self::SpawnInvalidBeadStatus => "SPAWN_INVALID_BEAD_STATUS",
            Self::SpawnBeadNotFound => "SPAWN_BEAD_NOT_FOUND",
            Self::SpawnWorkspaceCreationFailed => "SPAWN_WORKSPACE_CREATION_FAILED",
            Self::SpawnAgentSpawnFailed => "SPAWN_AGENT_SPAWN_FAILED",
            Self::SpawnTimeout => "SPAWN_TIMEOUT",
            Self::SpawnMergeFailed => "SPAWN_MERGE_FAILED",
            Self::SpawnCleanupFailed => "SPAWN_CLEANUP_FAILED",
            Self::SpawnDatabaseError => "SPAWN_DATABASE_ERROR",
            Self::SpawnJjCommandFailed => "SPAWN_JJ_COMMAND_FAILED",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}

/// Classify an error into a semantic exit code.
///
/// Exit codes follow this semantic mapping:
/// - 1: Validation errors (user input issues)
/// - 2: Not found errors (missing resources)
/// - 3: System errors (IO, database issues)
/// - 4: External command errors
const fn classify_exit_code(error: &crate::Error) -> i32 {
    use crate::Error;
    match error {
        // Validation errors: exit code 1
        Error::InvalidConfig(_) | Error::ValidationError(_) | Error::ParseError(_) => 1,
        // Not found errors: exit code 2
        Error::NotFound(_) => 2,
        // System errors: exit code 3
        Error::IoError(_) | Error::DatabaseError(_) => 3,
        // External command errors: exit code 4
        Error::Command(_)
        | Error::JjCommandError { .. }
        | Error::HookFailed { .. }
        | Error::HookExecutionFailed { .. }
        | Error::Unknown(_) => 4,
        // Lock contention errors: exit code 5
        Error::SessionLocked { .. } | Error::NotLockHolder { .. } => 5,
        // Operation cancelled: exit code 130
        Error::OperationCancelled(_) => 130,
    }
}

impl ErrorDetail {
    /// Construct an `ErrorDetail` from an Error.
    ///
    /// This is the standard way to convert errors to JSON-serializable format.
    #[must_use]
    pub fn from_error(error: &crate::Error) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            exit_code: classify_exit_code(error),
            details: error.context_map(),
            suggestion: error.suggestion(),
        }
    }
}

/// Map a `crate::Error` to (`ErrorCode`, message, optional suggestion)
fn map_error_to_parts(err: &crate::Error) -> (ErrorCode, String, Option<String>) {
    use crate::Error;

    match err {
        Error::InvalidConfig(msg) => (
            ErrorCode::ConfigParseError,
            format!("Invalid configuration: {msg}"),
            Some("Check your configuration file for errors".to_string()),
        ),
        Error::IoError(msg) => (
            ErrorCode::Unknown,
            format!("IO error: {msg}"),
            None,
        ),
        Error::ParseError(msg) => (
            ErrorCode::ConfigParseError,
            format!("Parse error: {msg}"),
            None,
        ),
        Error::ValidationError(msg) => (
            ErrorCode::InvalidArgument,
            format!("Validation error: {msg}"),
            None,
        ),
        Error::NotFound(msg) => (
            ErrorCode::SessionNotFound,
            format!("Not found: {msg}"),
            Some("Use 'zjj list' to see available sessions".to_string()),
        ),
        Error::DatabaseError(msg) => (
            ErrorCode::StateDbCorrupted,
            format!("Database error: {msg}"),
            Some("Try running 'zjj doctor --fix' to repair the database".to_string()),
        ),
        Error::Command(msg) => (
            ErrorCode::Unknown,
            format!("Command error: {msg}"),
            None,
        ),
        Error::HookFailed {
            hook_type,
            command,
            exit_code,
            stdout: _,
            stderr,
        } => (
            ErrorCode::HookFailed,
            format!(
                "Hook '{hook_type}' failed: {command}\nExit code: {exit_code:?}\nStderr: {stderr}"
            ),
            Some("Check your hook configuration and ensure the command is correct".to_string()),
        ),
        Error::HookExecutionFailed { command, source } => (
            ErrorCode::HookExecutionError,
            format!("Failed to execute hook '{command}': {source}"),
            Some("Ensure the hook command exists and is executable".to_string()),
        ),
        Error::JjCommandError {
            operation,
            source,
            is_not_found,
        } => {
            if *is_not_found {
                (
                    ErrorCode::JjNotInstalled,
                    format!("Failed to {operation}: JJ is not installed or not in PATH"),
                    Some("Install JJ: cargo install jj-cli or brew install jj".to_string()),
                )
            } else {
                (
                    ErrorCode::JjCommandFailed,
                    format!("Failed to {operation}: {source}"),
                    None,
                )
            }
        }
        Error::Unknown(msg) => (
            ErrorCode::Unknown,
            format!("Unknown error: {msg}"),
            None,
        ),
        Error::SessionLocked { session, holder } => (
            ErrorCode::Unknown,
            format!("Session '{session}' is locked by agent '{holder}'"),
            Some("Wait for the other agent to finish or check lock status".to_string()),
        ),
        Error::NotLockHolder { session, agent_id } => (
            ErrorCode::Unknown,
            format!("Agent '{agent_id}' does not hold the lock for session '{session}'"),
            None,
        ),
        Error::OperationCancelled(reason) => (
            ErrorCode::Unknown,
            format!("Operation cancelled: {reason}"),
            Some("Operation was interrupted by shutdown signal".to_string()),
        ),
    }
}

impl From<&crate::Error> for JsonError {
    fn from(err: &crate::Error) -> Self {
        let (code, message, suggestion) = map_error_to_parts(err);

        let mut json_error = Self::new(code, message);
        if let Some(sugg) = suggestion {
            json_error = json_error.with_suggestion(sugg);
        }
        // Override exit code to match the error classification
        json_error.error.exit_code = classify_exit_code(err);
        json_error
    }
}

impl From<crate::Error> for JsonError {
    fn from(err: crate::Error) -> Self {
        Self::from(&err)
    }
}

// Note: from_anyhow method removed as zjj-core doesn't depend on anyhow
// If needed, implement this in the zjj crate instead

/// Trait for types that can be serialized to JSON
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string
    fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize to JSON: {e}")))
    }
}

// Implement for all Serialize types
impl<T: Serialize> JsonSerializable for T {}

/// HATEOAS-style link for API discoverability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HateoasLink {
    /// Link relation type (e.g., "self", "next", "parent")
    pub rel: String,
    /// The command or action to take
    pub href: String,
    /// HTTP-like method hint ("GET" for read, "POST" for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl HateoasLink {
    /// Create a self-reference link
    #[must_use]
    pub fn self_link(command: impl Into<String>) -> Self {
        Self {
            rel: "self".to_string(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create a related resource link
    #[must_use]
    pub fn related(rel: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("GET".to_string()),
            title: None,
        }
    }

    /// Create an action link (mutating operation)
    #[must_use]
    pub fn action(
        rel: impl Into<String>,
        command: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: command.into(),
            method: Some("POST".to_string()),
            title: Some(title.into()),
        }
    }

    /// Add a title to this link
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Related resource information for cross-referencing
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelatedResources {
    /// Related sessions
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sessions: Vec<String>,
    /// Related beads/issues
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub beads: Vec<String>,
    /// Related workspaces
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub workspaces: Vec<String>,
    /// Related commits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub commits: Vec<String>,
    /// Parent resource (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Child resources
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<String>,
}

impl RelatedResources {
    /// Check if there are any related resources
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sessions.is_empty()
            && self.beads.is_empty()
            && self.workspaces.is_empty()
            && self.commits.is_empty()
            && self.parent.is_none()
            && self.children.is_empty()
    }
}

/// Response metadata for debugging and tracing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseMeta {
    /// Command that generated this response
    pub command: String,
    /// Timestamp of response generation (ISO 8601)
    pub timestamp: String,
    /// Duration of command execution in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether this was a dry-run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    /// Whether the operation is reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    /// Command to undo this operation (if reversible)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Agent ID if executed by an agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

impl ResponseMeta {
    /// Create new metadata for a command
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
            dry_run: None,
            reversible: None,
            undo_command: None,
            request_id: None,
            agent_id: None,
        }
    }

    /// Set duration
    #[must_use]
    pub const fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Mark as dry run
    #[must_use]
    pub const fn as_dry_run(mut self) -> Self {
        self.dry_run = Some(true);
        self
    }

    /// Mark as reversible with undo command
    #[must_use]
    pub fn with_undo(mut self, undo_cmd: impl Into<String>) -> Self {
        self.reversible = Some(true);
        self.undo_command = Some(undo_cmd.into());
        self
    }

    /// Set agent ID
    #[must_use]
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set request ID
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

/// Generic schema envelope for protocol-compliant JSON responses
///
/// Wraps response data with schema metadata (`$schema`, `_schema_version`) for AI-first CLI design.
/// All JSON outputs should be wrapped with this envelope to conform to `ResponseEnvelope` pattern.
///
/// Includes HATEOAS-style navigation with `_links`, `_related`, and `_meta` blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelope<T> {
    /// JSON Schema reference (e.g., `zjj://status-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("single" for objects, "array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Response data (flattened into envelope at JSON level)
    #[serde(flatten)]
    pub data: T,
    /// Suggested next actions for AI agents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub next: Vec<NextAction>,
    /// Available fixes for errors (empty for success responses)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<Fix>,
    /// HATEOAS-style navigation links
    #[serde(rename = "_links", skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<HateoasLink>,
    /// Related resources for cross-referencing
    #[serde(rename = "_related", skip_serializing_if = "Option::is_none")]
    pub related: Option<RelatedResources>,
    /// Response metadata for debugging and tracing
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SchemaEnvelope<T> {
    /// Create a new schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "status-response")
    /// * `schema_type` - Response shape ("single" or "array")
    /// * `data` - The response data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// let envelope = SchemaEnvelope::new("status-response", "single", data);
    /// ```
    pub fn new(schema_name: &str, schema_type: &str, data: T) -> Self {
        Self {
            schema: format!("zjj://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Create a schema envelope with next actions
    pub fn with_next(schema_name: &str, schema_type: &str, data: T, next: Vec<NextAction>) -> Self {
        Self {
            schema: format!("zjj://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: schema_type.to_string(),
            success: true,
            data,
            next,
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links to envelope
    #[must_use]
    pub fn with_links(mut self, links: Vec<HateoasLink>) -> Self {
        self.links = links;
        self
    }

    /// Add a single link
    #[must_use]
    pub fn add_link(mut self, link: HateoasLink) -> Self {
        self.links.push(link);
        self
    }

    /// Add related resources
    #[must_use]
    pub fn with_related(mut self, related: RelatedResources) -> Self {
        if !related.is_empty() {
            self.related = Some(related);
        }
        self
    }

    /// Add response metadata
    #[must_use]
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add fixes to envelope
    #[must_use]
    pub fn with_fixes(mut self, fixes: Vec<Fix>) -> Self {
        self.fixes = fixes;
        self
    }

    /// Mark as failed response
    #[must_use]
    pub const fn as_error(mut self) -> Self {
        self.success = false;
        self
    }
}

/// Schema envelope for array responses
///
/// Unlike `SchemaEnvelope` which uses flatten for single objects,
/// `SchemaEnvelopeArray` explicitly wraps array data because serde flatten
/// cannot serialize sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelopeArray<T> {
    /// JSON Schema reference (e.g., `zjj://list-response/v1`)
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Schema version for compatibility tracking
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    /// Response shape type ("array" for collections)
    pub schema_type: String,
    /// Success flag
    pub success: bool,
    /// Array data (cannot be flattened, so stored as explicit field)
    pub data: Vec<T>,
    /// Suggested next actions for AI agents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub next: Vec<NextAction>,
    /// Available fixes for errors (empty for success responses)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<Fix>,
    /// HATEOAS-style navigation links
    #[serde(rename = "_links", skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<HateoasLink>,
    /// Related resources for cross-referencing
    #[serde(rename = "_related", skip_serializing_if = "Option::is_none")]
    pub related: Option<RelatedResources>,
    /// Response metadata for debugging and tracing
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

impl<T> SchemaEnvelopeArray<T> {
    /// Create a new array schema envelope
    ///
    /// # Arguments
    /// * `schema_name` - Command/response type (e.g., "list-response")
    /// * `data` - The array data to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// let envelope = SchemaEnvelopeArray::new("list-response", items);
    /// ```
    pub fn new(schema_name: &str, data: Vec<T>) -> Self {
        Self {
            schema: format!("zjj://{schema_name}/v1"),
            schema_version: "1.0".to_string(),
            schema_type: "array".to_string(),
            success: true,
            data,
            next: Vec::new(),
            fixes: Vec::new(),
            links: Vec::new(),
            related: None,
            meta: None,
        }
    }

    /// Add HATEOAS links
    #[must_use]
    pub fn with_links(mut self, links: Vec<HateoasLink>) -> Self {
        self.links = links;
        self
    }

    /// Add related resources
    #[must_use]
    pub fn with_related(mut self, related: RelatedResources) -> Self {
        if !related.is_empty() {
            self.related = Some(related);
        }
        self
    }

    /// Add response metadata
    #[must_use]
    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add next actions
    #[must_use]
    pub fn with_next(mut self, next: Vec<NextAction>) -> Self {
        self.next = next;
        self
    }
}

/// Helper to create error details with available sessions
pub fn error_with_available_sessions(
    code: ErrorCode,
    message: impl Into<String>,
    session_name: impl Into<String>,
    available: &[String],
) -> JsonError {
    let details = serde_json::json!({
        "session_name": session_name.into(),
        "available_sessions": available,
    });

    JsonError::new(code, message)
        .with_details(details)
        .with_suggestion("Use 'zjj list' to see available sessions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_basic() {
        let err = JsonError::new("TEST_ERROR", "Test error message");
        assert_eq!(err.error.code, "TEST_ERROR");
        assert_eq!(err.error.message, "Test error message");
        assert!(err.error.details.is_none());
        assert!(err.error.suggestion.is_none());
    }

    #[test]
    fn test_json_error_with_details() {
        let details = serde_json::json!({"key": "value"});
        let err = JsonError::new("TEST_ERROR", "Test").with_details(details.clone());

        assert!(err.error.details.is_some());
        assert_eq!(err.error.details, Some(details));
    }

    #[test]
    fn test_json_error_with_suggestion() {
        let err = JsonError::new("TEST_ERROR", "Test").with_suggestion("Try this instead");

        assert_eq!(err.error.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::SessionNotFound.as_str(), "SESSION_NOT_FOUND");
        assert_eq!(ErrorCode::JjNotInstalled.as_str(), "JJ_NOT_INSTALLED");
        assert_eq!(ErrorCode::HookFailed.as_str(), "HOOK_FAILED");
    }

    #[test]
    fn test_error_code_to_string() {
        let code: String = ErrorCode::SessionNotFound.into();
        assert_eq!(code, "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_json_error_serialization() -> crate::Result<()> {
        let err = JsonError::new("TEST_ERROR", "Test message");
        let json = err.to_json()?;

        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));

        Ok(())
    }

    #[test]
    fn test_error_with_available_sessions() {
        let available = vec!["session1".to_string(), "session2".to_string()];
        let err = error_with_available_sessions(
            ErrorCode::SessionNotFound,
            "Session 'foo' not found",
            "foo",
            &available,
        );

        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.details.is_some());
        assert!(err.error.suggestion.is_some());
    }

    #[test]
    fn test_json_serializable_trait() -> crate::Result<()> {
        #[derive(Serialize)]
        struct TestStruct {
            field: String,
        }

        let test = TestStruct {
            field: "value".to_string(),
        };

        let json = test.to_json()?;
        assert!(json.contains("\"field\""));
        assert!(json.contains("\"value\""));

        Ok(())
    }

    #[test]
    fn test_json_success_wrapper() -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
            count: usize,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        let success = JsonSuccess {
            success: true,
            data,
        };
        let json = success.to_json()?;

        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
        assert!(json.contains("\"count\""));
        assert!(json.contains("42"));

        Ok(())
    }

    #[test]
    fn test_error_detail_skip_none() -> crate::Result<()> {
        let err = JsonError::new("TEST", "message");
        let json = err.to_json()?;

        // Should not contain "details" or "suggestion" fields when they're None
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"suggestion\""));

        Ok(())
    }

    // Tests for ErrorDetail::from_error() constructor (zjj-lgkf Phase 4 - RED)
    #[test]
    fn test_error_detail_from_validation_error() {
        let err = crate::Error::ValidationError("invalid session name".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "VALIDATION_ERROR");
        assert!(detail.message.contains("Validation error"));
        assert_eq!(detail.exit_code, 1);
    }

    #[test]
    fn test_error_detail_from_io_error() {
        let err = crate::Error::IoError("file not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "IO_ERROR");
        assert!(detail.message.contains("IO error"));
        assert_eq!(detail.exit_code, 3);
    }

    #[test]
    fn test_error_detail_from_not_found_error() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        assert_eq!(detail.code, "NOT_FOUND");
        assert!(detail.message.contains("Not found"));
        assert_eq!(detail.exit_code, 2);
    }

    #[test]
    fn test_error_detail_preserves_context() {
        let err = crate::Error::ValidationError("invalid input".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have context map populated
        assert!(detail.details.is_some());
    }

    #[test]
    fn test_error_detail_includes_suggestion() {
        let err = crate::Error::NotFound("session not found".into());
        let detail = ErrorDetail::from_error(&err);

        // Should have suggestion populated
        assert!(detail.suggestion.is_some());
        if let Some(sugg) = detail.suggestion {
            assert!(sugg.contains("list"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // HATEOAS LINK TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_hateoas_link_self() {
        let link = HateoasLink::self_link("zjj status test");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "zjj status test");
        assert_eq!(link.method, Some("GET".to_string()));
        assert!(link.title.is_none());
    }

    #[test]
    fn test_hateoas_link_related() {
        let link = HateoasLink::related("parent", "zjj list");
        assert_eq!(link.rel, "parent");
        assert_eq!(link.href, "zjj list");
        assert_eq!(link.method, Some("GET".to_string()));
    }

    #[test]
    fn test_hateoas_link_action() {
        let link = HateoasLink::action("remove", "zjj remove test", "Delete session");
        assert_eq!(link.rel, "remove");
        assert_eq!(link.href, "zjj remove test");
        assert_eq!(link.method, Some("POST".to_string()));
        assert_eq!(link.title, Some("Delete session".to_string()));
    }

    #[test]
    fn test_hateoas_link_with_title() {
        let link = HateoasLink::self_link("zjj status").with_title("Get current status");
        assert_eq!(link.title, Some("Get current status".to_string()));
    }

    #[test]
    fn test_hateoas_link_serialization() -> crate::Result<()> {
        let link = HateoasLink::action("sync", "zjj sync test", "Sync session");
        let json =
            serde_json::to_string(&link).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"rel\":\"sync\""));
        assert!(json.contains("\"href\":\"zjj sync test\""));
        assert!(json.contains("\"method\":\"POST\""));
        assert!(json.contains("\"title\":\"Sync session\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RELATED RESOURCES TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_related_resources_empty() {
        let related = RelatedResources::default();
        assert!(related.is_empty());
    }

    #[test]
    fn test_related_resources_with_sessions() {
        let related = RelatedResources {
            sessions: vec!["session-1".to_string(), "session-2".to_string()],
            ..Default::default()
        };
        assert!(!related.is_empty());
        assert_eq!(related.sessions.len(), 2);
    }

    #[test]
    fn test_related_resources_with_parent() {
        let related = RelatedResources {
            parent: Some("main".to_string()),
            ..Default::default()
        };
        assert!(!related.is_empty());
    }

    #[test]
    fn test_related_resources_serialization() -> crate::Result<()> {
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["zjj-1234".to_string()],
            commits: vec!["abc123".to_string()],
            ..Default::default()
        };
        let json =
            serde_json::to_string(&related).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"sessions\":[\"s1\"]"));
        assert!(json.contains("\"beads\":[\"zjj-1234\"]"));
        assert!(json.contains("\"commits\":[\"abc123\"]"));
        // Empty fields should be omitted
        assert!(!json.contains("\"workspaces\""));
        assert!(!json.contains("\"parent\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // RESPONSE META TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_response_meta_new() {
        let meta = ResponseMeta::new("status");
        assert_eq!(meta.command, "status");
        assert!(!meta.timestamp.is_empty());
        assert!(meta.duration_ms.is_none());
        assert!(meta.dry_run.is_none());
        assert!(meta.reversible.is_none());
        assert!(meta.undo_command.is_none());
    }

    #[test]
    fn test_response_meta_with_duration() {
        let meta = ResponseMeta::new("add").with_duration(150);
        assert_eq!(meta.duration_ms, Some(150));
    }

    #[test]
    fn test_response_meta_as_dry_run() {
        let meta = ResponseMeta::new("remove").as_dry_run();
        assert_eq!(meta.dry_run, Some(true));
    }

    #[test]
    fn test_response_meta_with_undo() {
        let meta = ResponseMeta::new("remove test").with_undo("zjj undo");
        assert_eq!(meta.reversible, Some(true));
        assert_eq!(meta.undo_command, Some("zjj undo".to_string()));
    }

    #[test]
    fn test_response_meta_with_agent() {
        let meta = ResponseMeta::new("work").with_agent("agent-001");
        assert_eq!(meta.agent_id, Some("agent-001".to_string()));
    }

    #[test]
    fn test_response_meta_with_request_id() {
        let meta = ResponseMeta::new("status").with_request_id("req-123");
        assert_eq!(meta.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_response_meta_serialization() -> crate::Result<()> {
        let meta = ResponseMeta::new("add test")
            .with_duration(50)
            .with_undo("zjj undo")
            .with_agent("agent-x");
        let json =
            serde_json::to_string(&meta).map_err(|e| crate::Error::ParseError(e.to_string()))?;

        assert!(json.contains("\"command\":\"add test\""));
        assert!(json.contains("\"duration_ms\":50"));
        assert!(json.contains("\"reversible\":true"));
        assert!(json.contains("\"undo_command\":\"zjj undo\""));
        assert!(json.contains("\"agent_id\":\"agent-x\""));
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE WITH HATEOAS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_envelope_with_links() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test".to_string(),
        };
        let envelope = SchemaEnvelope::new("test-response", "single", data)
            .add_link(HateoasLink::self_link("zjj status test"))
            .add_link(HateoasLink::related("list", "zjj list"));

        assert_eq!(envelope.links.len(), 2);
        assert_eq!(
            envelope.links.first().map(|l| &l.rel),
            Some(&"self".to_string())
        );
        assert_eq!(
            envelope.links.get(1).map(|l| &l.rel),
            Some(&"list".to_string())
        );
    }

    #[test]
    fn test_schema_envelope_with_related() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            id: String,
        }

        let data = TestData {
            id: "abc".to_string(),
        };
        let related = RelatedResources {
            sessions: vec!["s1".to_string()],
            beads: vec!["zjj-001".to_string()],
            ..Default::default()
        };
        let envelope = SchemaEnvelope::new("test-response", "single", data).with_related(related);

        assert!(envelope.related.is_some());
        if let Some(rel) = envelope.related.as_ref() {
            assert_eq!(rel.sessions.len(), 1);
            assert_eq!(rel.beads.len(), 1);
        }
    }

    #[test]
    fn test_schema_envelope_with_meta() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            value: i32,
        }

        let data = TestData { value: 42 };
        let meta = ResponseMeta::new("test").with_duration(100);
        let envelope = SchemaEnvelope::new("test-response", "single", data).with_meta(meta);

        assert!(envelope.meta.is_some());
        if let Some(m) = envelope.meta {
            assert_eq!(m.command, "test");
            assert_eq!(m.duration_ms, Some(100));
        }
    }

    #[test]
    fn test_schema_envelope_as_error() {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            error: String,
        }

        let data = TestData {
            error: "failed".to_string(),
        };
        let envelope = SchemaEnvelope::new("error-response", "single", data).as_error();

        assert!(!envelope.success);
    }

    #[test]
    fn test_schema_envelope_with_fixes() {
        use crate::fix::Fix;

        #[derive(Serialize, Deserialize)]
        struct TestData {
            status: String,
        }

        let data = TestData {
            status: "error".to_string(),
        };
        let fixes = vec![Fix::safe("Try again", vec!["zjj retry".to_string()])];
        let envelope = SchemaEnvelope::new("error-response", "single", data).with_fixes(fixes);

        assert_eq!(envelope.fixes.len(), 1);
    }

    #[test]
    fn test_schema_envelope_full_serialization() -> crate::Result<()> {
        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
        }

        let data = TestData {
            name: "test-session".to_string(),
        };
        let envelope = SchemaEnvelope::new("session-response", "single", data)
            .add_link(HateoasLink::self_link("zjj status test-session"))
            .with_related(RelatedResources {
                beads: vec!["zjj-1".to_string()],
                ..Default::default()
            })
            .with_meta(ResponseMeta::new("status test-session").with_duration(25));

        // Functional approach: Serialize → Parse → Validate structure
        let result = serde_json::to_string_pretty(&envelope)
            .map_err(|e| crate::Error::ParseError(e.to_string()))
            .and_then(|json_str| {
                // Parse JSON to verify structure (type-safe, not fragile string matching)
                serde_json::from_str::<serde_json::Value>(&json_str)
                    .map(|value| (json_str, value))
                    .map_err(|e| crate::Error::ParseError(format!("Failed to parse JSON: {e}")))
            });

        let (json_str, parsed) = result?;

        // Validate structure via parsed JSON (immutable, composable checks)
        let checks = [
            (parsed.get("$schema").is_some(), "$schema field missing"),
            (
                parsed.get("_schema_version").is_some(),
                "_schema_version field missing",
            ),
            (
                parsed.get("_links").and_then(|v| v.as_array()).is_some(),
                "_links should be array",
            ),
            (parsed.get("_related").is_some(), "_related field missing"),
            (parsed.get("_meta").is_some(), "_meta field missing"),
            (
                parsed.get("name").and_then(|v| v.as_str()) == Some("test-session"),
                "name field should be 'test-session'",
            ),
        ];

        // Functional validation: all checks must pass (Railway pattern)
        for (passed, msg) in &checks {
            assert!(
                passed,
                "Schema validation failed: {msg}\n\nJSON:\n{json_str}"
            );
        }

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SCHEMA ENVELOPE ARRAY WITH HATEOAS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_schema_envelope_array_with_links() {
        let data = vec!["item1".to_string(), "item2".to_string()];
        let envelope = SchemaEnvelopeArray::new("list-response", data)
            .with_links(vec![HateoasLink::self_link("zjj list")]);

        assert_eq!(envelope.links.len(), 1);
        assert_eq!(envelope.data.len(), 2);
    }

    #[test]
    fn test_schema_envelope_array_with_meta() {
        let data = vec![1, 2, 3];
        let meta = ResponseMeta::new("list").with_duration(10);
        let envelope = SchemaEnvelopeArray::new("numbers-response", data).with_meta(meta);

        assert!(envelope.meta.is_some());
        assert_eq!(envelope.data.len(), 3);
    }

    #[test]
    fn test_schema_envelope_array_with_next() {
        use crate::hints::{ActionRisk, NextAction};

        let data: Vec<String> = vec![];
        let next = vec![NextAction {
            action: "Create first item".to_string(),
            commands: vec!["zjj add item".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        }];
        let envelope = SchemaEnvelopeArray::new("empty-list", data).with_next(next);

        assert_eq!(envelope.next.len(), 1);
        assert!(envelope.data.is_empty());
    }
}

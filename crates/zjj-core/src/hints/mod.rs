//! Contextual hints and smart suggestions for AI agents
//!
//! Provides context-aware hints based on system state:
//! - Suggested next actions
//! - State explanations
//! - Learning from errors
//! - Predictive hints
//!
//! # Structure
//!
//! Hints are organized by category:
//! - **Session hints**: Suggestions based on session state
//! - **Workflow hints**: Next actions and workflow recommendations
//! - **Error hints**: Context-aware solutions for errors
//! - **Beads hints**: Issue tracking and task management suggestions

mod error_hints;
mod session_hints;
mod workflow_hints;

use serde::{Deserialize, Serialize};

use crate::{
    types::{BeadsSummary, Session, SessionStatus},
    Result,
};

// Re-export hint generators for public API
pub use error_hints::{hints_for_error, hints_for_error_code};
pub use session_hints::{hints_for_beads, suggest_session_actions};
pub use workflow_hints::{suggest_next_actions, suggest_workflow_hints};

// ═══════════════════════════════════════════════════════════════════════════
// HINT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A contextual hint from jjz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hint {
    /// Hint type
    #[serde(rename = "type")]
    pub hint_type: HintType,

    /// Human-readable message
    pub message: String,

    /// Suggested command to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<String>,

    /// Rationale for this hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    /// Information about current state
    Info,
    /// Suggested next action
    Suggestion,
    /// Warning about potential issue
    Warning,
    /// Explanation of error
    Error,
    /// Learning tip
    Tip,
}

/// System state for hint generation
#[derive(Debug, Clone)]
pub struct SystemState {
    /// All sessions
    pub sessions: Vec<Session>,

    /// Whether system is initialized
    pub initialized: bool,

    /// Whether JJ repo exists
    pub jj_repo: bool,
}

/// Next action suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    /// Action description
    pub action: String,

    /// Commands to execute
    pub commands: Vec<String>,
}

/// Complete hints response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintsResponse {
    /// Current system context
    pub context: SystemContext,

    /// Generated hints
    pub hints: Vec<Hint>,

    /// Suggested next actions
    pub next_actions: Vec<NextAction>,
}

/// System context summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemContext {
    /// Is jjz initialized?
    pub initialized: bool,

    /// Is this a JJ repository?
    pub jj_repo: bool,

    /// Total number of sessions
    pub sessions_count: usize,

    /// Number of active sessions
    pub active_sessions: usize,

    /// Are there uncommitted changes?
    pub has_changes: bool,
}

/// Error hint container for structured error suggestions
#[derive(Debug, Clone)]
pub(crate) struct ErrorHintParams {
    /// Error code for classification
    pub error_code: String,
    /// Error message with context
    pub error_msg: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// HINT BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

impl Hint {
    /// Create an info hint
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Info,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a suggestion hint
    #[must_use]
    pub fn suggestion(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Suggestion,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a warning hint
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Warning,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a tip hint
    #[must_use]
    pub fn tip(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Tip,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create an error hint
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Error,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Add a suggested command
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.suggested_command = Some(command.into());
        self
    }

    /// Add a rationale
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add context
    #[must_use]
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

/// Generate contextual hints based on system state
///
/// Combines session-based hints into a comprehensive list of suggestions.
///
/// # Errors
///
/// Returns error if unable to analyze state
pub fn generate_hints(state: &SystemState) -> Result<Vec<Hint>> {
    let hints = session_hints::generate_session_hints(state)?;
    Ok(hints)
}

/// Generate hints for a specific error (public re-export from error module)
/// Delegates to error_hints module for hint generation
pub fn hints_for_error(error_code: &str, error_msg: &str) -> Vec<Hint> {
    error_hints::hints_for_error(error_code, error_msg)
}

/// Generate suggested next actions based on state (public re-export from workflow module)
pub fn suggest_next_actions(state: &SystemState) -> Vec<NextAction> {
    workflow_hints::suggest_next_actions(state)
}

/// Generate complete hints response
///
/// # Errors
///
/// Returns error if unable to generate hints
pub fn generate_hints_response(state: &SystemState) -> Result<HintsResponse> {
    let hints = generate_hints(state)?;
    let next_actions = suggest_next_actions(state);

    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    // Detect uncommitted changes if in a JJ repo using functional composition
    let has_changes = if state.jj_repo {
        crate::jj::check_in_jj_repo()
            .ok()
            .and_then(|repo_path| crate::jj::has_uncommitted_changes(&repo_path).ok())
            .unwrap_or(false)
    } else {
        false
    };

    let context = SystemContext {
        initialized: state.initialized,
        jj_repo: state.jj_repo,
        sessions_count: state.sessions.len(),
        active_sessions: active_count,
        has_changes,
    };

    Ok(HintsResponse {
        context,
        hints,
        next_actions,
    })
}

/// Generate hints for beads status (public re-export from session module)
pub fn hints_for_beads(session_name: &str, beads: &BeadsSummary) -> Vec<Hint> {
    session_hints::hints_for_beads(session_name, beads)
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;

    fn create_test_session(name: &str, status: SessionStatus) -> Session {
        Session {
            id: format!("id-{name}"),
            name: name.to_string(),
            status,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_hint_builders() {
        let hint = Hint::info("Test message")
            .with_command("jjz test")
            .with_rationale("Testing");

        assert_eq!(hint.hint_type, HintType::Info);
        assert_eq!(hint.message, "Test message");
        assert_eq!(hint.suggested_command, Some("jjz test".to_string()));
        assert_eq!(hint.rationale, Some("Testing".to_string()));
    }

    #[test]
    fn test_generate_hints_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("first parallel workspace"));
    }

    #[test]
    fn test_generate_hints_response() {
        let state = SystemState {
            sessions: vec![create_test_session("active", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        let response = generate_hints_response(&state).unwrap_or_else(|_| HintsResponse {
            context: SystemContext {
                initialized: true,
                jj_repo: true,
                sessions_count: 0,
                active_sessions: 0,
                has_changes: false,
            },
            hints: Vec::new(),
            next_actions: Vec::new(),
        });

        assert_eq!(response.context.sessions_count, 1);
        assert_eq!(response.context.active_sessions, 1);
        assert!(!response.hints.is_empty());
        assert!(!response.next_actions.is_empty());
    }
}

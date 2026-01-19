//! Workflow-based hint generation
//!
//! Pure functions for generating hints and next actions based on
//! system workflow state and initialization status.

use im::{vector, Vector};
use tap::Pipe;

use crate::{
    hints::{NextAction, SystemState},
    types::SessionStatus,
};

// ═══════════════════════════════════════════════════════════════════════════
// NEXT ACTION BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

/// Create "initialize system" next action
#[must_use]
pub(crate) fn action_initialize() -> NextAction {
    NextAction {
        action: "Initialize zjj".to_string(),
        commands: vec!["zjj init".to_string()],
    }
}

/// Create "create first session" next action
#[must_use]
pub(crate) fn action_create_first_session() -> NextAction {
    NextAction {
        action: "Create first session".to_string(),
        commands: vec!["zjj add <name>".to_string()],
    }
}

/// Create "review session status" next action
#[must_use]
pub(crate) fn action_review_status() -> NextAction {
    NextAction {
        action: "Review session status".to_string(),
        commands: vec!["zjj status".to_string(), "zjj dashboard".to_string()],
    }
}

/// Create "clean up completed sessions" next action
#[must_use]
pub(crate) fn action_cleanup_completed(session_name: &str) -> NextAction {
    NextAction {
        action: "Clean up completed sessions".to_string(),
        commands: vec![format!("zjj remove {session_name} --merge")],
    }
}

/// Create "create new session" next action
#[must_use]
pub(crate) fn action_create_new_session() -> NextAction {
    NextAction {
        action: "Create new session".to_string(),
        commands: vec!["zjj add <name>".to_string()],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKFLOW STATE ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════

/// Suggest workflow hints based on current state
///
/// Analyzes system state to provide workflow recommendations
/// for user guidance and next steps.
#[must_use]
pub fn suggest_workflow_hints(state: &SystemState) -> Vector<String> {
    if !state.initialized {
        return vector!["System not initialized - run `zjj init` to get started".to_string()];
    }

    if state.sessions.is_empty() {
        return vector!["No active sessions - create one with `zjj add <name>`".to_string()];
    }

    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    match active_count {
        0 => vector!["No active sessions - all sessions are idle or completed".to_string()],
        1 => vector!["You have 1 active session".to_string()],
        n => vector![format!("You have {n} active sessions")],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// NEXT ACTIONS GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/// Generate suggested next actions based on system state
///
/// Uses functional composition to build a list of contextually
/// appropriate next actions for the user to take.
#[must_use]
pub fn suggest_next_actions(state: &SystemState) -> Vector<NextAction> {
    // Early returns for initialization path
    if !state.initialized {
        return vector![action_initialize()];
    }

    if state.sessions.is_empty() {
        return vector![action_create_first_session()];
    }

    let has_active = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Active);

    let completed_session = state
        .sessions
        .iter()
        .find(|s| s.status == SessionStatus::Completed);

    Vector::new()
        .pipe(|v| {
            if has_active {
                let mut v = v;
                v.push_back(action_review_status());
                v
            } else {
                v
            }
        })
        .pipe(|v| {
            if let Some(s) = completed_session {
                let mut v = v;
                v.push_back(action_cleanup_completed(&s.name));
                v
            } else {
                v
            }
        })
        .pipe(|v| {
            let mut v = v;
            v.push_back(action_create_new_session());
            v
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::types::Session;

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
    fn test_action_initialize() {
        let action = action_initialize();
        assert_eq!(action.action, "Initialize zjj");
        assert_eq!(action.commands[0], "zjj init");
    }

    #[test]
    fn test_action_create_first_session() {
        let action = action_create_first_session();
        assert_eq!(action.action, "Create first session");
        assert!(action.commands[0].contains("add"));
    }

    #[test]
    fn test_action_review_status() {
        let action = action_review_status();
        assert_eq!(action.action, "Review session status");
        assert_eq!(action.commands.len(), 2);
    }

    #[test]
    fn test_suggest_next_actions_not_initialized() {
        let state = SystemState {
            sessions: Vector::new(),
            initialized: false,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "Initialize zjj");
    }

    #[test]
    fn test_suggest_next_actions_no_sessions() {
        let state = SystemState {
            sessions: Vector::new(),
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_suggest_next_actions_with_active() {
        let state = SystemState {
            sessions: vector![create_test_session("active", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("Review")));
    }

    #[test]
    fn test_suggest_next_actions_with_completed() {
        let state = SystemState {
            sessions: vector![create_test_session("done", SessionStatus::Completed)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("Clean up")));
    }

    #[test]
    fn test_suggest_workflow_hints_not_initialized() {
        let state = SystemState {
            sessions: Vector::new(),
            initialized: false,
            jj_repo: true,
        };

        let hints = suggest_workflow_hints(&state);
        assert!(hints.iter().any(|h| h.contains("initialized")));
    }

    #[test]
    fn test_suggest_workflow_hints_empty() {
        let state = SystemState {
            sessions: Vector::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = suggest_workflow_hints(&state);
        // Case-insensitive check for "no active sessions"
        assert!(hints
            .iter()
            .any(|h| h.to_lowercase().contains("no active sessions")));
    }

    #[test]
    fn test_suggest_workflow_hints_multiple_active() {
        let state = SystemState {
            sessions: vector![
                create_test_session("a1", SessionStatus::Active),
                create_test_session("a2", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let hints = suggest_workflow_hints(&state);
        assert!(hints.iter().any(|h| h.contains("2 active")));
    }
}

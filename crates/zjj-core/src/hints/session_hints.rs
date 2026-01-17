//! Session-based hint generation
//!
//! Pure functions for generating hints based on session state,
//! session status changes, and beads issue tracking.

use chrono::Utc;

use crate::{
    hints::{Hint, HintType, SystemState},
    types::{BeadsSummary, SessionStatus},
    Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION HINTS
// ═══════════════════════════════════════════════════════════════════════════

/// Generate hints for a single active session
#[must_use]
pub(crate) fn hint_for_active_session(session_name: &str) -> Hint {
    Hint::info(format!("Session '{session_name}' is active"))
        .with_command(format!("jjz status {session_name}"))
        .with_rationale("Review session status regularly")
}

/// Generate hints for old completed sessions
#[must_use]
pub(crate) fn hint_for_completed_session(session_name: &str, age_days: i64) -> Hint {
    Hint::suggestion(format!(
        "Session '{session_name}' completed {age_days} day(s) ago, consider removing"
    ))
    .with_command(format!("jjz remove {session_name} --merge"))
    .with_rationale("Clean up completed work")
    .with_context(serde_json::json!({
        "session": session_name,
        "age_days": age_days,
    }))
}

/// Generate hints for failed sessions
#[must_use]
pub(crate) fn hint_for_failed_session(session_name: &str) -> Hint {
    Hint::warning(format!("Session '{session_name}' failed during creation"))
        .with_command(format!("jjz remove {session_name}"))
        .with_rationale("Clean up failed session and retry")
}

/// Generate hints for empty session list
#[must_use]
pub(crate) fn hint_for_no_sessions() -> Hint {
    Hint::suggestion("No sessions yet. Create your first parallel workspace!")
        .with_command("jjz add <name>")
        .with_rationale("Sessions enable parallel work on multiple features")
}

/// Generate hints for multiple active sessions
#[must_use]
pub(crate) fn hint_for_multiple_active_sessions() -> Hint {
    Hint::tip("You have multiple active sessions. Use the dashboard for an overview")
        .with_command("jjz dashboard")
        .with_rationale("Visual overview helps manage multiple sessions")
}

/// Generate all session-based hints from system state
///
/// # Errors
/// Returns error if unable to process sessions
pub fn generate_session_hints(state: &SystemState) -> Result<Vec<Hint>> {
    // No sessions - encourage creation
    if state.sessions.is_empty() {
        return Ok(vec![hint_for_no_sessions()]);
    }

    // Active session hints
    let active_hints: Vec<Hint> = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .map(|session| hint_for_active_session(&session.name))
        .collect();

    // Completed sessions hints (only if older than 1 day)
    let now = Utc::now();
    let completed_hints: Vec<Hint> = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Completed)
        .filter_map(|session| {
            let duration = now.signed_duration_since(session.updated_at);
            let age = duration.num_days();
            (age > 1).then(|| hint_for_completed_session(&session.name, age))
        })
        .collect();

    // Failed sessions hints
    let failed_hints: Vec<Hint> = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Failed)
        .map(|session| hint_for_failed_session(&session.name))
        .collect();

    // Multiple active sessions - suggest dashboard
    let active_count = active_hints.len();
    let dashboard_hint = if active_count > 2 {
        vec![hint_for_multiple_active_sessions()]
    } else {
        vec![]
    };

    // Chain all hints together
    let hints: Vec<Hint> = active_hints
        .into_iter()
        .chain(completed_hints)
        .chain(failed_hints)
        .chain(dashboard_hint)
        .collect();

    Ok(hints)
}

// ═══════════════════════════════════════════════════════════════════════════
// BEADS HINTS
// ═══════════════════════════════════════════════════════════════════════════

/// Generate hints for beads blockers
#[must_use]
pub(crate) fn hint_for_blocked_issues(session_name: &str, blocked_count: usize) -> Hint {
    Hint::warning(format!(
        "Session '{session_name}' has {blocked_count} blocked issue(s)"
    ))
    .with_command("bv")
    .with_rationale("Resolve blockers to make progress")
    .with_context(serde_json::json!({
        "session": session_name,
        "blocked_count": blocked_count,
    }))
}

/// Generate hints for too much work in progress
#[must_use]
pub(crate) fn hint_for_excessive_wip(session_name: &str, active_count: usize) -> Hint {
    Hint::tip(format!(
        "Session '{session_name}' has {active_count} active issues - consider focusing on fewer tasks"
    ))
    .with_rationale("Limiting work in progress improves focus")
}

/// Generate hints for no beads issues
#[must_use]
pub(crate) fn hint_for_no_beads_issues(session_name: &str) -> Hint {
    Hint::info(format!("Session '{session_name}' has no beads issues"))
        .with_command("bd new")
        .with_rationale("Track your work with beads for better organization")
}

/// Generate hints based on beads summary
///
/// Analyzes issue count, blocked items, and work-in-progress levels
/// to provide context-aware suggestions for task management.
#[must_use]
pub fn hints_for_beads(session_name: &str, beads: &BeadsSummary) -> Vec<Hint> {
    let blocker_hint = beads
        .has_blockers()
        .then(|| hint_for_blocked_issues(session_name, beads.blocked));

    let wip_hint =
        (beads.active() > 5).then(|| hint_for_excessive_wip(session_name, beads.active()));

    let empty_hint = (beads.total == 0).then(|| hint_for_no_beads_issues(session_name));

    blocker_hint
        .into_iter()
        .chain(wip_hint)
        .chain(empty_hint)
        .collect()
}

/// Suggest actions based on session activity
///
/// Provides contextual suggestions for session management
/// based on current session state.
#[must_use]
pub fn suggest_session_actions(state: &SystemState) -> Vec<String> {
    let mut actions = Vec::new();

    // Check for completed sessions that might need cleanup
    let has_completed = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Completed);

    if has_completed {
        actions.push("Review completed sessions for cleanup".to_string());
    }

    // Check for failed sessions
    let has_failed = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Failed);

    if has_failed {
        actions.push("Remove failed sessions and retry".to_string());
    }

    // Check for too many active sessions
    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    if active_count > 3 {
        actions.push("Consider consolidating active sessions".to_string());
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Session;
    use std::path::PathBuf;

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
    fn test_hint_for_active_session() {
        let hint = hint_for_active_session("test-session");
        assert_eq!(hint.hint_type, HintType::Info);
        assert!(hint.message.contains("active"));
    }

    #[test]
    fn test_hint_for_completed_session() {
        let hint = hint_for_completed_session("test-session", 5);
        assert_eq!(hint.hint_type, HintType::Suggestion);
        assert!(hint.message.contains("5 day"));
    }

    #[test]
    fn test_hint_for_failed_session() {
        let hint = hint_for_failed_session("test-session");
        assert_eq!(hint.hint_type, HintType::Warning);
        assert!(hint.message.contains("failed"));
    }

    #[test]
    fn test_generate_session_hints_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_session_hints(&state).unwrap_or_default();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("first parallel workspace"));
    }

    #[test]
    fn test_generate_session_hints_with_active() {
        let state = SystemState {
            sessions: vec![create_test_session("active-1", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_session_hints(&state).unwrap_or_default();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("active"));
    }

    #[test]
    fn test_generate_session_hints_multiple_active() {
        let state = SystemState {
            sessions: vec![
                create_test_session("active-1", SessionStatus::Active),
                create_test_session("active-2", SessionStatus::Active),
                create_test_session("active-3", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_session_hints(&state).unwrap_or_default();
        assert!(hints.iter().any(|h| h.message.contains("dashboard")));
    }

    #[test]
    fn test_hints_for_beads_blockers() {
        let beads = BeadsSummary {
            total: 11,
            open: 2,
            in_progress: 1,
            blocked: 3,
            deferred: 0,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
        assert!(hints.iter().any(|h| h.message.contains("blocked")));
    }

    #[test]
    fn test_hints_for_beads_excessive_wip() {
        let beads = BeadsSummary {
            total: 12,
            open: 7,
            in_progress: 5,
            blocked: 0,
            deferred: 0,
            closed: 0,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("fewer tasks")));
    }

    #[test]
    fn test_hints_for_beads_empty() {
        let beads = BeadsSummary::default();
        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("no beads")));
    }

    #[test]
    fn test_suggest_session_actions_completed() {
        let state = SystemState {
            sessions: vec![create_test_session("done", SessionStatus::Completed)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_session_actions(&state);
        assert!(actions.iter().any(|a| a.contains("cleanup")));
    }

    #[test]
    fn test_suggest_session_actions_many_active() {
        let state = SystemState {
            sessions: vec![
                create_test_session("a1", SessionStatus::Active),
                create_test_session("a2", SessionStatus::Active),
                create_test_session("a3", SessionStatus::Active),
                create_test_session("a4", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_session_actions(&state);
        assert!(actions.iter().any(|a| a.contains("consolidating")));
    }
}

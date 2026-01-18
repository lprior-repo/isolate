//! Session validation and lookup

use anyhow::{Context, Result};
use im;

use crate::session::{Session, SessionStatus};

/// Validate and retrieve a session by name
///
/// This function:
/// 1. Validates the session name format
/// 2. Looks up the session in the database
/// 3. Provides helpful error messages with suggestions if not found
pub async fn validate_and_get_session(
    db: &crate::database::SessionDb,
    name: &str,
) -> Result<Session> {
    // Validate session name FIRST before any operations (zjj-audit-002)
    crate::session::validate_session_name(name).context("Invalid session name")?;

    // Get all sessions for suggestions
    let all_sessions = db
        .list(None)
        .await
        .context("Failed to list sessions from database")?;

    // Try to get the specific session
    db.get(name)
        .await?
        .ok_or_else(|| build_session_not_found_error(name, &all_sessions))
}

/// Build a helpful error message when session is not found
fn build_session_not_found_error(name: &str, all_sessions: &[Session]) -> anyhow::Error {
    let active_sessions: im::Vector<String> = all_sessions
        .iter()
        .filter(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed)
        .map(|s| s.name.clone())
        .collect();

    // Find similar names (simple starts-with match for suggestions)
    let suggestions: im::Vector<String> = active_sessions
        .iter()
        .filter(|s| {
            s.starts_with(&name[..name.len().min(3)]) || name.starts_with(&s[..s.len().min(3)])
        })
        .cloned()
        .collect();

    let suggestion_text = if suggestions.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nDid you mean one of these?\n{}",
            suggestions
                .iter()
                .map(|s| format!("  • {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let active_list = if active_sessions.is_empty() {
        "No active sessions found.".to_string()
    } else {
        format!(
            "Active sessions:\n{}",
            active_sessions
                .iter()
                .map(|s| format!("  • {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    anyhow::anyhow!(
        "Session '{name}' not found\n\
         \n\
         {active_list}\n\
         \n\
         Suggestions:\n\
         • Use 'zjj list' to see all active sessions\n\
         • Use 'zjj list --all' to include completed/failed sessions\n\
         • Check the spelling of the session name{suggestion_text}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_error_format() {
        let sessions = vec![];
        let error = build_session_not_found_error("test", &sessions);
        let msg = error.to_string();
        assert!(msg.contains("Session 'test' not found"));
        assert!(msg.contains("No active sessions found"));
    }
}

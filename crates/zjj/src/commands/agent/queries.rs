//! Agent queries - session fetching and agent metadata extraction

use anyhow::Result;

use crate::{commands::get_session_db, session::SessionStatus};

/// Session with agent data
pub struct SessionWithAgent {
    pub name: String,
    pub metadata: Option<serde_json::Value>,
}

/// Get sessions filtered by criteria
///
/// # Arguments
///
/// * `session` - Optional session name to filter by, otherwise gets all active sessions
///
/// # Returns
///
/// Vector of sessions with agent metadata
pub async fn get_sessions(session: Option<&str>) -> Result<Vec<SessionWithAgent>> {
    let db = get_session_db().await?;

    if let Some(name) = session {
        // Get specific session
        let s = db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {name}"))?;
        Ok(vec![SessionWithAgent {
            name: s.name,
            metadata: s.metadata,
        }])
    } else {
        // Get all active sessions
        let sessions = db
            .list(None)
            .await?
            .into_iter()
            .filter(|s| s.status == SessionStatus::Active || s.status == SessionStatus::Creating)
            .map(|s| SessionWithAgent {
                name: s.name,
                metadata: s.metadata,
            })
            .collect();
        Ok(sessions)
    }
}

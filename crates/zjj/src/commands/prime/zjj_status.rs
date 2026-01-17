//! ZJJ initialization and session status gathering
//!
//! This module handles querying ZJJ for initialization status,
//! session counts, and active session information.

use anyhow::Result;

use crate::commands::{get_session_db, zjj_data_dir};

use super::output_types::{SessionInfo, ZjjStatus};

/// Gather ZJJ initialization and session status
///
/// Returns information about whether ZJJ is initialized and
/// provides session counts if available.
pub async fn gather_zjj_status() -> ZjjStatus {
    let initialized = zjj_data_dir().is_ok();

    if !initialized {
        return ZjjStatus {
            initialized: false,
            data_dir: None,
            total_sessions: 0,
            active_sessions: 0,
        };
    }

    let data_dir = zjj_data_dir().ok().map(|p| p.display().to_string());

    let (total_sessions, active_sessions) = match get_session_db().await {
        Ok(db) => {
            let sessions = db.list(None).await.unwrap_or_default();
            let total = sessions.len();
            let active = sessions
                .iter()
                .filter(|s| s.status.to_string() == "active")
                .count();
            (total, active)
        }
        Err(_) => (0, 0),
    };

    ZjjStatus {
        initialized: true,
        data_dir,
        total_sessions,
        active_sessions,
    }
}

/// Gather active sessions
///
/// Returns a list of all active sessions with their details.
///
/// # Returns
///
/// A vector of `SessionInfo` containing active sessions,
/// or an empty vector if no sessions are found or database access fails.
pub async fn gather_active_sessions() -> Vec<SessionInfo> {
    match get_session_db().await {
        Ok(db) => {
            let sessions = db.list(None).await.unwrap_or_default();
            sessions
                .into_iter()
                .filter(|s| s.status.to_string() == "active")
                .map(|s| SessionInfo {
                    name: s.name,
                    status: s.status.to_string(),
                    workspace_path: s.workspace_path,
                    zellij_tab: s.zellij_tab,
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

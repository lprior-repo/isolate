//! System state detection and gathering
//!
//! This module provides functionality to gather current system state,
//! including initialization status, repository info, and session statistics.

use zjj_core::introspection::SystemState;

use crate::{
    cli::is_jj_repo,
    commands::{get_session_db, zjj_data_dir},
};

/// Get session statistics from the database
///
/// Returns (`total_sessions`, `active_sessions`) tuple.
/// Returns (0, 0) if database access fails.
async fn get_session_stats() -> (usize, usize) {
    match get_session_db().await {
        Ok(db) => db.list(None).await.map_or((0, 0), |sessions| {
            let total = sessions.len();
            let active = sessions
                .iter()
                .filter(|s| s.status.to_string() == "active")
                .count();
            (total, active)
        }),
        Err(_) => (0, 0),
    }
}

/// Get paths for config and database files
///
/// Returns (`config_path`, `db_path`) tuple of optional strings.
fn get_data_paths() -> (Option<String>, Option<String>) {
    zjj_data_dir().ok().map_or((None, None), |data_dir| {
        let config = Some(data_dir.join("config.toml").display().to_string());
        let db = Some(data_dir.join("sessions.db").display().to_string());
        (config, db)
    })
}

/// Get current system state
///
/// Gathers information about:
/// - Whether zjj is initialized
/// - Whether we're in a JJ repository
/// - Config and database file paths
/// - Session statistics (total and active counts)
pub async fn get_system_state() -> SystemState {
    let jj_repo = is_jj_repo().unwrap_or(false);
    let initialized = zjj_data_dir().is_ok();

    let (config_path, state_db, sessions_count, active_sessions) = if initialized {
        let (config, db) = get_data_paths();
        let (count, active) = get_session_stats().await;
        (config, db, count, active)
    } else {
        (None, None, 0, 0)
    };

    SystemState {
        initialized,
        jj_repo,
        config_path,
        state_db,
        sessions_count,
        active_sessions,
    }
}

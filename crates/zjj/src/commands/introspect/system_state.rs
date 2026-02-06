use zjj_core::introspection::SystemState;

use crate::{
    cli::is_jj_repo,
    commands::{get_session_db, zjj_data_dir},
};

/// Get current system state
pub(super) async fn get_system_state() -> SystemState {
    let jj_repo = is_jj_repo().await.unwrap_or(false);
    let initialized = zjj_data_dir().await.is_ok();

    let (config_path, state_db, sessions_count, active_sessions) = if initialized {
        let data_dir = zjj_data_dir().await.ok();
        let config = data_dir
            .as_ref()
            .map(|d| d.join("config.toml").display().to_string());
        let db = data_dir
            .as_ref()
            .map(|d| d.join("state.db").display().to_string());

        let (count, active) = if let Ok(db) = get_session_db().await {
            if let Ok(sessions) = db.list(None).await {
                let total = sessions.len();
                let active = sessions
                    .iter()
                    .filter(|s| s.status.to_string() == "active")
                    .count();
                (total, active)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

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

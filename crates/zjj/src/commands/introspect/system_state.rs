use zjj_core::introspection::SystemState;

use crate::{
    cli::is_jj_repo,
    commands::{get_session_db, zjj_data_dir},
};

/// Get current system state
pub(super) fn get_system_state() -> SystemState {
    let jj_repo = is_jj_repo().unwrap_or(false);
    let initialized = zjj_data_dir().is_ok();

    let (config_path, state_db, sessions_count, active_sessions) = if initialized {
        let data_dir = zjj_data_dir().ok();
        let config = data_dir
            .as_ref()
            .map(|d| d.join("config.toml").display().to_string());
        let db = data_dir
            .as_ref()
            .map(|d| d.join("state.db").display().to_string());

        let (count, active) = get_session_db()
            .ok()
            .and_then(|db| {
                db.list_blocking(None).ok().map(|sessions| {
                    let total = sessions.len();
                    let active = sessions
                        .iter()
                        .filter(|s| s.status.to_string() == "active")
                        .count();
                    (total, active)
                })
            })
            .unwrap_or((0, 0));

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

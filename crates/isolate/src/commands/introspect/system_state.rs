use isolate_core::introspection::SystemState;

use crate::{
    cli::is_jj_repo,
    commands::{get_session_db, isolate_data_dir},
};

/// Get current system state
pub(super) async fn get_system_state() -> SystemState {
    let jj_repo = is_jj_repo().await.unwrap_or(false);
    let initialized = isolate_data_dir().await.is_ok();

    let (config_path, state_db, sessions_count, active_sessions) = if initialized {
        let data_dir = isolate_data_dir().await.ok();
        let config = data_dir
            .as_ref()
            .map(|d| d.join("config.toml").display().to_string());
        let db = data_dir
            .as_ref()
            .map(|d| d.join("state.db").display().to_string());

        let sessions_result: Option<Vec<_>> = match get_session_db().await {
            Ok(db) => db.list(None).await.ok(),
            Err(_) => None,
        };

        let (count, active) = sessions_result
            .map(|sessions| {
                let total = sessions.len();
                let active = sessions
                    .iter()
                    .filter(|s| s.status.to_string() == "active")
                    .count();
                (total, active)
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

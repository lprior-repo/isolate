//! Types for the init command

use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
pub(super) struct InitResponse {
    pub message: String,
    pub root: String,
    pub paths: InitPaths,
    pub jj_initialized: bool,
    pub already_initialized: bool,
}

#[derive(Serialize)]
pub(super) struct InitPaths {
    pub data_directory: String,
    pub config: String,
    pub state_db: String,
    pub layouts: String,
}

pub(super) fn build_init_response(root: &Path, already_initialized: bool) -> InitResponse {
    InitResponse {
        message: if already_initialized {
            "isolate already initialized in this repository.".to_string()
        } else {
            format!("Initialized isolate in {}", root.display())
        },
        root: root.display().to_string(),
        paths: InitPaths {
            data_directory: ".isolate/".to_string(),
            config: ".isolate/config.toml".to_string(),
            state_db: ".isolate/state.db".to_string(),
            layouts: ".isolate/layouts/".to_string(),
        },
        jj_initialized: true,
        already_initialized,
    }
}

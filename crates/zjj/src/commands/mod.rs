//! Command implementations

pub mod add;
pub mod focus;
pub mod init;
pub mod list;
pub mod remove;
pub mod status;
pub mod sync;

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{cli::jj_root, db::SessionDb};

/// Get the ZJJ data directory for the current repository
pub fn zjj_data_dir() -> Result<PathBuf> {
    let root = jj_root()?;
    Ok(PathBuf::from(root).join(".jjz"))
}

/// Get the session database for the current repository
pub fn get_session_db() -> Result<SessionDb> {
    let data_dir = zjj_data_dir()?;

    anyhow::ensure!(
        data_dir.exists(),
        "ZJJ not initialized. Run 'jjz init' first."
    );

    let db_path = data_dir.join("sessions.db");
    SessionDb::open(&db_path).context("Failed to open session database")
}

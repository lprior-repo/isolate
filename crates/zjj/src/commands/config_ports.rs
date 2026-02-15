//! Config ports and default local adapter.
//!
//! This module defines the seam for configuration loading and path resolution.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{future::Future, path::PathBuf, pin::Pin};

use anyhow::{Context, Result};
use zjj_core::config::Config;

pub type ConfigLoadFuture<'a> = Pin<Box<dyn Future<Output = zjj_core::Result<Config>> + Send + 'a>>;

/// Port for configuration reads and scope path resolution.
pub trait ConfigReadPort: Send + Sync {
    /// Load merged configuration (defaults + global + project + env).
    fn load_merged(&self) -> ConfigLoadFuture<'_>;

    /// Load global-only configuration (defaults + global).
    fn load_global_only(&self) -> ConfigLoadFuture<'_>;

    /// Return global config path.
    fn global_config_path(&self) -> zjj_core::Result<PathBuf>;

    /// Return project config path.
    fn project_config_path(&self) -> zjj_core::Result<PathBuf>;
}

/// Default local filesystem-based config adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalConfigPort;

impl ConfigReadPort for LocalConfigPort {
    fn load_merged(&self) -> ConfigLoadFuture<'_> {
        Box::pin(async { zjj_core::config::load_config().await })
    }

    fn load_global_only(&self) -> ConfigLoadFuture<'_> {
        Box::pin(async {
            let mut config = Config::default();
            let global_path = self.global_config_path()?;
            match zjj_core::config::load_partial_toml_file(&global_path).await {
                Ok(global_partial) => config.merge_partial(global_partial),
                Err(zjj_core::Error::IoError(_)) => {}
                Err(err) => return Err(err),
            }
            Ok(config)
        })
    }

    fn global_config_path(&self) -> zjj_core::Result<PathBuf> {
        directories::ProjectDirs::from("", "", "zjj")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
            .ok_or_else(|| {
                zjj_core::Error::IoError("Failed to determine global config directory".to_string())
            })
    }

    fn project_config_path(&self) -> zjj_core::Result<PathBuf> {
        std::env::current_dir()
            .map(|dir| dir.join(".zjj/config.toml"))
            .map_err(|err| {
                zjj_core::Error::IoError(format!("Failed to get current directory: {err}"))
            })
    }
}

/// Resolve the state database path using env + config + defaults.
///
/// Priority: env `ZJJ_STATE_DB` > config `state_db` > default `.zjj/state.db`.
pub async fn resolve_state_db_path(
    port: &impl ConfigReadPort,
    repo_root: PathBuf,
) -> Result<PathBuf> {
    if let Ok(env_db) = std::env::var("ZJJ_STATE_DB") {
        return Ok(PathBuf::from(env_db));
    }

    if let Ok(cfg) = port.load_merged().await {
        if cfg.state_db != ".zjj/state.db" {
            let configured = PathBuf::from(cfg.state_db);
            return Ok(if configured.is_absolute() {
                configured
            } else {
                repo_root.join(configured)
            });
        }
    }

    let data_dir = repo_root.join(".zjj");
    let path = data_dir.join("state.db");
    path.canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(path.clone()))
        .with_context(|| format!("Failed to canonicalize state db path {}", path.display()))
}

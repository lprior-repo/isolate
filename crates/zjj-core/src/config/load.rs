//! Configuration loading from files and environment (Immutable functional pattern)
//!
//! This module handles loading configuration from:
//! 1. Built-in defaults
//! 2. Global config: ~/.config/zjj/config.toml
//! 3. Project config: .zjj/config.toml
//! 4. Environment variables: ZJJ_*
//!
//! All operations return new instances rather than mutating in place.

use std::path::PathBuf;

use super::types::Config;
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Load configuration from all sources with hierarchy (immutable functional pattern)
///
/// # Errors
///
/// Returns error if:
/// - Config file is malformed TOML
/// - Config values fail validation
/// - Unable to determine repository name for placeholder substitution
pub fn load_config() -> Result<Config> {
    // 1. Start with built-in defaults
    let config = Config::default();

    // 2. Load global config if exists
    let config = if let Some(global_path) = global_config_path() {
        if global_path.exists() {
            let global = load_toml_file(&global_path)?;
            config.merge(global)
        } else {
            config
        }
    } else {
        config
    };

    // 3. Load project config if exists
    let project_path = project_config_path()?;
    let config = if project_path.exists() {
        let project = load_toml_file(&project_path)?;
        config.merge(project) // Project overrides global
    } else {
        config
    };

    // 4. Apply environment variable overrides
    let config = config.apply_env_vars()?;

    // 5. Validate and substitute placeholders
    config.validate()?;
    let config = config.substitute_placeholders()?;

    Ok(config)
}

// ═══════════════════════════════════════════════════════════════════════════
// PATH HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to global config file
pub fn global_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "zjj")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
}

/// Get path to project config file
///
/// # Errors
///
/// Returns error if current directory cannot be determined
pub fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".zjj/config.toml"))
        .map_err(|e| Error::io_error(format!("Failed to get current directory: {e}")))
}

/// Load a TOML file into a partial Config
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - Path is a directory instead of a file
/// - TOML is malformed
pub fn load_toml_file(path: &std::path::Path) -> Result<Config> {
    // Check if path is a directory
    if path.is_dir() {
        return Err(Error::io_error(format!(
            "Config path is a directory, not a file: {}\n\
             \n\
             The config file path should point to a TOML file, not a directory.\n\
             Expected: .zjj/config.toml (file)\n\
             Found: {} (directory)",
            path.display(),
            path.display()
        )));
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        let err_str = e.to_string().to_lowercase();
        if err_str.contains("permission") || err_str.contains("denied") {
            Error::io_error(format!(
                "Permission denied reading config file {}: {e}\n\
                 \n\
                 Check file permissions: ls -l {}",
                path.display(),
                path.display()
            ))
        } else {
            Error::io_error(format!(
                "Failed to read config file {}: {e}",
                path.display()
            ))
        }
    })?;

    toml::from_str(&content).map_err(|e| {
        Error::parse_error(format!(
            "Failed to parse config file {}: {e}\n\
             \n\
             The config file contains invalid TOML syntax.\n\
             Please check the file for:\n\
             • Missing or extra brackets\n\
             • Unclosed quotes\n\
             • Invalid key names\n\
             \n\
             Error details: {e}",
            path.display()
        ))
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// ENVIRONMENT VARIABLE OVERRIDES (Immutable pattern)
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Apply environment variable overrides - immutable pattern
    ///
    /// # Errors
    ///
    /// Returns error if environment variable values are invalid
    pub fn apply_env_vars(mut self) -> Result<Self> {
        // ZJJ_WORKSPACE_DIR
        if let Ok(value) = std::env::var("ZJJ_WORKSPACE_DIR") {
            self.workspace_dir = value;
        }

        // ZJJ_MAIN_BRANCH
        if let Ok(value) = std::env::var("ZJJ_MAIN_BRANCH") {
            if value.is_empty() {
                return Err(Error::invalid_config(
                    "ZJJ_MAIN_BRANCH cannot be empty - unset the variable or provide a branch name",
                ));
            }
            self.main_branch = Some(value);
        }

        // ZJJ_DEFAULT_TEMPLATE
        if let Ok(value) = std::env::var("ZJJ_DEFAULT_TEMPLATE") {
            self.default_template = value;
        }

        // ZJJ_WATCH_ENABLED
        if let Ok(value) = std::env::var("ZJJ_WATCH_ENABLED") {
            self.watch.enabled = value.parse().map_err(|e| {
                Error::invalid_config(format!("Invalid ZJJ_WATCH_ENABLED value: {e}"))
            })?;
        }

        // ZJJ_WATCH_DEBOUNCE_MS
        if let Ok(value) = std::env::var("ZJJ_WATCH_DEBOUNCE_MS") {
            self.watch.debounce_ms = value.parse().map_err(|e| {
                Error::invalid_config(format!("Invalid ZJJ_WATCH_DEBOUNCE_MS value: {e}"))
            })?;
        }

        // ZJJ_ZELLIJ_USE_TABS
        if let Ok(value) = std::env::var("ZJJ_ZELLIJ_USE_TABS") {
            self.zellij.use_tabs = value.parse().map_err(|e| {
                Error::invalid_config(format!("Invalid ZJJ_ZELLIJ_USE_TABS value: {e}"))
            })?;
        }

        // ZJJ_DASHBOARD_REFRESH_MS
        if let Ok(value) = std::env::var("ZJJ_DASHBOARD_REFRESH_MS") {
            self.dashboard.refresh_ms = value.parse().map_err(|e| {
                Error::invalid_config(format!("Invalid ZJJ_DASHBOARD_REFRESH_MS value: {e}"))
            })?;
        }

        // ZJJ_DASHBOARD_VIM_KEYS
        if let Ok(value) = std::env::var("ZJJ_DASHBOARD_VIM_KEYS") {
            self.dashboard.vim_keys = value.parse().map_err(|e| {
                Error::invalid_config(format!("Invalid ZJJ_DASHBOARD_VIM_KEYS value: {e}"))
            })?;
        }

        // ZJJ_AGENT_COMMAND
        if let Ok(value) = std::env::var("ZJJ_AGENT_COMMAND") {
            self.agent.command = value;
        }

        Ok(self)
    }
}

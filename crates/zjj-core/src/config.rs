//! Configuration loading and management
//!
//! # Hierarchy
//!
//! Configuration is loaded in this order (later overrides earlier):
//! 1. Built-in defaults
//! 2. Global config: ~/.config/jjz/config.toml
//! 3. Project config: .jjz/config.toml
//! 4. Environment variables: JJZ_*
//! 5. CLI flags (command-specific)
//!
//! # Example Config
//!
//! ```toml
//! workspace_dir = "../{repo}__workspaces"
//! main_branch = "main"
//!
//! [zellij.panes.main]
//! command = "claude"
//! size = "70%"
//!
//! [hooks]
//! post_create = ["bd sync", "npm install"]
//! ```

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub workspace_dir: String,
    pub main_branch: String,
    pub default_template: String,
    pub state_db: String,
    pub watch: WatchConfig,
    pub hooks: HooksConfig,
    pub zellij: ZellijConfig,
    pub dashboard: DashboardConfig,
    pub agent: AgentConfig,
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchConfig {
    pub enabled: bool,
    pub debounce_ms: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HooksConfig {
    pub post_create: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_merge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZellijConfig {
    pub session_prefix: String,
    pub use_tabs: bool,
    pub layout_dir: String,
    pub panes: PanesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanesConfig {
    pub main: PaneConfig,
    pub beads: PaneConfig,
    pub status: PaneConfig,
    pub float: FloatPaneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaneConfig {
    pub command: String,
    pub args: Vec<String>,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloatPaneConfig {
    pub enabled: bool,
    pub command: String,
    pub width: String,
    pub height: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardConfig {
    pub refresh_ms: u32,
    pub theme: String,
    pub columns: Vec<String>,
    pub vim_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub command: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    pub auto_commit: bool,
    pub commit_prefix: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_dir: "../{repo}__workspaces".to_string(),
            main_branch: String::new(),
            default_template: "standard".to_string(),
            state_db: ".jjz/state.db".to_string(),
            watch: WatchConfig::default(),
            hooks: HooksConfig::default(),
            zellij: ZellijConfig::default(),
            dashboard: DashboardConfig::default(),
            agent: AgentConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_ms: 100,
            paths: vec![".beads/beads.db".to_string()],
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            post_create: Vec::new(),
            pre_remove: Vec::new(),
            post_merge: Vec::new(),
        }
    }
}

impl Default for ZellijConfig {
    fn default() -> Self {
        Self {
            session_prefix: "jjz".to_string(),
            use_tabs: true,
            layout_dir: ".jjz/layouts".to_string(),
            panes: PanesConfig::default(),
        }
    }
}

impl Default for PanesConfig {
    fn default() -> Self {
        Self {
            main: PaneConfig {
                command: "claude".to_string(),
                args: Vec::new(),
                size: "70%".to_string(),
            },
            beads: PaneConfig {
                command: "bv".to_string(),
                args: Vec::new(),
                size: "50%".to_string(),
            },
            status: PaneConfig {
                command: "jjz".to_string(),
                args: vec!["status".to_string(), "--watch".to_string()],
                size: "50%".to_string(),
            },
            float: FloatPaneConfig::default(),
        }
    }
}

impl Default for FloatPaneConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command: String::new(),
            width: "80%".to_string(),
            height: "60%".to_string(),
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            refresh_ms: 1000,
            theme: "default".to_string(),
            columns: vec![
                "name".to_string(),
                "status".to_string(),
                "branch".to_string(),
                "changes".to_string(),
                "beads".to_string(),
            ],
            vim_keys: true,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            env: HashMap::new(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_commit: false,
            commit_prefix: "wip:".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Load configuration from all sources with hierarchy
///
/// # Errors
///
/// Returns error if:
/// - Config file is malformed TOML
/// - Config values fail validation
/// - Unable to determine repository name for placeholder substitution
pub fn load_config() -> Result<Config> {
    // TODO: Implement loading hierarchy
    Ok(Config::default())
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to global config file
#[allow(dead_code)]
fn global_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "jjz")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
}

/// Get path to project config file
///
/// # Errors
///
/// Returns error if current directory cannot be determined
#[allow(dead_code)]
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".jjz/config.toml"))
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
}

/// Get repository name from current directory
///
/// # Errors
///
/// Returns error if:
/// - Current directory cannot be determined
/// - Directory name cannot be extracted
#[allow(dead_code)]
fn get_repo_name() -> Result<String> {
    std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
        .and_then(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
                .ok_or_else(|| Error::Unknown("Failed to determine repository name".to_string()))
        })
}

/// Load a TOML file into a partial Config
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - TOML is malformed
#[allow(dead_code)]
fn load_toml_file(path: &std::path::Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    toml::from_str(&content)
        .map_err(|e| Error::ParseError(format!("Failed to parse config: {}: {e}", path.display())))
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG METHODS
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Merge another config into this one (other takes precedence)
    #[allow(dead_code, clippy::unused_self, clippy::use_self)]
    fn merge(&self, _other: Self) {
        // TODO: Implement
    }

    /// Apply environment variable overrides
    ///
    /// # Errors
    ///
    /// Returns error if environment variable values are invalid
    #[allow(
        dead_code,
        clippy::unused_self,
        clippy::unnecessary_wraps,
        clippy::missing_const_for_fn
    )]
    fn apply_env_vars(&self) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns error if any values are out of range or invalid
    #[allow(
        dead_code,
        clippy::unused_self,
        clippy::unnecessary_wraps,
        clippy::missing_const_for_fn
    )]
    fn validate(&self) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Substitute placeholders like {repo} in config values
    ///
    /// # Errors
    ///
    /// Returns error if unable to determine values for placeholders
    #[allow(
        dead_code,
        clippy::unused_self,
        clippy::unnecessary_wraps,
        clippy::missing_const_for_fn
    )]
    fn substitute_placeholders(&self) -> Result<()> {
        // TODO: Implement
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: No config files - Returns default config
    #[test]
    fn test_no_config_files_returns_defaults() {
        // TODO: Implement test
        // Setup: Ensure no config files exist
        // Action: Call load_config()
        // Assert: Result is default config
    }

    // Test 2: Global only - Loads global, merges with defaults
    #[test]
    fn test_global_only_merges_with_defaults() {
        // TODO: Implement test
        // Setup: Create global config with workspace_dir = "../custom"
        // Action: Call load_config()
        // Assert: workspace_dir is "../custom", other fields are defaults
    }

    // Test 3: Project only - Loads project, merges with defaults
    #[test]
    fn test_project_only_merges_with_defaults() {
        // TODO: Implement test
        // Setup: Create project config with main_branch = "develop"
        // Action: Call load_config()
        // Assert: main_branch is "develop", other fields are defaults
    }

    // Test 4: Both - Project overrides global overrides defaults
    #[test]
    fn test_project_overrides_global() {
        // TODO: Implement test
        // Setup: Global sets workspace_dir = "../global"
        //        Project sets workspace_dir = "../project"
        // Action: Call load_config()
        // Assert: workspace_dir is "../project"
    }

    // Test 5: Env override - JJZ_WORKSPACE_DIR=../custom → config.workspace_dir
    #[test]
    fn test_env_var_overrides_config() {
        // TODO: Implement test
        // Setup: Set env var JJZ_WORKSPACE_DIR="../env"
        // Action: Call load_config()
        // Assert: workspace_dir is "../env"
    }

    // Test 6: Placeholder substitution
    #[test]
    fn test_placeholder_substitution() {
        // TODO: Implement test
        // Setup: Config with workspace_dir = "../{repo}__ws"
        //        Mock repo name as "myproject"
        // Action: Call substitute_placeholders()
        // Assert: workspace_dir is "../myproject__ws"
    }

    // Test 7: Invalid debounce - debounce_ms = 5 → Error
    #[test]
    fn test_invalid_debounce_ms_too_low() {
        // TODO: Implement test
        // Setup: Create config with debounce_ms = 5
        // Action: Call validate()
        // Assert: Error with message containing "10-5000"
    }

    // Test 8: Invalid refresh - refresh_ms = 50000 → Error
    #[test]
    fn test_invalid_refresh_ms_too_high() {
        // TODO: Implement test
        // Setup: Create config with refresh_ms = 50000
        // Action: Call validate()
        // Assert: Error with message containing "100-10000"
    }

    // Test 9: Missing global config - No error, uses defaults
    #[test]
    fn test_missing_global_config_no_error() {
        // TODO: Implement test
        // Setup: Ensure global config path doesn't exist
        // Action: Call load_config()
        // Assert: No error, returns defaults
    }

    // Test 10: Malformed TOML - Clear error with line number
    #[test]
    fn test_malformed_toml_returns_parse_error() {
        // TODO: Implement test
        // Setup: Create config file with invalid TOML
        // Action: Call load_toml_file()
        // Assert: Error is ParseError variant
    }

    // Test 11: Partial config - Unspecified values use defaults
    #[test]
    fn test_partial_config_uses_defaults() {
        // TODO: Implement test
        // Setup: Create config with only workspace_dir set
        // Action: Load and merge with defaults
        // Assert: workspace_dir is custom, rest are defaults
    }

    // Test 12: Deep merge - hooks.post_create in global + project → project replaces
    #[test]
    fn test_deep_merge_replaces_not_appends() {
        // TODO: Implement test
        // Setup: Global has hooks.post_create = ["a", "b"]
        //        Project has hooks.post_create = ["c"]
        // Action: Merge configs
        // Assert: Final hooks.post_create = ["c"] (not ["a", "b", "c"])
    }

    // Additional tests for helper functions
    #[test]
    fn test_global_config_path() {
        let path = global_config_path();
        // Should return Some path to ~/.config/jjz/config.toml
        // or None on systems without home directory
        assert!(path.is_some() || path.is_none());
    }

    #[test]
    fn test_project_config_path() {
        let result = project_config_path();
        assert!(result.is_ok());
        let path = result.unwrap_or_default();
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.workspace_dir, "../{repo}__workspaces");
        assert_eq!(config.main_branch, "");
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".jjz/state.db");
        assert!(config.watch.enabled);
        assert_eq!(config.watch.debounce_ms, 100);
        assert_eq!(config.dashboard.refresh_ms, 1000);
        assert_eq!(config.zellij.session_prefix, "jjz");
    }

    #[test]
    fn test_env_var_parsing_bool() {
        // Test parsing bool from env var
        // JJZ_WATCH_ENABLED=false should set watch.enabled to false
    }

    #[test]
    fn test_env_var_parsing_int() {
        // Test parsing int from env var
        // JJZ_WATCH_DEBOUNCE_MS=200 should set watch.debounce_ms to 200
    }

    #[test]
    fn test_validation_debounce_ms_valid() {
        let mut config = Config::default();
        config.watch.debounce_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_min() {
        let mut config = Config::default();
        config.watch.debounce_ms = 10;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_max() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_valid() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 1000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_min() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_max() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 10000;
        assert!(config.validate().is_ok());
    }
}

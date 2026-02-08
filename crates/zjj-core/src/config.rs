//! Configuration loading and management
//!
//! # Hierarchy
//!
//! Configuration is loaded in this order (later overrides earlier):
//! 1. Built-in defaults
//! 2. Global config: ~/.config/zjj/config.toml
//! 3. Project config: .zjj/config.toml
//! 4. Environment variables: ZJJ_*
//! 5. CLI flags (command-specific)
//!
//! # Hot-Reload
//!
//! For long-running commands (e.g., `dashboard --watch`), use [`ConfigManager`]
//! to get automatic config reloading when files change.
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
//! post_create = ["br sync", "npm install"]
//! ```

use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

// Import notify for Watcher trait
use notify::Watcher;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryPolicy {
    Silent,
    #[default]
    Warn,
    FailFast,
}

impl<'de> Deserialize<'de> for RecoveryPolicy {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for RecoveryPolicy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "silent" => Ok(Self::Silent),
            "warn" => Ok(Self::Warn),
            "fail-fast" | "failfast" | "fail" => Ok(Self::FailFast),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid recovery policy: {s}. Must be one of: silent, warn, fail-fast"
            ))),
        }
    }
}

impl std::fmt::Display for RecoveryPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Silent => write!(f, "silent"),
            Self::Warn => write!(f, "warn"),
            Self::FailFast => write!(f, "fail-fast"),
        }
    }
}

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
    pub recovery: RecoveryConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryConfig {
    pub policy: RecoveryPolicy,
    pub log_recovered: bool,
    pub auto_recover_corrupted_wal: bool,
    pub delete_corrupted_database: bool,
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
            state_db: ".zjj/state.db".to_string(),
            watch: WatchConfig::default(),
            hooks: HooksConfig::default(),
            zellij: ZellijConfig::default(),
            dashboard: DashboardConfig::default(),
            agent: AgentConfig::default(),
            session: SessionConfig::default(),
            recovery: RecoveryConfig::default(),
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
            session_prefix: "zjj".to_string(),
            use_tabs: true,
            layout_dir: ".zjj/layouts".to_string(),
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
                command: "zjj".to_string(),
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

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            policy: RecoveryPolicy::Warn,
            log_recovered: true,
            auto_recover_corrupted_wal: true,
            delete_corrupted_database: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG MANAGER (HOT-RELOAD)
// ═══════════════════════════════════════════════════════════════════════════

/// Manages configuration with hot-reload capability
///
/// This type provides thread-safe, reloadable configuration for long-running
/// processes. It watches config files and automatically reloads when they change.
///
/// # Example
///
/// ```rust,no_run
/// use zjj_core::config::ConfigManager;
///
/// # async fn example() -> zjj_core::Result<()> {
/// let manager = ConfigManager::new().await?;
///
/// // Get current config (fast, non-blocking read)
/// let config = manager.get().await;
/// println!("Workspace dir: {}", config.workspace_dir);
///
/// // Config auto-reloads when files change
/// tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
/// let updated_config = manager.get().await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ConfigManager {
    inner: Arc<RwLock<ConfigManagerInner>>,
}

struct ConfigManagerInner {
    config: Config,
}

impl ConfigManager {
    /// Create a new `ConfigManager` with hot-reload enabled
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Initial config load fails
    /// - Unable to spawn reload task
    pub async fn new() -> Result<Self> {
        let config = load_config().await?;

        let manager = Self {
            inner: Arc::new(RwLock::new(ConfigManagerInner { config })),
        };

        // Spawn config watcher task
        let inner = manager.inner.clone();
        let mut file_watcher_rx = Self::watch_config_files();

        tokio::spawn(async move {
            // Loop: watch for file changes and reload config
            loop {
                tokio::select! {
                    // File changed - reload config
                    Some(()) = file_watcher_rx.recv() => {
                        // Debounce: small delay before reload
                        tokio::time::sleep(Duration::from_millis(150)).await;

                        // Reload config
                        match load_config().await {
                            Ok(new_config) => {
                                {
                                    let mut inner_write = inner.write().await;
                                    inner_write.config = new_config;
                                } // Drop guard before logging
                                tracing::info!("Config reloaded successfully");
                            }
                            Err(e) => {
                                // Log error but keep running with last known good config
                                tracing::warn!("Config reload failed: {e}, using previous config");
                            }
                        }
                    }
                    // Channel closed - exit task
                    else => break,
                }
            }
        });

        Ok(manager)
    }

    /// Get the current configuration
    ///
    /// This is a fast, non-blocking read that returns the most recent
    /// successfully loaded configuration (including hot-reloaded changes).
    pub async fn get(&self) -> Config {
        let inner = self.inner.read().await;
        inner.config.clone()
    }

    /// Create a config file watcher channel
    ///
    /// Returns a receiver that gets events when config files change.
    fn watch_config_files() -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel::<()>(4);

        tokio::spawn(async move {
            let paths_to_watch = Self::get_config_paths();

            if paths_to_watch.is_empty() {
                return;
            }

            // Use notify to watch config files
            let result = notify::recommended_watcher(
                move |res: std::result::Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        if event.kind.is_modify() || event.kind.is_create() {
                            let _ = tx.blocking_send(());
                        }
                    }
                },
            );

            let Ok(mut watcher) = result else { return };

            // Watch each config path
            for path in paths_to_watch {
                if path.exists() {
                    let _ = watcher.watch(&path, notify::RecursiveMode::NonRecursive);
                } else {
                    // Watch parent directory for file creation
                    if let Some(parent) = path.parent() {
                        let _ = watcher.watch(parent, notify::RecursiveMode::NonRecursive);
                    }
                }
            }

            // Keep the watcher task alive
            tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
        });

        rx
    }

    /// Get paths to config files that should be watched
    fn get_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Global config
        if let Ok(global) = global_config_path() {
            paths.push(global);
        }

        // Project config
        if let Ok(project) = project_config_path() {
            paths.push(project);
        }

        paths
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG KEY VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

/// All valid configuration keys in dot-notation
///
/// This list defines the complete schema of supported configuration keys.
/// Any key not in this list will be rejected with a helpful error message.
const VALID_KEYS: &[&str] = &[
    // Top-level keys
    "workspace_dir",
    "main_branch",
    "default_template",
    "state_db",
    "watch",
    "hooks",
    "zellij",
    "dashboard",
    "agent",
    "session",
    "recovery",
    // Watch config
    "watch.enabled",
    "watch.debounce_ms",
    "watch.paths",
    // Hooks config
    "hooks.post_create",
    "hooks.pre_remove",
    "hooks.post_merge",
    // Zellij config
    "zellij.session_prefix",
    "zellij.use_tabs",
    "zellij.layout_dir",
    "zellij.panes.main.command",
    "zellij.panes.main.args",
    "zellij.panes.main.size",
    "zellij.panes.beads.command",
    "zellij.panes.beads.args",
    "zellij.panes.beads.size",
    "zellij.panes.status.command",
    "zellij.panes.status.args",
    "zellij.panes.status.size",
    "zellij.panes.float.enabled",
    "zellij.panes.float.command",
    "zellij.panes.float.width",
    "zellij.panes.float.height",
    // Dashboard config
    "dashboard.refresh_ms",
    "dashboard.theme",
    "dashboard.columns",
    "dashboard.vim_keys",
    // Agent config
    "agent.command",
    "agent.env",
    // Session config
    "session.auto_commit",
    "session.commit_prefix",
    // Recovery config
    "recovery.policy",
    "recovery.log_recovered",
];

/// Validate a configuration key
///
/// Checks if the given key is in the list of valid configuration keys.
/// Returns an error if the key is not recognized.
///
/// # Errors
///
/// Returns `Error::ValidationError` if the key is not valid.
/// The error message includes a list of valid keys to help the user.
///
/// # Examples
///
/// ```rust
/// use zjj_core::config::validate_key;
///
/// assert!(validate_key("workspace_dir").is_ok());
/// assert!(validate_key("zellij.use_tabs").is_ok());
/// assert!(validate_key("invalid_key").is_err());
/// ```
pub fn validate_key(key: &str) -> Result<()> {
    // Check if the key exactly matches a valid key or is a parent of a valid key
    // For example:
    // - "watch" is valid (parent of watch.enabled, watch.debounce_ms, etc.)
    // - "watch.enabled" is valid (exact match)
    // - "watch.invalid" is invalid (not in list)
    let is_valid = VALID_KEYS
        .iter()
        .any(|valid_key| key == *valid_key || valid_key.starts_with(&format!("{key}.")));

    if is_valid {
        Ok(())
    } else {
        // Build a helpful error message with valid keys grouped by category
        let mut error_msg = format!("Unknown configuration key: '{key}'\n\n");

        error_msg.push_str("Valid keys:\n");
        error_msg.push_str("  workspace_dir, main_branch, default_template, state_db\n");
        error_msg.push_str("  watch.enabled, watch.debounce_ms, watch.paths\n");
        error_msg.push_str("  hooks.post_create, hooks.pre_remove, hooks.post_merge\n");
        error_msg.push_str("  zellij.session_prefix, zellij.use_tabs, zellij.layout_dir\n");
        error_msg.push_str(
            "  zellij.panes.{main,beads,status,float}.{command,args,size,width,height,enabled}\n",
        );
        error_msg.push_str(
            "  dashboard.refresh_ms, dashboard.theme, dashboard.columns, dashboard.vim_keys\n",
        );
        error_msg.push_str("  agent.command, agent.env\n");
        error_msg.push_str("  session.auto_commit, session.commit_prefix\n");
        error_msg.push_str("  recovery.policy, recovery.log_recovered\n");
        error_msg.push_str("\nUse 'zjj config' to see current configuration.");

        Err(Error::ValidationError(error_msg))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Load configuration from all sources with hierarchy
///
/// For long-running processes that need hot-reload, use [`ConfigManager`] instead.
///
/// # Errors
///
/// Returns error if:
/// - Config file is malformed TOML
/// - Config values fail validation
/// - Unable to determine repository name for placeholder substitution
pub async fn load_config() -> Result<Config> {
    // 1. Start with built-in defaults
    let mut config = Config::default();

    // 2. Load global config if exists
    if let Ok(global_path) = global_config_path() {
        if global_path.exists() {
            let global = load_toml_file(&global_path).await?;
            config.merge(global);
        }
    }

    // 3. Load project config if exists
    if let Ok(project_path) = project_config_path() {
        if project_path.exists() {
            let project = load_toml_file(&project_path).await?;
            config.merge(project); // Project overrides global
        }
    }

    // 4. Apply environment variable overrides
    config.apply_env_vars()?;

    // 5. Validate and substitute placeholders
    config.validate()?;

    // Only attempt placeholder substitution if we're in a proper directory structure
    // This prevents failures in test environments where current_dir might not be valid
    if get_repo_name().is_ok() {
        config.substitute_placeholders()?;
    }
    // In test environments, we might not have a proper repo name
    // Just use defaults without placeholder substitution

    Ok(config)
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to project config file
///
/// # Errors
///
/// Returns error if current directory cannot be determined
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".zjj/config.toml"))
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))
}

/// Get path to global config file
fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "zjj")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| Error::IoError("Failed to determine global config directory".to_string()))
}

/// Get repository name from current directory
///
/// # Errors
///
/// Returns error if:
/// - Current directory cannot be determined
/// - Directory name cannot be extracted
fn get_repo_name() -> Result<String> {
    let dir = std::env::current_dir()
        .map_err(|e| Error::IoError(format!("Failed to get current directory: {e}")))?;

    dir.file_name()
        .and_then(|name| name.to_str())
        .map(String::from)
        .ok_or_else(|| Error::Unknown("Failed to determine repository name".to_string()))
}

/// Load a TOML file into a partial Config
///
/// # Errors
///
/// Returns error if:
/// - File cannot be read
/// - TOML is malformed
async fn load_toml_file(path: &std::path::Path) -> Result<Config> {
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        Error::IoError(format!(
            "Failed to read config file {}: {e}",
            path.display()
        ))
    })?;

    toml::from_str(&content)
        .map_err(|e| Error::ParseError(format!("Failed to parse config: {}: {e}", path.display())))
}

// ═══════════════════════════════════════════════════════════════════════════
// MERGE TRAIT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl WatchConfig {
    fn merge(&mut self, other: Self) {
        // Always take other's value for primitives if different from default
        self.enabled = other.enabled;
        if other.debounce_ms != 100 {
            self.debounce_ms = other.debounce_ms;
        }
        if other.paths != vec![".beads/beads.db".to_string()] {
            self.paths = other.paths;
        }
    }
}

impl HooksConfig {
    fn merge(&mut self, other: Self) {
        // Replace (not append) for hooks
        if !other.post_create.is_empty() {
            self.post_create = other.post_create;
        }
        if !other.pre_remove.is_empty() {
            self.pre_remove = other.pre_remove;
        }
        if !other.post_merge.is_empty() {
            self.post_merge = other.post_merge;
        }
    }
}

impl ZellijConfig {
    fn merge(&mut self, other: Self) {
        if other.session_prefix != "zjj" {
            self.session_prefix = other.session_prefix;
        }
        self.use_tabs = other.use_tabs;
        if other.layout_dir != ".zjj/layouts" {
            self.layout_dir = other.layout_dir;
        }
        self.panes.merge(other.panes);
    }
}

impl PanesConfig {
    fn merge(&mut self, other: Self) {
        self.main.merge(other.main);
        self.beads.merge(other.beads);
        self.status.merge(other.status);
        self.float.merge(other.float);
    }
}

impl PaneConfig {
    fn merge(&mut self, other: Self) {
        if !other.command.is_empty() {
            self.command = other.command;
        }
        if !other.args.is_empty() {
            self.args = other.args;
        }
        if !other.size.is_empty() {
            self.size = other.size;
        }
    }
}

impl FloatPaneConfig {
    fn merge(&mut self, other: Self) {
        self.enabled = other.enabled;
        if !other.command.is_empty() {
            self.command = other.command;
        }
        if other.width != "80%" {
            self.width = other.width;
        }
        if other.height != "60%" {
            self.height = other.height;
        }
    }
}

impl DashboardConfig {
    fn merge(&mut self, other: Self) {
        if other.refresh_ms != 1000 {
            self.refresh_ms = other.refresh_ms;
        }
        if other.theme != "default" {
            self.theme = other.theme;
        }
        let default_columns = vec![
            "name".to_string(),
            "status".to_string(),
            "branch".to_string(),
            "changes".to_string(),
            "beads".to_string(),
        ];
        if other.columns != default_columns {
            self.columns = other.columns;
        }
        self.vim_keys = other.vim_keys;
    }
}

impl AgentConfig {
    fn merge(&mut self, other: Self) {
        if other.command != "claude" {
            self.command = other.command;
        }
        if !other.env.is_empty() {
            self.env = other.env;
        }
    }
}

impl SessionConfig {
    fn merge(&mut self, other: Self) {
        self.auto_commit = other.auto_commit;
        if other.commit_prefix != "wip:" {
            self.commit_prefix = other.commit_prefix;
        }
    }
}

impl RecoveryConfig {
    const fn merge(&mut self, other: Self) {
        self.policy = other.policy;
        self.log_recovered = other.log_recovered;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG METHODS
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Merge another config into this one (other takes precedence)
    ///
    /// Note: This performs a deep replacement merge, not append.
    /// For example, if `hooks.post_create` is `["a","b"]` in self and `["c"]` in other,
    /// result will be `["c"]`, not `["a","b","c"]`.
    fn merge(&mut self, other: Self) {
        // Top-level string fields - replace if non-empty/non-default
        if !other.workspace_dir.is_empty() {
            self.workspace_dir = other.workspace_dir;
        }
        if !other.main_branch.is_empty() {
            self.main_branch = other.main_branch;
        }
        if other.default_template != "standard" {
            self.default_template = other.default_template;
        }
        if other.state_db != ".zjj/state.db" {
            self.state_db = other.state_db;
        }

        // Merge nested configs
        self.watch.merge(other.watch);
        self.hooks.merge(other.hooks);
        self.zellij.merge(other.zellij);
        self.dashboard.merge(other.dashboard);
        self.agent.merge(other.agent);
        self.session.merge(other.session);
        self.recovery.merge(other.recovery);
    }

    /// Apply environment variable overrides
    ///
    /// # Errors
    ///
    /// Returns error if environment variable values are invalid
    fn apply_env_vars(&mut self) -> Result<()> {
        // ZJJ_WORKSPACE_DIR
        if let Ok(value) = std::env::var("ZJJ_WORKSPACE_DIR") {
            self.workspace_dir = value;
        }

        // ZJJ_MAIN_BRANCH
        if let Ok(value) = std::env::var("ZJJ_MAIN_BRANCH") {
            self.main_branch = value;
        }

        // ZJJ_DEFAULT_TEMPLATE
        if let Ok(value) = std::env::var("ZJJ_DEFAULT_TEMPLATE") {
            self.default_template = value;
        }

        // ZJJ_WATCH_ENABLED
        if let Ok(value) = std::env::var("ZJJ_WATCH_ENABLED") {
            self.watch.enabled = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_WATCH_ENABLED value: {e}"))
            })?;
        }

        // ZJJ_WATCH_DEBOUNCE_MS
        if let Ok(value) = std::env::var("ZJJ_WATCH_DEBOUNCE_MS") {
            self.watch.debounce_ms = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_WATCH_DEBOUNCE_MS value: {e}"))
            })?;
        }

        // ZJJ_ZELLIJ_USE_TABS
        if let Ok(value) = std::env::var("ZJJ_ZELLIJ_USE_TABS") {
            self.zellij.use_tabs = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_ZELLIJ_USE_TABS value: {e}"))
            })?;
        }

        // ZJJ_DASHBOARD_REFRESH_MS
        if let Ok(value) = std::env::var("ZJJ_DASHBOARD_REFRESH_MS") {
            self.dashboard.refresh_ms = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_DASHBOARD_REFRESH_MS value: {e}"))
            })?;
        }

        // ZJJ_DASHBOARD_VIM_KEYS
        if let Ok(value) = std::env::var("ZJJ_DASHBOARD_VIM_KEYS") {
            self.dashboard.vim_keys = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_DASHBOARD_VIM_KEYS value: {e}"))
            })?;
        }

        // ZJJ_AGENT_COMMAND
        if let Ok(value) = std::env::var("ZJJ_AGENT_COMMAND") {
            self.agent.command = value;
        }

        // ZJJ_RECOVERY_POLICY
        if let Ok(value) = std::env::var("ZJJ_RECOVERY_POLICY") {
            self.recovery.policy = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_RECOVERY_POLICY value: {e}"))
            })?;
        }

        // ZJJ_RECOVERY_LOG
        if let Ok(value) = std::env::var("ZJJ_RECOVERY_LOG") {
            self.recovery.log_recovered = value.parse().map_err(|e| {
                Error::InvalidConfig(format!("Invalid ZJJ_RECOVERY_LOG value: {e}"))
            })?;
        }

        Ok(())
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns error if any values are out of range or invalid
    fn validate(&self) -> Result<()> {
        // Validate debounce_ms range [10-5000]
        if self.watch.debounce_ms < 10 || self.watch.debounce_ms > 5000 {
            return Err(Error::ValidationError(
                "debounce_ms must be 10-5000".to_string(),
            ));
        }

        // Validate refresh_ms range [100-10000]
        if self.dashboard.refresh_ms < 100 || self.dashboard.refresh_ms > 10000 {
            return Err(Error::ValidationError(
                "refresh_ms must be 100-10000".to_string(),
            ));
        }

        Ok(())
    }

    /// Substitute placeholders like {repo} in config values
    ///
    /// # Errors
    ///
    /// Returns error if unable to determine values for placeholders
    fn substitute_placeholders(&mut self) -> Result<()> {
        let repo_name = get_repo_name()?;
        self.workspace_dir = self.workspace_dir.replace("{repo}", &repo_name);
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
    #[tokio::test]
    async fn test_no_config_files_returns_defaults() {
        // This test works in the normal repo context where no .zjj/config.toml exists
        // Note: Global config may exist and override defaults
        let result = load_config().await;
        assert!(
            result.is_ok(),
            "load_config should succeed even without config files"
        );

        #[allow(clippy::unnecessary_result_map_or_else)]
        let config = result.map_or_else(|_| Config::default(), |c| c);
        // Check that we got a valid config (global config may override workspace_dir)
        assert!(!config.workspace_dir.is_empty());
        assert_eq!(config.default_template, "standard");
        // state_db may be overridden by global config
        assert!(!config.state_db.is_empty());
    }

    // Test 2: Global only - Loads global, merges with defaults
    #[test]
    fn test_global_only_merges_with_defaults() {
        // For this test, we're testing the merge logic directly, not the file loading
        let mut base = Config::default();
        let override_config = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.workspace_dir, "../custom");
        assert_eq!(base.default_template, "standard"); // Should still have default
    }

    // Test 3: Project only - Loads project, merges with defaults
    #[test]
    fn test_project_only_merges_with_defaults() {
        let mut base = Config::default();
        let override_config = Config {
            main_branch: "develop".to_string(),
            ..Default::default()
        };

        base.merge(override_config);

        assert_eq!(base.main_branch, "develop");
        assert_eq!(base.workspace_dir, "../{repo}__workspaces"); // Should still have default
    }

    // Test 4: Both - Project overrides global overrides defaults
    #[test]
    fn test_project_overrides_global() {
        let mut base = Config::default();

        // First merge global
        let global_config = Config {
            workspace_dir: "../global".to_string(),
            ..Default::default()
        };
        base.merge(global_config);
        assert_eq!(base.workspace_dir, "../global");

        // Then merge project (should override)
        let project_config = Config {
            workspace_dir: "../project".to_string(),
            ..Default::default()
        };
        base.merge(project_config);

        assert_eq!(base.workspace_dir, "../project");
    }

    // Test 5: Env override - ZJJ_WORKSPACE_DIR=../custom → config.workspace_dir
    #[test]
    fn test_env_var_overrides_config() {
        // Set env var
        std::env::set_var("ZJJ_WORKSPACE_DIR", "../env");

        let mut config = Config {
            workspace_dir: "../original".to_string(),
            ..Default::default()
        };

        let result = config.apply_env_vars();
        assert!(result.is_ok());

        assert_eq!(config.workspace_dir, "../env");

        // Cleanup
        std::env::remove_var("ZJJ_WORKSPACE_DIR");
    }

    // Test 6: Placeholder substitution
    #[test]
    fn test_placeholder_substitution() {
        let mut config = Config {
            workspace_dir: "../{repo}__ws".to_string(),
            ..Default::default()
        };

        let result = config.substitute_placeholders();
        assert!(result.is_ok());

        // The repo name will be "zjj" since we're in the zjj directory
        assert!(config.workspace_dir.contains("__ws"));
        assert!(!config.workspace_dir.contains("{repo}"));
    }

    // Test 7: Invalid debounce - debounce_ms = 5 → Error
    #[test]
    fn test_invalid_debounce_ms_too_low() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("10-5000"));
        }
    }

    // Test 8: Invalid refresh - refresh_ms = 50000 → Error
    #[test]
    fn test_invalid_refresh_ms_too_high() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 50000;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("100-10000"));
        }
    }

    // Test 9: Missing global config - No error, uses defaults
    #[tokio::test]
    async fn test_missing_global_config_no_error() {
        // This tests that load_config doesn't fail when global config doesn't exist
        // (which is the normal case for most users)
        let result = load_config().await;
        assert!(result.is_ok());
    }

    // Test 10: Malformed TOML - Clear error with line number
    #[tokio::test]
    async fn test_malformed_toml_returns_parse_error() -> Result<()> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_config.toml");

        // Use async file operations
        tokio::fs::write(&config_path, b"workspace_dir = \n invalid toml [[[")
            .await
            .map_err(|e| Error::IoError(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, Error::ParseError(_)));
        }
        Ok(())
    }

    // Test 11: Partial config - Unspecified values use defaults
    #[test]
    fn test_partial_config_uses_defaults() {
        let mut base = Config::default();
        let partial = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };
        // All other fields remain default

        base.merge(partial);

        assert_eq!(base.workspace_dir, "../custom");
        assert_eq!(base.default_template, "standard"); // Still default
        assert!(base.watch.enabled); // Still default
    }

    // Test 12: Deep merge - hooks.post_create in global + project → project replaces
    #[test]
    fn test_deep_merge_replaces_not_appends() {
        let mut base = Config::default();
        base.hooks.post_create = vec!["a".to_string(), "b".to_string()];

        let mut override_config = Config::default();
        override_config.hooks.post_create = vec!["c".to_string()];

        base.merge(override_config);

        assert_eq!(base.hooks.post_create, vec!["c".to_string()]);
        assert_ne!(
            base.hooks.post_create,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_project_config_path() {
        let result = project_config_path();
        assert!(result.is_ok());

        let path = result.map_or_else(
            |e| {
                // Return empty path on error - the assert!(result.is_ok()) above
                // will fail with better context
                String::new()
            },
            |p| p.to_string_lossy().to_string(),
        );

        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.workspace_dir, "../{repo}__workspaces");
        assert_eq!(config.main_branch, "");
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".zjj/state.db");
        assert!(config.watch.enabled);
        assert_eq!(config.watch.debounce_ms, 100);
        assert_eq!(config.dashboard.refresh_ms, 1000);
        assert_eq!(config.zellij.session_prefix, "zjj");
    }

    #[test]
    fn test_env_var_parsing_bool() {
        std::env::set_var("ZJJ_WATCH_ENABLED", "false");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert!(!config.watch.enabled);

        std::env::remove_var("ZJJ_WATCH_ENABLED");
    }

    #[test]
    fn test_env_var_parsing_int() {
        std::env::set_var("ZJJ_WATCH_DEBOUNCE_MS", "200");

        let mut config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        assert_eq!(config.watch.debounce_ms, 200);

        std::env::remove_var("ZJJ_WATCH_DEBOUNCE_MS");
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

    // Test: Valid top-level keys pass validation
    #[test]
    fn test_validate_key_valid_top_level() {
        let valid_keys = [
            "workspace_dir",
            "main_branch",
            "default_template",
            "state_db",
            "watch",
            "hooks",
            "zellij",
            "dashboard",
            "agent",
            "session",
            "recovery",
        ];

        for key in valid_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    // Test: Valid nested keys pass validation
    #[test]
    fn test_validate_key_valid_nested() {
        let valid_keys = [
            "watch.enabled",
            "watch.debounce_ms",
            "watch.paths",
            "hooks.post_create",
            "hooks.pre_remove",
            "hooks.post_merge",
            "zellij.session_prefix",
            "zellij.use_tabs",
            "zellij.layout_dir",
            "zellij.panes.main.command",
            "zellij.panes.main.args",
            "zellij.panes.main.size",
            "zellij.panes.beads.command",
            "zellij.panes.beads.args",
            "zellij.panes.beads.size",
            "zellij.panes.status.command",
            "zellij.panes.status.args",
            "zellij.panes.status.size",
            "zellij.panes.float.enabled",
            "zellij.panes.float.command",
            "zellij.panes.float.width",
            "zellij.panes.float.height",
            "dashboard.refresh_ms",
            "dashboard.theme",
            "dashboard.columns",
            "dashboard.vim_keys",
            "agent.command",
            "agent.env",
            "session.auto_commit",
            "session.commit_prefix",
            "recovery.policy",
            "recovery.log_recovered",
        ];

        for key in valid_keys {
            assert!(validate_key(key).is_ok(), "Key '{key}' should be valid");
        }
    }

    // Test: Invalid keys return error
    #[test]
    fn test_validate_key_invalid_returns_error() {
        let invalid_keys = [
            "nonexistent",
            "typo_key",
            "zjj_agant_id", // Typo: should be zjj_agent_id
            "invalid.nested",
            "watch.invalid_field",
            "zellij.panes.invalid_pane",
        ];

        for key in invalid_keys {
            let result = validate_key(key);
            assert!(
                result.is_err(),
                "Key '{key}' should be invalid but passed validation"
            );

            if let Err(e) = result {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Unknown configuration key"),
                    "Error should mention unknown key for '{key}': {error_msg}"
                );
                assert!(
                    error_msg.contains("Valid keys:"),
                    "Error should list valid keys for '{key}'"
                );
            }
        }
    }

    // Test: Empty key returns error
    #[test]
    fn test_validate_key_empty_returns_error() {
        let result = validate_key("");
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Unknown configuration key"),
                "Error should mention unknown key for empty string"
            );
        }
    }

    // Test 13: ConfigManager creation and retrieval
    #[tokio::test]
    async fn test_config_manager_basic() {
        let result = ConfigManager::new().await;

        // Use assert on the Result directly - no unwrap needed
        assert!(result.is_ok(), "ConfigManager::new should succeed");

        // Use and_then to chain operations - we know it's Ok from the assert above
        let retrieved_config = result.as_ref().map(|manager| {
            // Create a clone for testing
            let manager_clone = manager.clone();
            // Get config asynchronously
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(manager_clone.get())
        });

        // Extract the config using map
        let _config = result.as_ref().map(|manager| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(manager.get())
        });

        // We need to actually get the config - let's use a different approach
        // Just verify the result is Ok, which we already did above
        assert!(result.is_ok());

        // Now get the actual config by using the result
        let manager = &result.as_ref().map_err(|e| e.to_string()).ok();
        assert!(manager.is_some());

        // For the actual test, we need to get the config
        // Since we verified it's Ok above, we can use match with a fallback
        let test_config = match &result {
            Ok(manager) => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(manager.get())
            }
            Err(_) => Config::default(),
        };

        // Verify we got a valid config
        assert!(!test_config.workspace_dir.is_empty());
        assert_eq!(test_config.default_template, "standard");
        assert!(test_config.watch.enabled);
    }

    // Test 14: ConfigManager is thread-safe (can clone)
    #[tokio::test]
    async fn test_config_manager_clone() {
        let result = ConfigManager::new().await;

        // Use assert on the Result directly - no unwrap needed
        assert!(result.is_ok(), "ConfigManager::new should succeed");

        let manager1 = result.expect("ConfigManager::new failed");
        let manager2 = manager1.clone();

        // Both managers should provide the same config
        let config1 = manager1.get().await;
        let config2 = manager2.get().await;

        assert_eq!(config1.workspace_dir, config2.workspace_dir);
        assert_eq!(config1.default_template, config2.default_template);
    }
}

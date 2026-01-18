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
//!
//! # Module Structure
//!
//! - `types`: Configuration structure definitions
//! - `defaults`: Default value implementations
//! - `load`: Loading from files and environment
//! - `merge`: Configuration merging logic
//! - `validate`: Validation and placeholder substitution

// Module declarations
mod defaults;
mod load;
mod merge;
mod types;
mod validate;

// Test modules (organized by concern)
#[cfg(test)]
mod tests_defaults;
#[cfg(test)]
mod tests_loading;
#[cfg(test)]
mod tests_validation;

// Re-export public API
pub use load::{global_config_path, load_config, load_toml_file, project_config_path};
pub use types::{
    AgentConfig, Config, DashboardConfig, FloatPaneConfig, HooksConfig, PaneConfig, PanesConfig,
    SessionConfig, WatchConfig, ZellijConfig,
};

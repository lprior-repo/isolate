//! Configuration type definitions
//!
//! This module contains all configuration structures without behavior.
//! Each structure is a pure data holder with derived traits.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// MAIN CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

/// Root configuration structure
///
/// Loaded from defaults → global → project → env vars → CLI flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub workspace_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_branch: Option<String>,
    pub default_template: String,
    pub state_db: String,
    pub watch: WatchConfig,
    pub hooks: HooksConfig,
    pub zellij: ZellijConfig,
    pub dashboard: DashboardConfig,
    pub agent: AgentConfig,
    pub session: SessionConfig,
}

// ═══════════════════════════════════════════════════════════════════════════
// NESTED CONFIGURATION STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WatchConfig {
    pub enabled: bool,
    pub debounce_ms: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct HooksConfig {
    pub post_create: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_merge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ZellijConfig {
    pub session_prefix: String,
    pub use_tabs: bool,
    pub layout_dir: String,
    pub panes: PanesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PanesConfig {
    pub main: PaneConfig,
    pub beads: PaneConfig,
    pub status: PaneConfig,
    pub float: FloatPaneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PaneConfig {
    pub command: String,
    pub args: Vec<String>,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FloatPaneConfig {
    pub enabled: bool,
    pub command: String,
    pub width: String,
    pub height: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DashboardConfig {
    pub refresh_ms: u32,
    pub theme: String,
    pub columns: Vec<String>,
    pub vim_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AgentConfig {
    pub command: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SessionConfig {
    pub auto_commit: bool,
    pub commit_prefix: String,
    pub bead_auto_close: bool,
}

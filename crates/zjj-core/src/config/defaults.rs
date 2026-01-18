//! Default configuration values
//!
//! This module provides Default trait implementations for all configuration types.

use std::collections::HashMap;

use super::types::{
    AgentConfig, Config, DashboardConfig, FloatPaneConfig, HooksConfig, PaneConfig, PanesConfig,
    SessionConfig, WatchConfig, ZellijConfig,
};

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_dir: "../{repo}__workspaces".to_string(),
            main_branch: None,
            default_template: "standard".to_string(),
            state_db: ".zjj/state.db".to_string(),
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

impl Default for PaneConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            size: "50%".to_string(),
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
            bead_auto_close: true,
        }
    }
}

//! Configuration merging logic (Immutable functional pattern)
//!
//! This module handles merging configurations with proper precedence.
//! Later configs override earlier ones (defaults → global → project → env → CLI).
//!
//! All merge operations return new instances rather than mutating in place,
//! following functional programming principles.

use super::types::{
    AgentConfig, Config, DashboardConfig, FloatPaneConfig, HooksConfig, PaneConfig, PanesConfig,
    SessionConfig, WatchConfig, ZellijConfig,
};

// ═══════════════════════════════════════════════════════════════════════════
// MERGE IMPLEMENTATIONS (Immutable pattern)
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Merge another config into this one (other takes precedence) - immutable pattern
    ///
    /// Note: This performs a deep replacement merge, not append.
    /// For example, if `hooks.post_create` is `["a","b"]` in self and `["c"]` in other,
    /// the result will be `["c"]`, not `["a","b","c"]`.
    pub fn merge(self, other: Self) -> Self {
        Self {
            workspace_dir: if other.workspace_dir.is_empty() {
                self.workspace_dir
            } else {
                other.workspace_dir
            },
            main_branch: other.main_branch.or(self.main_branch),
            default_template: if other.default_template == "standard" {
                self.default_template
            } else {
                other.default_template
            },
            state_db: if other.state_db == ".jjz/state.db" {
                self.state_db
            } else {
                other.state_db
            },
            watch: self.watch.merge(other.watch),
            hooks: self.hooks.merge(other.hooks),
            zellij: self.zellij.merge(other.zellij),
            dashboard: self.dashboard.merge(other.dashboard),
            agent: self.agent.merge(other.agent),
            session: self.session.merge(other.session),
        }
    }
}

impl WatchConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            enabled: other.enabled,
            debounce_ms: if other.debounce_ms == 100 {
                self.debounce_ms
            } else {
                other.debounce_ms
            },
            paths: if other.paths == vec![".beads/beads.db".to_string()] {
                self.paths
            } else {
                other.paths
            },
        }
    }
}

impl HooksConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            post_create: if other.post_create.is_empty() {
                self.post_create
            } else {
                other.post_create
            },
            pre_remove: if other.pre_remove.is_empty() {
                self.pre_remove
            } else {
                other.pre_remove
            },
            post_merge: if other.post_merge.is_empty() {
                self.post_merge
            } else {
                other.post_merge
            },
        }
    }
}

impl ZellijConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            session_prefix: if other.session_prefix == "jjz" {
                self.session_prefix
            } else {
                other.session_prefix
            },
            use_tabs: other.use_tabs,
            layout_dir: if other.layout_dir == ".jjz/layouts" {
                self.layout_dir
            } else {
                other.layout_dir
            },
            panes: self.panes.merge(other.panes),
        }
    }
}

impl PanesConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            main: self.main.merge(other.main),
            beads: self.beads.merge(other.beads),
            status: self.status.merge(other.status),
            float: self.float.merge(other.float),
        }
    }
}

impl PaneConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            command: if other.command.is_empty() {
                self.command
            } else {
                other.command
            },
            args: if other.args.is_empty() {
                self.args
            } else {
                other.args
            },
            size: if other.size.is_empty() {
                self.size
            } else {
                other.size
            },
        }
    }
}

impl FloatPaneConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            enabled: other.enabled,
            command: if other.command.is_empty() {
                self.command
            } else {
                other.command
            },
            width: if other.width == "80%" {
                self.width
            } else {
                other.width
            },
            height: if other.height == "60%" {
                self.height
            } else {
                other.height
            },
        }
    }
}

impl DashboardConfig {
    fn merge(self, other: Self) -> Self {
        let default_columns = vec![
            "name".to_string(),
            "status".to_string(),
            "branch".to_string(),
            "changes".to_string(),
            "beads".to_string(),
        ];
        Self {
            refresh_ms: if other.refresh_ms == 1000 {
                self.refresh_ms
            } else {
                other.refresh_ms
            },
            theme: if other.theme == "default" {
                self.theme
            } else {
                other.theme
            },
            columns: if other.columns == default_columns {
                self.columns
            } else {
                other.columns
            },
            vim_keys: other.vim_keys,
        }
    }
}

impl AgentConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            command: if other.command == "claude" {
                self.command
            } else {
                other.command
            },
            env: if other.env.is_empty() {
                self.env
            } else {
                other.env
            },
        }
    }
}

impl SessionConfig {
    fn merge(self, other: Self) -> Self {
        Self {
            auto_commit: other.auto_commit,
            commit_prefix: if other.commit_prefix == "wip:" {
                self.commit_prefix
            } else {
                other.commit_prefix
            },
            bead_auto_close: other.bead_auto_close,
        }
    }
}

//! Type definitions for the context command

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete context output for the isolate environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextOutput {
    /// Current location (main or workspace)
    pub location: Location,
    /// Session information if in a workspace
    pub session: Option<SessionContext>,
    /// Repository state information
    pub repository: RepositoryContext,
    /// Beads tracking information
    pub beads: Option<BeadsContext>,
    /// Health status of the system
    pub health: HealthStatus,
    /// Actionable suggestions based on context
    pub suggestions: Vec<String>,
}

/// Location within the isolate environment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Location {
    /// On the main branch
    Main,
    /// In a workspace
    Workspace { name: String, path: String },
}

/// Session context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Session name
    pub name: String,
    /// Current status
    pub status: String,
    /// Associated bead ID if any
    pub bead_id: Option<String>,
    /// Agent ID if session is owned by an agent
    pub agent: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Last sync timestamp if any
    pub last_synced: Option<DateTime<Utc>>,
}

/// Repository context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryContext {
    /// Repository root path
    pub root: String,
    /// Current branch/change ID
    pub branch: String,
    /// Number of uncommitted files
    pub uncommitted_files: usize,
    /// Number of commits ahead of main
    pub commits_ahead: usize,
    /// Whether conflicts exist
    pub has_conflicts: bool,
}

/// Beads tracking context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsContext {
    /// Active bead ID if any
    pub active: Option<String>,
    /// List of blocking bead IDs
    pub blocked_by: Vec<String>,
    /// Number of ready beads
    pub ready_count: usize,
    /// Number of in-progress beads
    pub in_progress_count: usize,
}

/// Health status of the isolate system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HealthStatus {
    /// System is healthy
    Good,
    /// System has warnings
    Warn { issues: Vec<String> },
    /// System has critical errors
    Error { critical: Vec<String> },
}

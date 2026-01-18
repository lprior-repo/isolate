//! Output types for introspection capabilities

use im::HashMap;
use serde::{Deserialize, Serialize};

/// Complete introspection output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectOutput {
    /// Version (top-level for AI compatibility)
    pub version: String,
    /// ZJJ version (kept for backwards compatibility with zjj_version)
    pub zjj_version: String,
    /// Categorized capabilities
    pub capabilities: Capabilities,
    /// External dependency status
    pub dependencies: HashMap<String, DependencyInfo>,
    /// Current system state
    pub system_state: SystemState,
}

/// Categorized capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Session management capabilities
    pub session_management: CapabilityCategory,
    /// Configuration capabilities
    pub configuration: CapabilityCategory,
    /// Version control capabilities
    pub version_control: CapabilityCategory,
    /// Introspection and diagnostics
    pub introspection: CapabilityCategory,
}

/// A category of related capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCategory {
    /// Available commands in this category
    pub commands: Vec<String>,
    /// Feature descriptions
    pub features: Vec<String>,
}

/// Information about an external dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Whether this dependency is required for core functionality
    pub required: bool,
    /// Whether the dependency is currently installed
    pub installed: bool,
    /// Installed version if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Command name
    pub command: String,
}

/// Current system state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    /// Whether zjj has been initialized in this repo
    pub initialized: bool,
    /// Whether current directory is a JJ repository
    pub jj_repo: bool,
    /// Path to config file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Path to state database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_db: Option<String>,
    /// Total number of sessions
    pub sessions_count: usize,
    /// Number of active sessions
    pub active_sessions: usize,
}

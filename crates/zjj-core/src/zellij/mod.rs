//! Zellij layout generation and tab management
//!
//! This module provides safe, functional APIs for managing Zellij layouts and tabs.
//! All operations return `Result` and never panic.
//!
//! # Architecture
//!
//! This module follows Functional Core / Imperative Shell (FC/IS) pattern:
//!
//! - **Functional Core (fc)**: `kdl` module - pure KDL generation logic, zero I/O
//! - **Imperative Shell**: `generate` module - file I/O operations
//! - **Imperative Shell**: `tabs` module - external process operations
//! - **Configuration**: `config` module - types and builders
//!
//! # Requirements
//!
//! - REQ-ZELLIJ-001: Generate valid KDL layout files
//! - REQ-ZELLIJ-002: Use tabs within current session
//! - REQ-ZELLIJ-003: Main pane at 70% width
//! - REQ-ZELLIJ-004: Side pane for beads and status
//! - REQ-ZELLIJ-006: Open tabs via zellij action new-tab
//! - REQ-ZELLIJ-007: Close tabs via zellij action close-tab
//! - REQ-ZELLIJ-008: Focus tabs via zellij action go-to-tab-name
//! - REQ-ZELLIJ-009: Set pane cwd to workspace directory
//! - REQ-ZELLIJ-010: Support variable substitution
//! - REQ-ZELLIJ-011: Name tabs with session name
//! - REQ-ZELLIJ-012: Configure main pane command (default: claude)
//! - REQ-ZELLIJ-013: Configure beads pane command (default: bv)

pub mod config;
pub mod generate;
pub mod kdl;
pub mod tabs;

// Re-export public API for convenience

// Configuration types
pub use config::{LayoutConfig, LayoutTemplate};

// Layout information
pub use generate::Layout;

// Generation function (combines pure logic with I/O)
pub use generate::layout_generate;

// Tab operations
pub use tabs::{check_zellij_running, tab_close, tab_focus, tab_open};

// Pure KDL generation functions (for advanced users)
pub use kdl::{
    generate_full_kdl, generate_minimal_kdl, generate_review_kdl, generate_split_kdl,
    generate_standard_kdl, generate_template_kdl, validate_kdl,
};

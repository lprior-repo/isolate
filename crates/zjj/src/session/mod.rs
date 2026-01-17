//! Session management module
//!
//! This module provides types and functions for managing ZJJ sessions,
//! which represent JJ workspace + Zellij tab pairs.
//!
//! ## Module Structure
//!
//! - `status`: Session lifecycle status types
//! - `types`: Core session data structures
//! - `validation`: Session validation logic
//! - `tests`: Comprehensive test suite
//!
//! ## Agent Metadata Fields
//!
//! Sessions can track AI agents working on them through the metadata field.
//! Agent metadata is stored as JSON in the `metadata` field of a session.
//!
//! Supported agent metadata fields:
//!
//! - `agent_id`: String - Agent identifier (e.g., "claude-code-1234")
//! - `task_id`: Option<String> - Task/bead ID (e.g., "zjj-1fei")
//! - `spawned_at`: Option<u64> - Unix timestamp when agent was spawned
//! - `pid`: Option<u32> - Agent process ID
//! - `exit_code`: Option<i32> - Agent exit code after completion
//! - `artifacts_path`: Option<String> - Path to agent outputs
//!
//! Agent metadata can be stored either:
//! 1. As top-level fields in session metadata
//! 2. Nested under an "agent" key in session metadata
//!
//! Example metadata JSON:
//! ```json
//! {
//!   "agent_id": "claude-code-1234",
//!   "task_id": "zjj-bq9g",
//!   "spawned_at": 1234567890,
//!   "pid": 5678
//! }
//! ```

pub mod status;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export main types for convenience
pub use status::SessionStatus;
pub use types::{Session, SessionUpdate};
pub use validation::validate_session_name;

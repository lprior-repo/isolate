//! Core domain types for zjj with contracts and validation
//!
//! All types implement the `HasContract` trait, providing:
//! - Type constraints and validation
//! - Contextual hints for AI agents
//! - JSON Schema generation
//! - Self-documenting APIs

pub mod beads;
pub mod changes;
pub mod diff;
pub mod session;

// Re-export all types for convenient access
pub use beads::{BeadsIssue, BeadsSummary, IssueStatus};
pub use changes::{ChangesSummary, FileChange, FileStatus};
pub use diff::{DiffSummary, FileDiffStat};
pub use session::{Operation, Session, SessionStatus};

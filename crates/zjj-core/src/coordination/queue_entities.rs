//! `SQLx` database entities for the merge queue.
//!
//! This module contains infrastructure layer types (`sqlx::FromRow` structs)
//! separated from domain logic. These types are database row representations.
//!
//! Domain logic types are in `queue_status.rs` and operations in `queue.rs`.

use std::ops::Deref;

use super::queue_status::{QueueEventType, QueueStatus, StackMergeState, WorkspaceQueueState};
use crate::Error;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEPENDENTS LIST (JSON wrapper for stack children)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A list of dependent (child) workspace names stored as JSON in the database.
///
/// This newtype wrapper allows proper JSON serialization/deserialization
/// for the `dependents` column while maintaining ergonomic Vec-like access.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dependents(Vec<String>);

impl Dependents {
    /// Create an empty dependents list.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Create a dependents list from a vector of workspace names.
    #[must_use]
    pub const fn from_vec(workspaces: Vec<String>) -> Self {
        Self(workspaces)
    }

    /// Convert to a vector of workspace names.
    #[must_use]
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }

    /// Check if the dependents list is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of dependents.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl Deref for Dependents {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Option<String>> for Dependents {
    type Error = Error;

    fn try_from(value: Option<String>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(Self::new()),
            Some(s) if s.is_empty() => Ok(Self::new()),
            Some(s) => serde_json::from_str(&s).map(Self).map_err(|e| {
                Error::DatabaseError(format!("Failed to deserialize dependents JSON: {e}"))
            }),
        }
    }
}

impl From<Vec<String>> for Dependents {
    fn from(workspaces: Vec<String>) -> Self {
        Self(workspaces)
    }
}

impl From<Dependents> for Vec<String> {
    fn from(dependents: Dependents) -> Self {
        dependents.0
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE ENTRY (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A row in the `merge_queue` table.
///
/// This is the infrastructure representation of a queue entry,
/// directly mapping to the database schema.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QueueEntry {
    pub id: i64,
    pub workspace: String,
    pub bead_id: Option<String>,
    pub priority: i32,
    #[sqlx(try_from = "String")]
    pub status: QueueStatus,
    pub added_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
    pub agent_id: Option<String>,
    pub dedupe_key: Option<String>,
    #[sqlx(default, try_from = "String")]
    pub workspace_state: WorkspaceQueueState,
    pub previous_state: Option<String>,
    pub state_changed_at: Option<i64>,
    pub head_sha: Option<String>,
    #[sqlx(default)]
    pub tested_against_sha: Option<String>,
    #[sqlx(default)]
    pub attempt_count: i32,
    #[sqlx(default)]
    pub max_attempts: i32,
    #[sqlx(default)]
    pub rebase_count: i32,
    #[sqlx(default)]
    pub last_rebase_at: Option<i64>,
    #[sqlx(default)]
    pub parent_workspace: Option<String>,
    #[sqlx(default)]
    pub stack_depth: i32,
    #[sqlx(default, try_from = "Option<String>")]
    pub dependents: Dependents,
    #[sqlx(default)]
    pub stack_root: Option<String>,
    #[sqlx(default, try_from = "String")]
    pub stack_merge_state: StackMergeState,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PROCESSING LOCK (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A row in the `queue_processing_lock` table.
///
/// Represents an acquired processing lock for serialized queue processing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProcessingLock {
    pub agent_id: String,
    pub acquired_at: i64,
    pub expires_at: i64,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE EVENT (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// An event in the queue audit trail.
///
/// Events are append-only with monotonically increasing IDs.
/// They provide an audit trail for queue entry lifecycle.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QueueEvent {
    pub id: i64,
    pub queue_id: i64,
    #[sqlx(try_from = "String")]
    pub event_type: QueueEventType,
    pub details_json: Option<String>,
    pub created_at: i64,
}

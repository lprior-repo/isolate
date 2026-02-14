//! `SQLx` database entities for the merge queue.
//!
//! This module contains infrastructure layer types (sqlx::FromRow structs)
//! separated from domain logic. These types are database row representations.
//!
//! Domain logic types are in `queue_status.rs` and operations in `queue.rs`.

use super::queue_status::{QueueEventType, QueueStatus, WorkspaceQueueState};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE ENTRY (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A row in the merge_queue table.
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
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PROCESSING LOCK (Infrastructure Layer - sqlx dependent)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A row in the queue_processing_lock table.
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

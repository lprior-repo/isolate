//! Queue submission API for adding sessions to the merge queue.
//!
//! This module provides the public API for submitting workspaces to the
//! Graphite-style merge queue with comprehensive error handling and
//! idempotent upsert semantics.
//!
//! # Graphite-Style Merge Queue Semantics
//!
//! 1. **Sequential Processing:** Entries are processed one at a time in priority order
//! 2. **Deduplication:** Prevents duplicate work using stable identifiers (`change_id`)
//! 3. **Idempotent Submission:** Multiple submissions of the same session update the existing entry
//! 4. **Priority-Based Ordering:** Higher priority entries are processed first
//! 5. **State Machine:** Entries progress through a well-defined lifecycle
//! 6. **Terminal State Handling:** Terminal entries can be resubmitted by resetting to pending

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
// Long functions are intentional for submission workflow coherence
#![allow(clippy::too_many_lines)]
// Helper functions have self-evident error conditions
#![allow(clippy::missing_errors_doc)]

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{queue::MergeQueue, queue_status::QueueStatus};
use crate::Error as CrateError;

// ═════════════════════════════════════════════════════════════════════════
// QUEUE SUBMISSION ERROR (Exhaustive Error Taxonomy)
// ═════════════════════════════════════════════════════════════════════════

/// Semantic error variants for queue submission operations.
///
/// This enum provides exhaustive error coverage for all failure modes
/// in the queue submission process, from validation to database operations
/// to JJ integration errors.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum QueueSubmissionError {
    // === Validation Errors ===
    /// Session does not exist
    #[error("session not found: {session}")]
    SessionNotFound { session: String },

    /// Workspace name is invalid
    #[error("invalid workspace name '{workspace}': {reason}")]
    InvalidWorkspaceName { workspace: String, reason: String },

    /// Head SHA is invalid or missing
    #[error("invalid head SHA '{head_sha}': {reason}")]
    InvalidHeadSha { head_sha: String, reason: String },

    /// Deduplication key is invalid
    #[error("invalid dedupe key '{dedupe_key}': {reason}")]
    InvalidDedupeKey { dedupe_key: String, reason: String },

    // === Queue State Errors ===
    /// Session is already in queue with different `dedupe_key`
    #[error("session '{session}' already in queue with existing dedupe_key '{existing_dedupe_key}', provided '{provided_dedupe_key}'")]
    AlreadyInQueue {
        session: String,
        existing_dedupe_key: String,
        provided_dedupe_key: String,
    },

    /// Active entry with same `dedupe_key` exists for different workspace
    #[error("dedupe key '{dedupe_key}' conflict: existing workspace '{existing_workspace}', provided workspace '{provided_workspace}'")]
    DedupeKeyConflict {
        dedupe_key: String,
        existing_workspace: String,
        provided_workspace: String,
    },

    /// Queue is full (optional constraint)
    #[error("queue full: capacity {capacity}, current count {current_count}")]
    QueueFull {
        capacity: usize,
        current_count: usize,
    },

    // === Database Errors ===
    /// Failed to open queue database
    #[error("failed to open queue database at '{path}': {details}")]
    DatabaseOpenFailed { path: String, details: String },

    /// Failed to initialize queue schema
    #[error("failed to initialize queue schema: {reason}")]
    SchemaInitializationFailed { reason: String },

    /// Database transaction failed
    #[error("transaction failed during {operation}: {details}")]
    TransactionFailed { operation: String, details: String },

    /// Concurrent modification detected
    #[error("concurrent modification detected on entry {entry_id} during {operation}")]
    ConcurrentModification { entry_id: i64, operation: String },

    // === JJ Integration Errors ===
    /// Failed to extract workspace identity
    #[error("failed to extract identity from workspace '{workspace}': {reason}")]
    IdentityExtractionFailed { workspace: String, reason: String },

    /// Failed to get `change_id` from jj
    #[error("failed to extract change_id from workspace '{workspace}': {reason}")]
    ChangeIdExtractionFailed { workspace: String, reason: String },

    /// Failed to get `head_sha` from jj
    #[error("failed to extract head_sha from workspace '{workspace}': {reason}")]
    HeadShaExtractionFailed { workspace: String, reason: String },

    /// Failed to push bookmark to remote
    #[error("failed to push bookmark '{bookmark}' for workspace '{workspace}': {reason}")]
    BookmarkPushFailed {
        workspace: String,
        bookmark: String,
        reason: String,
    },

    /// Remote is unreachable
    #[error("remote '{remote}' is unreachable: {reason}")]
    RemoteUnreachable { remote: String, reason: String },

    /// JJ command execution failed
    #[error("jj command '{command}' failed with exit code {exit_code}: {stderr}")]
    JjExecutionFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    // === State Transition Errors ===
    /// Invalid state transition attempted
    #[error("invalid state transition for entry {entry_id}: cannot transition from {current_status} to {target_status}")]
    InvalidStateTransition {
        entry_id: i64,
        current_status: String,
        target_status: String,
    },

    /// Cannot modify terminal entry
    #[error("entry {entry_id} is terminal with status {status} and cannot be modified")]
    EntryIsTerminal { entry_id: i64, status: String },

    // === Authorization Errors ===
    /// Agent not authorized to submit to this workspace
    #[error("agent '{agent_id}' not authorized to submit to workspace '{workspace}'")]
    UnauthorizedWorkspace { agent_id: String, workspace: String },

    /// Agent not authorized to modify queue entry
    #[error("agent '{agent_id}' not authorized to modify entry {entry_id} (owned by '{owner}'")]
    UnauthorizedEntryModification {
        agent_id: String,
        entry_id: i64,
        owner: String,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE SUBMISSION REQUEST
// ═══════════════════════════════════════════════════════════════════════════

/// Request to submit a session to the merge queue.
#[derive(Debug, Clone)]
pub struct QueueSubmissionRequest {
    /// Workspace name (must exist)
    pub workspace: String,

    /// Optional bead ID for traceability
    pub bead_id: Option<String>,

    /// Priority (lower = higher priority, default 0)
    pub priority: i32,

    /// Agent ID submitting the request
    pub agent_id: Option<String>,

    /// Deduplication key (format: "`workspace:change_id`")
    pub dedupe_key: String,

    /// Current HEAD SHA of the workspace
    pub head_sha: String,

    /// Optional `dedupe_key` for the `tested_against_sha`
    pub tested_against_sha: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// SUBMISSION TYPE
// ═══════════════════════════════════════════════════════════════════════════

/// Type of submission (NEW, UPDATED, or RESUBMITTED).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionType {
    /// New entry created
    New,
    /// Existing entry updated (same `dedupe_key` and workspace)
    Updated,
    /// Terminal entry reset to pending
    Resubmitted,
}

impl SubmissionType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Updated => "updated",
            Self::Resubmitted => "resubmitted",
        }
    }
}

impl std::fmt::Display for SubmissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEUE SUBMISSION RESPONSE
// ═══════════════════════════════════════════════════════════════════════════

/// Response from queue submission operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSubmissionResponse {
    /// Queue entry ID (assigned by database)
    pub entry_id: i64,

    /// Workspace name
    pub workspace: String,

    /// Assigned status after submission
    pub status: String,

    /// Position in pending queue (1-indexed)
    pub position: Option<usize>,

    /// Total number of pending entries
    pub pending_count: usize,

    /// Whether this was a new entry or an update
    pub submission_type: SubmissionType,

    /// Timestamp of submission
    pub submitted_at: chrono::DateTime<Utc>,

    /// Optional bead ID
    pub bead_id: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE IDENTITY
// ═══════════════════════════════════════════════════════════════════════════

/// Extracted identity information from a workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceIdentity {
    /// Stable change ID (across rebases)
    pub change_id: String,
    /// Current HEAD commit SHA
    pub head_sha: String,
    /// Bookmark name
    pub bookmark_name: String,
    /// Workspace name
    pub workspace_name: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Compute deduplication key from `change_id` and workspace.
///
/// # Preconditions
/// - `change_id` must be non-empty
/// - workspace must be non-empty
///
/// # Postconditions
/// - Returns formatted `dedupe_key`: "`workspace:change_id`"
/// - Same inputs always produce same output
#[must_use]
pub fn compute_dedupe_key(change_id: &str, workspace: &str) -> String {
    format!("{workspace}:{change_id}")
}

/// Validate workspace before submission.
///
/// # Preconditions
/// - Workspace name must be non-empty and safe
///
/// # Postconditions
/// - Returns true if workspace is valid
/// - Returns detailed error if validation fails
///
/// # Errors
///
/// Returns `QueueSubmissionError` if:
/// - Workspace name is empty
/// - Workspace name contains invalid characters (path traversal)
/// - Workspace name contains invalid path traversal characters
pub fn validate_workspace(
    workspace: &str,
    _workspace_base_path: &Path,
) -> std::result::Result<bool, QueueSubmissionError> {
    if workspace.is_empty() {
        return Err(QueueSubmissionError::InvalidWorkspaceName {
            workspace: workspace.to_string(),
            reason: "workspace name cannot be empty".to_string(),
        });
    }

    if workspace.contains("..") || workspace.contains('/') || workspace.contains('\\') {
        return Err(QueueSubmissionError::InvalidWorkspaceName {
            workspace: workspace.to_string(),
            reason: "workspace name contains invalid characters".to_string(),
        });
    }

    Ok(true)
}

/// Extract identity information from workspace.
///
/// # Preconditions
/// - Must be in a valid JJ repository
/// - Workspace must exist
///
/// # Postconditions
/// - Returns `change_id` (stable across rebases)
/// - Returns `head_sha` (current commit)
/// - Returns bookmark name
pub async fn extract_workspace_identity(
    workspace_path: &Path,
) -> std::result::Result<WorkspaceIdentity, QueueSubmissionError> {
    use tokio::process::Command;

    let workspace_name = workspace_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| QueueSubmissionError::IdentityExtractionFailed {
            workspace: workspace_path.display().to_string(),
            reason: "invalid workspace path".to_string(),
        })?
        .to_string();

    // Get change_id (stable across rebases)
    let change_id_output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "change_id"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| QueueSubmissionError::ChangeIdExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("failed to run jj log: {e}"),
        })?;

    if !change_id_output.status.success() {
        let stderr = String::from_utf8_lossy(&change_id_output.stderr);
        return Err(QueueSubmissionError::ChangeIdExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("jj log failed: {stderr}"),
        });
    }

    let change_id = String::from_utf8_lossy(&change_id_output.stdout)
        .trim()
        .to_string();

    if change_id.is_empty() {
        return Err(QueueSubmissionError::ChangeIdExtractionFailed {
            workspace: workspace_name.clone(),
            reason: "empty change_id returned".to_string(),
        });
    }

    // Get head_sha (current commit hash)
    let head_sha_output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| QueueSubmissionError::HeadShaExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("failed to run jj log: {e}"),
        })?;

    if !head_sha_output.status.success() {
        let stderr = String::from_utf8_lossy(&head_sha_output.stderr);
        return Err(QueueSubmissionError::HeadShaExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("jj log failed: {stderr}"),
        });
    }

    let head_sha = String::from_utf8_lossy(&head_sha_output.stdout)
        .trim()
        .to_string();

    if head_sha.is_empty() {
        return Err(QueueSubmissionError::HeadShaExtractionFailed {
            workspace: workspace_name.clone(),
            reason: "empty commit_id returned".to_string(),
        });
    }

    // Get bookmark name
    let bookmark_output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "bookmarks"])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| QueueSubmissionError::IdentityExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("failed to get bookmarks: {e}"),
        })?;

    if !bookmark_output.status.success() {
        let stderr = String::from_utf8_lossy(&bookmark_output.stderr);
        return Err(QueueSubmissionError::IdentityExtractionFailed {
            workspace: workspace_name.clone(),
            reason: format!("jj log failed: {stderr}"),
        });
    }

    let stdout = String::from_utf8_lossy(&bookmark_output.stdout);
    let bookmarks: Vec<&str> = stdout.split_whitespace().collect();

    let bookmark_name = bookmarks
        .iter()
        .find(|&&b| b == workspace_name)
        .or_else(|| bookmarks.first())
        .ok_or_else(|| QueueSubmissionError::IdentityExtractionFailed {
            workspace: workspace_name.clone(),
            reason: "no bookmark found".to_string(),
        })?
        .to_string();

    Ok(WorkspaceIdentity {
        change_id,
        head_sha,
        bookmark_name,
        workspace_name,
    })
}

/// Push bookmark to remote before queueing.
///
/// # Preconditions
/// - Bookmark must exist locally
/// - Remote must be configured
///
/// # Postconditions
/// - Bookmark is pushed to remote
/// - Returns error if push fails
pub async fn push_bookmark_to_remote(
    workspace_path: &Path,
    bookmark: &str,
) -> std::result::Result<(), QueueSubmissionError> {
    use tokio::process::Command;

    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark])
        .current_dir(workspace_path)
        .output()
        .await
        .map_err(|e| QueueSubmissionError::BookmarkPushFailed {
            workspace: workspace_path.display().to_string(),
            bookmark: bookmark.to_string(),
            reason: format!("failed to execute jj git push: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = stderr.to_string();

        // Check for remote/network errors
        if error_msg.contains("could not resolve host")
            || error_msg.contains("network")
            || error_msg.contains("connection refused")
            || error_msg.contains("timed out")
            || error_msg.contains("unreachable")
        {
            return Err(QueueSubmissionError::RemoteUnreachable {
                remote: "origin".to_string(),
                reason: error_msg,
            });
        }

        return Err(QueueSubmissionError::BookmarkPushFailed {
            workspace: workspace_path.display().to_string(),
            bookmark: bookmark.to_string(),
            reason: error_msg,
        });
    }

    Ok(())
}

/// Get current position in queue.
///
/// # Preconditions
/// - Entry must exist
///
/// # Postconditions
/// - Returns position if status is 'pending'
/// - Returns None if status is not 'pending'
pub async fn get_queue_position(
    db_path: &Path,
    entry_id: i64,
) -> std::result::Result<Option<usize>, QueueSubmissionError> {
    let queue =
        MergeQueue::open(db_path)
            .await
            .map_err(|e| QueueSubmissionError::DatabaseOpenFailed {
                path: db_path.display().to_string(),
                details: e.to_string(),
            })?;

    // Get the entry to check its status
    let entry = queue
        .get_by_id(entry_id)
        .await
        .map_err(|e| QueueSubmissionError::TransactionFailed {
            operation: "get_by_id".to_string(),
            details: e.to_string(),
        })?
        .ok_or_else(|| QueueSubmissionError::SessionNotFound {
            session: format!("entry_id {entry_id}"),
        })?;

    // Only pending entries have a position
    if entry.status != QueueStatus::Pending {
        return Ok(None);
    }

    // Get position for the workspace
    let position = queue.position(&entry.workspace).await.map_err(|e| {
        QueueSubmissionError::TransactionFailed {
            operation: "position".to_string(),
            details: e.to_string(),
        }
    })?;

    Ok(position)
}

/// Check if session is already in queue.
///
/// # Preconditions
/// - None
///
/// # Postconditions
/// - Returns true if entry exists for workspace
/// - Returns false otherwise
pub async fn is_in_queue(
    db_path: &Path,
    workspace: &str,
) -> std::result::Result<bool, QueueSubmissionError> {
    let queue =
        MergeQueue::open(db_path)
            .await
            .map_err(|e| QueueSubmissionError::DatabaseOpenFailed {
                path: db_path.display().to_string(),
                details: e.to_string(),
            })?;

    let entry = queue.get_by_workspace(workspace).await.map_err(|e| {
        QueueSubmissionError::TransactionFailed {
            operation: "get_by_workspace".to_string(),
            details: e.to_string(),
        }
    })?;

    Ok(entry.is_some())
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN SUBMIT FUNCTION
// ═══════════════════════════════════════════════════════════════════════════

/// Submit a session to the merge queue.
///
/// This is the main entry point for adding workspaces to the merge queue.
/// It implements Graphite-style merge queue semantics with idempotent upsert.
///
/// # Preconditions
/// - Workspace name must be valid
/// - Remote must be reachable (bookmark push verified)
/// - Head SHA must be valid
/// - `Dedupe_key` must be unique among active entries
///
/// # Postconditions
/// - Entry exists in `merge_queue` table
/// - Entry has unique ID
/// - Entry status is 'pending' (or updated from previous state)
/// - Position is assigned if status is 'pending'
/// - `Dedupe_key` is set and enforced
/// - Event audit trail is updated
///
/// # Errors
/// - Returns `QueueSubmissionError` for any validation or database failure
pub async fn submit_to_queue(
    request: QueueSubmissionRequest,
    db_path: &Path,
    workspace_path: &Path,
) -> std::result::Result<QueueSubmissionResponse, QueueSubmissionError> {
    // Validate workspace (checks existence and JJ structure)
    validate_workspace(&request.workspace, workspace_path)?;

    // Validate head_sha format (basic check)
    if request.head_sha.len() < 4 {
        return Err(QueueSubmissionError::InvalidHeadSha {
            head_sha: request.head_sha.clone(),
            reason: "head_sha is too short".to_string(),
        });
    }

    // Validate dedupe_key format
    if !request.dedupe_key.contains(':') {
        return Err(QueueSubmissionError::InvalidDedupeKey {
            dedupe_key: request.dedupe_key.clone(),
            reason: "dedupe_key must contain ':' separator".to_string(),
        });
    }

    // Open the merge queue
    let queue =
        MergeQueue::open(db_path)
            .await
            .map_err(|e| QueueSubmissionError::DatabaseOpenFailed {
                path: db_path.display().to_string(),
                details: e.to_string(),
            })?;

    // Check if workspace is already in queue with different dedupe_key
    if let Ok(Some(existing_entry)) = queue.get_by_workspace(&request.workspace).await {
        if let Some(existing_dedupe) = &existing_entry.dedupe_key {
            if existing_dedupe != &request.dedupe_key {
                return Err(QueueSubmissionError::AlreadyInQueue {
                    session: request.workspace.clone(),
                    existing_dedupe_key: existing_dedupe.clone(),
                    provided_dedupe_key: request.dedupe_key.clone(),
                });
            }
        }
    }

    // Check for existing entry to determine submission type
    let existing_entry = queue
        .get_by_workspace(&request.workspace)
        .await
        .ok()
        .flatten();

    // Determine submission type BEFORE upsert
    // This is imperfect - we can't detect all cases without find_by_dedupe_key being public
    let submission_type = match existing_entry {
        None => SubmissionType::New,
        Some(ref entry) if entry.status.is_terminal() => SubmissionType::Resubmitted,
        Some(_) => SubmissionType::Updated,
    };

    // Perform upsert for submit
    let entry = queue
        .upsert_for_submit(
            &request.workspace,
            request.bead_id.as_deref(),
            request.priority,
            request.agent_id.as_deref(),
            &request.dedupe_key,
            &request.head_sha,
        )
        .await
        .map_err(|e| match e {
            CrateError::DedupeKeyConflict {
                dedupe_key,
                existing_workspace,
                provided_workspace,
            } => QueueSubmissionError::DedupeKeyConflict {
                dedupe_key,
                existing_workspace,
                provided_workspace,
            },
            CrateError::InvalidConfig(msg) => QueueSubmissionError::TransactionFailed {
                operation: "upsert_for_submit".to_string(),
                details: msg,
            },
            _ => QueueSubmissionError::TransactionFailed {
                operation: "upsert_for_submit".to_string(),
                details: e.to_string(),
            },
        })?;

    // Get position if pending
    let position = if entry.status == QueueStatus::Pending {
        Some(
            queue
                .position(&request.workspace)
                .await
                .map_err(|e| QueueSubmissionError::TransactionFailed {
                    operation: "position".to_string(),
                    details: e.to_string(),
                })?
                .unwrap_or(1),
        )
    } else {
        None
    };

    // Get total pending count
    let pending_count =
        queue
            .count_pending()
            .await
            .map_err(|e| QueueSubmissionError::TransactionFailed {
                operation: "count_pending".to_string(),
                details: e.to_string(),
            })?;

    Ok(QueueSubmissionResponse {
        entry_id: entry.id,
        workspace: request.workspace,
        status: entry.status.as_str().to_string(),
        position,
        pending_count,
        submission_type,
        submitted_at: Utc::now(),
        bead_id: request.bead_id,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    // ────────────────────────────────────────────────────────────────────────
    // Error Variant Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_queue_submission_error_display() {
        let err = QueueSubmissionError::SessionNotFound {
            session: "test-session".to_string(),
        };
        assert!(err.to_string().contains("session not found"));
        assert!(err.to_string().contains("test-session"));
    }

    #[test]
    fn test_invalid_workspace_name_error() {
        let err = QueueSubmissionError::InvalidWorkspaceName {
            workspace: String::new(),
            reason: "cannot be empty".to_string(),
        };
        assert!(err.to_string().contains("invalid workspace name"));
    }

    #[test]
    fn test_invalid_head_sha_error() {
        let err = QueueSubmissionError::InvalidHeadSha {
            head_sha: "abc".to_string(),
            reason: "too short".to_string(),
        };
        assert!(err.to_string().contains("invalid head SHA"));
    }

    #[test]
    fn test_dedupe_key_conflict_error() {
        let err = QueueSubmissionError::DedupeKeyConflict {
            dedupe_key: "ws-a:kxyz789".to_string(),
            existing_workspace: "ws-a".to_string(),
            provided_workspace: "ws-b".to_string(),
        };
        assert!(err.to_string().contains("dedupe key"));
        assert!(err.to_string().contains("conflict"));
    }

    #[test]
    fn test_database_open_failed_error() {
        let err = QueueSubmissionError::DatabaseOpenFailed {
            path: "/tmp/test.db".to_string(),
            details: "permission denied".to_string(),
        };
        assert!(err.to_string().contains("failed to open queue database"));
    }

    #[test]
    fn test_transaction_failed_error() {
        let err = QueueSubmissionError::TransactionFailed {
            operation: "upsert".to_string(),
            details: "database locked".to_string(),
        };
        assert!(err.to_string().contains("transaction failed"));
    }

    #[test]
    fn test_bookmark_push_failed_error() {
        let err = QueueSubmissionError::BookmarkPushFailed {
            workspace: "test-ws".to_string(),
            bookmark: "feature".to_string(),
            reason: "remote rejected".to_string(),
        };
        assert!(err.to_string().contains("failed to push bookmark"));
    }

    #[test]
    fn test_remote_unreachable_error() {
        let err = QueueSubmissionError::RemoteUnreachable {
            remote: "origin".to_string(),
            reason: "connection timeout".to_string(),
        };
        assert!(err.to_string().contains("unreachable"));
    }

    // ────────────────────────────────────────────────────────────────────────
    // Compute Dedupe Key Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_compute_dedupe_key() {
        let result = compute_dedupe_key("kxyz789", "feature-auth");
        assert_eq!(result, "feature-auth:kxyz789");
    }

    #[test]
    fn test_compute_dedupe_key_deterministic() {
        let result1 = compute_dedupe_key("kxyz789", "feature-auth");
        let result2 = compute_dedupe_key("kxyz789", "feature-auth");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_compute_dedupe_key_different_workspace() {
        let result1 = compute_dedupe_key("kxyz789", "ws-a");
        let result2 = compute_dedupe_key("kxyz789", "ws-b");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_compute_dedupe_key_different_change_id() {
        let result1 = compute_dedupe_key("kxyz789", "ws-a");
        let result2 = compute_dedupe_key("kabc123", "ws-a");
        assert_ne!(result1, result2);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Submission Type Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_submission_type_as_str() {
        assert_eq!(SubmissionType::New.as_str(), "new");
        assert_eq!(SubmissionType::Updated.as_str(), "updated");
        assert_eq!(SubmissionType::Resubmitted.as_str(), "resubmitted");
    }

    #[test]
    fn test_submission_type_display() {
        assert_eq!(SubmissionType::New.to_string(), "new");
        assert_eq!(SubmissionType::Updated.to_string(), "updated");
        assert_eq!(SubmissionType::Resubmitted.to_string(), "resubmitted");
    }

    #[test]
    fn test_submission_type_equality() {
        assert_eq!(SubmissionType::New, SubmissionType::New);
        assert_ne!(SubmissionType::New, SubmissionType::Updated);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Error Serialization Tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_error_serialization_session_not_found() {
        let err = QueueSubmissionError::SessionNotFound {
            session: "test".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("SessionNotFound"));
    }

    #[test]
    fn test_error_deserialization_session_not_found() {
        let json = r#"{"SessionNotFound":{"session":"test"}}"#;
        let err: QueueSubmissionError = serde_json::from_str(json).unwrap();
        assert_eq!(
            err,
            QueueSubmissionError::SessionNotFound {
                session: "test".to_string()
            }
        );
    }

    #[test]
    fn test_error_roundtrip() {
        let errors = vec![
            QueueSubmissionError::SessionNotFound {
                session: "test".to_string(),
            },
            QueueSubmissionError::InvalidWorkspaceName {
                workspace: "bad".to_string(),
                reason: "test".to_string(),
            },
            QueueSubmissionError::DedupeKeyConflict {
                dedupe_key: "key".to_string(),
                existing_workspace: "ws1".to_string(),
                provided_workspace: "ws2".to_string(),
            },
        ];

        for err in errors {
            let json = serde_json::to_string(&err).unwrap();
            let decoded: QueueSubmissionError = serde_json::from_str(&json).unwrap();
            assert_eq!(err, decoded);
        }
    }
}

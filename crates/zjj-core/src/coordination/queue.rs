//! Merge queue for sequential multi-agent coordination.

use std::{fmt, path::Path, str::FromStr, time::Duration};

use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use thiserror::Error;
use tokio::time::sleep;

use crate::{Error, Result};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// STATE MACHINE ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for invalid queue state transitions.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid state transition: cannot transition from {from} to {to}")]
pub struct TransitionError {
    pub from: QueueStatus,
    pub to: QueueStatus,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE STATUS STATE MACHINE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// State machine for merge queue item lifecycle.
///
/// Valid transitions:
/// - pending -> claimed
/// - claimed -> rebasing
/// - rebasing -> testing
/// - testing -> ready_to_merge
/// - ready_to_merge -> merging
/// - merging -> merged
/// - claimed|rebasing|testing|ready_to_merge|merging -> failed_retryable
/// - claimed|rebasing|testing|ready_to_merge|merging -> failed_terminal
/// - pending|claimed|rebasing|testing|ready_to_merge|failed_retryable -> cancelled
/// - failed_retryable -> pending (manual retry path)
///
/// Terminal states (no outgoing transitions): merged, failed_terminal, cancelled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    /// Item is waiting to be claimed by an agent.
    Pending,
    /// Item has been claimed by an agent and is being prepared.
    Claimed,
    /// Item is currently being rebased onto the target branch.
    Rebasing,
    /// Item is undergoing testing/validation.
    Testing,
    /// Item has passed all checks and is ready for merge.
    ReadyToMerge,
    /// Item is actively being merged.
    Merging,
    /// Item has been successfully merged.
    Merged,
    /// Item failed but can be retried manually.
    FailedRetryable,
    /// Item failed with an unrecoverable error.
    FailedTerminal,
    /// Item was cancelled before completion.
    Cancelled,
}

impl QueueStatus {
    /// Returns the string representation of this status.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Rebasing => "rebasing",
            Self::Testing => "testing",
            Self::ReadyToMerge => "ready_to_merge",
            Self::Merging => "merging",
            Self::Merged => "merged",
            Self::FailedRetryable => "failed_retryable",
            Self::FailedTerminal => "failed_terminal",
            Self::Cancelled => "cancelled",
        }
    }

    /// Returns true if this status is terminal (no valid outgoing transitions).
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Merged | Self::FailedTerminal | Self::Cancelled
        )
    }

    /// Returns true if a transition from `self` to `target` is valid.
    #[must_use]
    pub fn can_transition_to(&self, target: Self) -> bool {
        self.validate_transition(target).is_ok()
    }

    /// Validates that a transition from `self` to `target` is allowed.
    ///
    /// Returns `Ok(())` if the transition is valid, or a `TransitionError` if not.
    pub fn validate_transition(&self, target: Self) -> std::result::Result<(), TransitionError> {
        // Same state is always valid (idempotent)
        if self == &target {
            return Ok(());
        }

        // Terminal states cannot transition to any other state
        if self.is_terminal() {
            return Err(TransitionError {
                from: *self,
                to: target,
            });
        }

        // Check specific valid transitions
        let is_valid = match self {
            Self::Pending => matches!(
                target,
                Self::Claimed | Self::Cancelled
            ),
            Self::Claimed => matches!(
                target,
                Self::Rebasing
                    | Self::FailedRetryable
                    | Self::FailedTerminal
                    | Self::Cancelled
            ),
            Self::Rebasing => matches!(
                target,
                Self::Testing
                    | Self::FailedRetryable
                    | Self::FailedTerminal
                    | Self::Cancelled
            ),
            Self::Testing => matches!(
                target,
                Self::ReadyToMerge
                    | Self::FailedRetryable
                    | Self::FailedTerminal
                    | Self::Cancelled
            ),
            Self::ReadyToMerge => matches!(
                target,
                Self::Merging
                    | Self::FailedRetryable
                    | Self::FailedTerminal
                    | Self::Cancelled
            ),
            Self::Merging => matches!(
                target,
                Self::Merged | Self::FailedRetryable | Self::FailedTerminal
            ),
            Self::FailedRetryable => matches!(target, Self::Pending | Self::Cancelled),
            // Terminal states handled above, but satisfy exhaustive match
            Self::Merged | Self::FailedTerminal | Self::Cancelled => false,
        };

        if is_valid {
            Ok(())
        } else {
            Err(TransitionError {
                from: *self,
                to: target,
            })
        }
    }
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for QueueStatus {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "claimed" => Ok(Self::Claimed),
            "rebasing" => Ok(Self::Rebasing),
            "testing" => Ok(Self::Testing),
            "ready_to_merge" => Ok(Self::ReadyToMerge),
            "merging" => Ok(Self::Merging),
            "merged" => Ok(Self::Merged),
            "failed_retryable" => Ok(Self::FailedRetryable),
            "failed_terminal" => Ok(Self::FailedTerminal),
            "cancelled" => Ok(Self::Cancelled),
            // Backward compatibility: map old statuses to new equivalents
            "processing" => Ok(Self::Claimed),
            "completed" => Ok(Self::Merged),
            "failed" => Ok(Self::FailedTerminal),
            _ => Err(Error::InvalidConfig(format!("Invalid queue status: {s}"))),
        }
    }
}

/// Workspace state for the queue state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceQueueState {
    #[default]
    Created,
    Working,
    Ready,
    Merged,
    Abandoned,
    Conflict,
}

impl WorkspaceQueueState {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Working => "working",
            Self::Ready => "ready",
            Self::Merged => "merged",
            Self::Abandoned => "abandoned",
            Self::Conflict => "conflict",
        }
    }
}

impl std::str::FromStr for WorkspaceQueueState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "created" => Ok(Self::Created),
            "working" => Ok(Self::Working),
            "ready" => Ok(Self::Ready),
            "merged" => Ok(Self::Merged),
            "abandoned" => Ok(Self::Abandoned),
            "conflict" => Ok(Self::Conflict),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid workspace queue state: {s}"
            ))),
        }
    }
}

impl TryFrom<String> for WorkspaceQueueState {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::from_str(&s)
    }
}

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
}

impl TryFrom<String> for QueueStatus {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

#[derive(Debug, Clone)]
pub struct QueueAddResponse {
    pub entry: QueueEntry,
    pub position: usize,
    pub total_pending: usize,
}

#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total: usize,
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProcessingLock {
    pub agent_id: String,
    pub acquired_at: i64,
    pub expires_at: i64,
}

#[derive(Clone)]
pub struct MergeQueue {
    pool: SqlitePool,
    lock_timeout_secs: i64,
}

impl MergeQueue {
    pub const DEFAULT_LOCK_TIMEOUT_SECS: i64 = 300;

    pub async fn open(db_path: &Path) -> Result<Self> {
        Self::open_with_timeout(db_path, Self::DEFAULT_LOCK_TIMEOUT_SECS).await
    }

    pub async fn open_with_timeout(db_path: &Path, lock_timeout_secs: i64) -> Result<Self> {
        // Create database file if it doesn't exist
        match tokio::fs::try_exists(db_path).await {
            Ok(false) => {
                if let Some(parent) = db_path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        Error::IoError(format!("Failed to create database directory: {e}"))
                    })?;
                }
                tokio::fs::File::create(db_path)
                    .await
                    .map_err(|e| Error::IoError(format!("Failed to create database file: {e}")))?;
            }
            Ok(true) => {}
            Err(e) => {
                return Err(Error::IoError(format!(
                    "Failed to check database existence: {e}"
                )))
            }
        }

        let db_url = format!("sqlite://{}", db_path.to_string_lossy());
        let pool = SqlitePoolOptions::new()
            .connect(&db_url)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to open queue database: {e}")))?;

        let queue = Self {
            pool,
            lock_timeout_secs,
        };
        queue.init_schema().await?;
        Ok(queue)
    }

    /// Open an in-memory merge queue for testing
    ///
    /// This creates a transient in-memory `SQLite` database that is
    /// discarded when the queue is dropped. Useful for testing and
    /// development without persisting state to disk.
    pub async fn open_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to open in-memory database: {e}")))?;

        Self::new(pool).await
    }

    /// Create a new merge queue from an existing connection pool
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let queue = Self {
            pool,
            lock_timeout_secs: Self::DEFAULT_LOCK_TIMEOUT_SECS,
        };
        queue.init_schema().await?;
        Ok(queue)
    }

    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS merge_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace TEXT NOT NULL UNIQUE,
                bead_id TEXT,
                priority INTEGER NOT NULL DEFAULT 5,
                status TEXT NOT NULL DEFAULT 'pending',
                added_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER,
                error_message TEXT,
                agent_id TEXT,
                dedupe_key TEXT,
                workspace_state TEXT DEFAULT 'created',
                previous_state TEXT,
                state_changed_at INTEGER
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create merge_queue table: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_merge_queue_status ON merge_queue(status)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create status index: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_merge_queue_priority_added ON merge_queue(priority, added_at)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create priority index: {e}")))?;

        // Create unique index on workspace + started_at to prevent concurrent processing
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_merge_queue_processing
             ON merge_queue(workspace, started_at)
             WHERE status = 'processing' AND started_at IS NOT NULL",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create processing index: {e}")))?;

        // Create unique index on dedupe_key for NON-TERMINAL entries only
        // Terminal states (merged, failed_terminal, cancelled) can have duplicate dedupe_keys
        // to allow re-submission after completion/failure
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_merge_queue_dedupe_key_active
             ON merge_queue(dedupe_key)
             WHERE dedupe_key IS NOT NULL
               AND status NOT IN ('merged', 'failed_terminal', 'cancelled')",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create dedupe_key index: {e}")))?;

        // Create index on workspace_state for efficient queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_merge_queue_workspace_state
             ON merge_queue(workspace_state)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to create workspace_state index: {e}"))
        })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS queue_processing_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                agent_id TEXT NOT NULL,
                acquired_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create lock table: {e}")))?;

        Ok(())
    }

    #[allow(clippy::cast_possible_wrap)]
    fn now() -> i64 {
        // Use chrono's battle-tested timestamp() which cannot fail
        Utc::now().timestamp()
    }

    pub async fn add(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
    ) -> Result<QueueAddResponse> {
        self.add_with_dedupe(workspace, bead_id, priority, agent_id, None)
            .await
    }

    /// Add a workspace to the queue with an optional deduplication key.
    ///
    /// The dedupe_key allows preventing duplicate work by rejecting entries
    /// with duplicate keys. NULL dedupe_keys are allowed multiple times.
    pub async fn add_with_dedupe(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: Option<&str>,
    ) -> Result<QueueAddResponse> {
        let now = Self::now();
        let result = sqlx::query(
            "INSERT INTO merge_queue (workspace, bead_id, priority, status, added_at, agent_id, dedupe_key, workspace_state, state_changed_at) \
                 VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, 'created', ?4)",
        )
        .bind(workspace)
        .bind(bead_id)
        .bind(priority)
        .bind(now)
        .bind(agent_id)
        .bind(dedupe_key)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("UNIQUE constraint failed: merge_queue.workspace") {
                Error::InvalidConfig(format!("Workspace '{workspace}' is already in the queue"))
            } else if error_str.contains("UNIQUE constraint failed: merge_queue.dedupe_key")
                || error_str.contains("idx_merge_queue_dedupe_key")
            {
                let key_display = dedupe_key.unwrap_or("<none>");
                Error::InvalidConfig(format!(
                    "An entry with dedupe_key '{key_display}' already exists in the queue"
                ))
            } else {
                Error::DatabaseError(format!("Failed to add to queue: {e}"))
            }
        })?;

        let id = result.last_insert_rowid();
        let entry = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve inserted entry".to_string()))?;

        // Position MUST exist after insertion
        let position = self.position(workspace).await?.ok_or_else(|| {
            Error::DatabaseError("Workspace not found in queue after insertion".to_string())
        })?;
        let total_pending = self.count_pending().await?;
        Ok(QueueAddResponse {
            entry,
            position,
            total_pending,
        })
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at FROM merge_queue WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get entry: {e}")))
    }

    pub async fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at FROM merge_queue WHERE workspace = ?1",
        )
        .bind(workspace)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get entry: {e}")))
    }

    pub async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
        let sql = match filter_status {
            Some(_) => {
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at FROM merge_queue WHERE status = ?1 \
                 ORDER BY priority ASC, added_at ASC"
            }
            None => {
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at FROM merge_queue \
                 ORDER BY priority ASC, added_at ASC"
            }
        };

        let query = sqlx::query_as::<_, QueueEntry>(sql);
        let query = if let Some(s) = filter_status {
            query.bind(s.as_str())
        } else {
            query
        };

        query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to list entries: {e}")))
    }

    pub async fn next(&self) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at FROM merge_queue WHERE status = 'pending' \
                 ORDER BY priority ASC, added_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get next entry: {e}")))
    }

    pub async fn remove(&self, workspace: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM merge_queue WHERE workspace = ?1")
            .bind(workspace)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to remove entry: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn position(&self, workspace: &str) -> Result<Option<usize>> {
        let pending = self.list(Some(QueueStatus::Pending)).await?;
        Ok(pending
            .iter()
            .position(|e| e.workspace == workspace)
            .map(|p| p + 1))
    }

    #[allow(clippy::cast_sign_loss)]
    pub async fn count_pending(&self) -> Result<usize> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM merge_queue WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to count pending: {e}")))?;
        Ok(count as usize)
    }

    #[allow(clippy::cast_sign_loss)]
    pub async fn stats(&self) -> Result<QueueStats> {
        let rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT status, COUNT(*) FROM merge_queue GROUP BY status")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to get stats: {e}")))?;

        Ok(rows
            .into_iter()
            .fold(QueueStats::default(), |mut acc, (status_str, cnt)| {
                let cnt = cnt as usize;
                acc.total += cnt;
                match status_str.as_str() {
                    "pending" => acc.pending = cnt,
                    "processing" => acc.processing = cnt,
                    "completed" => acc.completed = cnt,
                    "failed" => acc.failed = cnt,
                    _ => {}
                }
                acc
            }))
    }

    pub async fn acquire_processing_lock(&self, agent_id: &str) -> Result<bool> {
        let now = Self::now();
        let expires_at = now + self.lock_timeout_secs;
        let result = sqlx::query(
            "INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) \
                 VALUES (1, ?1, ?2, ?3) \
                 ON CONFLICT(id) DO UPDATE SET agent_id = ?1, acquired_at = ?2, expires_at = ?3 \
                 WHERE expires_at < ?2",
        )
        .bind(agent_id)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) => Ok(r.rows_affected() > 0),
            Err(e) => Err(Error::DatabaseError(format!("Failed to acquire lock: {e}"))),
        }
    }

    pub async fn release_processing_lock(&self, agent_id: &str) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM queue_processing_lock WHERE id = 1 AND agent_id = ?1")
                .bind(agent_id)
                .execute(&self.pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to release lock: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_processing_lock(&self) -> Result<Option<ProcessingLock>> {
        sqlx::query_as::<_, ProcessingLock>(
            "SELECT agent_id, acquired_at, expires_at FROM queue_processing_lock WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get lock: {e}")))
    }

    pub async fn mark_processing(&self, workspace: &str) -> Result<bool> {
        let now = Self::now();
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'processing', started_at = ?1 \
                 WHERE workspace = ?2 AND status = 'pending'",
        )
        .bind(now)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to mark processing: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_completed(&self, workspace: &str) -> Result<bool> {
        let now = Self::now();
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'completed', completed_at = ?1 \
                 WHERE workspace = ?2 AND status = 'processing'",
        )
        .bind(now)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to mark completed: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_failed(&self, workspace: &str, error: &str) -> Result<bool> {
        let now = Self::now();
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'failed', completed_at = ?1, error_message = ?2 \
                 WHERE workspace = ?3 AND status = 'processing'",
        )
        .bind(now)
        .bind(error)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to mark failed: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
        const MAX_RETRIES: u32 = 5;
        const INITIAL_BACKOFF_MS: u64 = 50;

        let mut attempt = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            attempt += 1;

            // Attempt to claim the next entry with atomic transaction
            let result = self.try_claim_next_entry(agent_id).await;

            // Match on result to determine if we should retry
            match &result {
                Ok(_entry) => {
                    // Success or no work available - return immediately
                    return result;
                }
                Err(e) => {
                    // Check if this is a retryable error
                    let error_str = e.to_string();
                    let is_constraint_violation = error_str.contains("UNIQUE constraint failed")
                        || error_str.contains("constraint");
                    let is_db_locked = error_str.contains("database is locked")
                        || error_str.contains("database table is locked")
                        || error_str.contains("SQLITE_BUSY");

                    if (is_constraint_violation || is_db_locked) && attempt < MAX_RETRIES {
                        // Retry with exponential backoff
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = backoff_ms.saturating_mul(2);
                        continue;
                    }

                    // Non-retryable error or max retries exceeded
                    return result;
                }
            }
        }
    }

    /// Attempts to claim the next queue entry atomically within a single transaction.
    /// This is the inner function that performs the actual database operations.
    async fn try_claim_next_entry(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
        // Use a transaction to ensure atomicity: lock acquisition, entry fetch, and status update
        // all happen as a single operation, preventing race conditions between agents
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to begin transaction: {e}")))?;

        let now = Self::now();
        let expires_at = now + self.lock_timeout_secs;

        // Step 1: Try to acquire the processing lock within the transaction
        let lock_acquired = sqlx::query(
            "INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) \
                 VALUES (1, ?1, ?2, ?3) \
                 ON CONFLICT(id) DO UPDATE SET agent_id = ?1, acquired_at = ?2, expires_at = ?3 \
                 WHERE expires_at < ?2",
        )
        .bind(agent_id)
        .bind(now)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to acquire lock: {e}")))?;

        if lock_acquired.rows_affected() == 0 {
            // Another agent holds the lock
            tx.rollback().await.map_err(|e| {
                Error::DatabaseError(format!("Failed to rollback transaction: {e}"))
            })?;
            return Ok(None);
        }

        // Step 2: Find and claim the next pending entry atomically within the same transaction
        // The unique index on (workspace, started_at) prevents concurrent processing of the same
        // workspace
        let entry = sqlx::query_as::<_, QueueEntry>(
            "UPDATE merge_queue
             SET status = 'processing',
                 started_at = ?1,
                 agent_id = ?2
             WHERE id = (
                 SELECT id FROM merge_queue
                 WHERE status = 'pending'
                 ORDER BY priority ASC, added_at ASC
                 LIMIT 1
             )
             RETURNING id, workspace, bead_id, priority, status, added_at, started_at,
                       completed_at, error_message, agent_id, dedupe_key, workspace_state,
                       previous_state, state_changed_at",
        )
        .bind(now)
        .bind(agent_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to claim next entry: {e}")))?;

        // Commit the transaction to make both the lock acquisition and entry claim atomic
        tx.commit()
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to commit transaction: {e}")))?;

        // If no entry was found, release the lock and return None
        if entry.is_none() {
            let _ = self.release_processing_lock(agent_id).await;
        }

        Ok(entry)
    }

    pub async fn extend_lock(&self, agent_id: &str, extra_secs: i64) -> Result<bool> {
        // Get the current expiration time and extend from that, not from now
        let current_lock = self.get_processing_lock().await?;

        let new_expires = match current_lock {
            Some(lock) => lock.expires_at + extra_secs,
            None => return Ok(false), // No lock to extend
        };

        let result = sqlx::query(
            "UPDATE queue_processing_lock SET expires_at = ?1 WHERE id = 1 AND agent_id = ?2",
        )
        .bind(new_expires)
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to extend lock: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    #[allow(clippy::cast_possible_wrap)]
    pub async fn cleanup(&self, max_age: Duration) -> Result<usize> {
        let result = if max_age.is_zero() {
            sqlx::query("DELETE FROM merge_queue WHERE status IN ('completed', 'failed')")
                .execute(&self.pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?
        } else {
            let cutoff = Self::now() - max_age.as_secs() as i64;
            sqlx::query(
                "DELETE FROM merge_queue WHERE status IN ('completed', 'failed') \
                     AND completed_at <= ?1",
            )
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?
        };
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SCHEMA TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_fresh_database_has_dedupe_key_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify dedupe_key column exists by querying it
        let _: Option<String> = sqlx::query_scalar("SELECT dedupe_key FROM merge_queue LIMIT 1")
            .fetch_optional(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("dedupe_key column should exist: {e}")))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_fresh_database_has_workspace_state_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify workspace_state column exists
        let _: Option<String> =
            sqlx::query_scalar("SELECT workspace_state FROM merge_queue LIMIT 1")
                .fetch_optional(&queue.pool)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("workspace_state column should exist: {e}"))
                })?;

        Ok(())
    }

    #[tokio::test]
    async fn test_dedupe_key_unique_constraint_enforced() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add entry with dedupe_key
        queue
            .add_with_dedupe("ws-1", None, 5, None, Some("unique-key-1"))
            .await?;

        // Attempt to add another entry with same dedupe_key should fail
        let result = queue
            .add_with_dedupe("ws-2", None, 5, None, Some("unique-key-1"))
            .await;
        assert!(result.is_err(), "Duplicate dedupe_key should be rejected");

        let error_msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error_msg.contains("UNIQUE") || error_msg.contains("dedupe"),
            "Error should mention UNIQUE constraint or dedupe, got: {error_msg}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_dedupe_key_null_allows_multiple() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add_with_dedupe("ws-1", None, 5, None, None).await?;
        queue.add_with_dedupe("ws-2", None, 5, None, None).await?;
        queue.add_with_dedupe("ws-3", None, 5, None, None).await?;

        let stats = queue.stats().await?;
        assert_eq!(
            stats.total, 3,
            "Should have 3 entries with NULL dedupe_keys"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_workspace_state_default_is_created() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-1", None, 5, None).await?;

        let state: String =
            sqlx::query_scalar("SELECT workspace_state FROM merge_queue WHERE workspace = 'ws-1'")
                .fetch_one(&queue.pool)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("Failed to query workspace_state: {e}"))
                })?;

        assert_eq!(
            state, "created",
            "New entries should default to 'created' state"
        );

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EXISTING TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_add_and_list() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue
            .add("ws-1", Some("bead-1"), 5, Some("agent-1"))
            .await?;
        assert_eq!(resp.entry.workspace, "ws-1");
        assert_eq!(resp.position, 1);
        let entries = queue.list(Some(QueueStatus::Pending)).await?;
        assert_eq!(entries.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_priority_ordering() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        queue.add("low", None, 10, None).await?;
        queue.add("high", None, 0, None).await?;
        queue.add("mid", None, 5, None).await?;
        let entries = queue.list(Some(QueueStatus::Pending)).await?;
        assert_eq!(entries[0].workspace, "high");
        assert_eq!(entries[1].workspace, "mid");
        assert_eq!(entries[2].workspace, "low");
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_adds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        for i in 0..40 {
            queue
                .add(&format!("ws-{i}"), None, i % 3, Some(&format!("agent-{i}")))
                .await?;
        }
        let queue_stats = queue.stats().await?;
        assert_eq!(queue_stats.total, 40);
        assert_eq!(queue_stats.pending, 40);
        Ok(())
    }

    #[tokio::test]
    async fn test_processing_lock_serializes_work() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-1", None, 5, Some("agent-1")).await?;
        queue.add("ws-2", None, 5, Some("agent-2")).await?;

        let first = queue.next_with_lock("agent-1").await?;
        assert!(first.is_some());

        let second = queue.next_with_lock("agent-2").await?;
        assert!(second.is_none());

        let released = queue.release_processing_lock("agent-1").await?;
        assert!(released);

        let next = queue.next_with_lock("agent-2").await?;
        assert!(next.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_claim_prevents_duplicates() -> Result<()> {
        // Test that the unique constraint and atomic UPDATE...RETURNING prevent
        // multiple agents from claiming the same entry simultaneously
        let queue = MergeQueue::open_in_memory().await?;
        queue.add("ws-1", None, 5, None).await?;

        // Spawn multiple tasks trying to claim the same entry concurrently
        let queue1 = queue.clone();
        let queue2 = queue.clone();
        let queue3 = queue.clone();

        let handle1 = tokio::spawn(async move { queue1.next_with_lock("agent-1").await });
        let handle2 = tokio::spawn(async move { queue2.next_with_lock("agent-2").await });
        let handle3 = tokio::spawn(async move { queue3.next_with_lock("agent-3").await });

        let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);

        // Unwrap the JoinHandle results, then unwrap the inner Result
        let entry1 = result1
            .map_err(|e| Error::DatabaseError(format!("Task 1 join failed: {e}")))?
            .map_err(|e| Error::DatabaseError(format!("Task 1 failed: {e}")))?;
        let entry2 = result2
            .map_err(|e| Error::DatabaseError(format!("Task 2 join failed: {e}")))?
            .map_err(|e| Error::DatabaseError(format!("Task 2 failed: {e}")))?;
        let entry3 = result3
            .map_err(|e| Error::DatabaseError(format!("Task 3 join failed: {e}")))?
            .map_err(|e| Error::DatabaseError(format!("Task 3 failed: {e}")))?;

        // Exactly one agent should have claimed the entry
        let claimed_count = [entry1.as_ref(), entry2.as_ref(), entry3.as_ref()]
            .iter()
            .filter(|e| e.is_some())
            .count();

        assert_eq!(claimed_count, 1, "Exactly one agent should claim the entry");

        // Verify the claimed entry has the correct status
        let claimed = [entry1, entry2, entry3]
            .into_iter()
            .find(Option::is_some)
            .flatten();

        let entry = claimed.ok_or_else(|| {
            Error::DatabaseError("Expected at least one claimed entry after assert".to_string())
        })?;
        assert_eq!(entry.status, QueueStatus::Claimed);
        assert_eq!(entry.workspace, "ws-1");

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_logic_handles_contention() -> Result<()> {
        // Test that retry logic handles concurrent access gracefully
        let queue = MergeQueue::open_in_memory().await?;

        // Add 10 entries to create contention
        for i in 0..10 {
            queue.add(&format!("ws-{i}"), None, 5, None).await?;
        }

        // Spawn 5 agents concurrently trying to claim entries
        let mut handles = vec![];
        for i in 0..5 {
            let q = queue.clone();
            let agent_id = format!("agent-{i}");
            let handle = tokio::spawn(async move { q.next_with_lock(&agent_id).await });
            handles.push(handle);
        }

        // Wait for all agents to complete
        let mut results = vec![];
        for handle in handles {
            let result = handle
                .await
                .map_err(|e| Error::DatabaseError(format!("Task join failed: {e}")))?
                .map_err(|e| Error::DatabaseError(format!("Task failed: {e}")))?;
            results.push(result);
        }

        // Count how many agents got entries
        let claimed_count = results.iter().filter(|r| r.is_some()).count();

        // With 10 entries and retry logic, all agents should eventually get entries
        // (except those blocked by the processing lock)
        assert!(
            claimed_count <= 5,
            "Should not claim more entries than agents"
        );

        // Verify no duplicate assignments (no workspace claimed twice)
        let claimed_workspaces: Vec<_> = results
            .iter()
            .filter_map(|e| e.as_ref().map(|entry| entry.workspace.clone()))
            .collect();

        let unique_workspaces: std::collections::HashSet<_> = claimed_workspaces.iter().collect();

        assert_eq!(
            claimed_workspaces.len(),
            unique_workspaces.len(),
            "No workspace should be claimed twice"
        );

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE MACHINE TRANSITION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    // --- Valid Happy Path Transitions ---

    #[test]
    fn test_pending_to_claimed_is_valid() {
        assert!(QueueStatus::Pending.can_transition_to(QueueStatus::Claimed));
        assert!(QueueStatus::Pending.validate_transition(QueueStatus::Claimed).is_ok());
    }

    #[test]
    fn test_claimed_to_rebasing_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::Rebasing));
        assert!(QueueStatus::Claimed.validate_transition(QueueStatus::Rebasing).is_ok());
    }

    #[test]
    fn test_rebasing_to_testing_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::Testing));
        assert!(QueueStatus::Rebasing.validate_transition(QueueStatus::Testing).is_ok());
    }

    #[test]
    fn test_testing_to_ready_to_merge_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::ReadyToMerge));
        assert!(QueueStatus::Testing.validate_transition(QueueStatus::ReadyToMerge).is_ok());
    }

    #[test]
    fn test_ready_to_merge_to_merging_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::Merging));
        assert!(QueueStatus::ReadyToMerge.validate_transition(QueueStatus::Merging).is_ok());
    }

    #[test]
    fn test_merging_to_merged_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::Merged));
        assert!(QueueStatus::Merging.validate_transition(QueueStatus::Merged).is_ok());
    }

    // --- Valid Failure Transitions ---

    #[test]
    fn test_claimed_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_rebasing_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_testing_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_ready_to_merge_to_failed_retryable_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_merging_to_failed_retryable_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_claimed_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_rebasing_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_testing_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_ready_to_merge_to_failed_terminal_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_merging_to_failed_terminal_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::FailedTerminal));
    }

    // --- Valid Cancel Transitions ---

    #[test]
    fn test_pending_to_cancelled_is_valid() {
        assert!(QueueStatus::Pending.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_claimed_to_cancelled_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_rebasing_to_cancelled_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_testing_to_cancelled_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_ready_to_merge_to_cancelled_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_failed_retryable_to_cancelled_is_valid() {
        assert!(QueueStatus::FailedRetryable.can_transition_to(QueueStatus::Cancelled));
    }

    // --- Valid Retry Path ---

    #[test]
    fn test_failed_retryable_to_pending_is_valid() {
        assert!(QueueStatus::FailedRetryable.can_transition_to(QueueStatus::Pending));
    }

    // --- Idempotent Transitions (same state) ---

    #[test]
    fn test_same_state_transition_is_always_valid() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            assert!(
                status.can_transition_to(status),
                "{status:?} should be able to transition to itself"
            );
        }
    }

    // --- Terminal State Tests ---

    #[test]
    fn test_merged_is_terminal() {
        assert!(QueueStatus::Merged.is_terminal());
    }

    #[test]
    fn test_failed_terminal_is_terminal() {
        assert!(QueueStatus::FailedTerminal.is_terminal());
    }

    #[test]
    fn test_cancelled_is_terminal() {
        assert!(QueueStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_non_terminal_states_are_not_terminal() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
        ] {
            assert!(
                !status.is_terminal(),
                "{status:?} should not be terminal"
            );
        }
    }

    // --- Invalid Transition Edge Cases ---

    #[test]
    fn test_merged_cannot_transition_to_anything() {
        let terminal = QueueStatus::Merged;
        for target in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            assert!(
                !terminal.can_transition_to(target),
                "Merged should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn test_failed_terminal_cannot_transition_to_anything() {
        let terminal = QueueStatus::FailedTerminal;
        for target in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::Cancelled,
        ] {
            assert!(
                !terminal.can_transition_to(target),
                "FailedTerminal should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn test_cancelled_cannot_transition_to_anything() {
        let terminal = QueueStatus::Cancelled;
        for target in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
        ] {
            assert!(
                !terminal.can_transition_to(target),
                "Cancelled should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn test_pending_cannot_skip_to_testing() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Testing));
    }

    #[test]
    fn test_pending_cannot_skip_to_ready_to_merge() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::ReadyToMerge));
    }

    #[test]
    fn test_pending_cannot_skip_to_merging() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_pending_cannot_go_directly_to_merged() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::Merged));
    }

    #[test]
    fn test_pending_cannot_go_to_failed_retryable_directly() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::FailedRetryable));
    }

    #[test]
    fn test_pending_cannot_go_to_failed_terminal_directly() {
        assert!(!QueueStatus::Pending.can_transition_to(QueueStatus::FailedTerminal));
    }

    #[test]
    fn test_claimed_cannot_skip_to_ready_to_merge() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::ReadyToMerge));
    }

    #[test]
    fn test_claimed_cannot_skip_to_merging() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_claimed_cannot_go_directly_to_merged() {
        assert!(!QueueStatus::Claimed.can_transition_to(QueueStatus::Merged));
    }

    #[test]
    fn test_failed_terminal_cannot_retry() {
        assert!(!QueueStatus::FailedTerminal.can_transition_to(QueueStatus::Pending));
    }

    #[test]
    fn test_cancelled_cannot_retry() {
        assert!(!QueueStatus::Cancelled.can_transition_to(QueueStatus::Pending));
    }

    #[test]
    fn test_merged_cannot_be_cancelled() {
        assert!(!QueueStatus::Merged.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_failed_terminal_cannot_be_cancelled() {
        assert!(!QueueStatus::FailedTerminal.can_transition_to(QueueStatus::Cancelled));
    }

    #[test]
    fn test_rebasing_cannot_skip_to_merging() {
        assert!(!QueueStatus::Rebasing.can_transition_to(QueueStatus::Merging));
    }

    #[test]
    fn test_testing_cannot_skip_to_merging() {
        assert!(!QueueStatus::Testing.can_transition_to(QueueStatus::Merging));
    }

    // --- Transition Error Tests ---

    #[test]
    fn test_transition_error_contains_from_and_to() -> Result<()> {
        let err = QueueStatus::Merged.validate_transition(QueueStatus::Pending);
        assert!(err.is_err());
        let transition_err = err.err();
        assert!(transition_err.is_some());
        let err = transition_err.ok_or_else(|| Error::DatabaseError("expected error".into()))?;
        assert_eq!(err.from, QueueStatus::Merged);
        assert_eq!(err.to, QueueStatus::Pending);
        Ok(())
    }

    #[test]
    fn test_transition_error_display() {
        let err = TransitionError {
            from: QueueStatus::Merged,
            to: QueueStatus::Pending,
        };
        let display = err.to_string();
        assert!(display.contains("merged"));
        assert!(display.contains("pending"));
        assert!(display.contains("invalid state transition"));
    }

    // --- Display and FromStr Tests ---

    #[test]
    fn test_status_display_roundtrip() -> Result<()> {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            let s = status.to_string();
            let parsed = QueueStatus::from_str(&s);
            assert!(
                parsed.is_ok(),
                "Failed to parse '{s}' back to QueueStatus"
            );
            assert_eq!(parsed?, status);
        }
        Ok(())
    }

    #[test]
    fn test_as_str_matches_display() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::Claimed,
            QueueStatus::Rebasing,
            QueueStatus::Testing,
            QueueStatus::ReadyToMerge,
            QueueStatus::Merging,
            QueueStatus::Merged,
            QueueStatus::FailedRetryable,
            QueueStatus::FailedTerminal,
            QueueStatus::Cancelled,
        ] {
            assert_eq!(status.as_str(), status.to_string());
        }
    }

    #[test]
    fn test_backward_compat_processing_maps_to_claimed() -> Result<()> {
        let status = QueueStatus::from_str("processing")?;
        assert_eq!(status, QueueStatus::Claimed);
        Ok(())
    }

    #[test]
    fn test_backward_compat_completed_maps_to_merged() -> Result<()> {
        let status = QueueStatus::from_str("completed")?;
        assert_eq!(status, QueueStatus::Merged);
        Ok(())
    }

    #[test]
    fn test_backward_compat_failed_maps_to_failed_terminal() -> Result<()> {
        let status = QueueStatus::from_str("failed")?;
        assert_eq!(status, QueueStatus::FailedTerminal);
        Ok(())
    }

    #[test]
    fn test_invalid_status_string_returns_error() {
        let result = QueueStatus::from_str("invalid_status");
        assert!(result.is_err());
    }

    // --- TryFrom<String> Tests ---

    #[test]
    fn test_try_from_string_valid() -> Result<()> {
        let status = QueueStatus::try_from("pending".to_string());
        assert!(status.is_ok());
        assert_eq!(status?, QueueStatus::Pending);
        Ok(())
    }

    #[test]
    fn test_try_from_string_invalid() {
        let result = QueueStatus::try_from("not_a_status".to_string());
        assert!(result.is_err());
    }
}

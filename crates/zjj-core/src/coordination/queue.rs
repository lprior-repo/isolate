//! Merge queue for sequential multi-agent coordination.
//!
//! This module contains infrastructure layer code (DB operations).
//! Domain logic (state machine) is in `queue_status.rs`.
//! `SQLx` entities are in `queue_entities.rs`.

use std::{path::Path, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use thiserror::Error;
use tokio::time::sleep;

// Re-export sqlx entities from queue_entities.rs for backward compatibility
pub use super::queue_entities::{ProcessingLock, QueueEntry, QueueEvent};
use super::queue_repository::QueueRepository;
// Re-export domain types from queue_status.rs for backward compatibility
pub use super::queue_status::{QueueEventType, QueueStatus, TransitionError, WorkspaceQueueState};
use crate::{Error, Result};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE CONTROL ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for queue control operations (retry/cancel).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum QueueControlError {
    /// Entry not found in queue.
    #[error("queue entry not found: {0}")]
    NotFound(i64),

    /// Entry is not in a retryable state.
    #[error("entry {id} is not retryable (current status: {status})")]
    NotRetryable { id: i64, status: QueueStatus },

    /// Entry is in a terminal state and cannot be cancelled.
    #[error("entry {id} is in terminal state and cannot be cancelled (current status: {status})")]
    NotCancellable { id: i64, status: QueueStatus },

    /// Maximum retry attempts exceeded.
    #[error("entry {id} has exceeded maximum retry attempts ({attempt_count}/{max_attempts})")]
    MaxAttemptsExceeded {
        id: i64,
        attempt_count: i32,
        max_attempts: i32,
    },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE ENTRY STRUCTS (Infrastructure Layer - pure Rust, no sqlx)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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

/// Statistics from automatic stale lock and entry recovery.
///
/// Returned by `detect_and_recover_stale()` and `get_recovery_stats()`
/// to provide visibility into self-healing operations.
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// Number of expired processing locks cleaned
    pub locks_cleaned: usize,
    /// Number of stale claimed entries reset to pending
    pub entries_reclaimed: usize,
    /// Unix timestamp when recovery was performed
    pub recovery_timestamp: i64,
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
    /// Create an in-memory merge queue for testing with a custom lock timeout.
    ///
    /// The lock timeout determines how long a worker can hold a lock before
    /// it's considered stale. Shorter timeouts are useful for testing
    /// automatic recovery behavior.
    pub async fn open_in_memory_with_timeout(lock_timeout_secs: i64) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to open in-memory database: {e}")))?;

        let queue = Self {
            pool,
            lock_timeout_secs,
        };
        queue.init_schema().await?;
        Ok(queue)
    }

    /// discarded when the queue is dropped. Useful for testing and
    /// development without persisting state to disk.
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_timeout(Self::DEFAULT_LOCK_TIMEOUT_SECS).await
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

    /// Get a reference to the underlying database pool.
    /// This is intended for testing purposes to allow direct SQL manipulation.
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
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
                state_changed_at INTEGER,
                head_sha TEXT,
                tested_against_sha TEXT,
                attempt_count INTEGER DEFAULT 0,
                max_attempts INTEGER DEFAULT 3,
                rebase_count INTEGER DEFAULT 0,
                last_rebase_at INTEGER,
                parent_workspace TEXT
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create merge_queue table: {e}")))?;

        self.migrate_merge_queue_columns().await?;
        self.create_merge_queue_indexes().await?;

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

        // Create queue_events table for audit trail
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS queue_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                queue_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                details_json TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (queue_id) REFERENCES merge_queue(id)
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create queue_events table: {e}")))?;

        // Create index on queue_id for efficient event lookups
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_queue_events_queue_id ON queue_events(queue_id)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create queue_events index: {e}")))?;

        Ok(())
    }

    async fn migrate_merge_queue_columns(&self) -> Result<()> {
        // SQLite doesn't support "IF NOT EXISTS" for ALTER TABLE, so we check schema first.
        // If this introspection fails, fail init to avoid running with a partially migrated DB.
        let _table_sql = sqlx::query_scalar::<_, String>(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='merge_queue'",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to check schema for migration: {e}")))?
        .ok_or_else(|| {
            Error::DatabaseError(
                "merge_queue table not found in sqlite_master during migration".to_string(),
            )
        })?;

        let migrations = [
            (
                "dedupe_key",
                "ALTER TABLE merge_queue ADD COLUMN dedupe_key TEXT",
            ),
            (
                "workspace_state",
                "ALTER TABLE merge_queue ADD COLUMN workspace_state TEXT DEFAULT 'created'",
            ),
            (
                "previous_state",
                "ALTER TABLE merge_queue ADD COLUMN previous_state TEXT",
            ),
            (
                "state_changed_at",
                "ALTER TABLE merge_queue ADD COLUMN state_changed_at INTEGER",
            ),
            (
                "head_sha",
                "ALTER TABLE merge_queue ADD COLUMN head_sha TEXT",
            ),
            (
                "tested_against_sha",
                "ALTER TABLE merge_queue ADD COLUMN tested_against_sha TEXT",
            ),
            (
                "attempt_count",
                "ALTER TABLE merge_queue ADD COLUMN attempt_count INTEGER DEFAULT 0",
            ),
            (
                "max_attempts",
                "ALTER TABLE merge_queue ADD COLUMN max_attempts INTEGER DEFAULT 3",
            ),
            (
                "rebase_count",
                "ALTER TABLE merge_queue ADD COLUMN rebase_count INTEGER DEFAULT 0",
            ),
            (
                "last_rebase_at",
                "ALTER TABLE merge_queue ADD COLUMN last_rebase_at INTEGER",
            ),
            (
                "parent_workspace",
                "ALTER TABLE merge_queue ADD COLUMN parent_workspace TEXT",
            ),
        ];

        for (column_name, alter_sql) in migrations {
            if self.merge_queue_column_exists(column_name).await? {
                continue;
            }

            match sqlx::query(alter_sql).execute(&self.pool).await {
                Ok(_) => {}
                Err(e) => {
                    let error_text = e.to_string();
                    if error_text.contains("duplicate column name") {
                        continue;
                    }

                    return Err(Error::DatabaseError(format!(
                        "Failed to add {column_name} column: {e}"
                    )));
                }
            }
        }

        Ok(())
    }

    async fn merge_queue_column_exists(&self, column_name: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM pragma_table_info('merge_queue') WHERE name = ?1 LIMIT 1",
        )
        .bind(column_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to inspect merge_queue columns during migration: {e}"
            ))
        })?
        .is_some();

        Ok(exists)
    }

    async fn create_merge_queue_indexes(&self) -> Result<()> {
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_merge_queue_status ON merge_queue(status)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create status index: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_merge_queue_priority_added ON merge_queue(priority, added_at)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create priority index: {e}")))?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_merge_queue_processing
             ON merge_queue(workspace, started_at)
             WHERE status = 'processing' AND started_at IS NOT NULL",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create processing index: {e}")))?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_merge_queue_dedupe_key_active
             ON merge_queue(dedupe_key)
             WHERE dedupe_key IS NOT NULL
               AND status NOT IN ('merged', 'failed_terminal', 'cancelled')",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create dedupe_key index: {e}")))?;

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
            "CREATE INDEX IF NOT EXISTS idx_merge_queue_parent_workspace
             ON merge_queue(parent_workspace)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to create parent_workspace index: {e}"))
        })?;

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
    /// The `dedupe_key` allows preventing duplicate work by rejecting entries
    /// with duplicate keys. NULL `dedupe_keys` are allowed multiple times.
    #[allow(clippy::too_many_arguments)]
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

    /// Idempotent upsert for submit operations.
    ///
    /// This method implements the submit contract:
    /// 1. If no entry with `dedupe_key` exists, create new pending entry
    /// 2. If entry with `dedupe_key` is active (non-terminal) and workspace matches, UPDATE it
    /// 3. If entry with `dedupe_key` is active (non-terminal) and workspace differs, return error
    /// 4. If entry with `dedupe_key` is terminal:
    ///    - If same workspace: RESET to pending (resubmit scenario)
    ///    - If different workspace: release `dedupe_key` from terminal, INSERT new
    ///
    /// # Arguments
    /// * `workspace` - The workspace name
    /// * `bead_id` - Optional bead identifier
    /// * `priority` - Queue priority (lower = higher priority)
    /// * `agent_id` - Optional agent identifier
    /// * `dedupe_key` - Deduplication key (required for idempotent behavior)
    /// * `head_sha` - The current HEAD SHA to store/update
    ///
    /// # Returns
    /// The created or updated `QueueEntry`
    ///
    /// # Errors
    /// Returns an error if the database operation fails, if workspace uniqueness is violated,
    /// or if an active entry with a different workspace already has this `dedupe_key`
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_for_submit(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: &str,
        head_sha: &str,
    ) -> Result<QueueEntry> {
        let now = Self::now();

        // First, check if there's an existing entry with this dedupe_key
        let existing = self.find_by_dedupe_key(dedupe_key).await?;

        match existing {
            // Case 1: No existing entry - INSERT new
            None => {
                self.insert_new_entry(
                    workspace, bead_id, priority, agent_id, dedupe_key, head_sha, now,
                )
                .await
            }

            // Case 2: Existing entry exists
            Some(entry) => {
                if entry.status.is_terminal() {
                    if entry.workspace == workspace {
                        // Case 2a-i: Terminal entry with same workspace - RESET to pending
                        // This handles the "resubmit after terminal" scenario
                        self.reset_terminal_to_pending(entry.id, bead_id, priority, head_sha, now)
                            .await
                    } else {
                        // Case 2a-ii: Terminal entry with different workspace
                        // Release its dedupe_key and INSERT new entry
                        self.release_dedupe_key(entry.id).await?;
                        self.insert_new_entry(
                            workspace, bead_id, priority, agent_id, dedupe_key, head_sha, now,
                        )
                        .await
                    }
                } else if entry.workspace == workspace {
                    // Case 2b: Active entry with matching workspace - UPDATE in place
                    self.update_active_entry(entry.id, head_sha, now).await
                } else {
                    // Case 2c: Active entry with DIFFERENT workspace - reject
                    Err(Error::DedupeKeyConflict {
                        dedupe_key: dedupe_key.to_string(),
                        existing_workspace: entry.workspace.clone(),
                        provided_workspace: workspace.to_string(),
                    })
                }
            }
        }
    }

    /// Reset a terminal entry back to pending status for resubmission.
    #[allow(clippy::too_many_arguments)]
    async fn reset_terminal_to_pending(
        &self,
        id: i64,
        bead_id: Option<&str>,
        priority: i32,
        head_sha: &str,
        now: i64,
    ) -> Result<QueueEntry> {
        sqlx::query(
            "UPDATE merge_queue SET status = 'pending', bead_id = ?1, priority = ?2, \
                 head_sha = ?3, state_changed_at = ?4, started_at = NULL, completed_at = NULL, \
                 error_message = NULL WHERE id = ?5",
        )
        .bind(bead_id)
        .bind(priority)
        .bind(head_sha)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to reset terminal entry: {e}")))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve reset entry".to_string()))
    }

    /// Release (clear) the `dedupe_key` from an entry, making it available for reuse.
    /// This is used when a terminal entry needs to allow a new entry with the same `dedupe_key`.
    async fn release_dedupe_key(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE merge_queue SET dedupe_key = NULL WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to release dedupe_key: {e}")))?;
        Ok(())
    }

    /// Find an entry by `dedupe_key`.
    async fn find_by_dedupe_key(&self, dedupe_key: &str) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue WHERE dedupe_key = ?1 \
                 ORDER BY id DESC LIMIT 1",
        )
        .bind(dedupe_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to find entry by dedupe_key: {e}")))
    }

    /// Insert a new queue entry.
    #[allow(clippy::too_many_arguments)]
    async fn insert_new_entry(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: &str,
        head_sha: &str,
        now: i64,
    ) -> Result<QueueEntry> {
        let result = sqlx::query(
            "INSERT INTO merge_queue (workspace, bead_id, priority, status, added_at, agent_id, dedupe_key, workspace_state, state_changed_at, head_sha) \
                 VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, 'created', ?4, ?7)",
        )
        .bind(workspace)
        .bind(bead_id)
        .bind(priority)
        .bind(now)
        .bind(agent_id)
        .bind(dedupe_key)
        .bind(head_sha)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("UNIQUE constraint failed: merge_queue.workspace") {
                Error::InvalidConfig(format!("Workspace '{workspace}' is already in the queue"))
            } else if error_str.contains("idx_merge_queue_dedupe_key")
                || error_str.contains("UNIQUE constraint failed")
            {
                Error::InvalidConfig(format!(
                    "An active entry with dedupe_key '{dedupe_key}' already exists in the queue"
                ))
            } else {
                Error::DatabaseError(format!("Failed to upsert entry: {e}"))
            }
        })?;

        let id = result.last_insert_rowid();
        self.get_by_id(id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve upserted entry".to_string()))
    }

    /// Update an active entry with new `head_sha` and timestamp.
    async fn update_active_entry(&self, id: i64, head_sha: &str, now: i64) -> Result<QueueEntry> {
        sqlx::query("UPDATE merge_queue SET head_sha = ?1, state_changed_at = ?2 WHERE id = ?3")
            .bind(head_sha)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update active entry: {e}")))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve updated entry".to_string()))
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue WHERE id = ?1",
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
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue WHERE workspace = ?1",
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
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue WHERE status = ?1 \
                 ORDER BY priority ASC, added_at ASC"
            }
            None => {
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id, dedupe_key, workspace_state, \
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue \
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
                 previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts, rebase_count, last_rebase_at FROM merge_queue WHERE status = 'pending' \
                 ORDER BY priority ASC, added_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get next entry: {e}")))
    }

    pub async fn remove(&self, workspace: &str) -> Result<bool> {
        // First delete related queue_events to avoid FK constraint violation
        sqlx::query(
            "DELETE FROM queue_events WHERE queue_id = (SELECT id FROM merge_queue WHERE workspace = ?1)",
        )
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to remove queue events: {e}")))?;

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
                    "claimed" | "rebasing" | "testing" | "ready_to_merge" | "merging" => {
                        acc.processing += cnt;
                    }
                    "completed" => acc.completed = cnt,
                    "failed" | "failed_retryable" | "failed_terminal" | "cancelled" => {
                        acc.failed += cnt;
                    }
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
                 WHERE workspace = ?2 AND status IN ('claimed', 'processing')",
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
                 WHERE workspace = ?3 AND status IN ('claimed', 'processing')",
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

            // Automatic recovery before claim attempt (bd-2i5)
            // Best-effort - failures logged but don't prevent claim
            if let Err(e) = self.detect_and_recover_stale().await {
                eprintln!("Warning: Automatic recovery failed: {e}");
                // Continue anyway - lock acquisition may still succeed
            }

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
             SET status = 'claimed',
                 started_at = ?1,
                 agent_id = ?2,
                 state_changed_at = ?1,
                 previous_state = 'pending'
             WHERE id = (
                 SELECT id FROM merge_queue
                 WHERE status = 'pending'
                 ORDER BY priority ASC, added_at ASC
                 LIMIT 1
             )
             RETURNING id, workspace, bead_id, priority, status, added_at, started_at,
                       completed_at, error_message, agent_id, dedupe_key, workspace_state,
                       previous_state, state_changed_at, head_sha, tested_against_sha, attempt_count, max_attempts",
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

        match entry {
            Some(ref e) => {
                // Emit audit event for the claim (pending -> claimed)
                let event_details =
                    format!(r#"{{"from": "pending", "to": "claimed", "agent": "{agent_id}"}}"#);
                let _ = self
                    .append_typed_event(e.id, QueueEventType::Claimed, Some(&event_details))
                    .await;
            }
            None => {
                // No entry was found, release the lock
                let _ = self.release_processing_lock(agent_id).await;
            }
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
        // Terminal statuses that should be cleaned up:
        // - merged: successfully merged entries
        // - failed_terminal: unrecoverable failures
        // - cancelled: manually cancelled entries
        // Note: 'completed' and 'failed' are legacy aliases for 'merged' and 'failed_terminal'
        const TERMINAL_STATUSES: &str =
            "'merged', 'failed_terminal', 'cancelled', 'completed', 'failed'";

        // First delete related queue_events to avoid FK constraint violation
        if max_age.is_zero() {
            sqlx::query(&format!(
                "DELETE FROM queue_events WHERE queue_id IN \
                     (SELECT id FROM merge_queue WHERE status IN ({TERMINAL_STATUSES}))"
            ))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup queue events: {e}")))?;
        } else {
            let cutoff = Self::now() - max_age.as_secs() as i64;
            sqlx::query(&format!(
                "DELETE FROM queue_events WHERE queue_id IN \
                     (SELECT id FROM merge_queue WHERE status IN ({TERMINAL_STATUSES}) \
                     AND completed_at <= ?1)"
            ))
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup queue events: {e}")))?;
        }

        let result = if max_age.is_zero() {
            sqlx::query(&format!(
                "DELETE FROM merge_queue WHERE status IN ({TERMINAL_STATUSES})"
            ))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?
        } else {
            let cutoff = Self::now() - max_age.as_secs() as i64;
            sqlx::query(&format!(
                "DELETE FROM merge_queue WHERE status IN ({TERMINAL_STATUSES}) \
                     AND completed_at <= ?1"
            ))
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?
        };
        Ok(result.rows_affected() as usize)
    }

    /// Reclaim stale entries that have been claimed but whose lease has expired.
    ///
    /// This resets entries with `status = 'claimed'` back to `pending`
    /// if they were claimed before the stale threshold. This is typically called
    /// when a worker starts up to recover work from crashed/disconnected workers.
    ///
    /// # Arguments
    /// * `stale_threshold_secs` - How long (in seconds) before a claimed entry is considered stale
    ///
    /// # Returns
    /// The number of entries that were reclaimed.
    #[allow(clippy::cast_sign_loss)]
    pub async fn reclaim_stale(&self, stale_threshold_secs: i64) -> Result<usize> {
        let cutoff = Self::now() - stale_threshold_secs;

        // Also release any expired processing locks
        sqlx::query("DELETE FROM queue_processing_lock WHERE expires_at < ?1")
            .bind(Self::now())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to release expired locks: {e}")))?;

        // Reset stale claimed entries back to pending
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'pending', started_at = NULL, agent_id = NULL, state_changed_at = ?1 \
             WHERE status = 'claimed' AND started_at IS NOT NULL AND started_at < ?2",
        )
        .bind(Self::now())
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to reclaim stale entries: {e}")))?;

        Ok(result.rows_affected() as usize)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // AUTOMATIC SELF-HEALING (bd-2i5)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Detect and automatically recover stale locks and entries.
    ///
    /// This method is called automatically by `next_with_lock()` before attempting
    /// to claim work, but can also be called explicitly for monitoring or
    /// maintenance purposes.
    ///
    /// # Recovery Operations
    ///
    /// 1. Deletes expired processing locks from `queue_processing_lock`
    /// 2. Resets stale claimed entries back to `pending` state
    ///
    /// An entry is considered stale if:
    /// - `status = 'claimed'`
    /// - `started_at IS NOT NULL`
    /// - `started_at < now - lock_timeout_secs`
    ///
    /// # Returns
    ///
    /// Statistics about recovery actions performed, including counts of
    /// locks cleaned and entries reclaimed.
    ///
    /// # Errors
    ///
    /// Returns a database error if the cleanup queries fail.
    ///
    /// # Idempotence
    ///
    /// This method is idempotent - calling it multiple times is safe.
    /// After the first call reclaims stale entries, subsequent calls will
    /// report zero entries reclaimed.
    #[allow(clippy::cast_sign_loss)]
    pub async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
        let now = Self::now();

        // Delete expired processing locks
        let locks_cleaned = sqlx::query("DELETE FROM queue_processing_lock WHERE expires_at < ?1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to clean expired locks: {e}")))?
            .rows_affected();

        // Reset stale claimed entries to pending
        let cutoff = now - self.lock_timeout_secs;
        let entries_reclaimed = sqlx::query(
            "UPDATE merge_queue
             SET status = 'pending',
                 started_at = NULL,
                 agent_id = NULL,
                 state_changed_at = ?1
             WHERE status = 'claimed'
               AND started_at IS NOT NULL
               AND started_at < ?2",
        )
        .bind(now)
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to reclaim stale entries: {e}")))?
        .rows_affected();

        Ok(RecoveryStats {
            locks_cleaned: locks_cleaned as usize,
            entries_reclaimed: entries_reclaimed as usize,
            recovery_timestamp: now,
        })
    }

    /// Get recovery statistics without performing recovery.
    ///
    /// This method counts expired locks and stale entries but does not
    /// modify the database. It's useful for monitoring and health checks
    /// where you want to observe system state without side effects.
    ///
    /// # Returns
    ///
    /// Statistics showing the count of expired locks and stale entries
    /// that would be reclaimed if `detect_and_recover_stale()` were called.
    ///
    /// # Errors
    ///
    /// Returns a database error if the count queries fail.
    #[allow(clippy::cast_sign_loss)]
    pub async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
        let now = Self::now();

        // Count expired locks (without deleting)
        let locks_cleaned: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM queue_processing_lock WHERE expires_at < ?1")
                .bind(now)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| Error::DatabaseError(format!("Failed to count expired locks: {e}")))?;

        // Count stale entries (without reclaiming)
        let cutoff = now - self.lock_timeout_secs;
        let entries_reclaimed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM merge_queue
             WHERE status = 'claimed'
               AND started_at IS NOT NULL
               AND started_at < ?1",
        )
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to count stale entries: {e}")))?;

        Ok(RecoveryStats {
            locks_cleaned: locks_cleaned as usize,
            entries_reclaimed: entries_reclaimed as usize,
            recovery_timestamp: now,
        })
    }

    /// Check if the processing lock is stale (expired).
    ///
    /// Returns `true` if a lock exists and its `expires_at` timestamp
    /// is in the past. Returns `false` if no lock exists or if the lock
    /// is still valid.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - Lock exists and is expired
    /// - `Ok(false)` - No lock exists or lock is still valid
    /// - `Err(_)` - Database error
    ///
    /// # Errors
    ///
    /// Returns a database error if unable to query the lock state.
    pub async fn is_lock_stale(&self) -> Result<bool> {
        let lock = self.get_processing_lock().await?;
        let now = Self::now();

        Ok(match lock {
            Some(l) => l.expires_at < now,
            None => false,
        })
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // CONTROL OPERATIONS (Retry & Cancel)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Retry a `failed_retryable` entry.
    ///
    /// Moves the entry from `failed_retryable` to `pending` and increments
    /// the attempt count.
    ///
    /// # Errors
    ///
    /// Returns `QueueControlError::NotFound` if the entry does not exist.
    /// Returns `QueueControlError::NotRetryable` if the entry is not in `failed_retryable` state.
    /// Returns `QueueControlError::MaxAttemptsExceeded` if the attempt count has reached
    /// `max_attempts`.
    pub async fn retry_entry(&self, id: i64) -> std::result::Result<QueueEntry, QueueControlError> {
        // Fetch the entry first
        let entry = self
            .get_by_id(id)
            .await
            .map_err(|_| QueueControlError::NotFound(id))?
            .ok_or(QueueControlError::NotFound(id))?;

        // Validate it's in failed_retryable state
        if entry.status != QueueStatus::FailedRetryable {
            return Err(QueueControlError::NotRetryable {
                id,
                status: entry.status,
            });
        }

        // Validate attempt_count < max_attempts
        // Note: attempt_count tracks how many times we've TRIED, not how many retries
        // So if attempt_count == max_attempts, we've exhausted our attempts
        let new_attempt_count = entry.attempt_count + 1;
        if new_attempt_count > entry.max_attempts {
            return Err(QueueControlError::MaxAttemptsExceeded {
                id,
                attempt_count: entry.attempt_count,
                max_attempts: entry.max_attempts,
            });
        }

        let now = Self::now();

        // Transition to pending and increment attempt_count.
        // Guard on both status and attempt_count so concurrent retries cannot
        // both succeed for the same entry.
        let update_result = sqlx::query(
            "UPDATE merge_queue SET status = 'pending', attempt_count = ?1, state_changed_at = ?2, \
                 started_at = NULL, completed_at = NULL, error_message = NULL \
                 WHERE id = ?3 AND status = 'failed_retryable' AND attempt_count = ?4",
        )
        .bind(new_attempt_count)
        .bind(now)
        .bind(id)
        .bind(entry.attempt_count)
        .execute(&self.pool)
        .await
        .map_err(|_| QueueControlError::NotFound(id))?;

        if update_result.rows_affected() == 0 {
            let current = self
                .get_by_id(id)
                .await
                .map_err(|_| QueueControlError::NotFound(id))?
                .ok_or(QueueControlError::NotFound(id))?;

            if current.status != QueueStatus::FailedRetryable {
                return Err(QueueControlError::NotRetryable {
                    id,
                    status: current.status,
                });
            }

            if current.attempt_count >= current.max_attempts {
                return Err(QueueControlError::MaxAttemptsExceeded {
                    id,
                    attempt_count: current.attempt_count,
                    max_attempts: current.max_attempts,
                });
            }

            return Err(QueueControlError::NotFound(id));
        }

        // Fetch and return the updated entry
        self.get_by_id(id)
            .await
            .map_err(|_| QueueControlError::NotFound(id))?
            .ok_or(QueueControlError::NotFound(id))
    }

    /// Cancel an active (non-terminal) entry.
    ///
    /// Moves the entry to `cancelled` state. Only non-terminal entries can be cancelled.
    ///
    /// # Errors
    ///
    /// Returns `QueueControlError::NotFound` if the entry does not exist.
    /// Returns `QueueControlError::NotCancellable` if the entry is in a terminal state
    /// (merged, `failed_terminal`, or cancelled).
    pub async fn cancel_entry(
        &self,
        id: i64,
    ) -> std::result::Result<QueueEntry, QueueControlError> {
        // Fetch the entry first
        let entry = self
            .get_by_id(id)
            .await
            .map_err(|_| QueueControlError::NotFound(id))?
            .ok_or(QueueControlError::NotFound(id))?;

        // Validate it's not in a terminal state
        if entry.status.is_terminal() {
            return Err(QueueControlError::NotCancellable {
                id,
                status: entry.status,
            });
        }

        let now = Self::now();

        // Transition to cancelled.
        // Guard on current status to prevent concurrent double-cancel from both succeeding.
        let update_result = sqlx::query(
            "UPDATE merge_queue SET status = 'cancelled', state_changed_at = ?1, completed_at = ?1 \
                 WHERE id = ?2 AND status NOT IN ('merged', 'failed_terminal', 'cancelled')",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|_| QueueControlError::NotFound(id))?;

        if update_result.rows_affected() == 0 {
            let current = self
                .get_by_id(id)
                .await
                .map_err(|_| QueueControlError::NotFound(id))?
                .ok_or(QueueControlError::NotFound(id))?;

            if current.status.is_terminal() {
                return Err(QueueControlError::NotCancellable {
                    id,
                    status: current.status,
                });
            }

            return Err(QueueControlError::NotFound(id));
        }

        // Fetch and return the updated entry
        self.get_by_id(id)
            .await
            .map_err(|_| QueueControlError::NotFound(id))?
            .ok_or(QueueControlError::NotFound(id))
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EVENT AUDIT TRAIL
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Append an event to the `queue_events` table.
    ///
    /// This is best-effort - failure should be logged but not fail the caller.
    /// Events are for audit trail purposes and are not critical path.
    ///
    /// # Arguments
    /// * `queue_id` - The queue entry ID this event belongs to
    /// * `event_type` - The type of event (e.g., "created", "claimed", "transitioned")
    /// * `details` - Optional JSON string with additional event details
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the database operation fails.
    pub async fn append_event(
        &self,
        queue_id: i64,
        event_type: &str,
        details: Option<&str>,
    ) -> Result<()> {
        let now = Self::now();

        sqlx::query(
            "INSERT INTO queue_events (queue_id, event_type, details_json, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(queue_id)
        .bind(event_type)
        .bind(details)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to append event: {e}")))?;

        Ok(())
    }

    /// Append a typed event to the `queue_events` table.
    ///
    /// This is a convenience wrapper around `append_event` that accepts a `QueueEventType`.
    pub async fn append_typed_event(
        &self,
        queue_id: i64,
        event_type: QueueEventType,
        details: Option<&str>,
    ) -> Result<()> {
        self.append_event(queue_id, event_type.as_str(), details)
            .await
    }

    /// Fetch all events for a queue entry, ordered by id ascending.
    ///
    /// Returns an empty vector if no events exist for the given `queue_id`.
    pub async fn fetch_events(&self, queue_id: i64) -> Result<Vec<QueueEvent>> {
        sqlx::query_as::<_, QueueEvent>(
            "SELECT id, queue_id, event_type, details_json, created_at \
             FROM queue_events \
             WHERE queue_id = ?1 \
             ORDER BY id ASC",
        )
        .bind(queue_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to fetch events: {e}")))
    }

    /// Fetch recent events (limit N) for a queue entry, ordered by id ascending.
    ///
    /// Returns an empty vector if no events exist for the given `queue_id`.
    /// If limit is 0, returns an empty vector.
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_wrap)]
    pub async fn fetch_recent_events(
        &self,
        queue_id: i64,
        limit: usize,
    ) -> Result<Vec<QueueEvent>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, QueueEvent>(
            "SELECT id, queue_id, event_type, details_json, created_at \
             FROM queue_events \
             WHERE queue_id = ?1 \
             ORDER BY id DESC \
             LIMIT ?2",
        )
        .bind(queue_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map(|mut events| {
            // Reverse to get chronological order (oldest first)
            events.reverse();
            events
        })
        .map_err(|e| Error::DatabaseError(format!("Failed to fetch recent events: {e}")))
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // STATE TRANSITION METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Transition a queue entry to a new status with validation and audit logging.
    ///
    /// This method validates that the transition is legal according to the state
    /// machine defined in `QueueStatus::validate_transition`. If the transition
    /// is valid, it updates the entry's status and emits an audit event.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name of the entry to transition
    /// * `new_status` - The target status to transition to
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err(Error::NotFound)` if the workspace is not in the queue
    /// - `Err(Error::InvalidConfig)` if the transition is invalid
    /// - `Err(Error::DatabaseError)` if the database operation fails
    pub async fn transition_to(&self, workspace: &str, new_status: QueueStatus) -> Result<()> {
        // Fetch the current entry
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        // Validate the transition
        entry.status.validate_transition(new_status).map_err(|e| {
            Error::InvalidConfig(format!(
                "Invalid state transition for workspace '{workspace}': {e}"
            ))
        })?;

        let now = Self::now();

        // Build the update query with status, state_changed_at, and conditional field updates
        let result = sqlx::query(
            "UPDATE merge_queue SET status = ?1, state_changed_at = ?2, previous_state = ?3 \
             WHERE workspace = ?4",
        )
        .bind(new_status.as_str())
        .bind(now)
        .bind(entry.status.as_str())
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to transition workspace '{workspace}': {e}"))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        // Emit audit event for the transition
        let event_details = format!(
            r#"{{"from": "{}", "to": "{}"}}"#,
            entry.status.as_str(),
            new_status.as_str()
        );
        let _ = self
            .append_typed_event(entry.id, QueueEventType::Transitioned, Some(&event_details))
            .await;

        Ok(())
    }

    /// Transition a queue entry to a failed state with error classification.
    ///
    /// This method handles failure scenarios by distinguishing between retryable
    /// and terminal failures, setting the appropriate status and error message.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name of the entry to transition
    /// * `error_message` - A description of the failure
    /// * `is_retryable` - If true, transition to `FailedRetryable`; otherwise `FailedTerminal`
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err(Error::NotFound)` if the workspace is not in the queue
    /// - `Err(Error::InvalidConfig)` if the transition is invalid
    /// - `Err(Error::DatabaseError)` if the database operation fails
    pub async fn transition_to_failed(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()> {
        // Fetch the current entry
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        // Determine target status based on retryability
        let new_status = if is_retryable {
            QueueStatus::FailedRetryable
        } else {
            QueueStatus::FailedTerminal
        };

        // Validate the transition
        entry.status.validate_transition(new_status).map_err(|e| {
            Error::InvalidConfig(format!(
                "Invalid state transition for workspace '{workspace}': {e}"
            ))
        })?;

        let now = Self::now();

        // Update status, error_message, and state_changed_at
        let result = sqlx::query(
            "UPDATE merge_queue SET status = ?1, error_message = ?2, state_changed_at = ?3, \
             completed_at = ?3, previous_state = ?4 WHERE workspace = ?5",
        )
        .bind(new_status.as_str())
        .bind(error_message)
        .bind(now)
        .bind(entry.status.as_str())
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to transition workspace '{workspace}': {e}"))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        // Emit audit event for the failure
        let event_details = format!(
            r#"{{"from": "{}", "to": "{}", "error": "{}", "retryable": {}}}"#,
            entry.status.as_str(),
            new_status.as_str(),
            error_message.replace('"', r#"\""#),
            is_retryable
        );
        let _ = self
            .append_typed_event(entry.id, QueueEventType::Failed, Some(&event_details))
            .await;

        Ok(())
    }

    /// Update rebase metadata after a successful rebase operation.
    ///
    /// This method updates both `head_sha` (the new workspace HEAD) and
    /// `tested_against_sha` (the main HEAD that was rebased onto).
    /// It also transitions the entry from `rebasing` to `testing`.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name of the entry to update
    /// * `head_sha` - The new HEAD SHA after rebase
    /// * `tested_against_sha` - The main branch SHA that was rebased onto
    ///
    /// # Returns
    /// - `Ok(())` if the update and transition succeeded
    /// - `Err(Error::NotFound)` if the workspace is not in the queue
    /// - `Err(Error::InvalidConfig)` if the entry is not in `rebasing` status
    /// - `Err(Error::DatabaseError)` if the database operation fails
    pub async fn update_rebase_metadata(
        &self,
        workspace: &str,
        head_sha: &str,
        tested_against_sha: &str,
    ) -> Result<()> {
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        if entry.status != QueueStatus::Rebasing {
            return Err(Error::InvalidConfig(format!(
                "Cannot update rebase metadata for workspace '{}' in status '{}', expected 'rebasing'",
                workspace,
                entry.status.as_str()
            )));
        }

        let now = Self::now();

        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'testing', head_sha = ?1, tested_against_sha = ?2, \
             state_changed_at = ?3, previous_state = 'rebasing' WHERE workspace = ?4",
        )
        .bind(head_sha)
        .bind(tested_against_sha)
        .bind(now)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to update rebase metadata for '{workspace}': {e}"
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        let event_details = format!(
            r#"{{"from": "rebasing", "to": "testing", "head_sha": "{head_sha}", "tested_against_sha": "{tested_against_sha}"}}"#
        );
        self.append_typed_event(entry.id, QueueEventType::Transitioned, Some(&event_details))
            .await?;

        Ok(())
    }

    /// Update rebase metadata with rebase count for observability.
    ///
    /// This method persists all rebase-related metadata including:
    /// - `head_sha`: The new HEAD SHA after rebase
    /// - `tested_against_sha`: The main branch SHA rebased onto
    /// - `rebase_count`: Total number of rebase attempts (incremented)
    /// - `last_rebase_at`: Timestamp of this rebase
    ///
    /// Also transitions status to 'testing' and emits a transitioned event.
    ///
    /// # Errors
    /// - `Err(Error::NotFound)` if the workspace is not found
    /// - `Err(Error::InvalidConfig)` if the entry is not in 'rebasing' status
    /// - `Err(Error::DatabaseError)` if the database operation fails
    #[allow(clippy::too_many_arguments)]
    pub async fn update_rebase_metadata_with_count(
        &self,
        workspace: &str,
        head_sha: &str,
        tested_against_sha: &str,
        rebase_count: i32,
        rebase_timestamp: i64,
    ) -> Result<()> {
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        if entry.status != QueueStatus::Rebasing {
            return Err(Error::InvalidConfig(format!(
                "Cannot update rebase metadata for workspace '{}' in status '{}', expected 'rebasing'",
                workspace,
                entry.status.as_str()
            )));
        }

        let now = Self::now();

        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'testing', head_sha = ?1, tested_against_sha = ?2, \
             state_changed_at = ?3, previous_state = 'rebasing', rebase_count = ?4, last_rebase_at = ?5 \
             WHERE workspace = ?6",
        )
        .bind(head_sha)
        .bind(tested_against_sha)
        .bind(now)
        .bind(rebase_count)
        .bind(rebase_timestamp)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to update rebase metadata for '{workspace}': {e}"
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        let event_details = format!(
            r#"{{"from": "rebasing", "to": "testing", "head_sha": "{head_sha}", "tested_against_sha": "{tested_against_sha}", "rebase_count": {rebase_count}}}"#
        );
        self.append_typed_event(entry.id, QueueEventType::Transitioned, Some(&event_details))
            .await?;

        Ok(())
    }

    /// Update the `tested_against_sha` for an entry (for testing purposes).
    pub async fn update_tested_against(
        &self,
        workspace: &str,
        tested_against_sha: &str,
    ) -> Result<()> {
        let _entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        let now = Self::now();

        let result = sqlx::query(
            "UPDATE merge_queue SET tested_against_sha = ?1, state_changed_at = ?2 WHERE workspace = ?3",
        )
        .bind(tested_against_sha)
        .bind(now)
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to update tested_against_sha for '{workspace}': {e}"
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // FRESHNESS GUARD METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Check if the main branch HEAD matches the stored `tested_against_sha`.
    ///
    /// The freshness guard ensures that we don't merge a stale result that was
    /// tested against an outdated version of main. If main has advanced since
    /// the entry was tested, we need to rebase and re-test.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name to check
    /// * `current_main_sha` - The current HEAD SHA of the main branch
    ///
    /// # Returns
    /// - `Ok(true)` if the entry is fresh (SHA matches)
    /// - `Ok(false)` if the entry is stale (SHA mismatch or missing baseline SHA)
    /// - `Err` if the entry is not found or database error
    pub async fn is_fresh(&self, workspace: &str, current_main_sha: &str) -> Result<bool> {
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        // Freshness is based on the main SHA used during the last successful rebase.
        // Missing baseline is treated as stale to fail closed.
        let Some(stored_sha) = &entry.tested_against_sha else {
            return Ok(false);
        };

        Ok(stored_sha == current_main_sha)
    }

    /// Transition entry back to rebasing state when freshness check fails.
    ///
    /// This is called when main has advanced since the entry was tested.
    /// The entry will need to be rebased onto the new main and re-tested.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name to transition
    /// * `new_main_sha` - The current main HEAD SHA for audit metadata
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err` if the entry is not found, invalid transition, or database error
    pub async fn return_to_rebasing(&self, workspace: &str, new_main_sha: &str) -> Result<()> {
        // Fetch the current entry
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        // Validate transition - only ready_to_merge should go back to rebasing
        if entry.status != QueueStatus::ReadyToMerge {
            return Err(Error::InvalidConfig(format!(
                "Cannot return to rebasing from status '{}', expected 'ready_to_merge'",
                entry.status.as_str()
            )));
        }

        let now = Self::now();

        // Update status to rebasing and clear stale tested-against baseline.
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'rebasing', tested_against_sha = NULL, state_changed_at = ?1, \
             previous_state = ?2 WHERE workspace = ?3",
        )
        .bind(now)
        .bind(entry.status.as_str())
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to transition '{workspace}' to rebasing: {e}"
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        // Emit audit event
        let event_details = format!(
            r#"{{"from": "ready_to_merge", "to": "rebasing", "reason": "freshness_guard", "new_main_sha": "{new_main_sha}"}}"#
        );
        self.append_typed_event(entry.id, QueueEventType::Transitioned, Some(&event_details))
            .await?;

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // MERGE STEP METHODS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Transition entry to merging state (after freshness check passes).
    ///
    /// This should be called right before performing the actual merge operation.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name to transition
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err` if the entry is not found, invalid transition, or database error
    pub async fn begin_merge(&self, workspace: &str) -> Result<()> {
        self.transition_to(workspace, QueueStatus::Merging).await
    }

    /// Complete the merge and record the merge commit SHA.
    ///
    /// This transitions the entry to the terminal `merged` state and records
    /// the SHA of the merge commit for audit purposes.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name to complete
    /// * `merged_sha` - The SHA of the resulting merge commit
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err` if the entry is not found, invalid transition, or database error
    pub async fn complete_merge(&self, workspace: &str, merged_sha: &str) -> Result<()> {
        // Fetch the current entry
        let entry = self.get_by_workspace(workspace).await?.ok_or_else(|| {
            Error::NotFound(format!("Workspace '{workspace}' not found in queue"))
        })?;

        // Validate transition (merging -> merged)
        entry
            .status
            .validate_transition(QueueStatus::Merged)
            .map_err(|e| {
                Error::InvalidConfig(format!(
                    "Invalid state transition for workspace '{workspace}': {e}"
                ))
            })?;

        let now = Self::now();

        // Update status to merged, record merged_sha in head_sha, and completed_at
        let result = sqlx::query(
            "UPDATE merge_queue SET status = 'merged', head_sha = ?1, completed_at = ?2, \
             state_changed_at = ?2, previous_state = ?3 WHERE workspace = ?4",
        )
        .bind(merged_sha)
        .bind(now)
        .bind(entry.status.as_str())
        .bind(workspace)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to complete merge for '{workspace}': {e}"))
        })?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Workspace '{workspace}' not found or update failed"
            )));
        }

        // Emit audit event for the merge
        let event_details =
            format!(r#"{{"from": "merging", "to": "merged", "merged_sha": "{merged_sha}"}}"#);
        self.append_typed_event(entry.id, QueueEventType::Merged, Some(&event_details))
            .await?;

        Ok(())
    }

    /// Mark a merge as failed with a retryable or terminal error.
    ///
    /// This is called when the merge operation fails. The entry will be
    /// transitioned to either `failed_retryable` or `failed_terminal` based
    /// on the nature of the failure.
    ///
    /// # Arguments
    /// * `workspace` - The workspace name that failed
    /// * `error_message` - Description of the failure
    /// * `is_retryable` - If true, transition to `failed_retryable`; otherwise `failed_terminal`
    ///
    /// # Returns
    /// - `Ok(())` if the transition succeeded
    /// - `Err` if the entry is not found, invalid transition, or database error
    pub async fn fail_merge(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()> {
        self.transition_to_failed(workspace, error_message, is_retryable)
            .await
    }
}

#[async_trait]
impl QueueRepository for MergeQueue {
    async fn add(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
    ) -> Result<QueueAddResponse> {
        self.add(workspace, bead_id, priority, agent_id).await
    }

    async fn add_with_dedupe(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: Option<&str>,
    ) -> Result<QueueAddResponse> {
        self.add_with_dedupe(workspace, bead_id, priority, agent_id, dedupe_key)
            .await
    }

    async fn upsert_for_submit(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
        dedupe_key: &str,
        head_sha: &str,
    ) -> Result<QueueEntry> {
        self.upsert_for_submit(workspace, bead_id, priority, agent_id, dedupe_key, head_sha)
            .await
    }

    async fn get_by_id(&self, id: i64) -> Result<Option<QueueEntry>> {
        self.get_by_id(id).await
    }

    async fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>> {
        self.get_by_workspace(workspace).await
    }

    async fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
        self.list(filter_status).await
    }

    async fn next(&self) -> Result<Option<QueueEntry>> {
        self.next().await
    }

    async fn remove(&self, workspace: &str) -> Result<bool> {
        self.remove(workspace).await
    }

    async fn position(&self, workspace: &str) -> Result<Option<usize>> {
        self.position(workspace).await
    }

    async fn count_pending(&self) -> Result<usize> {
        self.count_pending().await
    }

    async fn stats(&self) -> Result<QueueStats> {
        self.stats().await
    }

    async fn acquire_processing_lock(&self, agent_id: &str) -> Result<bool> {
        self.acquire_processing_lock(agent_id).await
    }

    async fn release_processing_lock(&self, agent_id: &str) -> Result<bool> {
        self.release_processing_lock(agent_id).await
    }

    async fn get_processing_lock(&self) -> Result<Option<ProcessingLock>> {
        self.get_processing_lock().await
    }

    async fn extend_lock(&self, agent_id: &str, extra_secs: i64) -> Result<bool> {
        self.extend_lock(agent_id, extra_secs).await
    }

    async fn mark_processing(&self, workspace: &str) -> Result<bool> {
        self.mark_processing(workspace).await
    }

    async fn mark_completed(&self, workspace: &str) -> Result<bool> {
        self.mark_completed(workspace).await
    }

    async fn mark_failed(&self, workspace: &str, error: &str) -> Result<bool> {
        self.mark_failed(workspace, error).await
    }

    async fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
        self.next_with_lock(agent_id).await
    }

    async fn transition_to(&self, workspace: &str, new_status: QueueStatus) -> Result<()> {
        self.transition_to(workspace, new_status).await
    }

    async fn transition_to_failed(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()> {
        self.transition_to_failed(workspace, error_message, is_retryable)
            .await
    }

    async fn update_rebase_metadata(
        &self,
        workspace: &str,
        head_sha: &str,
        tested_against_sha: &str,
    ) -> Result<()> {
        self.update_rebase_metadata(workspace, head_sha, tested_against_sha)
            .await
    }

    async fn update_rebase_metadata_with_count(
        &self,
        workspace: &str,
        head_sha: &str,
        tested_against_sha: &str,
        rebase_count: i32,
        rebase_timestamp: i64,
    ) -> Result<()> {
        self.update_rebase_metadata_with_count(
            workspace,
            head_sha,
            tested_against_sha,
            rebase_count,
            rebase_timestamp,
        )
        .await
    }

    async fn is_fresh(&self, workspace: &str, current_main_sha: &str) -> Result<bool> {
        self.is_fresh(workspace, current_main_sha).await
    }

    async fn return_to_rebasing(&self, workspace: &str, new_main_sha: &str) -> Result<()> {
        self.return_to_rebasing(workspace, new_main_sha).await
    }

    async fn begin_merge(&self, workspace: &str) -> Result<()> {
        self.begin_merge(workspace).await
    }

    async fn complete_merge(&self, workspace: &str, merged_sha: &str) -> Result<()> {
        self.complete_merge(workspace, merged_sha).await
    }

    async fn fail_merge(
        &self,
        workspace: &str,
        error_message: &str,
        is_retryable: bool,
    ) -> Result<()> {
        self.fail_merge(workspace, error_message, is_retryable)
            .await
    }

    async fn retry_entry(&self, id: i64) -> std::result::Result<QueueEntry, QueueControlError> {
        self.retry_entry(id).await
    }

    async fn cancel_entry(&self, id: i64) -> std::result::Result<QueueEntry, QueueControlError> {
        self.cancel_entry(id).await
    }

    async fn append_event(
        &self,
        queue_id: i64,
        event_type: &str,
        details: Option<&str>,
    ) -> Result<()> {
        self.append_event(queue_id, event_type, details).await
    }

    async fn append_typed_event(
        &self,
        queue_id: i64,
        event_type: QueueEventType,
        details: Option<&str>,
    ) -> Result<()> {
        self.append_typed_event(queue_id, event_type, details).await
    }

    async fn fetch_events(&self, queue_id: i64) -> Result<Vec<QueueEvent>> {
        self.fetch_events(queue_id).await
    }

    async fn fetch_recent_events(&self, queue_id: i64, limit: usize) -> Result<Vec<QueueEvent>> {
        self.fetch_recent_events(queue_id, limit).await
    }

    async fn cleanup(&self, max_age: Duration) -> Result<usize> {
        self.cleanup(max_age).await
    }

    async fn reclaim_stale(&self, stale_threshold_secs: i64) -> Result<usize> {
        self.reclaim_stale(stale_threshold_secs).await
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // AUTOMATIC SELF-HEALING (bd-2i5)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    async fn detect_and_recover_stale(&self) -> Result<RecoveryStats> {
        self.detect_and_recover_stale().await
    }

    async fn get_recovery_stats(&self) -> Result<RecoveryStats> {
        self.get_recovery_stats().await
    }

    async fn is_lock_stale(&self) -> Result<bool> {
        self.is_lock_stale().await
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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

    #[tokio::test]
    async fn test_fresh_database_has_head_sha_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify head_sha column exists
        let _: Option<String> = sqlx::query_scalar("SELECT head_sha FROM merge_queue LIMIT 1")
            .fetch_optional(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("head_sha column should exist: {e}")))?;

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // UPSERT FOR SUBMIT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_upsert_creates_new_entry_when_none_exists() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let entry = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        assert_eq!(entry.workspace, "ws-1");
        assert_eq!(entry.bead_id, Some("bead-1".to_string()));
        assert_eq!(entry.status, QueueStatus::Pending);
        assert_eq!(entry.head_sha, Some("abc123".to_string()));
        assert_eq!(entry.dedupe_key, Some("dedupe-key-1".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_active_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        assert_eq!(first.head_sha, Some("abc123".to_string()));
        let first_id = first.id;

        // Second upsert with same dedupe_key should UPDATE (not create new)
        let second = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "def456")
            .await?;

        // Same ID means it was updated, not created
        assert_eq!(second.id, first_id, "Should update same entry");
        assert_eq!(second.head_sha, Some("def456".to_string()));
        assert_eq!(second.status, QueueStatus::Pending);

        // Verify only one entry exists
        let stats = queue.stats().await?;
        assert_eq!(stats.pending, 1, "Should have exactly one pending entry");

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_resets_terminal_entry_same_workspace() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Manually set entry to terminal state (merged)
        sqlx::query("UPDATE merge_queue SET status = 'merged' WHERE id = ?1")
            .bind(first.id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Second upsert with SAME workspace should RESET the terminal entry
        let second = queue
            .upsert_for_submit(
                "ws-1",         // Same workspace
                Some("bead-2"), // New bead_id
                3,              // New priority
                None,
                "dedupe-key-1", // Same dedupe_key
                "def456",
            )
            .await?;

        // SAME ID means entry was reset, not created
        assert_eq!(
            second.id, first.id,
            "Should reset same entry for same workspace"
        );
        assert_eq!(second.head_sha, Some("def456".to_string()));
        assert_eq!(second.status, QueueStatus::Pending);
        assert_eq!(second.bead_id, Some("bead-2".to_string()));
        assert_eq!(second.priority, 3);

        // Verify only one entry exists
        let all = queue.list(None).await?;
        assert_eq!(all.len(), 1, "Should have exactly 1 entry");

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_creates_new_entry_different_workspace_after_terminal() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry with workspace ws-1
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Manually set entry to terminal state (merged)
        sqlx::query("UPDATE merge_queue SET status = 'merged' WHERE id = ?1")
            .bind(first.id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Second upsert with DIFFERENT workspace should CREATE new entry
        let second = queue
            .upsert_for_submit(
                "ws-2", // Different workspace
                Some("bead-2"),
                5,
                None,
                "dedupe-key-1", // Same dedupe_key
                "def456",
            )
            .await?;

        // Different ID means new entry was created
        assert_ne!(
            second.id, first.id,
            "Should create new entry for different workspace"
        );
        assert_eq!(second.workspace, "ws-2");
        assert_eq!(second.head_sha, Some("def456".to_string()));
        assert_eq!(second.status, QueueStatus::Pending);

        // Verify both entries exist (one merged with NULL dedupe_key, one pending)
        let all = queue.list(None).await?;
        assert_eq!(all.len(), 2, "Should have 2 entries");

        // Old entry should have dedupe_key cleared
        let old_entry = queue
            .get_by_id(first.id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Old entry not found".to_string()))?;
        assert_eq!(
            old_entry.dedupe_key, None,
            "Old terminal entry should have dedupe_key cleared"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_resets_failed_terminal_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Manually set entry to failed_terminal
        sqlx::query("UPDATE merge_queue SET status = 'failed_terminal' WHERE id = ?1")
            .bind(first.id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Second upsert should reset the terminal entry
        let second = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "def456")
            .await?;

        assert_eq!(second.id, first.id, "Should reset same entry");
        assert_eq!(second.status, QueueStatus::Pending);
        assert_eq!(second.head_sha, Some("def456".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_resets_cancelled_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Manually set entry to cancelled
        sqlx::query("UPDATE merge_queue SET status = 'cancelled' WHERE id = ?1")
            .bind(first.id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Second upsert should reset the terminal entry
        let second = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "def456")
            .await?;

        assert_eq!(second.id, first.id, "Should reset same entry");
        assert_eq!(second.status, QueueStatus::Pending);
        assert_eq!(second.head_sha, Some("def456".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_updates_claimed_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry
        let first = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Manually set to claimed (active but not pending)
        sqlx::query("UPDATE merge_queue SET status = 'claimed' WHERE id = ?1")
            .bind(first.id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Second upsert should UPDATE (claimed is active state)
        let second = queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "def456")
            .await?;

        assert_eq!(second.id, first.id, "Should update same entry");
        assert_eq!(second.head_sha, Some("def456".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_rejects_duplicate_dedupe_key_for_active_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // First upsert creates entry with dedupe_key
        queue
            .upsert_for_submit("ws-1", Some("bead-1"), 5, None, "dedupe-key-1", "abc123")
            .await?;

        // Attempt to upsert with SAME dedupe_key but DIFFERENT workspace should fail
        let result = queue
            .upsert_for_submit(
                "ws-2", // Different workspace
                Some("bead-2"),
                5,
                None,
                "dedupe-key-1", // Same dedupe_key
                "def456",
            )
            .await;

        assert!(
            result.is_err(),
            "Should reject duplicate dedupe_key for active entries"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_concurrent_same_dedupe_key() -> Result<()> {
        // Test that concurrent upserts with the same dedupe_key are handled correctly
        let queue = MergeQueue::open_in_memory().await?;
        let queue1 = queue.clone();
        let queue2 = queue.clone();
        let queue3 = queue.clone();

        // Spawn multiple concurrent upserts with same dedupe_key
        let handle1 = tokio::spawn(async move {
            queue1
                .upsert_for_submit("ws-1", None, 5, None, "concurrent-key", "sha1")
                .await
        });
        let handle2 = tokio::spawn(async move {
            queue2
                .upsert_for_submit("ws-1", None, 5, None, "concurrent-key", "sha2")
                .await
        });
        let handle3 = tokio::spawn(async move {
            queue3
                .upsert_for_submit("ws-1", None, 5, None, "concurrent-key", "sha3")
                .await
        });

        let (r1, r2, r3) = tokio::join!(handle1, handle2, handle3);

        // Unwrap JoinHandle results
        let results = [
            r1.map_err(|e| Error::DatabaseError(format!("Task 1 join failed: {e}")))?,
            r2.map_err(|e| Error::DatabaseError(format!("Task 2 join failed: {e}")))?,
            r3.map_err(|e| Error::DatabaseError(format!("Task 3 join failed: {e}")))?,
        ];

        // Count successes and failures
        let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();

        // At least one should succeed
        assert!(
            !successes.is_empty(),
            "At least one concurrent upsert should succeed"
        );

        // All entries in DB should have unique workspaces (due to workspace UNIQUE constraint)
        let all = queue.list(None).await?;
        let workspaces: std::collections::HashSet<_> =
            all.iter().map(|e| e.workspace.as_str()).collect();
        assert_eq!(
            all.len(),
            workspaces.len(),
            "All entries should have unique workspaces"
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
        assert!(QueueStatus::Pending
            .validate_transition(QueueStatus::Claimed)
            .is_ok());
    }

    #[test]
    fn test_claimed_to_rebasing_is_valid() {
        assert!(QueueStatus::Claimed.can_transition_to(QueueStatus::Rebasing));
        assert!(QueueStatus::Claimed
            .validate_transition(QueueStatus::Rebasing)
            .is_ok());
    }

    #[test]
    fn test_rebasing_to_testing_is_valid() {
        assert!(QueueStatus::Rebasing.can_transition_to(QueueStatus::Testing));
        assert!(QueueStatus::Rebasing
            .validate_transition(QueueStatus::Testing)
            .is_ok());
    }

    #[test]
    fn test_testing_to_ready_to_merge_is_valid() {
        assert!(QueueStatus::Testing.can_transition_to(QueueStatus::ReadyToMerge));
        assert!(QueueStatus::Testing
            .validate_transition(QueueStatus::ReadyToMerge)
            .is_ok());
    }

    #[test]
    fn test_ready_to_merge_to_merging_is_valid() {
        assert!(QueueStatus::ReadyToMerge.can_transition_to(QueueStatus::Merging));
        assert!(QueueStatus::ReadyToMerge
            .validate_transition(QueueStatus::Merging)
            .is_ok());
    }

    #[test]
    fn test_merging_to_merged_is_valid() {
        assert!(QueueStatus::Merging.can_transition_to(QueueStatus::Merged));
        assert!(QueueStatus::Merging
            .validate_transition(QueueStatus::Merged)
            .is_ok());
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
            assert!(!status.is_terminal(), "{status:?} should not be terminal");
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
            assert!(parsed.is_ok(), "Failed to parse '{s}' back to QueueStatus");
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

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // RETRY ENTRY TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_retry_failed_retryable_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add an entry and set it to failed_retryable
        let resp = queue.add("ws-retry-1", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to failed_retryable with attempt_count = 0
        sqlx::query(
            "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 0 WHERE id = ?1",
        )
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Retry should succeed
        let retried = queue
            .retry_entry(entry_id)
            .await
            .map_err(|e| Error::DatabaseError(format!("Retry failed: {e}")))?;

        assert_eq!(retried.status, QueueStatus::Pending);
        assert_eq!(retried.attempt_count, 1);
        assert!(retried.error_message.is_none());
        assert!(retried.state_changed_at.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_fails_when_not_failed_retryable() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add an entry (it will be in pending state)
        let resp = queue.add("ws-retry-2", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Retry should fail because it's not in failed_retryable
        let result = queue.retry_entry(entry_id).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotRetryable { id, status }) => {
                assert_eq!(id, entry_id);
                assert_eq!(status, QueueStatus::Pending);
            }
            _ => panic!("Expected NotRetryable error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_fails_when_max_attempts_exceeded() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add an entry
        let resp = queue.add("ws-retry-3", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to failed_retryable with attempt_count at max (3)
        sqlx::query(
            "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 3 WHERE id = ?1",
        )
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Retry should fail because max_attempts exceeded
        let result = queue.retry_entry(entry_id).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::MaxAttemptsExceeded {
                id,
                attempt_count,
                max_attempts,
            }) => {
                assert_eq!(id, entry_id);
                assert_eq!(attempt_count, 3);
                assert_eq!(max_attempts, 3);
            }
            _ => panic!("Expected MaxAttemptsExceeded error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_fails_for_nonexistent_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let result = queue.retry_entry(99999).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotFound(id)) => {
                assert_eq!(id, 99999);
            }
            _ => panic!("Expected NotFound error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_allows_up_to_max_attempts() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add an entry
        let resp = queue.add("ws-retry-4", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Set to failed_retryable with attempt_count = 2 (max is 3)
        sqlx::query(
            "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 2 WHERE id = ?1",
        )
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Retry should succeed (attempt_count becomes 3, which is still valid)
        let retried = queue
            .retry_entry(entry_id)
            .await
            .map_err(|e| Error::DatabaseError(format!("Retry failed: {e}")))?;

        assert_eq!(retried.status, QueueStatus::Pending);
        assert_eq!(retried.attempt_count, 3);

        // Now set back to failed_retryable and retry should fail
        sqlx::query("UPDATE merge_queue SET status = 'failed_retryable' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        let result = queue.retry_entry(entry_id).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_retry_on_same_entry_allows_single_winner() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue.add("ws-retry-race", None, 5, None).await?;
        let entry_id = resp.entry.id;

        sqlx::query(
            "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 0 WHERE id = ?1",
        )
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to prepare retry race entry: {e}")))?;

        let queue1 = queue.clone();
        let queue2 = queue.clone();
        let queue3 = queue.clone();
        let queue4 = queue.clone();
        let queue5 = queue.clone();

        let h1 = tokio::spawn(async move { queue1.retry_entry(entry_id).await });
        let h2 = tokio::spawn(async move { queue2.retry_entry(entry_id).await });
        let h3 = tokio::spawn(async move { queue3.retry_entry(entry_id).await });
        let h4 = tokio::spawn(async move { queue4.retry_entry(entry_id).await });
        let h5 = tokio::spawn(async move { queue5.retry_entry(entry_id).await });

        let (r1, r2, r3, r4, r5) = tokio::join!(h1, h2, h3, h4, h5);
        let results = [
            r1.map_err(|e| Error::DatabaseError(format!("Task 1 join failed: {e}")))?,
            r2.map_err(|e| Error::DatabaseError(format!("Task 2 join failed: {e}")))?,
            r3.map_err(|e| Error::DatabaseError(format!("Task 3 join failed: {e}")))?,
            r4.map_err(|e| Error::DatabaseError(format!("Task 4 join failed: {e}")))?,
            r5.map_err(|e| Error::DatabaseError(format!("Task 5 join failed: {e}")))?,
        ];

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 1, "Exactly one retry should win");

        for outcome in results.iter().filter_map(|r| r.as_ref().err()) {
            assert!(
                matches!(
                    outcome,
                    QueueControlError::NotRetryable { .. }
                        | QueueControlError::MaxAttemptsExceeded { .. }
                ),
                "Losers should fail with retry-state errors, got: {outcome:?}"
            );
        }

        let final_entry = queue.get_by_id(entry_id).await?.ok_or_else(|| {
            Error::DatabaseError("Expected entry to exist after retry race".to_string())
        })?;
        assert_eq!(final_entry.status, QueueStatus::Pending);
        assert_eq!(final_entry.attempt_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_and_cancel_race_never_leaves_failed_retryable() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue.add("ws-retry-cancel-race", None, 5, None).await?;
        let entry_id = resp.entry.id;

        sqlx::query(
            "UPDATE merge_queue SET status = 'failed_retryable', attempt_count = 0 WHERE id = ?1",
        )
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| {
            Error::DatabaseError(format!("Failed to prepare retry/cancel race entry: {e}"))
        })?;

        let queue_retry = queue.clone();
        let queue_cancel = queue.clone();
        let retry_handle = tokio::spawn(async move { queue_retry.retry_entry(entry_id).await });
        let cancel_handle = tokio::spawn(async move { queue_cancel.cancel_entry(entry_id).await });

        let retry_result = retry_handle
            .await
            .map_err(|e| Error::DatabaseError(format!("Retry task join failed: {e}")))?;
        let cancel_result = cancel_handle
            .await
            .map_err(|e| Error::DatabaseError(format!("Cancel task join failed: {e}")))?;

        assert!(
            retry_result.is_ok() || cancel_result.is_ok(),
            "At least one operation should succeed"
        );

        let final_entry = queue.get_by_id(entry_id).await?.ok_or_else(|| {
            Error::DatabaseError("Expected entry to exist after retry/cancel race".to_string())
        })?;

        assert!(
            matches!(
                final_entry.status,
                QueueStatus::Pending | QueueStatus::Cancelled
            ),
            "Final state must be pending or cancelled, got: {:?}",
            final_entry.status
        );
        assert_ne!(final_entry.status, QueueStatus::FailedRetryable);

        Ok(())
    }

    #[tokio::test]
    async fn test_reclaim_stale_parallel_reclaims_single_claimed_entry_once() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue.add("ws-reclaim-race", None, 5, None).await?;
        let entry_id = resp.entry.id;

        let old_timestamp = MergeQueue::now() - 10_000;
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'agent-x' WHERE id = ?2",
        )
        .bind(old_timestamp)
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to prepare reclaim race entry: {e}")))?;

        let queue1 = queue.clone();
        let queue2 = queue.clone();
        let queue3 = queue.clone();
        let queue4 = queue.clone();
        let queue5 = queue.clone();

        let h1 = tokio::spawn(async move { queue1.reclaim_stale(300).await });
        let h2 = tokio::spawn(async move { queue2.reclaim_stale(300).await });
        let h3 = tokio::spawn(async move { queue3.reclaim_stale(300).await });
        let h4 = tokio::spawn(async move { queue4.reclaim_stale(300).await });
        let h5 = tokio::spawn(async move { queue5.reclaim_stale(300).await });

        let (r1, r2, r3, r4, r5) = tokio::join!(h1, h2, h3, h4, h5);
        let reclaimed_counts = [
            r1.map_err(|e| Error::DatabaseError(format!("Reclaim task 1 join failed: {e}")))??,
            r2.map_err(|e| Error::DatabaseError(format!("Reclaim task 2 join failed: {e}")))??,
            r3.map_err(|e| Error::DatabaseError(format!("Reclaim task 3 join failed: {e}")))??,
            r4.map_err(|e| Error::DatabaseError(format!("Reclaim task 4 join failed: {e}")))??,
            r5.map_err(|e| Error::DatabaseError(format!("Reclaim task 5 join failed: {e}")))??,
        ];

        let total_reclaimed: usize = reclaimed_counts.into_iter().sum();
        assert_eq!(
            total_reclaimed, 1,
            "Only one reclaim should affect one entry"
        );

        let final_entry = queue.get_by_id(entry_id).await?.ok_or_else(|| {
            Error::DatabaseError("Expected entry to exist after reclaim race".to_string())
        })?;

        assert_eq!(final_entry.status, QueueStatus::Pending);
        assert!(final_entry.started_at.is_none());
        assert!(final_entry.agent_id.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_reclaim_stale_does_not_touch_fresh_claimed_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue.add("ws-fresh-claim", None, 5, None).await?;
        let entry_id = resp.entry.id;

        let now = MergeQueue::now();
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'agent-fresh' WHERE id = ?2",
        )
        .bind(now)
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to prepare fresh claim entry: {e}")))?;

        let reclaimed = queue.reclaim_stale(300).await?;
        assert_eq!(reclaimed, 0, "Fresh claims must not be reclaimed");

        let final_entry = queue.get_by_id(entry_id).await?.ok_or_else(|| {
            Error::DatabaseError("Expected entry to exist after fresh reclaim check".to_string())
        })?;

        assert_eq!(final_entry.status, QueueStatus::Claimed);
        assert_eq!(final_entry.agent_id.as_deref(), Some("agent-fresh"));

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_and_reclaim_stale_race_converges_to_cancelled() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;
        let resp = queue.add("ws-cancel-reclaim-race", None, 5, None).await?;
        let entry_id = resp.entry.id;

        let old_timestamp = MergeQueue::now() - 10_000;
        sqlx::query(
            "UPDATE merge_queue SET status = 'claimed', started_at = ?1, agent_id = 'agent-stale' WHERE id = ?2",
        )
        .bind(old_timestamp)
        .bind(entry_id)
        .execute(&queue.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to prepare cancel/reclaim race entry: {e}")))?;

        let queue_cancel = queue.clone();
        let queue_reclaim = queue.clone();

        let cancel_handle = tokio::spawn(async move { queue_cancel.cancel_entry(entry_id).await });
        let reclaim_handle = tokio::spawn(async move { queue_reclaim.reclaim_stale(300).await });

        let cancel_result = cancel_handle
            .await
            .map_err(|e| Error::DatabaseError(format!("Cancel task join failed: {e}")))?;
        let _ = reclaim_handle
            .await
            .map_err(|e| Error::DatabaseError(format!("Reclaim task join failed: {e}")))??;

        assert!(
            cancel_result.is_ok(),
            "Cancelling stale claimed entries should succeed"
        );

        let final_entry = queue.get_by_id(entry_id).await?.ok_or_else(|| {
            Error::DatabaseError("Expected entry to exist after cancel/reclaim race".to_string())
        })?;

        assert_eq!(final_entry.status, QueueStatus::Cancelled);

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // CANCEL ENTRY TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_cancel_pending_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-1", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Cancel should succeed for pending
        let cancelled = queue
            .cancel_entry(entry_id)
            .await
            .map_err(|e| Error::DatabaseError(format!("Cancel failed: {e}")))?;

        assert_eq!(cancelled.status, QueueStatus::Cancelled);
        assert!(cancelled.state_changed_at.is_some());
        assert!(cancelled.completed_at.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_claimed_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-2", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to claimed
        sqlx::query("UPDATE merge_queue SET status = 'claimed' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Cancel should succeed for claimed
        let cancelled = queue
            .cancel_entry(entry_id)
            .await
            .map_err(|e| Error::DatabaseError(format!("Cancel failed: {e}")))?;

        assert_eq!(cancelled.status, QueueStatus::Cancelled);

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_failed_retryable_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-3", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to failed_retryable
        sqlx::query("UPDATE merge_queue SET status = 'failed_retryable' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Cancel should succeed for failed_retryable
        let cancelled = queue
            .cancel_entry(entry_id)
            .await
            .map_err(|e| Error::DatabaseError(format!("Cancel failed: {e}")))?;

        assert_eq!(cancelled.status, QueueStatus::Cancelled);

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_fails_for_merged() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-4", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to merged
        sqlx::query("UPDATE merge_queue SET status = 'merged' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Cancel should fail for merged
        let result = queue.cancel_entry(entry_id).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotCancellable { id, status }) => {
                assert_eq!(id, entry_id);
                assert_eq!(status, QueueStatus::Merged);
            }
            _ => panic!("Expected NotCancellable error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_fails_for_failed_terminal() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-5", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to failed_terminal
        sqlx::query("UPDATE merge_queue SET status = 'failed_terminal' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Cancel should fail for failed_terminal
        let result = queue.cancel_entry(entry_id).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotCancellable { id, status }) => {
                assert_eq!(id, entry_id);
                assert_eq!(status, QueueStatus::FailedTerminal);
            }
            _ => panic!("Expected NotCancellable error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_fails_for_cancelled() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-cancel-6", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Manually set to cancelled
        sqlx::query("UPDATE merge_queue SET status = 'cancelled' WHERE id = ?1")
            .bind(entry_id)
            .execute(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to update status: {e}")))?;

        // Cancel should fail for already cancelled
        let result = queue.cancel_entry(entry_id).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotCancellable { id, status }) => {
                assert_eq!(id, entry_id);
                assert_eq!(status, QueueStatus::Cancelled);
            }
            _ => panic!("Expected NotCancellable error, got: {result:?}"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_fails_for_nonexistent_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let result = queue.cancel_entry(99999).await;

        assert!(result.is_err());
        match result {
            Err(QueueControlError::NotFound(id)) => {
                assert_eq!(id, 99999);
            }
            _ => panic!("Expected NotFound error, got: {result:?}"),
        }

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SCHEMA COLUMN TESTS (attempt_count, max_attempts)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_fresh_database_has_attempt_count_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify attempt_count column exists
        let _: Option<i32> = sqlx::query_scalar("SELECT attempt_count FROM merge_queue LIMIT 1")
            .fetch_optional(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("attempt_count column should exist: {e}")))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_fresh_database_has_max_attempts_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify max_attempts column exists
        let _: Option<i32> = sqlx::query_scalar("SELECT max_attempts FROM merge_queue LIMIT 1")
            .fetch_optional(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("max_attempts column should exist: {e}")))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_new_entry_defaults_attempt_count_to_zero() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-default-attempt", None, 5, None).await?;

        assert_eq!(
            resp.entry.attempt_count, 0,
            "New entries should have attempt_count = 0"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_new_entry_defaults_max_attempts_to_three() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-default-max", None, 5, None).await?;

        assert_eq!(
            resp.entry.max_attempts, 3,
            "New entries should have max_attempts = 3"
        );

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // EVENT AUDIT TRAIL TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_fresh_database_has_queue_events_table() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Verify queue_events table exists by querying it
        let _: Option<i64> = sqlx::query_scalar("SELECT id FROM queue_events LIMIT 1")
            .fetch_optional(&queue.pool)
            .await
            .map_err(|e| Error::DatabaseError(format!("queue_events table should exist: {e}")))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_append_event_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add a queue entry first
        let resp = queue.add("ws-event-1", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Append an event
        queue
            .append_event(entry_id, "created", Some(r#"{"source": "test"}"#))
            .await?;

        // Verify the event was inserted
        let events = queue.fetch_events(entry_id).await?;
        assert_eq!(events.len(), 1, "Should have exactly one event");
        let event = events
            .first()
            .ok_or_else(|| Error::DatabaseError("Expected event to exist".to_string()))?;
        assert_eq!(event.queue_id, entry_id);
        assert_eq!(event.event_type, QueueEventType::Created);
        assert_eq!(
            event.details_json,
            Some(r#"{"source": "test"}"#.to_string())
        );
        assert!(event.created_at > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_append_typed_event_succeeds() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-event-typed", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Append a typed event
        queue
            .append_typed_event(entry_id, QueueEventType::Claimed, None)
            .await?;

        let events = queue.fetch_events(entry_id).await?;
        assert_eq!(events.len(), 1);
        let event = events
            .first()
            .ok_or_else(|| Error::DatabaseError("Expected event to exist".to_string()))?;
        assert_eq!(event.event_type, QueueEventType::Claimed);
        assert!(event.details_json.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_events_returns_empty_for_nonexistent() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Fetch events for a queue_id that doesn't exist
        let events = queue.fetch_events(99999).await?;
        assert!(
            events.is_empty(),
            "Should return empty vector for nonexistent queue_id"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_events_returns_events_in_order() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-event-order", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Append multiple events with small delays to ensure different timestamps
        queue.append_event(entry_id, "created", None).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        queue.append_event(entry_id, "claimed", None).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        queue.append_event(entry_id, "transitioned", None).await?;

        let events = queue.fetch_events(entry_id).await?;
        assert_eq!(events.len(), 3, "Should have three events");

        // Verify order is ascending by id
        assert_eq!(events[0].event_type, QueueEventType::Created);
        assert_eq!(events[1].event_type, QueueEventType::Claimed);
        assert_eq!(events[2].event_type, QueueEventType::Transitioned);

        // Verify IDs are monotonically increasing
        assert!(events[0].id < events[1].id);
        assert!(events[1].id < events[2].id);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_recent_events_limits_results() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-event-limit", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Append 5 events
        for i in 0..5 {
            queue
                .append_event(entry_id, "heartbeat", Some(&format!(r#"{{"seq": {i}}}"#)))
                .await?;
        }

        // Fetch only 2 most recent
        let events = queue.fetch_recent_events(entry_id, 2).await?;
        assert_eq!(events.len(), 2, "Should return only 2 events");

        // Should be the last 2 in chronological order
        assert_eq!(events[0].details_json, Some(r#"{"seq": 3}"#.to_string()));
        assert_eq!(events[1].details_json, Some(r#"{"seq": 4}"#.to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_recent_events_with_limit_zero() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-event-zero", None, 5, None).await?;
        let entry_id = resp.entry.id;

        queue.append_event(entry_id, "created", None).await?;

        // Limit of 0 should return empty
        let events = queue.fetch_recent_events(entry_id, 0).await?;
        assert!(events.is_empty(), "Limit 0 should return empty vector");

        Ok(())
    }

    #[tokio::test]
    async fn test_events_have_monotonically_increasing_ids() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Create two different queue entries
        let resp1 = queue.add("ws-event-mono-1", None, 5, None).await?;
        let resp2 = queue.add("ws-event-mono-2", None, 5, None).await?;

        // Append events to both queues in interleaved order
        queue.append_event(resp1.entry.id, "created", None).await?;
        queue.append_event(resp2.entry.id, "created", None).await?;
        queue.append_event(resp1.entry.id, "claimed", None).await?;
        queue.append_event(resp2.entry.id, "claimed", None).await?;

        // Get all events from both queues
        let events1 = queue.fetch_events(resp1.entry.id).await?;
        let events2 = queue.fetch_events(resp2.entry.id).await?;

        // Combine and sort by id to verify global monotonicity
        let all_events: Vec<_> = events1.iter().chain(events2.iter()).collect();
        let mut sorted_events = all_events.clone();
        sorted_events.sort_by_key(|e| e.id);

        // Verify sorted order matches the original combined order (proves IDs are unique)
        let ids: Vec<i64> = sorted_events.iter().map(|e| e.id).collect();

        // Verify IDs are strictly increasing when sorted
        for i in 1..ids.len() {
            assert!(
                ids[i - 1] < ids[i],
                "Event IDs should be monotonically increasing: {:?}",
                ids
            );
        }

        // Also verify that within each queue, events are in ID order (ascending)
        for window in events1.windows(2) {
            assert!(
                window[0].id < window[1].id,
                "Events within queue 1 should be ordered by ID"
            );
        }
        for window in events2.windows(2) {
            assert!(
                window[0].id < window[1].id,
                "Events within queue 2 should be ordered by ID"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_event_type_roundtrip() -> Result<()> {
        // Test all event types roundtrip through string conversion
        for event_type in [
            QueueEventType::Created,
            QueueEventType::Claimed,
            QueueEventType::Transitioned,
            QueueEventType::Failed,
            QueueEventType::Retried,
            QueueEventType::Cancelled,
            QueueEventType::Merged,
            QueueEventType::Heartbeat,
        ] {
            let s = event_type.to_string();
            let parsed = QueueEventType::from_str(&s)?;
            assert_eq!(parsed, event_type, "Roundtrip failed for {event_type:?}");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_event_type_returns_error() -> Result<()> {
        let result = QueueEventType::from_str("invalid_event_type");
        assert!(result.is_err(), "Invalid event type should return error");

        Ok(())
    }

    #[tokio::test]
    async fn test_append_event_with_null_details() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-event-null", None, 5, None).await?;
        let entry_id = resp.entry.id;

        // Append event with None details
        queue.append_event(entry_id, "created", None).await?;

        let events = queue.fetch_events(entry_id).await?;
        assert_eq!(events.len(), 1);
        let event = events
            .first()
            .ok_or_else(|| Error::DatabaseError("Expected event to exist".to_string()))?;
        assert!(event.details_json.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_events_isolated_per_queue_entry() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Create two queue entries
        let resp1 = queue.add("ws-isolated-1", None, 5, None).await?;
        let resp2 = queue.add("ws-isolated-2", None, 5, None).await?;

        // Add events to each
        queue
            .append_event(resp1.entry.id, "created", Some(r#"{"queue": 1}"#))
            .await?;
        queue
            .append_event(resp2.entry.id, "created", Some(r#"{"queue": 2}"#))
            .await?;

        // Fetch events for each - should be isolated
        let events1 = queue.fetch_events(resp1.entry.id).await?;
        let events2 = queue.fetch_events(resp2.entry.id).await?;

        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
        assert_eq!(events1[0].details_json, Some(r#"{"queue": 1}"#.to_string()));
        assert_eq!(events2[0].details_json, Some(r#"{"queue": 2}"#.to_string()));

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // REBASE METADATA TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_fresh_database_has_tested_against_column() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let _: Option<String> =
            sqlx::query_scalar("SELECT tested_against_sha FROM merge_queue LIMIT 1")
                .fetch_optional(&queue.pool)
                .await
                .map_err(|e| {
                    Error::DatabaseError(format!("tested_against_sha column should exist: {e}"))
                })?;

        Ok(())
    }

    #[tokio::test]
    async fn test_new_entry_has_null_tested_against() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let resp = queue.add("ws-tested-new", None, 5, None).await?;

        assert!(
            resp.entry.tested_against_sha.is_none(),
            "New entries should have tested_against_sha = NULL"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_rebase_info() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-rebase-info", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-info").await?;
        assert!(claimed.is_some());
        let _entry = claimed.ok_or_else(|| Error::DatabaseError("expected entry".into()))?;

        queue
            .transition_to("ws-rebase-info", QueueStatus::Rebasing)
            .await?;

        queue
            .update_rebase_metadata("ws-rebase-info", "head123", "main456")
            .await?;

        let updated = queue
            .get_by_workspace("ws-rebase-info")
            .await?
            .ok_or_else(|| Error::DatabaseError("entry should exist".into()))?;

        assert_eq!(updated.head_sha, Some("head123".to_string()));
        assert_eq!(updated.tested_against_sha, Some("main456".to_string()));
        assert_eq!(updated.status, QueueStatus::Testing);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_rebase_wrong_state() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-rebase-wrong", None, 5, None).await?;

        let result: Result<()> = queue
            .update_rebase_metadata("ws-rebase-wrong", "head123", "main456")
            .await;

        assert!(result.is_err());
        let error_msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error_msg.contains("expected 'rebasing'") || error_msg.contains("pending"),
            "Error should mention expected status, got: {error_msg}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_rebase_nonexistent() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        let result: Result<()> = queue
            .update_rebase_metadata("nonexistent-rebase", "head123", "main456")
            .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_rebase_event() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-rebase-ev", None, 5, None).await?;
        let claimed = queue.next_with_lock("agent-ev").await?;
        let entry_id = claimed
            .ok_or_else(|| Error::DatabaseError("expected entry".into()))?
            .id;

        queue
            .transition_to("ws-rebase-ev", QueueStatus::Rebasing)
            .await?;

        // Get events before the metadata update
        let events_before = queue.fetch_events(entry_id).await?;
        let transitioned_count_before = events_before
            .iter()
            .filter(|e| e.event_type == QueueEventType::Transitioned)
            .count();

        queue
            .update_rebase_metadata("ws-rebase-ev", "head789", "main012")
            .await?;

        let events = queue.fetch_events(entry_id).await?;

        // Find the transitioned event from rebasing to testing (should contain the SHAs)
        let rebase_transition_event = events
            .iter()
            .filter(|e| e.event_type == QueueEventType::Transitioned)
            .skip(transitioned_count_before)
            .find(|e| {
                e.details_json
                    .as_ref()
                    .is_some_and(|d| d.contains("head789"))
            });

        assert!(
            rebase_transition_event.is_some(),
            "Should have a transitioned event with head789"
        );

        let event =
            rebase_transition_event.ok_or_else(|| Error::DatabaseError("expected event".into()))?;
        assert!(event
            .details_json
            .as_ref()
            .is_some_and(|d| d.contains("main012")));

        Ok(())
    }

    #[tokio::test]
    async fn test_is_fresh_uses_tested_against_sha() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-fresh", None, 5, None).await?;
        queue
            .transition_to("ws-fresh", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to("ws-fresh", QueueStatus::Rebasing)
            .await?;
        queue
            .update_rebase_metadata("ws-fresh", "head-a", "main-a")
            .await?;
        queue
            .transition_to("ws-fresh", QueueStatus::ReadyToMerge)
            .await?;

        assert!(queue.is_fresh("ws-fresh", "main-a").await?);
        assert!(!queue.is_fresh("ws-fresh", "main-b").await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_is_fresh_missing_baseline_fails_closed() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-no-baseline", None, 5, None).await?;

        assert!(!queue.is_fresh("ws-no-baseline", "main-a").await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_return_to_rebasing_clears_stale_tested_against_sha() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-stale", None, 5, None).await?;
        queue
            .transition_to("ws-stale", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to("ws-stale", QueueStatus::Rebasing)
            .await?;
        queue
            .update_rebase_metadata("ws-stale", "head-before", "main-old")
            .await?;
        queue
            .transition_to("ws-stale", QueueStatus::ReadyToMerge)
            .await?;

        queue.return_to_rebasing("ws-stale", "main-new").await?;

        let updated = queue
            .get_by_workspace("ws-stale")
            .await?
            .ok_or_else(|| Error::DatabaseError("entry should exist".into()))?;
        assert_eq!(updated.status, QueueStatus::Rebasing);
        assert_eq!(updated.head_sha, Some("head-before".to_string()));
        assert!(updated.tested_against_sha.is_none());

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // CLEANUP TESTS - Proves terminal statuses are cleaned
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_cleanup_deletes_merged_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add and merge a workspace following correct state machine:
        // pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
        queue.add("ws-merged", None, 5, None).await?;
        queue
            .transition_to("ws-merged", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to("ws-merged", QueueStatus::Rebasing)
            .await?;
        queue
            .transition_to("ws-merged", QueueStatus::Testing)
            .await?;
        queue
            .transition_to("ws-merged", QueueStatus::ReadyToMerge)
            .await?;
        queue.begin_merge("ws-merged").await?;
        queue.complete_merge("ws-merged", "merged-sha-123").await?;

        // Verify entry exists with merged status
        let entry = queue.get_by_workspace("ws-merged").await?;
        assert!(entry.is_some(), "Entry should exist before cleanup");
        assert_eq!(
            entry.as_ref().map(|e| e.status),
            Some(QueueStatus::Merged),
            "Entry should be merged before cleanup"
        );

        // Run cleanup with zero duration (cleanup all terminal)
        let cleaned = queue.cleanup(Duration::ZERO).await?;
        assert_eq!(cleaned, 1, "Should clean up 1 merged entry");

        // Verify entry is gone
        let entry_after = queue.get_by_workspace("ws-merged").await?;
        assert!(
            entry_after.is_none(),
            "Merged entry should be deleted after cleanup"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_deletes_cancelled_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        queue.add("ws-cancelled", None, 5, None).await?;
        queue
            .transition_to("ws-cancelled", QueueStatus::Cancelled)
            .await?;

        let cleaned = queue.cleanup(Duration::ZERO).await?;
        assert_eq!(cleaned, 1, "Should clean up 1 cancelled entry");

        let entry = queue.get_by_workspace("ws-cancelled").await?;
        assert!(
            entry.is_none(),
            "Cancelled entry should be deleted after cleanup"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_deletes_failed_terminal_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Must go through claimed first, then fail from claimed
        queue.add("ws-failed-terminal", None, 5, None).await?;
        queue
            .transition_to("ws-failed-terminal", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to_failed("ws-failed-terminal", "Unrecoverable error", false)
            .await?;

        // Verify it's failed_terminal
        let entry = queue.get_by_workspace("ws-failed-terminal").await?;
        assert_eq!(
            entry.as_ref().map(|e| e.status),
            Some(QueueStatus::FailedTerminal),
            "Entry should be failed_terminal"
        );

        let cleaned = queue.cleanup(Duration::ZERO).await?;
        assert_eq!(cleaned, 1, "Should clean up 1 failed_terminal entry");

        let entry_after = queue.get_by_workspace("ws-failed-terminal").await?;
        assert!(
            entry_after.is_none(),
            "Failed terminal entry should be deleted after cleanup"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_does_not_delete_pending_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Add entries in various non-terminal states
        queue.add("ws-pending", None, 5, None).await?;
        queue.add("ws-pending-2", None, 5, None).await?;

        // Run cleanup
        let cleaned = queue.cleanup(Duration::ZERO).await?;
        assert_eq!(cleaned, 0, "Should not clean up pending entries");

        // Verify entries still exist
        let stats = queue.stats().await?;
        assert_eq!(stats.pending, 2, "Pending entries should remain");

        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_deletes_multiple_terminal_entries() -> Result<()> {
        let queue = MergeQueue::open_in_memory().await?;

        // Create merged entry (full state machine path)
        queue.add("ws-merged-1", None, 5, None).await?;
        queue
            .transition_to("ws-merged-1", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to("ws-merged-1", QueueStatus::Rebasing)
            .await?;
        queue
            .transition_to("ws-merged-1", QueueStatus::Testing)
            .await?;
        queue
            .transition_to("ws-merged-1", QueueStatus::ReadyToMerge)
            .await?;
        queue.begin_merge("ws-merged-1").await?;
        queue.complete_merge("ws-merged-1", "sha-1").await?;

        // Create cancelled entry
        queue.add("ws-cancelled-1", None, 5, None).await?;
        queue
            .transition_to("ws-cancelled-1", QueueStatus::Cancelled)
            .await?;

        // Create failed_terminal entry
        queue.add("ws-failed-1", None, 5, None).await?;
        queue
            .transition_to("ws-failed-1", QueueStatus::Claimed)
            .await?;
        queue
            .transition_to_failed("ws-failed-1", "Error", false)
            .await?;

        // Add a pending entry that should NOT be cleaned
        queue.add("ws-pending-keep", None, 5, None).await?;

        // Run cleanup
        let cleaned = queue.cleanup(Duration::ZERO).await?;
        assert_eq!(
            cleaned, 3,
            "Should clean up 3 terminal entries (merged, cancelled, failed_terminal)"
        );

        // Verify only pending entry remains
        let stats = queue.stats().await?;
        assert_eq!(stats.pending, 1, "Only pending entry should remain");
        assert_eq!(stats.total, 1, "Total should be 1");

        Ok(())
    }
}

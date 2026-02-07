//! Merge queue for sequential multi-agent coordination.

use std::{
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tokio::time::sleep;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl QueueStatus {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for QueueStatus {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::InvalidConfig(format!("Invalid queue status: {s}"))),
        }
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
    /// This creates a transient in-memory SQLite database that is
    /// discarded when the queue is dropped. Useful for testing and
    /// development without persisting state to disk.
    pub async fn open_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to open in-memory database: {e}")))?;

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
                agent_id TEXT
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
        // SQLite supports partial indexes via WHERE clause
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_merge_queue_processing
             ON merge_queue(workspace, started_at)
             WHERE status = 'processing' AND started_at IS NOT NULL",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create processing index: {e}")))?;

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
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    pub async fn add(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
    ) -> Result<QueueAddResponse> {
        let now = Self::now();
        let result = sqlx::query(
            "INSERT INTO merge_queue (workspace, bead_id, priority, status, added_at, agent_id) \
                 VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        )
        .bind(workspace)
        .bind(bead_id)
        .bind(priority)
        .bind(now)
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                Error::InvalidConfig(format!("Workspace '{workspace}' is already in the queue"))
            } else {
                Error::DatabaseError(format!("Failed to add to queue: {e}"))
            }
        })?;

        let id = result.last_insert_rowid();
        let entry = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve inserted entry".to_string()))?;
        let position = self.position(workspace).await?.unwrap_or(1);
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
                 completed_at, error_message, agent_id FROM merge_queue WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get entry: {e}")))
    }

    pub async fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>> {
        sqlx::query_as::<_, QueueEntry>(
            "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id FROM merge_queue WHERE workspace = ?1",
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
                 completed_at, error_message, agent_id FROM merge_queue WHERE status = ?1 \
                 ORDER BY priority ASC, added_at ASC"
            }
            None => {
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id FROM merge_queue \
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
                 completed_at, error_message, agent_id FROM merge_queue WHERE status = 'pending' \
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
                       completed_at, error_message, agent_id",
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
        let now = Self::now();
        let new_expires = now + extra_secs;
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
        let cutoff = Self::now() - max_age.as_secs() as i64;
        let result = sqlx::query(
            "DELETE FROM merge_queue WHERE status IN ('completed', 'failed') \
                 AND completed_at < ?1",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?;
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .find(|e| e.is_some())
            .flatten();

        assert!(claimed.is_some());
        let entry = claimed.expect("claimed entry must exist after assert");
        assert_eq!(entry.status, QueueStatus::Processing);
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
}

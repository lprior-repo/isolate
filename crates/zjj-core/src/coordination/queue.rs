//! Merge queue for sequential multi-agent coordination.

use std::{
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OptionalExtension};

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

#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: i64,
    pub workspace: String,
    pub bead_id: Option<String>,
    pub priority: i32,
    pub status: QueueStatus,
    pub added_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
    pub agent_id: Option<String>,
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

#[derive(Debug, Clone)]
pub struct ProcessingLock {
    pub agent_id: String,
    pub acquired_at: i64,
    pub expires_at: i64,
}

pub struct MergeQueue {
    conn: Connection,
    lock_timeout_secs: i64,
}

impl MergeQueue {
    pub const DEFAULT_LOCK_TIMEOUT_SECS: i64 = 300;

    pub fn open(db_path: &Path) -> Result<Self> {
        Self::open_with_timeout(db_path, Self::DEFAULT_LOCK_TIMEOUT_SECS)
    }

    pub fn open_with_timeout(db_path: &Path, lock_timeout_secs: i64) -> Result<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| Error::DatabaseError(format!("Failed to open queue database: {e}")))?;
        let queue = Self {
            conn,
            lock_timeout_secs,
        };
        queue.init_schema()?;
        Ok(queue)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::DatabaseError(format!("Failed to open in-memory database: {e}")))?;
        let queue = Self {
            conn,
            lock_timeout_secs: Self::DEFAULT_LOCK_TIMEOUT_SECS,
        };
        queue.init_schema()?;
        Ok(queue)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(
                r"
            CREATE TABLE IF NOT EXISTS merge_queue (
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
            );
            CREATE INDEX IF NOT EXISTS idx_merge_queue_status ON merge_queue(status);
            CREATE INDEX IF NOT EXISTS idx_merge_queue_priority_added ON merge_queue(priority, added_at);
            CREATE TABLE IF NOT EXISTS queue_processing_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                agent_id TEXT NOT NULL,
                acquired_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );
        ",
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to initialize schema: {e}")))?;
        Ok(())
    }

    #[allow(clippy::cast_possible_wrap)]
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    pub fn add(
        &self,
        workspace: &str,
        bead_id: Option<&str>,
        priority: i32,
        agent_id: Option<&str>,
    ) -> Result<QueueAddResponse> {
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO merge_queue (workspace, bead_id, priority, status, added_at, agent_id) \
                 VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
                params![workspace, bead_id, priority, now, agent_id],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    Error::InvalidConfig(format!("Workspace '{workspace}' is already in the queue"))
                } else {
                    Error::DatabaseError(format!("Failed to add to queue: {e}"))
                }
            })?;
        let id = self.conn.last_insert_rowid();
        let entry = self
            .get_by_id(id)?
            .ok_or_else(|| Error::DatabaseError("Failed to retrieve inserted entry".to_string()))?;
        let position = self.position(workspace)?.unwrap_or(1);
        let total_pending = self.count_pending()?;
        Ok(QueueAddResponse {
            entry,
            position,
            total_pending,
        })
    }

    fn get_by_id(&self, id: i64) -> Result<Option<QueueEntry>> {
        self.conn
            .query_row(
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id FROM merge_queue WHERE id = ?1",
                params![id],
                |row| {
                    Ok(QueueEntry {
                        id: row.get(0)?,
                        workspace: row.get(1)?,
                        bead_id: row.get(2)?,
                        priority: row.get(3)?,
                        status: QueueStatus::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or(QueueStatus::Pending),
                        added_at: row.get(5)?,
                        started_at: row.get(6)?,
                        completed_at: row.get(7)?,
                        error_message: row.get(8)?,
                        agent_id: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to get entry: {e}")))
    }

    pub fn get_by_workspace(&self, workspace: &str) -> Result<Option<QueueEntry>> {
        self.conn
            .query_row(
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id FROM merge_queue WHERE workspace = ?1",
                params![workspace],
                |row| {
                    Ok(QueueEntry {
                        id: row.get(0)?,
                        workspace: row.get(1)?,
                        bead_id: row.get(2)?,
                        priority: row.get(3)?,
                        status: QueueStatus::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or(QueueStatus::Pending),
                        added_at: row.get(5)?,
                        started_at: row.get(6)?,
                        completed_at: row.get(7)?,
                        error_message: row.get(8)?,
                        agent_id: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to get entry: {e}")))
    }

    pub fn list(&self, filter_status: Option<QueueStatus>) -> Result<Vec<QueueEntry>> {
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
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| Error::DatabaseError(format!("Failed to prepare query: {e}")))?;
        let rows = if let Some(s) = filter_status {
            stmt.query(params![s.as_str()])
        } else {
            stmt.query([])
        }
        .map_err(|e| Error::DatabaseError(format!("Failed to execute query: {e}")))?;
        rows.mapped(|row| {
            Ok(QueueEntry {
                id: row.get(0)?,
                workspace: row.get(1)?,
                bead_id: row.get(2)?,
                priority: row.get(3)?,
                status: QueueStatus::from_str(&row.get::<_, String>(4)?)
                    .unwrap_or(QueueStatus::Pending),
                added_at: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
                agent_id: row.get(9)?,
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::DatabaseError(format!("Failed to collect entries: {e}")))
    }

    pub fn next(&self) -> Result<Option<QueueEntry>> {
        self.conn
            .query_row(
                "SELECT id, workspace, bead_id, priority, status, added_at, started_at, \
                 completed_at, error_message, agent_id FROM merge_queue WHERE status = 'pending' \
                 ORDER BY priority ASC, added_at ASC LIMIT 1",
                [],
                |row| {
                    Ok(QueueEntry {
                        id: row.get(0)?,
                        workspace: row.get(1)?,
                        bead_id: row.get(2)?,
                        priority: row.get(3)?,
                        status: QueueStatus::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or(QueueStatus::Pending),
                        added_at: row.get(5)?,
                        started_at: row.get(6)?,
                        completed_at: row.get(7)?,
                        error_message: row.get(8)?,
                        agent_id: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to get next entry: {e}")))
    }

    pub fn remove(&self, workspace: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM merge_queue WHERE workspace = ?1",
                params![workspace],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to remove entry: {e}")))?;
        Ok(rows > 0)
    }

    pub fn position(&self, workspace: &str) -> Result<Option<usize>> {
        let pending = self.list(Some(QueueStatus::Pending))?;
        Ok(pending
            .iter()
            .position(|e| e.workspace == workspace)
            .map(|p| p + 1))
    }

    #[allow(clippy::cast_sign_loss)]
    pub fn count_pending(&self) -> Result<usize> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM merge_queue WHERE status = 'pending'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c as usize)
            .map_err(|e| Error::DatabaseError(format!("Failed to count pending: {e}")))
    }

    #[allow(clippy::cast_sign_loss)]
    pub fn stats(&self) -> Result<QueueStats> {
        let mut queue_stats = QueueStats::default();
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM merge_queue GROUP BY status")
            .map_err(|e| Error::DatabaseError(format!("Failed to prepare stats query: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| Error::DatabaseError(format!("Failed to execute stats query: {e}")))?;
        for row in rows {
            let (status_str, cnt) =
                row.map_err(|e| Error::DatabaseError(format!("Failed to read stats row: {e}")))?;
            let cnt = cnt as usize;
            queue_stats.total += cnt;
            match status_str.as_str() {
                "pending" => queue_stats.pending = cnt,
                "processing" => queue_stats.processing = cnt,
                "completed" => queue_stats.completed = cnt,
                "failed" => queue_stats.failed = cnt,
                _ => {}
            }
        }
        Ok(queue_stats)
    }

    pub fn acquire_processing_lock(&self, agent_id: &str) -> Result<bool> {
        let now = Self::now();
        let expires_at = now + self.lock_timeout_secs;
        let result = self.conn.execute(
            "INSERT INTO queue_processing_lock (id, agent_id, acquired_at, expires_at) \
             VALUES (1, ?1, ?2, ?3) \
             ON CONFLICT(id) DO UPDATE SET agent_id = ?1, acquired_at = ?2, expires_at = ?3 \
             WHERE expires_at < ?2",
            params![agent_id, now, expires_at],
        );
        match result {
            Ok(rows) => Ok(rows > 0),
            Err(e) => Err(Error::DatabaseError(format!("Failed to acquire lock: {e}"))),
        }
    }

    pub fn release_processing_lock(&self, agent_id: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM queue_processing_lock WHERE id = 1 AND agent_id = ?1",
                params![agent_id],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to release lock: {e}")))?;
        Ok(rows > 0)
    }

    pub fn get_processing_lock(&self) -> Result<Option<ProcessingLock>> {
        self.conn
            .query_row(
                "SELECT agent_id, acquired_at, expires_at FROM queue_processing_lock WHERE id = 1",
                [],
                |row| {
                    Ok(ProcessingLock {
                        agent_id: row.get(0)?,
                        acquired_at: row.get(1)?,
                        expires_at: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(|e| Error::DatabaseError(format!("Failed to get lock: {e}")))
    }

    pub fn mark_processing(&self, workspace: &str) -> Result<bool> {
        let now = Self::now();
        let rows = self
            .conn
            .execute(
                "UPDATE merge_queue SET status = 'processing', started_at = ?1 \
                 WHERE workspace = ?2 AND status = 'pending'",
                params![now, workspace],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to mark processing: {e}")))?;
        Ok(rows > 0)
    }

    pub fn mark_completed(&self, workspace: &str) -> Result<bool> {
        let now = Self::now();
        let rows = self
            .conn
            .execute(
                "UPDATE merge_queue SET status = 'completed', completed_at = ?1 \
                 WHERE workspace = ?2 AND status = 'processing'",
                params![now, workspace],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to mark completed: {e}")))?;
        Ok(rows > 0)
    }

    pub fn mark_failed(&self, workspace: &str, error: &str) -> Result<bool> {
        let now = Self::now();
        let rows = self
            .conn
            .execute(
                "UPDATE merge_queue SET status = 'failed', completed_at = ?1, error_message = ?2 \
                 WHERE workspace = ?3 AND status = 'processing'",
                params![now, error, workspace],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to mark failed: {e}")))?;
        Ok(rows > 0)
    }

    pub fn next_with_lock(&self, agent_id: &str) -> Result<Option<QueueEntry>> {
        if !self.acquire_processing_lock(agent_id)? {
            return Ok(None);
        }
        let Some(entry) = self.next()? else {
            let _ = self.release_processing_lock(agent_id);
            return Ok(None);
        };
        if !self.mark_processing(&entry.workspace)? {
            let _ = self.release_processing_lock(agent_id);
            return Ok(None);
        }
        Ok(Some(entry))
    }

    pub fn extend_lock(&self, agent_id: &str, extra_secs: i64) -> Result<bool> {
        let now = Self::now();
        let new_expires = now + extra_secs;
        let rows = self
            .conn
            .execute(
                "UPDATE queue_processing_lock SET expires_at = ?1 WHERE id = 1 AND agent_id = ?2",
                params![new_expires, agent_id],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to extend lock: {e}")))?;
        Ok(rows > 0)
    }

    #[allow(clippy::cast_possible_wrap)]
    pub fn cleanup(&self, max_age: Duration) -> Result<usize> {
        let cutoff = Self::now() - max_age.as_secs() as i64;
        let rows = self
            .conn
            .execute(
                "DELETE FROM merge_queue WHERE status IN ('completed', 'failed') \
                 AND completed_at < ?1",
                params![cutoff],
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to cleanup: {e}")))?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list() -> Result<()> {
        let queue = MergeQueue::open_in_memory()?;
        let resp = queue.add("ws-1", Some("bead-1"), 5, Some("agent-1"))?;
        assert_eq!(resp.entry.workspace, "ws-1");
        assert_eq!(resp.position, 1);
        let entries = queue.list(Some(QueueStatus::Pending))?;
        assert_eq!(entries.len(), 1);
        Ok(())
    }

    #[test]
    fn test_priority_ordering() -> Result<()> {
        let queue = MergeQueue::open_in_memory()?;
        queue.add("low", None, 10, None)?;
        queue.add("high", None, 0, None)?;
        queue.add("mid", None, 5, None)?;
        let entries = queue.list(Some(QueueStatus::Pending))?;
        assert_eq!(entries[0].workspace, "high");
        assert_eq!(entries[1].workspace, "mid");
        assert_eq!(entries[2].workspace, "low");
        Ok(())
    }

    #[test]
    fn test_concurrent_adds() -> Result<()> {
        let queue = MergeQueue::open_in_memory()?;
        for i in 0..40 {
            queue.add(&format!("ws-{i}"), None, i % 3, Some(&format!("agent-{i}")))?;
        }
        let queue_stats = queue.stats()?;
        assert_eq!(queue_stats.total, 40);
        assert_eq!(queue_stats.pending, 40);
        Ok(())
    }

    #[test]
    fn test_processing_lock_serializes_work() -> Result<()> {
        let queue = MergeQueue::open_in_memory()?;
        queue.add("ws-1", None, 5, Some("agent-1"))?;
        queue.add("ws-2", None, 5, Some("agent-2"))?;

        let first = queue.next_with_lock("agent-1")?;
        assert!(first.is_some());

        let second = queue.next_with_lock("agent-2")?;
        assert!(second.is_none());

        let released = queue.release_processing_lock("agent-1")?;
        assert!(released);

        let next = queue.next_with_lock("agent-2")?;
        assert!(next.is_some());
        Ok(())
    }
}

//! Queue management types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique queue entry identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueEntryId(String);

impl QueueEntryId {
    /// Create a new queue entry ID
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for QueueEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A queue entry representing a session waiting to be merged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Unique identifier
    pub id: QueueEntryId,
    /// Session name
    pub session: String,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// When enqueued
    pub enqueued_at: DateTime<Utc>,
    /// Current status
    pub status: QueueStatus,
    /// Agent claiming this entry
    pub claimed_by: Option<String>,
}

impl QueueEntry {
    /// Create a new queue entry
    #[must_use]
    pub fn new(id: QueueEntryId, session: String, priority: u32) -> Self {
        Self {
            id,
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
            claimed_by: None,
        }
    }
}

/// Status of a queue entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    /// Waiting to be processed
    Pending,
    /// Claimed by an agent
    Claimed,
    /// Currently being rebased
    Rebasing,
    /// Running tests
    Testing,
    /// Ready to merge
    ReadyToMerge,
    /// Currently merging
    Merging,
    /// Successfully merged
    Merged,
    /// Failed with retryable error
    FailedRetryable,
    /// Failed terminally
    FailedTerminal,
    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Claimed => write!(f, "claimed"),
            Self::Rebasing => write!(f, "rebasing"),
            Self::Testing => write!(f, "testing"),
            Self::ReadyToMerge => write!(f, "ready_to_merge"),
            Self::Merging => write!(f, "merging"),
            Self::Merged => write!(f, "merged"),
            Self::FailedRetryable => write!(f, "failed_retryable"),
            Self::FailedTerminal => write!(f, "failed_terminal"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl QueueStatus {
    /// Check if this is a terminal state
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Merged | Self::FailedTerminal | Self::Cancelled)
    }

    /// Check if this is a failed state
    #[must_use]
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::FailedRetryable | Self::FailedTerminal)
    }
}

/// The merge queue
#[derive(Debug, Clone, Default)]
pub struct Queue {
    entries: Vec<QueueEntry>,
}

impl Queue {
    /// Create a new empty queue
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of entries in the queue
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the queue is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries
    #[must_use]
    pub fn entries(&self) -> &[QueueEntry] {
        &self.entries
    }

    /// Add an entry to the queue (maintains priority order)
    pub fn enqueue(&mut self, entry: QueueEntry) {
        self.entries.push(entry);
        self.entries.sort_by_key(|e| e.priority);
    }

    /// Remove an entry from the queue by ID
    pub fn dequeue(&mut self, id: &QueueEntryId) -> Option<QueueEntry> {
        self.entries
            .iter()
            .position(|e| &e.id == id)
            .map(|i| self.entries.remove(i))
    }

    /// Find an entry by ID
    #[must_use]
    pub fn find(&self, id: &QueueEntryId) -> Option<&QueueEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Find an entry by session name
    #[must_use]
    pub fn find_by_session(&self, session: &str) -> Option<&QueueEntry> {
        self.entries.iter().find(|e| e.session == session)
    }

    /// Get the next pending entry
    #[must_use]
    pub fn next_pending(&self) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|e| e.status == QueueStatus::Pending)
    }
}

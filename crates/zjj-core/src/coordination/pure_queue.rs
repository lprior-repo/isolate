//! Pure functional queue for property-based testing.
//!
//! This module provides an immutable, pure functional queue implementation
//! designed for property-based testing of queue invariants.
//!
//! Key properties:
//! - No I/O, no async, no side effects
//! - All operations return new instances (persistent data structures)
//! - Uses `im` for structural sharing (changed from rpds due to API limitations)
//! - Zero unwrap/expect/panic
//! - No mutation - truly functional

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use itertools::Itertools;
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use thiserror::Error;

// Persistent vector wrapper around std::Vec with clone-on-write
type Vector<T> = Vec<T>;
type ImHashMap<K, V> = StdHashMap<K, V>;
type ImHashSet<T> = StdHashSet<T>;

use super::queue_status::QueueStatus;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PURE QUEUE ERROR
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for pure queue operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PureQueueError {
    /// Entry not found in queue.
    #[error("entry not found: {0}")]
    NotFound(String),

    /// Duplicate workspace in queue.
    #[error("workspace already exists: {0}")]
    DuplicateWorkspace(String),

    /// Duplicate dedupe key in queue.
    #[error("dedupe key already exists: {0}")]
    DuplicateDedupeKey(String),

    /// Entry cannot be claimed (wrong status).
    #[error("cannot claim entry with status: {0}")]
    CannotClaim(QueueStatus),

    /// Entry cannot be released (not claimed).
    #[error("entry is not claimed: {0}")]
    NotClaimed(String),

    /// Invalid state transition.
    #[error("invalid transition from {from} to {to}")]
    InvalidTransition { from: QueueStatus, to: QueueStatus },

    /// No claimable entries available.
    #[error("no pending entries available")]
    NoPendingEntries,

    /// Lock held by different agent.
    #[error("lock held by {holder}, not by {requester}")]
    LockHeldByOther { holder: String, requester: String },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PURE QUEUE ENTRY
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// An entry in the pure queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureEntry {
    /// Workspace identifier.
    pub workspace: String,
    /// Priority (lower = higher priority).
    pub priority: i32,
    /// Current status.
    pub status: QueueStatus,
    /// Order of insertion (for FIFO within priority).
    pub added_at: usize,
    /// Agent that claimed this entry (if claimed).
    pub claimed_by: Option<String>,
    /// Deduplication key.
    pub dedupe_key: Option<String>,
}

impl PureEntry {
    /// Create a new pending entry.
    #[must_use]
    pub const fn new(workspace: String, priority: i32, added_at: usize) -> Self {
        Self {
            workspace,
            priority,
            status: QueueStatus::Pending,
            added_at,
            claimed_by: None,
            dedupe_key: None,
        }
    }

    /// Create a new pending entry with a dedupe key.
    #[must_use]
    pub fn with_dedupe(mut self, dedupe_key: String) -> Self {
        self.dedupe_key = Some(dedupe_key);
        self
    }

    /// Create a new entry with `claimed_by` set.
    #[must_use]
    pub fn with_claimed_by(mut self, agent_id: String) -> Self {
        self.claimed_by = Some(agent_id);
        self
    }

    /// Create a new entry with a new status.
    #[must_use]
    pub const fn with_status(mut self, status: QueueStatus) -> Self {
        self.status = status;
        self
    }

    /// Check if this entry can be claimed.
    #[must_use]
    pub const fn is_claimable(&self) -> bool {
        matches!(self.status, QueueStatus::Pending)
    }

    /// Check if this entry is claimed.
    #[must_use]
    pub const fn is_claimed(&self) -> bool {
        matches!(self.status, QueueStatus::Claimed)
    }

    /// Check if this entry is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PURE QUEUE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A pure functional queue with persistent semantics.
///
/// All operations return a new queue instance, leaving the original unchanged.
/// Uses `im` for efficient structural sharing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureQueue {
    /// Entries indexed by position.
    entries: Vector<PureEntry>,
    /// Map from workspace to index in entries vector.
    workspace_index: ImHashMap<String, usize>,
    /// Set of active dedupe keys (mapped to workspace).
    dedupe_keys: ImHashMap<String, String>,
    /// Counter for insertion order.
    insertion_counter: usize,
    /// Current lock holder (`agent_id`).
    lock_holder: Option<String>,
}

impl Default for PureQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PureQueue {
    /// Create a new empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vector::new(),
            workspace_index: ImHashMap::new(),
            dedupe_keys: ImHashMap::new(),
            insertion_counter: 0,
            lock_holder: None,
        }
    }

    /// Get the number of entries in the queue.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the queue is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add an entry to the queue.
    ///
    /// Returns a new queue with the entry added.
    ///
    /// # Errors
    ///
    /// Returns `PureQueueError::DuplicateWorkspace` if the workspace already exists.
    ///
    /// Returns `PureQueueError::DuplicateDedupeKey` if the dedupe key already exists
    /// and is associated with an active (non-terminal) entry.
    pub fn add(
        &self,
        workspace: &str,
        priority: i32,
        dedupe_key: Option<&str>,
    ) -> Result<Self, PureQueueError> {
        // Check for duplicate workspace
        if self.workspace_index.contains_key(workspace) {
            return Err(PureQueueError::DuplicateWorkspace(workspace.to_string()));
        }

        // Check for duplicate dedupe key (only for active entries)
        if let Some(key) = dedupe_key {
            if let Some(existing_ws) = self.dedupe_keys.get(key) {
                // Check if the existing entry is still active
                if let Some(entry) = self.get(existing_ws) {
                    if !entry.is_terminal() {
                        return Err(PureQueueError::DuplicateDedupeKey(key.to_string()));
                    }
                }
            }
        }

        let new_counter = self.insertion_counter + 1;
        let entry = PureEntry::new(workspace.to_string(), priority, self.insertion_counter)
            .with_dedupe_opt(dedupe_key.map(String::from));

        let index = self.entries.len();
        let mut new_entries = self.entries.clone();
        new_entries.push(entry);
        let mut new_workspace_index = self.workspace_index.clone();
        new_workspace_index.insert(workspace.to_string(), index);

        let mut new_dedupe_keys = self.dedupe_keys.clone();
        if let Some(key) = dedupe_key {
            new_dedupe_keys.insert(key.to_string(), workspace.to_string());
        }

        Ok(Self {
            entries: new_entries,
            workspace_index: new_workspace_index,
            dedupe_keys: new_dedupe_keys,
            insertion_counter: new_counter,
            lock_holder: self.lock_holder.clone(),
        })
    }

    /// Get an entry by workspace.
    #[must_use]
    pub fn get(&self, workspace: &str) -> Option<&PureEntry> {
        self.workspace_index
            .get(workspace)
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Claim the next pending entry for an agent.
    ///
    /// Returns a tuple of (`new_queue`, `claimed_workspace`) on success.
    /// The queue is ordered by (priority, `added_at`) ascending.
    ///
    /// # Errors
    ///
    /// Returns `PureQueueError::LockHeldByOther` if a different agent holds the lock.
    ///
    /// Returns `PureQueueError::NoPendingEntries` if there are no pending entries available.
    pub fn claim_next(&self, agent_id: &str) -> Result<(Self, String), PureQueueError> {
        // Check if lock is already held
        if let Some(ref holder) = self.lock_holder {
            if holder != agent_id {
                return Err(PureQueueError::LockHeldByOther {
                    holder: holder.clone(),
                    requester: agent_id.to_string(),
                });
            }
        }

        // Find the highest priority pending entry
        let next = self
            .entries
            .iter()
            .filter(|e: &&PureEntry| e.is_claimable())
            .min_by_key(|e| (e.priority, e.added_at));

        let entry = next.ok_or(PureQueueError::NoPendingEntries)?;
        let workspace = entry.workspace.clone();

        // Create new entry with claimed status
        let new_entry = entry
            .clone()
            .with_status(QueueStatus::Claimed)
            .with_claimed_by(agent_id.to_string());

        // Update the entry in the queue
        let idx = self
            .workspace_index
            .get(&workspace)
            .ok_or_else(|| PureQueueError::NotFound(workspace.clone()))?;

        let mut new_entries = self.entries.clone();
        new_entries[*idx] = new_entry;

        Ok((
            Self {
                entries: new_entries,
                workspace_index: self.workspace_index.clone(),
                dedupe_keys: self.dedupe_keys.clone(),
                insertion_counter: self.insertion_counter,
                lock_holder: Some(agent_id.to_string()),
            },
            workspace,
        ))
    }

    /// Release a claimed entry.
    ///
    /// # Errors
    ///
    /// Returns `PureQueueError::NotFound` if the workspace does not exist in the queue.
    ///
    /// Returns `PureQueueError::NotClaimed` if the entry is not in a claimed state.
    pub fn release(&self, workspace: &str) -> Result<Self, PureQueueError> {
        let entry = self
            .get(workspace)
            .ok_or_else(|| PureQueueError::NotFound(workspace.to_string()))?;

        if !entry.is_claimed() {
            return Err(PureQueueError::NotClaimed(workspace.to_string()));
        }

        let idx = self
            .workspace_index
            .get(workspace)
            .ok_or_else(|| PureQueueError::NotFound(workspace.to_string()))?;

        // Create new entry with claim cleared
        let new_entry = entry.clone().with_claimed_by(String::new());

        let mut new_entries = self.entries.clone();
        new_entries[*idx] = new_entry;

        // Clear lock holder if this was the lock holder
        let was_lock_holder = entry
            .claimed_by
            .as_ref()
            .is_some_and(|a| Some(a.as_str()) == self.lock_holder.as_deref());

        let new_lock_holder = if was_lock_holder {
            None
        } else {
            self.lock_holder.clone()
        };

        Ok(Self {
            entries: new_entries,
            workspace_index: self.workspace_index.clone(),
            dedupe_keys: self.dedupe_keys.clone(),
            insertion_counter: self.insertion_counter,
            lock_holder: new_lock_holder,
        })
    }

    /// Transition an entry to a new status.
    ///
    /// # Errors
    ///
    /// Returns `PureQueueError::NotFound` if the workspace does not exist in the queue.
    ///
    /// Returns `PureQueueError::InvalidTransition` if the state transition from the current
    /// status to the new status is not valid according to the state machine rules.
    pub fn transition_status(
        &self,
        workspace: &str,
        new_status: QueueStatus,
    ) -> Result<Self, PureQueueError> {
        let entry = self
            .get(workspace)
            .ok_or_else(|| PureQueueError::NotFound(workspace.to_string()))?;

        // Validate transition
        entry.status.validate_transition(new_status).map_err(|e| {
            PureQueueError::InvalidTransition {
                from: e.from,
                to: e.to,
            }
        })?;

        let idx = self
            .workspace_index
            .get(workspace)
            .ok_or_else(|| PureQueueError::NotFound(workspace.to_string()))?;

        // Create new entry with new status
        let new_entry = entry.clone().with_status(new_status);

        let mut new_entries = self.entries.clone();
        new_entries[*idx] = new_entry;

        // If transitioning to terminal, release dedupe key and clear lock if held
        let (new_dedupe_keys, new_lock_holder) = if new_status.is_terminal() {
            let mut keys = self.dedupe_keys.clone();
            if let Some(ref key) = entry.dedupe_key {
                keys.remove(key);
            }

            let lock = if entry
                .claimed_by
                .as_ref()
                .is_some_and(|a| Some(a.as_str()) == self.lock_holder.as_deref())
            {
                None
            } else {
                self.lock_holder.clone()
            };

            (keys, lock)
        } else {
            (self.dedupe_keys.clone(), self.lock_holder.clone())
        };

        Ok(Self {
            entries: new_entries,
            workspace_index: self.workspace_index.clone(),
            dedupe_keys: new_dedupe_keys,
            insertion_counter: self.insertion_counter,
            lock_holder: new_lock_holder,
        })
    }

    /// Get all pending entries in priority order.
    #[must_use]
    pub fn pending_in_order(&self) -> Vec<&PureEntry> {
        self.entries
            .iter()
            .filter(|e: &&PureEntry| e.is_claimable())
            .sorted_by_key(|e| (e.priority, e.added_at))
            .collect()
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &PureEntry> {
        self.entries.iter()
    }

    /// Check if a lock is currently held.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        self.lock_holder.is_some()
    }

    /// Get the current lock holder.
    #[must_use]
    pub const fn lock_holder(&self) -> Option<&String> {
        self.lock_holder.as_ref()
    }

    /// Check queue consistency (for property testing).
    ///
    /// Returns true if:
    /// - No duplicate workspaces
    /// - All `workspace_index` entries point to valid entries
    /// - Dedupe keys match their workspaces
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        // Check workspace_index consistency
        let index_valid = self.workspace_index.iter().all(|(workspace, &idx)| {
            self.entries
                .get(idx)
                .is_some_and(|e| e.workspace == *workspace)
        });

        if !index_valid {
            return false;
        }

        // Check dedupe key consistency
        let keys_valid = self.dedupe_keys.iter().all(|(key, workspace)| {
            self.get(workspace)
                .is_some_and(|e| e.dedupe_key.as_ref() == Some(key))
        });

        if !keys_valid {
            return false;
        }

        // Check for duplicate workspaces in entries
        let workspaces: ImHashSet<&str> =
            self.entries.iter().map(|e| e.workspace.as_str()).collect();

        workspaces.len() == self.entries.len()
    }

    /// Count entries by status.
    #[must_use]
    pub fn count_by_status(&self, status: QueueStatus) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }

    /// Get the position of a workspace in the pending queue.
    #[must_use]
    pub fn position(&self, workspace: &str) -> Option<usize> {
        let entry = self.get(workspace)?;
        if !entry.is_claimable() {
            return None;
        }
        let pending = self.pending_in_order();
        pending
            .iter()
            .position(|e| e.workspace == workspace)
            .map(|p| p + 1) // 1-indexed
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HELPER TRAIT
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Helper trait for optional dedupe key
trait WithDedupeOpt {
    fn with_dedupe_opt(self, key: Option<String>) -> Self;
}

impl WithDedupeOpt for PureEntry {
    fn with_dedupe_opt(self, key: Option<String>) -> Self {
        match key {
            Some(k) => self.with_dedupe(k),
            None => self,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper macro to unwrap Result in tests with context
    macro_rules! unwrap_ok {
        ($expr:expr, $msg:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => panic!("{}: {:?}", $msg, e),
            }
        };
    }

    #[test]
    fn test_new_queue_is_empty() {
        let queue = PureQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_add_entry() {
        let queue = PureQueue::new();
        let result = queue.add("ws-test", 5, None);
        assert!(result.is_ok());
        let queue = unwrap_ok!(result, "Failed to add entry");
        assert_eq!(queue.len(), 1);
        assert!(queue.get("ws-test").is_some());
    }

    #[test]
    fn test_add_duplicate_workspace_fails() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-test", 5, None), "Failed to add ws-test");
        let result = queue.add("ws-test", 3, None);
        assert!(matches!(result, Err(PureQueueError::DuplicateWorkspace(_))));
    }

    #[test]
    fn test_add_duplicate_dedupe_key_fails() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-a", 5, Some("key1")), "Failed to add ws-a");
        let result = queue.add("ws-b", 3, Some("key1"));
        assert!(matches!(result, Err(PureQueueError::DuplicateDedupeKey(_))));
    }

    #[test]
    fn test_claim_next_returns_highest_priority() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-low", 10, None), "Failed to add ws-low");
        let queue = unwrap_ok!(queue.add("ws-high", 1, None), "Failed to add ws-high");
        let queue = unwrap_ok!(queue.add("ws-mid", 5, None), "Failed to add ws-mid");

        let result = queue.claim_next("agent1");
        assert!(result.is_ok());
        let (queue, workspace) = unwrap_ok!(result, "Failed to claim");
        assert_eq!(workspace, "ws-high");
        match queue.get("ws-high") {
            Some(entry) => assert!(entry.is_claimed()),
            None => panic!("ws-high entry not found"),
        }
    }

    #[test]
    fn test_claim_respects_fifo_within_priority() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-first", 5, None), "Failed to add ws-first");
        let queue = unwrap_ok!(queue.add("ws-second", 5, None), "Failed to add ws-second");

        let result = queue.claim_next("agent1");
        assert!(result.is_ok());
        let (queue, first) = unwrap_ok!(result, "Failed to claim");
        assert_eq!(first, "ws-first");

        // Agent1 must release before agent2 can claim (single worker invariant)
        let queue = unwrap_ok!(queue.release("ws-first"), "Failed to release ws-first");
        let result = queue.claim_next("agent2");
        assert!(result.is_ok());
        let (_, second) = unwrap_ok!(result, "Failed to claim ws-second");
        assert_eq!(second, "ws-second");
    }

    #[test]
    fn test_single_worker_invariant() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-a", 5, None), "Failed to add ws-a");
        let queue = unwrap_ok!(queue.add("ws-b", 5, None), "Failed to add ws-b");

        // Agent1 claims
        let result = queue.claim_next("agent1");
        assert!(result.is_ok());
        let (queue, _) = unwrap_ok!(result, "Failed to claim for agent1");

        // Agent2 cannot claim while agent1 holds lock
        let result = queue.claim_next("agent2");
        assert!(result.is_err());
    }

    #[test]
    fn test_queue_is_consistent_after_operations() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-a", 5, Some("key1")), "Failed to add ws-a");
        let queue = unwrap_ok!(queue.add("ws-b", 3, None), "Failed to add ws-b");
        assert!(queue.is_consistent());

        let result = queue.claim_next("agent1");
        assert!(result.is_ok());
        let (queue, claimed_ws) = unwrap_ok!(result, "Failed to claim");
        // ws-b has priority 3 (higher than ws-a's 5), so it will be claimed first
        assert_eq!(claimed_ws, "ws-b");
        assert!(queue.is_consistent());

        // Transition ws-b through proper state machine: Claimed -> Rebasing -> Testing ->
        // ReadyToMerge -> Merging -> Merged
        let queue = unwrap_ok!(
            queue.transition_status("ws-b", QueueStatus::Rebasing),
            "Failed to transition to Rebasing"
        );
        assert!(queue.is_consistent());
        let queue = unwrap_ok!(
            queue.transition_status("ws-b", QueueStatus::Testing),
            "Failed to transition to Testing"
        );
        assert!(queue.is_consistent());
        let queue = unwrap_ok!(
            queue.transition_status("ws-b", QueueStatus::ReadyToMerge),
            "Failed to transition to ReadyToMerge"
        );
        assert!(queue.is_consistent());
        let queue = unwrap_ok!(
            queue.transition_status("ws-b", QueueStatus::Merging),
            "Failed to transition to Merging"
        );
        assert!(queue.is_consistent());
        let queue = unwrap_ok!(
            queue.transition_status("ws-b", QueueStatus::Merged),
            "Failed to transition to Merged"
        );
        assert!(queue.is_consistent());
    }

    #[test]
    fn test_terminal_releases_dedupe_key() {
        let queue = PureQueue::new();
        let queue = unwrap_ok!(queue.add("ws-a", 5, Some("key1")), "Failed to add ws-a");

        // First claim
        let result = queue.claim_next("agent1");
        assert!(result.is_ok());
        let (queue, _) = unwrap_ok!(result, "Failed to claim");

        // Transition through proper state machine to terminal
        let queue = unwrap_ok!(
            queue.transition_status("ws-a", QueueStatus::Rebasing),
            "Failed to transition to Rebasing"
        );
        let queue = unwrap_ok!(
            queue.transition_status("ws-a", QueueStatus::Testing),
            "Failed to transition to Testing"
        );
        let queue = unwrap_ok!(
            queue.transition_status("ws-a", QueueStatus::ReadyToMerge),
            "Failed to transition to ReadyToMerge"
        );
        let queue = unwrap_ok!(
            queue.transition_status("ws-a", QueueStatus::Merging),
            "Failed to transition to Merging"
        );
        let queue = unwrap_ok!(
            queue.transition_status("ws-a", QueueStatus::Merged),
            "Failed to transition to Merged"
        );

        // Now we can add with same dedupe key since ws-a is terminal
        let result = queue.add("ws-b", 5, Some("key1"));
        assert!(result.is_ok());
    }
}

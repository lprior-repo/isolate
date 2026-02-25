//! Lock management types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A lock on a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lock {
    /// Resource being locked
    pub resource: String,
    /// Lock holder
    pub holder: String,
    /// When acquired
    pub acquired_at: DateTime<Utc>,
    /// Lock timeout
    pub expires_at: DateTime<Utc>,
}

impl Lock {
    /// Create a new lock
    #[must_use]
    pub fn new(resource: String, holder: String, ttl_seconds: i64) -> Self {
        let now = Utc::now();
        Self {
            resource,
            holder,
            acquired_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
        }
    }

    /// Check if the lock is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the lock is valid (not expired)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}

/// Lock manager
#[derive(Debug, Clone, Default)]
pub struct LockManager {
    locks: Vec<Lock>,
}

impl LockManager {
    /// Create a new lock manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to acquire a lock on a resource
    pub fn acquire(&mut self, resource: &str, holder: &str, ttl_seconds: i64) -> Option<Lock> {
        // Check if already locked (clean up expired locks first)
        if self.is_locked(resource) {
            return None;
        }

        let lock = Lock::new(resource.to_string(), holder.to_string(), ttl_seconds);
        self.locks.push(lock.clone());
        Some(lock)
    }

    /// Release a lock on a resource
    pub fn release(&mut self, resource: &str, holder: &str) -> bool {
        let initial_len = self.locks.len();
        self.locks
            .retain(|l| !(l.resource == resource && l.holder == holder));
        self.locks.len() != initial_len
    }

    /// Check if a resource is locked
    #[must_use]
    pub fn is_locked(&self, resource: &str) -> bool {
        self.locks
            .iter()
            .any(|l| l.resource == resource && l.is_valid())
    }

    /// Get the holder of a lock on a resource
    #[must_use]
    pub fn get_holder(&self, resource: &str) -> Option<&str> {
        self.locks
            .iter()
            .find(|l| l.resource == resource && l.is_valid())
            .map(|l| l.holder.as_str())
    }

    /// Get all active locks
    #[must_use]
    pub fn active_locks(&self) -> Vec<&Lock> {
        self.locks.iter().filter(|l| l.is_valid()).collect()
    }

    /// Clean up expired locks
    pub fn cleanup_expired(&mut self) -> usize {
        let initial_len = self.locks.len();
        self.locks.retain(Lock::is_valid);
        initial_len - self.locks.len()
    }
}

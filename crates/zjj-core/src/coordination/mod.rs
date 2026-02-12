//! Coordination primitives for multi-agent session management.

pub mod locks;
pub mod queue;

pub use locks::{LockInfo, LockManager, LockResponse};
pub use queue::{
    MergeQueue, ProcessingLock, QueueAddResponse, QueueControlError, QueueEntry, QueueStats,
    QueueStatus, TransitionError,
};

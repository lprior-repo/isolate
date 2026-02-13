//! Coordination primitives for multi-agent session management.

pub mod locks;
pub mod queue;
pub mod worker_steps;

pub use locks::{LockInfo, LockManager, LockResponse};
pub use queue::{
    MergeQueue, ProcessingLock, QueueAddResponse, QueueControlError, QueueEntry, QueueStats,
    QueueStatus, TransitionError,
};
pub use worker_steps::{rebase_step, RebaseError, RebaseSuccess};

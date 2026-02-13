//! Coordination primitives for multi-agent session management.

pub mod locks;
pub mod queue;
pub mod worker_lifecycle;
pub mod worker_steps;

pub use locks::{LockInfo, LockManager, LockResponse};
pub use queue::{
    MergeQueue, ProcessingLock, QueueAddResponse, QueueControlError, QueueEntry, QueueStats,
    QueueStatus, TransitionError,
};
pub use worker_steps::{
    classify_step_error, classify_with_attempts, determine_failure_status, handle_step_failure,
    rebase_step, should_retry, ErrorClass, RebaseError, RebaseSuccess, WorkerError,
};

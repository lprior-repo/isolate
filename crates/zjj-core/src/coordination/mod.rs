//! Coordination primitives for multi-agent session management.

pub mod locks;
pub mod queue;
pub mod queue_entities;
pub mod queue_repository;
pub mod queue_status;
pub mod worker_lifecycle;
pub mod worker_steps;

pub use locks::{LockInfo, LockManager, LockResponse};
pub use queue::{
    MergeQueue, ProcessingLock, QueueAddResponse, QueueControlError, QueueEntry, QueueEvent,
    QueueStats,
};
pub use queue_repository::QueueRepository;
pub use queue_status::{QueueEventType, QueueStatus, TransitionError, WorkspaceQueueState};
pub use worker_lifecycle::{
    graceful_shutdown, wait_for_shutdown_signal, ActiveClaim, ClaimTracker, ShutdownResult,
};
pub use worker_steps::{
    classify_step_error, determine_failure_status, handle_step_failure, moon_gate_step,
    rebase_step, MoonGateConfig, MoonGateError, MoonGateSuccess, RebaseError, RebaseSuccess,
};

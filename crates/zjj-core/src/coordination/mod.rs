//! Coordination primitives for multi-agent session management.

pub mod conflict_resolutions;
pub mod conflict_resolutions_entities;
pub mod locks;
pub mod queue;
pub mod queue_entities;
pub mod queue_repository;
pub mod queue_status;
pub mod queue_submission;
pub mod stack_depth;
pub mod stack_error;
pub mod train;
pub mod worker_application;
pub mod worker_lifecycle;
pub mod worker_steps;

pub use conflict_resolutions::{
    get_conflict_resolutions, get_resolutions_by_decider, get_resolutions_by_time_range,
    init_conflict_resolutions_schema, insert_conflict_resolution,
};
pub use conflict_resolutions_entities::{ConflictResolution, ConflictResolutionError};
pub use locks::{LockInfo, LockManager, LockResponse};
pub use queue::{
    MergeQueue, ProcessingLock, QueueAddResponse, QueueControlError, QueueEntry, QueueEvent,
    QueueStats, RecoveryStats,
};
pub use queue_repository::QueueRepository;
pub use queue_status::{QueueEventType, QueueStatus, TransitionError, WorkspaceQueueState};
pub use queue_submission::{
    compute_dedupe_key, extract_workspace_identity, get_queue_position, is_in_queue,
    push_bookmark_to_remote, submit_to_queue, QueueSubmissionError, QueueSubmissionRequest,
    QueueSubmissionResponse, SubmissionType, WorkspaceIdentity,
};
pub use stack_depth::{calculate_stack_depth, find_stack_root};
pub use stack_error::StackError;
pub use train::{
    calculate_positions, filter_processable, sort_by_priority, EntryResult, EntryResultKind,
    MergeExecutor, QualityGate, TrainConfig, TrainError, TrainProcessor, TrainResult, TrainStep,
    TrainStepKind, TrainStepStatus,
};
pub use worker_application::{QualityGateRuntime, WorkerPipelineOutcome, WorkerPipelineService};
pub use worker_lifecycle::{
    graceful_shutdown, wait_for_shutdown_signal, ActiveClaim, ClaimTracker, ShutdownResult,
};
pub use worker_steps::{
    classify_step_error, determine_failure_status, handle_step_failure, moon_gate_step,
    rebase_step, MoonGateConfig, MoonGateError, MoonGateSuccess, RebaseError, RebaseSuccess,
};

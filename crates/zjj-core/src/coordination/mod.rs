//! # Coordination Layer
//!
//! This module implements the **coordination layer** for multi-agent session management.
//! It provides distributed coordination primitives for managing concurrent operations across
//! multiple agents and workspaces.
//!
//! ## Overview
//!
//! The coordination layer handles:
//! - **Queue management** - Distributed work queue with priority and deduplication
//! - **Conflict resolution** - Detecting and resolving merge conflicts
//! - **Worker lifecycle** - Agent claiming, heartbeat, and expiration
//! - **Merge trains** - Coordinated workspace processing
//! - **Locking** - Distributed locking for critical sections
//!
//! ## Module Structure
//!
//! ### Queue Management
//!
//! **Core queue types:**
//! - [`MergeQueue`] - Distributed queue for workspace processing
//! - [`QueueEntry`] - Individual queue entry with claim state
//! - [`QueueStatus`] - Current status of a workspace in the queue
//! - [`QueueStats`] - Statistics about queue state
//!
//! **Queue operations:**
//! - [`queue_submission`] - Submit work to the queue with deduplication
//! - [`pure_queue`] - Pure functional queue operations
//! - [`queue_repository`] - Queue persistence layer
//!
//! **Queue concepts:**
//! - **Priority** - Lower values = higher priority
//! - **Deduplication** - Prevent duplicate entries for same workspace
//! - **Claiming** - Agents claim entries for processing
//! - **Expiration** - Claims expire if not renewed
//!
//! ### Conflict Resolution
//!
//! **Conflict detection:**
//! - [`conflict_resolutions`] - Store and retrieve conflict resolutions
//! - [`conflict_resolutions_entities`] - Conflict resolution domain types
//! - [`ConflictResolution`] - Represents a resolved conflict
//!
//! Conflict resolution enables:
//! - Automatic conflict detection during merges
//! - Storing resolution decisions for reuse
//! - Tracking who resolved what and when
//!
//! ### Worker Management
//!
//! **Worker lifecycle:**
//! - [`worker_application`] - Worker pipeline and quality gates
//! - [`worker_lifecycle`] - Claim tracking and graceful shutdown
//! - [`worker_steps`] - Individual worker steps (rebase, moon-gate)
//!
//! **Worker concepts:**
//! - **Claiming** - Atomically claim work from queue
//! - **Heartbeat** - Periodic renewal of claims
//! - **Graceful shutdown** - Complete work before exiting
//! - **Quality gates** - Validate work before completion
//!
//! ### Merge Trains
//!
//! **Train processing:**
//! - [`train`] - Merge train orchestration
//! - [`TrainProcessor`] - Process entries in priority order
//! - [`TrainStep`] - Individual processing steps
//! - [`MergeExecutor`] - Execute merge operations
//!
//! **Train concepts:**
//! - **Priority order** - Process higher priority entries first
//! - **Quality gates** - Validate before proceeding
//! - **Step results** - Track success/failure of each step
//! - **Recovery** - Handle failures gracefully
//!
//! ### Locking
//!
//! **Distributed locks:**
//! - [`locks`] - Distributed lock manager
//! - [`LockManager`] - Acquire and release locks
//! - [`LockInfo`] - Lock metadata (owner, expiration)
//!
//! Locking ensures:
//! - Mutual exclusion for critical sections
//! - Automatic expiration on failure
//! - Safe cleanup on release
//!
//! ### Stack Management
//!
//! **Stack operations:**
//! - [`stack_depth`] - Calculate depth of workspace stacks
//! - [`find_stack_root`] - Find the root of a stack
//! - [`StackError`] - Stack operation errors
//!
//! Stack concepts:
//! - **Nested workspaces** - Workspaces can be nested (stacked)
//! - **Root detection** - Find the base workspace
//! - **Depth calculation** - Determine nesting level
//!
//! ## Common Patterns
//!
//! ### Queue Submission
//!
//! ```rust
//! use zjj_core::coordination::queue_submission::{submit_to_queue, QueueSubmissionRequest};
//! use zjj_core::domain::WorkspaceName;
//!
//! let request = QueueSubmissionRequest {
//!     workspace: WorkspaceName::parse("my-workspace")?,
//!     bead_id: Some(bead_id),
//!     priority: 1,
//! };
//!
//! let response = submit_to_queue(&repo, &request)?;
//! println!("Queue position: {}", response.position);
//! ```
//!
//! ### Worker Claim Processing
//!
//! ```rust
//! use zjj_core::coordination::worker_lifecycle::ClaimTracker;
//! use zjj_core::coordination::queue::claim_next_entry;
//!
//! let mut tracker = ClaimTracker::new();
//!
//! // Claim next entry
//! if let Some(entry) = claim_next_entry(&repo, &agent_id, 300)? {
//!     tracker.track_claim(entry.id, agent_id.clone());
//!
//!     // Process entry
//!     process_entry(&entry)?;
//!
//!     // Release claim
//!     release_entry(&repo, entry.id, &agent_id)?;
//!     tracker.untrack_claim(&entry.id);
//! }
//! ```
//!
//! ### Merge Train Processing
//!
//! ```rust
//! use zjj_core::coordination::train::TrainProcessor;
//!
//! let processor = TrainProcessor::new(config);
//!
//! let result = processor.process_train(
//!     &repo,
//!     &entries,
//!     &agent_id,
//! )?;
//!
//! match result {
//!     TrainResult::Success => println!("All entries processed"),
//!     TrainResult::Partial { failed, .. } => println!("{} entries failed", failed),
//!     TrainResult::Failure(error) => println!("Train failed: {}", error),
//! }
//! ```
//!
//! ## Domain Types
//!
//! This module re-exports domain types from [`domain_types`]:
//! - [`AgentId`] - Agent identifier
//! - [`BeadId`] - Bead identifier
//! - [`WorkspaceName`] - Workspace name
//! - [`DedupeKey`] - Deduplication key for queue entries
//! - [`Priority`] - Queue priority value
//! - [`DomainError`] - Domain error type
//!
//! ## Related Modules
//!
//! - **`crate::domain`** - Core domain types and aggregates
//! - **`crate::output`** - Output types for coordination operations
//! - **`crate::beads`** - Beads issue tracker integration

pub mod conflict_resolutions;
pub mod conflict_resolutions_entities;
pub mod domain_types;
pub mod locks;
pub mod pure_queue;
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
pub use domain_types::{
    AgentId, BeadId, DedupeKey, DomainError, Priority, QueueEntryId, WorkspaceName,
};
pub use locks::{LockInfo, LockManager, LockResponse};
pub use pure_queue::{PureEntry, PureQueue, PureQueueError};
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

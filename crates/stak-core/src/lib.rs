//! Stak-core - Core coordination types and logic
//!
//! This crate provides:
//! - Queue management types
//! - Agent coordination types
//! - Lock management types
//! - Event types

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

pub mod agent;
pub mod error;
pub mod events;
pub mod lock;
pub mod queue;

pub use agent::{Agent, AgentId, AgentStatus};
pub use error::{Error, Result};
pub use events::{Event, EventType};
pub use lock::{Lock, LockManager};
pub use queue::{Queue, QueueEntry, QueueEntryId, QueueStatus};

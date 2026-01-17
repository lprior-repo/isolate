//! ZJJ library interface
//!
//! This module exposes internal types for testing and benchmarking purposes.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

// Re-export modules needed by benchmarks and tests
pub mod database;
pub mod json_output;
pub mod session;

// Re-export commonly used types
pub use database::SessionDb;
pub use session::{Session, SessionStatus, SessionUpdate};

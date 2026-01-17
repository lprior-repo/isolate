//! JSON output structures for AI-first CLI design.
//!
//! This module provides consistent JSON output formats across all commands,
//! organized into logical layers:
//!
//! - **types**: Core data structures (JsonSuccess, JsonError, ErrorDetail, ErrorCode)
//! - **builders**: Error construction and conversion utilities
//! - **serialization**: Generic JSON serialization trait and helpers

mod builders;
mod serialization;
mod types;

// Re-export public API
pub use builders::{error_with_available_sessions, JsonError};
pub use serialization::JsonSerializable;
pub use types::{ErrorCode, ErrorDetail, JsonSuccess};

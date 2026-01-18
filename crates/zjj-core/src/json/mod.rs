//! JSON output structures for AI-first CLI design.
//!
//! This module provides consistent JSON output formats across all commands,
//! organized into logical layers:
//!
//! - **types**: Core data structures (JsonSuccess, JsonError, ErrorDetail, ErrorCode)
//! - **builders**: Error construction and conversion utilities
//! - **serialization**: Generic JSON serialization trait and helpers
//! - **schema**: Schema versioning for API outputs

mod builders;
mod schema;
mod serialization;
mod types;

// Re-export public API
pub use builders::error_with_available_sessions;
pub use schema::{SchemaEnvelope, SchemaType, WithSchema, SCHEMA_BASE_URL, SCHEMA_VERSION};
pub use serialization::JsonSerializable;
pub use types::{ErrorCode, ErrorDetail, JsonError, JsonSuccess};

//! JSON schema generation helpers for command outputs.
//!
//! This module provides utilities for generating JSON schemas for zjj command outputs.
//! Schemas are useful for:
//! - Documentation generation
//! - API contract validation
//! - Client code generation
//! - IDE autocomplete
//!
//! ## Module Organization
//!
//! - **types**: Core JSON schema data structures (JsonSchema, PropertySchema)
//! - **builders**: Builder methods for fluent API construction
//! - **generators**: Schema generators for specific command outputs

mod builders;
mod generators;
mod types;

// Re-export public API
pub use generators::{
    config_output_schema, diff_output_schema, list_output_schema, status_output_schema,
};
pub use types::{JsonSchema, PropertySchema, SCHEMA_VERSION};

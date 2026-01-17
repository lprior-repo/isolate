//! Contract trait for types with contracts
//!
//! This module defines the HasContract trait for types that have contracts.
//! The trait provides:
//! - contract(): Get the contract for a type
//! - validate(&self): Validate an instance against its contract
//! - json_schema(): Get JSON Schema representation

use serde_json::Value;

use super::types::TypeContract;

/// Trait for types that have contracts
///
/// Implementing this trait enables AI-first type introspection and validation.
/// Types with contracts provide:
/// - Machine-readable schemas (JSON Schema compatible)
/// - Constraint information (regex patterns, ranges, enums, etc.)
/// - Contextual hints for AI agents (best practices, security notes, examples)
/// - Field-level contracts for composite types
pub trait HasContract {
    /// Get the contract for this type
    fn contract() -> TypeContract;

    /// Validate an instance against its contract
    fn validate(&self) -> crate::Result<()>;

    /// Get JSON Schema representation
    ///
    /// This is a convenience method that converts the type's contract
    /// to JSON Schema format for use with validators and API documentation tools.
    fn json_schema() -> Value {
        Self::contract().to_json_schema()
    }
}

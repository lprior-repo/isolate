//! Type contracts and validation system
//!
//! Provides rich type information for AI-first design:
//! - Constraints (min/max, regex patterns, enums)
//! - Contextual hints (examples, suggestions, security notes)
//! - Dependencies between fields
//! - Machine-readable JSON schemas for validation
//!
//! ## Architecture
//!
//! This module is organized into functional layers:
//!
//! - **types.rs**: Core type definitions (TypeContract, FieldContract, Constraint, etc.)
//! - **builders.rs**: Fluent builders for constructing contracts (TypeContractBuilder, FieldContractBuilder)
//! - **serialization.rs**: JSON Schema conversion and validation logic
//! - **traits.rs**: HasContract trait for types with contracts
//! - **tests.rs**: Comprehensive test suite
//!
//! ## Usage
//!
//! ```ignore
//! use zjj_core::contracts::{TypeContract, FieldContract, Constraint, HasContract};
//!
//! // Build a contract using fluent API
//! let contract = TypeContract::builder("Session")
//!     .description("A development session")
//!     .field(
//!         "name",
//!         FieldContract::builder("name", "String")
//!             .required()
//!             .constraint(Constraint::Length {
//!                 min: Some(1),
//!                 max: Some(64),
//!             })
//!             .build(),
//!     )
//!     .build();
//!
//! // Convert to JSON Schema for validation
//! let schema = contract.to_json_schema();
//! ```

pub mod builders;
pub mod serialization;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export core types for convenience
pub use builders::{FieldContractBuilder, TypeContractBuilder};
pub use traits::HasContract;
pub use types::{Constraint, ContextualHint, FieldContract, HintType, TypeContract};

// Impl blocks re-exported via serialization module
pub use serialization::*;

/// Convenience method to create a TypeContract builder
impl TypeContract {
    /// Create a builder for constructing contracts
    pub fn builder(name: impl Into<String>) -> TypeContractBuilder {
        TypeContractBuilder::new(name)
    }
}

/// Convenience method to create a FieldContract builder
impl FieldContract {
    /// Create a builder for field contracts
    pub fn builder(name: impl Into<String>, field_type: impl Into<String>) -> FieldContractBuilder {
        FieldContractBuilder::new(name, field_type)
    }
}

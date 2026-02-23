//! Domain primitive types (semantic newtypes).
//!
//! This module provides type-safe identifiers and domain primitives
//! to prevent primitive obsession and make illegal states unrepresentable.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DOMAIN ERRORS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Error type for domain primitive validation.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DomainError {
    /// Empty string provided where non-empty was required.
    #[error("empty string not allowed for {field}")]
    Empty { field: &'static str },

    /// Invalid format for domain primitive.
    #[error("invalid format for {field}: {value}")]
    InvalidFormat { field: &'static str, value: String },

    /// ID cannot be parsed.
    #[error("cannot parse {field} from '{value}'")]
    ParseError { field: &'static str, value: String },
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// QUEUE ENTRY ID - RE-EXPORTED FROM DOMAIN
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// QueueEntryId is now defined in `crate::domain::identifiers` as the
// single source of truth. This module re-exports it for backward compatibility.
//
// Migration guide:
// - No changes needed - API remains the same
// - Error type changed from `DomainError` to `IdentifierError`
//
// The canonical implementation:
// - Uses i64 representation (database auto-increment ID)
// - Validates value is positive (> 0)
// - Prevents confusion with other i64 values

pub use crate::domain::identifiers::QueueEntryId;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// WORKSPACE NAME - RE-EXPORTED FROM DOMAIN
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// WorkspaceName is now defined in `crate::domain::identifiers` as the
// single source of truth. This module re-exports it for backward compatibility.
//
// Migration guide:
// - Old: `WorkspaceName::new(value)` or `WorkspaceName::from_str(value)`
// - New: `WorkspaceName::parse(value)`
//
// The canonical implementation validates:
// - Non-empty
// - No path separators (/ or \)
// - No null bytes
// - Max 255 characters

pub use crate::domain::identifiers::WorkspaceName;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AGENT ID (Re-export from domain layer)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// Re-export AgentId from domain layer (single source of truth)
//
// The domain::identifiers module provides the canonical AgentId implementation
// following DDD principles with comprehensive validation (1-128 chars, alphanumeric
// plus hyphen, underscore, dot, and colon).
pub use crate::domain::identifiers::AgentId;

// Re-export BeadId from domain layer (single source of truth)
//
// BeadId is a type alias for TaskId in the domain layer, as beads and tasks
// use the same identifier format (bd-{hex}). The canonical implementation
// in domain::identifiers provides full validation for this format.
pub use crate::domain::identifiers::BeadId;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// DEDUPE KEY
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Deduplication key for queue entries.
///
/// Prevents duplicate work by rejecting entries with duplicate keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DedupeKey(String);

impl DedupeKey {
    /// Create a new `DedupeKey`, ensuring non-empty.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Empty` if the string is empty.
    pub fn new(value: String) -> Result<Self, DomainError> {
        if value.is_empty() {
            return Err(DomainError::Empty {
                field: "dedupe_key",
            });
        }
        Ok(Self(value))
    }

    /// Create a `DedupeKey` from a str, ensuring non-empty.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::Empty` if the string is empty.
    pub fn new_from_str(value: &str) -> Result<Self, DomainError> {
        if value.is_empty() {
            return Err(DomainError::Empty {
                field: "dedupe_key",
            });
        }
        Ok(Self(value.to_string()))
    }

    /// Get the underlying string value.
    #[must_use]
    pub fn as_str(&self) -> &str {

        &self.0
    }

    /// Convert into the underlying String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for DedupeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// PRIORITY
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Queue priority value.
///
/// Lower values indicate higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Priority(i32);

impl Priority {
    /// Create a new Priority value.
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    /// Get the underlying i32 value.
    #[must_use]
    pub const fn value(self) -> i32 {
        self.0
    }

    /// Default priority (5).
    #[must_use]
    pub const fn default() -> Self {
        Self(5)
    }

    /// High priority (1).
    #[must_use]
    pub const fn high() -> Self {
        Self(1)
    }

    /// Low priority (10).
    #[must_use]
    pub const fn low() -> Self {
        Self(10)
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // --- QueueEntryId Tests ---

    #[test]
    fn test_queue_entry_id_valid() {
        match QueueEntryId::new(1) {
            Ok(id) => {
                assert_eq!(id.value(), 1);
            }
            Err(e) => panic!("Failed to create QueueEntryId: {e}"),
        }
    }

    #[test]
    fn test_queue_entry_id_zero_is_invalid() {
        let id = QueueEntryId::new(0);
        assert!(matches!(id, Err(crate::domain::identifiers::IdentifierError::InvalidFormat { .. })));
    }

    #[test]
    fn test_queue_entry_id_negative_is_invalid() {
        let id = QueueEntryId::new(-1);
        assert!(matches!(id, Err(crate::domain::identifiers::IdentifierError::InvalidFormat { .. })));
    }

    #[test]
    fn test_queue_entry_id_from_str_valid() {
        match "42".parse::<QueueEntryId>() {
            Ok(id) => {
                assert_eq!(id.value(), 42);
            }
            Err(e) => panic!("Failed to parse QueueEntryId: {e}"),
        }
    }

    #[test]
    fn test_queue_entry_id_from_str_invalid() {
        let id = "abc".parse::<QueueEntryId>();
        assert!(matches!(id, Err(crate::domain::identifiers::IdentifierError::InvalidFormat { .. })));
    }

    // --- WorkspaceName Tests ---
    //
    // WorkspaceName is now re-exported from domain::identifiers.
    // See domain/identifiers.rs for comprehensive tests.

    #[test]
    fn test_workspace_name_valid() {
        match WorkspaceName::parse("test-workspace") {
            Ok(name) => {
                assert_eq!(name.as_str(), "test-workspace");
            }
            Err(e) => panic!("Failed to create WorkspaceName: {e}"),
        }
    }

    #[test]
    fn test_workspace_name_empty_is_invalid() {
        let name = WorkspaceName::parse("");
        assert!(matches!(name, Err(crate::domain::identifiers::IdError::Empty)));
    }

    #[test]
    fn test_workspace_name_from_str() {
        match WorkspaceName::parse("my-workspace") {
            Ok(name) => {
                assert_eq!(name.as_str(), "my-workspace");
            }
            Err(e) => panic!("Failed to parse WorkspaceName: {e}"),
        }
    }

    // --- AgentId Tests ---
    // AgentId is now re-exported from domain::identifiers
    // Tests are in domain/identifiers.rs

    // --- BeadId Tests ---
    // BeadId is now re-exported from domain::identifiers (as alias to TaskId)
    // Tests are in domain/identifiers.rs
    // BeadId validates bd-{hex} format, e.g., bd-abc123

    // --- DedupeKey Tests ---

    #[test]
    fn test_dedupe_key_valid() {
        let key = DedupeKey::new("key-abc".to_string());
        assert!(key.is_ok());
        assert_eq!(key.unwrap().as_str(), "key-abc");
    }

    #[test]
    fn test_dedupe_key_empty_is_invalid() {
        let key = DedupeKey::new(String::new());
        assert!(matches!(key, Err(DomainError::Empty { .. })));
    }

    // --- Priority Tests ---

    #[test]
    fn test_priority_new() {
        let p = Priority::new(7);
        assert_eq!(p.value(), 7);
    }

    #[test]
    fn test_priority_default() {
        let p = Priority::default();
        assert_eq!(p.value(), 5);
    }

    #[test]
    fn test_priority_high() {
        let p = Priority::high();
        assert_eq!(p.value(), 1);
    }

    #[test]
    fn test_priority_low() {
        let p = Priority::low();
        assert_eq!(p.value(), 10);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::high() < Priority::default());
        assert!(Priority::default() < Priority::low());
    }
}

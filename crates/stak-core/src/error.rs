//! Error types for stak-core

use thiserror::Error;

/// Core error type for stak operations
#[derive(Debug, Error)]
pub enum Error {
    /// Queue-related errors
    #[error("Queue error: {0}")]
    QueueError(String),

    /// Lock-related errors
    #[error("Lock error: {0}")]
    LockError(String),

    /// Agent-related errors
    #[error("Agent error: {0}")]
    AgentError(String),

    /// Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Conflict errors (e.g., duplicate entry)
    #[error("Conflict: {0}")]
    Conflict(String),
}

/// Result type alias for stak-core operations
pub type Result<T> = std::result::Result<T, Error>;

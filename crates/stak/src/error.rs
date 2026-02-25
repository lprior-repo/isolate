use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Lock error: {0}")]
    LockError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

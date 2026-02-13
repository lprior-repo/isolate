//! Error classification for queue worker operations.
//!
//! Re-exports from zjj-core for backward compatibility.

pub use zjj_core::worker_error::{
    classify_anyhow_with_attempts, classify_error_message, classify_with_attempts, from_anyhow,
    should_retry, ErrorClass, WorkerError,
};

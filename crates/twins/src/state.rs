#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! State management module for twin runtime
//!
//! Provides in-memory state tracking for requests and responses.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recorded request/response pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    /// Unique identifier for this request
    pub id: String,
    /// Timestamp of the request
    pub timestamp: DateTime<Utc>,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub request_headers: HashMap<String, String>,
    /// Request body (if present)
    #[serde(default)]
    pub request_body: Option<String>,
    /// Response status code
    pub status: u16,
    /// Response headers
    pub response_headers: HashMap<String, String>,
    /// Response body
    #[serde(default)]
    pub response_body: Option<String>,
}

impl RequestRecord {
    /// Create a new request record
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        method: String,
        path: String,
        request_headers: HashMap<String, String>,
        request_body: Option<String>,
        status: u16,
        response_headers: HashMap<String, String>,
        response_body: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method,
            path,
            request_headers,
            request_body,
            status,
            response_headers,
            response_body,
        }
    }
}

/// Trait for twin state storage
pub trait TwinState: Default {
    /// Add a request record
    #[must_use]
    fn add_record(&self, record: RequestRecord) -> Self;

    /// Get all request records
    fn get_records(&self) -> Vector<RequestRecord>;

    /// Get record count
    fn record_count(&self) -> usize;

    /// Clear all records
    #[must_use]
    fn clear(&self) -> Self;
}

/// In-memory twin state using immutable data structures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InMemoryTwinState {
    /// Request/response history
    records: Vector<RequestRecord>,
}

impl InMemoryTwinState {
    /// Create a new empty state
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vector::new(),
        }
    }
}

impl TwinState for InMemoryTwinState {
    fn add_record(&self, record: RequestRecord) -> Self {
        let mut new_records = self.records.clone();
        new_records.push_back(record);
        Self {
            records: new_records,
        }
    }

    fn get_records(&self) -> Vector<RequestRecord> {
        self.records.clone()
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn clear(&self) -> Self {
        Self {
            records: Vector::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_record() {
        let state = InMemoryTwinState::new();
        let record = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let new_state = state.add_record(record);
        assert_eq!(new_state.record_count(), 1);
    }

    #[test]
    fn test_clear() {
        let state = InMemoryTwinState::new();
        let record = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let state_with_record = state.add_record(record);
        let cleared = state_with_record.clear();
        assert_eq!(cleared.record_count(), 0);
    }
}

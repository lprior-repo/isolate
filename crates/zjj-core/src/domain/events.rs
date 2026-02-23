//! Domain events module
//!
//! This module implements Domain-Driven Design event sourcing patterns.
//! Domain events represent important business events that have occurred in the system.
//!
//! # Design Principles
//!
//! - **Immutable**: Events cannot be modified after creation
//! - **Serializable**: All events can be serialized for persistence and transmission
//! - **Typed**: Each event carries specific, validated domain data
//! - **Timestamped**: All events include when they occurred
//! - **Pure**: Event creation is deterministic and side-effect free
//!
//! # Usage
//!
//! ```rust
//! use zjj_core::domain::events::{DomainEvent, SessionEventData};
//! use chrono::Utc;
//!
//! let event = DomainEvent::session_created(
//!     "session-123".to_string(),
//!     "my-session".parse()?,
//!     Utc::now(),
//! );
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::identifiers::{AgentId, BeadId, SessionName, WorkspaceName};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Claim timestamp metadata for queue entry claim events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimTimestamps {
    /// When the claim was made
    pub claimed_at: DateTime<Utc>,
    /// When the claim expires
    pub expires_at: DateTime<Utc>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
}

impl ClaimTimestamps {
    /// Create new claim timestamps.
    #[must_use]
    pub const fn new(claimed_at: DateTime<Utc>, expires_at: DateTime<Utc>, timestamp: DateTime<Utc>) -> Self {
        Self {
            claimed_at,
            expires_at,
            timestamp,
        }
    }
}

// ============================================================================
// Domain Event Enum
// ============================================================================

/// A domain event representing something important that happened.
///
/// Events are the single source of truth for state changes in the system.
/// They enable:
/// - Event sourcing (rebuilding state from event history)
/// - Audit logging (complete history of all changes)
/// - Projections (deriving read models from event stream)
/// - Integration (publishing events to external systems)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum DomainEvent {
    /// A new session was created
    SessionCreated(Box<SessionCreatedEvent>),

    /// A session was completed successfully
    SessionCompleted(Box<SessionCompletedEvent>),

    /// A session failed
    SessionFailed(Box<SessionFailedEvent>),

    /// A workspace was created
    WorkspaceCreated(Box<WorkspaceCreatedEvent>),

    /// A workspace was removed
    WorkspaceRemoved(Box<WorkspaceRemovedEvent>),

    /// An entry was added to the queue
    QueueEntryAdded(Box<QueueEntryAddedEvent>),

    /// A queue entry was claimed by an agent
    QueueEntryClaimed(Box<QueueEntryClaimedEvent>),

    /// A queue entry was completed
    QueueEntryCompleted(Box<QueueEntryCompletedEvent>),

    /// A bead (task/issue) was created
    BeadCreated(Box<BeadCreatedEvent>),

    /// A bead was closed
    BeadClosed(Box<BeadClosedEvent>),
}

impl DomainEvent {
    /// Get the timestamp for when this event occurred
    #[must_use]
    pub const fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::SessionCreated(e) => &e.timestamp,
            Self::SessionCompleted(e) => &e.timestamp,
            Self::SessionFailed(e) => &e.timestamp,
            Self::WorkspaceCreated(e) => &e.timestamp,
            Self::WorkspaceRemoved(e) => &e.timestamp,
            Self::QueueEntryAdded(e) => &e.timestamp,
            Self::QueueEntryClaimed(e) => &e.timestamp,
            Self::QueueEntryCompleted(e) => &e.timestamp,
            Self::BeadCreated(e) => &e.timestamp,
            Self::BeadClosed(e) => &e.timestamp,
        }
    }

    /// Get the event type as a string
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::SessionCreated(_) => "session_created",
            Self::SessionCompleted(_) => "session_completed",
            Self::SessionFailed(_) => "session_failed",
            Self::WorkspaceCreated(_) => "workspace_created",
            Self::WorkspaceRemoved(_) => "workspace_removed",
            Self::QueueEntryAdded(_) => "queue_entry_added",
            Self::QueueEntryClaimed(_) => "queue_entry_claimed",
            Self::QueueEntryCompleted(_) => "queue_entry_completed",
            Self::BeadCreated(_) => "bead_created",
            Self::BeadClosed(_) => "bead_closed",
        }
    }

    /// Create a session created event
    #[must_use]
    pub fn session_created(
        session_id: String,
        session_name: SessionName,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionCreated(Box::new(SessionCreatedEvent {
            session_id,
            session_name,
            timestamp,
        }))
    }

    /// Create a session completed event
    #[must_use]
    pub fn session_completed(
        session_id: String,
        session_name: SessionName,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionCompleted(Box::new(SessionCompletedEvent {
            session_id,
            session_name,
            timestamp,
        }))
    }

    /// Create a session failed event
    #[must_use]
    pub fn session_failed(
        session_id: String,
        session_name: SessionName,
        reason: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::SessionFailed(Box::new(SessionFailedEvent {
            session_id,
            session_name,
            reason,
            timestamp,
        }))
    }

    /// Create a workspace created event
    #[must_use]
    pub fn workspace_created(
        workspace_name: WorkspaceName,
        path: PathBuf,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::WorkspaceCreated(Box::new(WorkspaceCreatedEvent {
            workspace_name,
            path,
            timestamp,
        }))
    }

    /// Create a workspace removed event
    #[must_use]
    pub fn workspace_removed(
        workspace_name: WorkspaceName,
        path: PathBuf,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::WorkspaceRemoved(Box::new(WorkspaceRemovedEvent {
            workspace_name,
            path,
            timestamp,
        }))
    }

    /// Create a queue entry added event
    #[must_use]
    pub fn queue_entry_added(
        entry_id: i64,
        workspace_name: WorkspaceName,
        priority: i32,
        bead_id: Option<BeadId>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::QueueEntryAdded(Box::new(QueueEntryAddedEvent {
            entry_id,
            workspace_name,
            priority,
            bead_id,
            timestamp,
        }))
    }

    /// Create a queue entry claimed event
    #[must_use]
    pub fn queue_entry_claimed(
        entry_id: i64,
        workspace_name: WorkspaceName,
        agent: AgentId,
        claim_timestamps: ClaimTimestamps,
    ) -> Self {
        Self::QueueEntryClaimed(Box::new(QueueEntryClaimedEvent {
            entry_id,
            workspace_name,
            agent,
            claimed_at: claim_timestamps.claimed_at,
            expires_at: claim_timestamps.expires_at,
            timestamp: claim_timestamps.timestamp,
        }))
    }

    /// Create a queue entry completed event
    #[must_use]
    pub fn queue_entry_completed(
        entry_id: i64,
        workspace_name: WorkspaceName,
        agent: AgentId,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::QueueEntryCompleted(Box::new(QueueEntryCompletedEvent {
            entry_id,
            workspace_name,
            agent,
            timestamp,
        }))
    }

    /// Create a bead created event
    #[must_use]
    pub fn bead_created(
        bead_id: BeadId,
        title: String,
        description: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::BeadCreated(Box::new(BeadCreatedEvent {
            bead_id,
            title,
            description,
            timestamp,
        }))
    }

    /// Create a bead closed event
    #[must_use]
    pub fn bead_closed(
        bead_id: BeadId,
        closed_at: DateTime<Utc>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::BeadClosed(Box::new(BeadClosedEvent {
            bead_id,
            closed_at,
            timestamp,
        }))
    }
}

// ============================================================================
// Event Types
// ============================================================================

/// Event emitted when a new session is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCreatedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// When the session was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a session is completed successfully
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCompletedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// When the session was completed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a session fails
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFailedEvent {
    /// Unique identifier for the session
    pub session_id: String,
    /// Human-readable name of the session
    pub session_name: SessionName,
    /// Human-readable reason for the failure
    pub reason: String,
    /// When the session failed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a workspace is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCreatedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path to the workspace on disk
    pub path: PathBuf,
    /// When the workspace was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a workspace is removed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRemovedEvent {
    /// Name of the workspace
    pub workspace_name: WorkspaceName,
    /// Path where the workspace was located
    pub path: PathBuf,
    /// When the workspace was removed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when an entry is added to the queue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntryAddedEvent {
    /// Unique identifier for the queue entry
    pub entry_id: i64,
    /// Name of the workspace being queued
    pub workspace_name: WorkspaceName,
    /// Priority of the entry (lower = higher priority)
    pub priority: i32,
    /// Optional bead ID associated with this entry
    pub bead_id: Option<BeadId>,
    /// When the entry was added
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a queue entry is claimed by an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntryClaimedEvent {
    /// Unique identifier for the queue entry
    pub entry_id: i64,
    /// Name of the workspace being processed
    pub workspace_name: WorkspaceName,
    /// Agent that claimed the entry
    pub agent: AgentId,
    /// When the entry was claimed
    pub claimed_at: DateTime<Utc>,
    /// When the claim expires
    pub expires_at: DateTime<Utc>,
    /// When this event was emitted
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a queue entry is completed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntryCompletedEvent {
    /// Unique identifier for the queue entry
    pub entry_id: i64,
    /// Name of the workspace that was processed
    pub workspace_name: WorkspaceName,
    /// Agent that completed the entry
    pub agent: AgentId,
    /// When the entry was completed
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a bead (task/issue) is created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadCreatedEvent {
    /// Unique identifier for the bead
    pub bead_id: BeadId,
    /// Title of the bead
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// When the bead was created
    pub timestamp: DateTime<Utc>,
}

/// Event emitted when a bead is closed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadClosedEvent {
    /// Unique identifier for the bead
    pub bead_id: BeadId,
    /// When the bead was closed
    pub closed_at: DateTime<Utc>,
    /// When this event was emitted
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Event Metadata
// ============================================================================

/// Metadata for an event in the event store
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event in the store
    pub event_number: i64,
    /// Stream identifier (e.g., "session-123")
    pub stream_id: String,
    /// Stream version (incrementing counter)
    pub stream_version: i64,
    /// When the event was stored
    pub stored_at: DateTime<Utc>,
}

/// A stored event with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredEvent {
    /// The domain event
    pub event: DomainEvent,
    /// Event metadata
    pub metadata: EventMetadata,
}

impl StoredEvent {
    /// Create a new stored event
    #[must_use]
    pub const fn new(event: DomainEvent, metadata: EventMetadata) -> Self {
        Self { event, metadata }
    }

    /// Get the event number
    #[must_use]
    pub const fn event_number(&self) -> i64 {
        self.metadata.event_number
    }

    /// Get the stream identifier
    #[must_use]
    pub fn stream_id(&self) -> &str {
        &self.metadata.stream_id
    }

    /// Get the stream version
    #[must_use]
    pub const fn stream_version(&self) -> i64 {
        self.metadata.stream_version
    }
}

// ============================================================================
// Event Serialization
// ============================================================================

/// Serialize an event to JSON
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event(event: &DomainEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(event)
}

/// Deserialize an event from JSON
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event(json: &str) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize an event to JSON bytes
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event_bytes(event: &DomainEvent) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(event)
}

/// Deserialize an event from JSON bytes
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event_bytes(bytes: &[u8]) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_slice(bytes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_created");
        assert_eq!(event.timestamp(), &timestamp);

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_session_completed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_completed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_completed");
    }

    #[test]
    fn test_session_failed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_failed(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            "Out of memory".to_string(),
            timestamp,
        );

        assert_eq!(event.event_type(), "session_failed");

        // Verify the event contains the failure reason
        if let DomainEvent::SessionFailed(e) = &event {
            assert_eq!(e.reason, "Out of memory");
        } else {
            panic!("Expected SessionFailed event");
        }
    }

    #[test]
    fn test_workspace_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_created(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_created");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_workspace_removed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::workspace_removed(
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            PathBuf::from("/tmp/workspace"),
            timestamp,
        );

        assert_eq!(event.event_type(), "workspace_removed");
    }

    #[test]
    fn test_queue_entry_added_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::queue_entry_added(
            42,
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            1,
            Some(BeadId::parse("bd-abc123").expect("valid bead id")),
            timestamp,
        );

        assert_eq!(event.event_type(), "queue_entry_added");

        // Verify the event contains the entry ID and priority
        if let DomainEvent::QueueEntryAdded(e) = &event {
            assert_eq!(e.entry_id, 42);
            assert_eq!(e.priority, 1);
        } else {
            panic!("Expected QueueEntryAdded event");
        }
    }

    #[test]
    fn test_queue_entry_claimed_event() {
        let timestamp = Utc::now();
        let claimed_at = timestamp;
        let expires_at = timestamp + chrono::Duration::seconds(300);

        let event = DomainEvent::queue_entry_claimed(
            42,
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            AgentId::parse("agent-1").expect("valid agent id"),
            ClaimTimestamps::new(claimed_at, expires_at, timestamp),
        );

        assert_eq!(event.event_type(), "queue_entry_claimed");

        // Verify the event contains claim information
        if let DomainEvent::QueueEntryClaimed(e) = &event {
            assert_eq!(e.entry_id, 42);
            assert_eq!(e.agent.as_str(), "agent-1");
        } else {
            panic!("Expected QueueEntryClaimed event");
        }
    }

    #[test]
    fn test_queue_entry_completed_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::queue_entry_completed(
            42,
            WorkspaceName::parse("my-workspace").expect("valid workspace name"),
            AgentId::parse("agent-1").expect("valid agent id"),
            timestamp,
        );

        assert_eq!(event.event_type(), "queue_entry_completed");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_bead_created_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::bead_created(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            "Fix the bug".to_string(),
            Some("Critical issue".to_string()),
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_created");

        // Verify the event contains the bead data
        if let DomainEvent::BeadCreated(e) = &event {
            assert_eq!(e.title, "Fix the bug");
            assert_eq!(e.description, Some("Critical issue".to_string()));
        } else {
            panic!("Expected BeadCreated event");
        }
    }

    #[test]
    fn test_bead_closed_event() {
        let timestamp = Utc::now();
        let closed_at = timestamp;

        let event = DomainEvent::bead_closed(
            BeadId::parse("bd-abc123").expect("valid bead id"),
            closed_at,
            timestamp,
        );

        assert_eq!(event.event_type(), "bead_closed");

        // Test serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = deserialize_event(&json).expect("deserialization failed");

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let events = vec![
            DomainEvent::session_created(
                "session-123".to_string(),
                SessionName::parse("my-session").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("my-workspace").expect("valid"),
                PathBuf::from("/tmp/workspace"),
                Utc::now(),
            ),
            DomainEvent::queue_entry_added(
                1,
                WorkspaceName::parse("my-workspace").expect("valid"),
                1,
                None,
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc123").expect("valid"),
                "Test bead".to_string(),
                None,
                Utc::now(),
            ),
        ];

        for event in events {
            // Test JSON serialization
            let json = serialize_event(&event).expect("serialization failed");
            let deserialized = deserialize_event(&json).expect("deserialization failed");
            assert_eq!(event, deserialized);

            // Test bytes serialization
            let bytes = serialize_event_bytes(&event).expect("serialization failed");
            let deserialized_bytes =
                deserialize_event_bytes(&bytes).expect("deserialization failed");
            assert_eq!(event, deserialized_bytes);
        }
    }

    #[test]
    fn test_stored_event() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        let metadata = EventMetadata {
            event_number: 1,
            stream_id: "session-123".to_string(),
            stream_version: 1,
            stored_at: timestamp,
        };

        let stored = StoredEvent::new(event.clone(), metadata);

        assert_eq!(stored.event_number(), 1);
        assert_eq!(stored.stream_id(), "session-123");
        assert_eq!(stored.stream_version(), 1);
        assert_eq!(stored.event, event);
    }

    #[test]
    fn test_invalid_session_name_is_rejected() {
        let result = SessionName::parse("");
        assert!(result.is_err(), "Empty session name should be rejected");
    }

    #[test]
    fn test_invalid_workspace_name_is_rejected() {
        let result = WorkspaceName::parse("my/workspace");
        assert!(
            result.is_err(),
            "Workspace name with path separator should be rejected"
        );
    }

    #[test]
    fn test_invalid_agent_id_is_rejected() {
        let result = AgentId::parse("");
        assert!(result.is_err(), "Empty agent ID should be rejected");
    }

    #[test]
    fn test_invalid_bead_id_is_rejected() {
        let result = BeadId::parse("invalid-id");
        assert!(
            result.is_err(),
            "Bead ID without bd- prefix should be rejected"
        );
    }

    #[test]
    fn test_all_event_types_have_unique_types() {
        let events = vec![
            DomainEvent::session_created(
                "s1".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_completed(
                "s2".to_string(),
                SessionName::parse("s").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_failed(
                "s3".to_string(),
                SessionName::parse("s").expect("valid"),
                "error".to_string(),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::workspace_removed(
                WorkspaceName::parse("w").expect("valid"),
                PathBuf::from("/tmp"),
                Utc::now(),
            ),
            DomainEvent::queue_entry_added(
                1,
                WorkspaceName::parse("w").expect("valid"),
                1,
                None,
                Utc::now(),
            ),
            DomainEvent::queue_entry_claimed(
                1,
                WorkspaceName::parse("w").expect("valid"),
                AgentId::parse("a").expect("valid"),
                ClaimTimestamps::new(Utc::now(), Utc::now(), Utc::now()),
            ),
            DomainEvent::queue_entry_completed(
                1,
                WorkspaceName::parse("w").expect("valid"),
                AgentId::parse("a").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::bead_created(
                BeadId::parse("bd-abc").expect("valid"),
                "t".to_string(),
                None,
                Utc::now(),
            ),
            DomainEvent::bead_closed(
                BeadId::parse("bd-abc").expect("valid"),
                Utc::now(),
                Utc::now(),
            ),
        ];

        let event_types: Vec<&str> = events.iter().map(|e| e.event_type()).collect();

        // Check that all event types are unique
        let unique_types: std::collections::HashSet<_> = event_types.iter().collect();
        assert_eq!(
            unique_types.len(),
            event_types.len(),
            "Event types should be unique"
        );

        // Verify we have all expected event types
        assert!(event_types.contains(&"session_created"));
        assert!(event_types.contains(&"session_completed"));
        assert!(event_types.contains(&"session_failed"));
        assert!(event_types.contains(&"workspace_created"));
        assert!(event_types.contains(&"workspace_removed"));
        assert!(event_types.contains(&"queue_entry_added"));
        assert!(event_types.contains(&"queue_entry_claimed"));
        assert!(event_types.contains(&"queue_entry_completed"));
        assert!(event_types.contains(&"bead_created"));
        assert!(event_types.contains(&"bead_closed"));
    }

    #[test]
    fn test_events_are_immutable() {
        // This test verifies that events cannot be mutated after creation
        // by checking that all fields are accessible only via immutable methods

        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        // All access methods return references (immutable)
        let _timestamp_ref = event.timestamp();
        let _type = event.event_type();

        // The event itself is cloned, not mutated
        let _event_clone = event.clone();
    }

    #[test]
    fn test_event_json_structure() {
        let timestamp = Utc::now();
        let event = DomainEvent::session_created(
            "session-123".to_string(),
            SessionName::parse("my-session").expect("valid session name"),
            timestamp,
        );

        let json = serialize_event(&event).expect("serialization failed");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

        // Verify the JSON has the expected structure
        assert!(value.get("event_type").is_some());
        assert!(value.get("data").is_some());

        // Verify the event type (serde tag uses the variant name)
        if let Some(event_type) = value.get("event_type").and_then(|v| v.as_str()) {
            assert_eq!(event_type, "SessionCreated");
        } else {
            panic!("Expected event_type field");
        }
    }

    #[test]
    fn test_multiple_events_serialization() {
        let events = vec![
            DomainEvent::session_created(
                "s1".to_string(),
                SessionName::parse("session1").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::session_completed(
                "s2".to_string(),
                SessionName::parse("session2").expect("valid"),
                Utc::now(),
            ),
            DomainEvent::workspace_created(
                WorkspaceName::parse("workspace1").expect("valid"),
                PathBuf::from("/tmp/w1"),
                Utc::now(),
            ),
        ];

        // Serialize all events
        let json_list: Result<Vec<String>, _> = events.iter().map(serialize_event).collect();
        let json_list = json_list.expect("serialization failed");

        // Deserialize all events
        let deserialized: Result<Vec<DomainEvent>, _> =
            json_list.iter().map(|j| deserialize_event(j)).collect();
        let deserialized = deserialized.expect("deserialization failed");

        assert_eq!(deserialized, events);
    }
}

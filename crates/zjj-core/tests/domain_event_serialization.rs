//! Domain event serialization tests
//!
//! Comprehensive tests for all domain event types to ensure they
//! serialize and deserialize correctly.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::Utc;
use std::path::PathBuf;
use zjj_core::domain::events::{
    serialize_event, serialize_event_bytes, ClaimTimestamps, DomainEvent, EventMetadata, StoredEvent,
};
use zjj_core::domain::identifiers::{AgentId, BeadId, SessionName, WorkspaceName};

// ============================================================================
// SESSION EVENTS
// ============================================================================

#[test]
fn test_session_created_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_created(
        "session-123".to_string(),
        SessionName::parse("my-session").expect("valid name"),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "session_created");
}

#[test]
fn test_session_completed_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_completed(
        "session-456".to_string(),
        SessionName::parse("completed-session").expect("valid name"),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "session_completed");
}

#[test]
fn test_session_failed_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_failed(
        "session-789".to_string(),
        SessionName::parse("failed-session").expect("valid name"),
        "Out of memory error".to_string(),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "session_failed");

    // Verify failure reason is preserved
    if let DomainEvent::SessionFailed(e) = &deserialized {
        assert_eq!(e.reason, "Out of memory error");
    } else {
        panic!("Expected SessionFailed event");
    }
}

// ============================================================================
// WORKSPACE EVENTS
// ============================================================================

#[test]
fn test_workspace_created_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::workspace_created(
        WorkspaceName::parse("my-workspace").expect("valid name"),
        PathBuf::from("/tmp/workspace"),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "workspace_created");
}

#[test]
fn test_workspace_removed_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::workspace_removed(
        WorkspaceName::parse("removed-workspace").expect("valid name"),
        PathBuf::from("/tmp/workspace-to-remove"),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "workspace_removed");
}

// ============================================================================
// QUEUE EVENTS
// ============================================================================

#[test]
fn test_queue_entry_added_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::queue_entry_added(
        42,
        WorkspaceName::parse("queued-workspace").expect("valid name"),
        1,
        Some(BeadId::parse("bd-abc123").expect("valid id")),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "queue_entry_added");

    // Verify entry details
    if let DomainEvent::QueueEntryAdded(e) = &deserialized {
        assert_eq!(e.entry_id, 42);
        assert_eq!(e.priority, 1);
        assert!(e.bead_id.is_some());
    } else {
        panic!("Expected QueueEntryAdded event");
    }
}

#[test]
fn test_queue_entry_added_without_bead_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::queue_entry_added(
        43,
        WorkspaceName::parse("workspace-no-bead").expect("valid name"),
        5,
        None,
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);

    // Verify bead_id is None
    if let DomainEvent::QueueEntryAdded(e) = &deserialized {
        assert!(e.bead_id.is_none());
    } else {
        panic!("Expected QueueEntryAdded event");
    }
}

#[test]
fn test_queue_entry_claimed_event_serialization() {
    use zjj_core::domain::events::ClaimTimestamps;

    let timestamp = Utc::now();
    let claimed_at = timestamp;
    let expires_at = timestamp + chrono::Duration::seconds(300);

    let event = DomainEvent::queue_entry_claimed(
        44,
        WorkspaceName::parse("claimed-workspace").expect("valid name"),
        AgentId::parse("agent-1").expect("valid agent"),
        ClaimTimestamps::new(claimed_at, expires_at, timestamp),
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "queue_entry_claimed");

    // Verify claim details
    if let DomainEvent::QueueEntryClaimed(e) = &deserialized {
        assert_eq!(e.entry_id, 44);
        assert_eq!(e.agent.as_str(), "agent-1");
    } else {
        panic!("Expected QueueEntryClaimed event");
    }
}

#[test]
fn test_queue_entry_completed_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::queue_entry_completed(
        45,
        WorkspaceName::parse("completed-workspace").expect("valid name"),
        AgentId::parse("agent-2").expect("valid agent"),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "queue_entry_completed");
}

// ============================================================================
// BEAD EVENTS
// ============================================================================

#[test]
fn test_bead_created_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::bead_created(
        BeadId::parse("bd-abc123").expect("valid id"),
        "Fix the critical bug".to_string(),
        Some("High priority issue affecting production".to_string()),
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "bead_created");

    // Verify bead details
    if let DomainEvent::BeadCreated(e) = &deserialized {
        assert_eq!(e.title, "Fix the critical bug");
        assert_eq!(e.description, Some("High priority issue affecting production".to_string()));
    } else {
        panic!("Expected BeadCreated event");
    }
}

#[test]
fn test_bead_created_without_description_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::bead_created(
        BeadId::parse("bd-abc789").expect("valid id"),
        "Simple task".to_string(),
        None,
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);

    // Verify description is None
    if let DomainEvent::BeadCreated(e) = &deserialized {
        assert_eq!(e.title, "Simple task");
        assert!(e.description.is_none());
    } else {
        panic!("Expected BeadCreated event");
    }
}

#[test]
fn test_bead_closed_event_serialization() {
    let timestamp = Utc::now();
    let closed_at = timestamp;

    let event = DomainEvent::bead_closed(
        BeadId::parse("bd-abc456").expect("valid id"),
        closed_at,
        timestamp,
    );

    // Test JSON serialization
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    assert_eq!(event, deserialized);
    assert_eq!(event.event_type(), "bead_closed");
}

// ============================================================================
// BYTES SERIALIZATION
// ============================================================================

#[test]
fn test_event_bytes_serialization() {
    let timestamp = Utc::now();
    let events = vec![
        DomainEvent::session_created(
            "s1".to_string(),
            SessionName::parse("session1").expect("valid"),
            timestamp,
        ),
        DomainEvent::workspace_created(
            WorkspaceName::parse("workspace1").expect("valid"),
            PathBuf::from("/tmp/w1"),
            timestamp,
        ),
        DomainEvent::bead_created(
            BeadId::parse("bd-abc1").expect("valid"),
            "Task 1".to_string(),
            None,
            timestamp,
        ),
    ];

    for event in events {
        // Test bytes serialization
        let bytes = serialize_event_bytes(&event).expect("serialization failed");
        let deserialized = serde_json::from_slice::<DomainEvent>(&bytes)
            .expect("deserialization failed");

        assert_eq!(event, deserialized);
    }
}

// ============================================================================
// STORED EVENT TESTS
// ============================================================================

#[test]
fn test_stored_event_serialization() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_created(
        "session-stored".to_string(),
        SessionName::parse("stored-session").expect("valid name"),
        timestamp,
    );

    let metadata = EventMetadata {
        event_number: 42,
        stream_id: "session-stored".to_string(),
        stream_version: 1,
        stored_at: timestamp,
    };

    let stored = StoredEvent::new(event.clone(), metadata);

    // Test serialization of stored event
    let json = serde_json::to_string(&stored).expect("serialization failed");
    let deserialized: StoredEvent =
        serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(stored.event_number(), deserialized.event_number());
    assert_eq!(stored.stream_id(), deserialized.stream_id());
    assert_eq!(stored.stream_version(), deserialized.stream_version());
    assert_eq!(stored.event, deserialized.event);
}

#[test]
fn test_stored_event_metadata() {
    let timestamp = Utc::now();
    let event = DomainEvent::workspace_created(
        WorkspaceName::parse("test").expect("valid name"),
        PathBuf::from("/tmp/test"),
        timestamp,
    );

    let metadata = EventMetadata {
        event_number: 100,
        stream_id: "workspace-test".to_string(),
        stream_version: 5,
        stored_at: timestamp,
    };

    let stored = StoredEvent::new(event, metadata);

    assert_eq!(stored.event_number(), 100);
    assert_eq!(stored.stream_id(), "workspace-test");
    assert_eq!(stored.stream_version(), 5);
}

// ============================================================================
// CROSS-EVENT TYPE TESTS
// ============================================================================

#[test]
fn test_all_event_types_have_unique_types() {
    let timestamp = Utc::now();
    let events = vec![
        DomainEvent::session_created(
            "s1".to_string(),
            SessionName::parse("s").expect("valid"),
            timestamp,
        ),
        DomainEvent::session_completed(
            "s2".to_string(),
            SessionName::parse("s").expect("valid"),
            timestamp,
        ),
        DomainEvent::session_failed(
            "s3".to_string(),
            SessionName::parse("s").expect("valid"),
            "error".to_string(),
            timestamp,
        ),
        DomainEvent::workspace_created(
            WorkspaceName::parse("w").expect("valid"),
            PathBuf::from("/tmp"),
            timestamp,
        ),
        DomainEvent::workspace_removed(
            WorkspaceName::parse("w").expect("valid"),
            PathBuf::from("/tmp"),
            timestamp,
        ),
        DomainEvent::queue_entry_added(
            1,
            WorkspaceName::parse("w").expect("valid"),
            1,
            None,
            timestamp,
        ),
        DomainEvent::queue_entry_claimed(
            1,
            WorkspaceName::parse("w").expect("valid"),
            AgentId::parse("a").expect("valid"),
            ClaimTimestamps::new(timestamp, timestamp, timestamp),
        ),
        DomainEvent::queue_entry_completed(
            1,
            WorkspaceName::parse("w").expect("valid"),
            AgentId::parse("a").expect("valid"),
            timestamp,
        ),
        DomainEvent::bead_created(
            BeadId::parse("bd-abc").expect("valid"),
            "t".to_string(),
            None,
            timestamp,
        ),
        DomainEvent::bead_closed(
            BeadId::parse("bd-abc").expect("valid"),
            timestamp,
            timestamp,
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
    assert_eq!(unique_types.len(), 10, "Should have 10 unique event types");
}

#[test]
fn test_all_events_serialize_and_deserialize() {
    let timestamp = Utc::now();
    let events = vec![
        DomainEvent::session_created(
            "s1".to_string(),
            SessionName::parse("session1").expect("valid"),
            timestamp,
        ),
        DomainEvent::session_completed(
            "s2".to_string(),
            SessionName::parse("session2").expect("valid"),
            timestamp,
        ),
        DomainEvent::session_failed(
            "s3".to_string(),
            SessionName::parse("session3").expect("valid"),
            "failure".to_string(),
            timestamp,
        ),
        DomainEvent::workspace_created(
            WorkspaceName::parse("workspace1").expect("valid"),
            PathBuf::from("/tmp/w1"),
            timestamp,
        ),
        DomainEvent::workspace_removed(
            WorkspaceName::parse("workspace2").expect("valid"),
            PathBuf::from("/tmp/w2"),
            timestamp,
        ),
        DomainEvent::queue_entry_added(
            1,
            WorkspaceName::parse("workspace1").expect("valid"),
            1,
            Some(BeadId::parse("bd-abc1").expect("valid")),
            timestamp,
        ),
        DomainEvent::queue_entry_claimed(
            1,
            WorkspaceName::parse("workspace1").expect("valid"),
            AgentId::parse("agent-1").expect("valid"),
            ClaimTimestamps::new(timestamp, timestamp + chrono::Duration::seconds(300), timestamp),
        ),
        DomainEvent::queue_entry_completed(
            1,
            WorkspaceName::parse("workspace1").expect("valid"),
            AgentId::parse("agent-1").expect("valid"),
            timestamp,
        ),
        DomainEvent::bead_created(
            BeadId::parse("bd-abc1").expect("valid"),
            "Task 1".to_string(),
            Some("Description".to_string()),
            timestamp,
        ),
        DomainEvent::bead_closed(
            BeadId::parse("bd-abc1").expect("valid"),
            timestamp,
            timestamp,
        ),
    ];

    for event in events {
        // Test JSON serialization
        let json = serialize_event(&event).expect("serialization failed");
        let deserialized = serde_json::from_str::<DomainEvent>(&json)
            .expect("deserialization failed");

        assert_eq!(event, deserialized);

        // Test bytes serialization
        let bytes = serialize_event_bytes(&event).expect("serialization failed");
        let deserialized_bytes = serde_json::from_slice::<DomainEvent>(&bytes)
            .expect("deserialization failed");

        assert_eq!(event, deserialized_bytes);
    }
}

// ============================================================================
// JSON STRUCTURE TESTS
// ============================================================================

#[test]
fn test_event_json_has_correct_structure() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_created(
        "session-123".to_string(),
        SessionName::parse("my-session").expect("valid name"),
        timestamp,
    );

    let json = serialize_event(&event).expect("serialization failed");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    // Verify the JSON has the expected structure
    assert!(value.get("event_type").is_some());
    assert!(value.get("data").is_some());

    // Verify the event type
    if let Some(event_type) = value.get("event_type").and_then(|v| v.as_str()) {
        // The JSON serialization uses the Rust enum name
        assert_eq!(event_type, "SessionCreated");
    } else {
        panic!("Expected event_type field");
    }
}

#[test]
fn test_events_are_immutable() {
    let timestamp = Utc::now();
    let event = DomainEvent::session_created(
        "session-immutable".to_string(),
        SessionName::parse("immutable-session").expect("valid name"),
        timestamp,
    );

    // Clone to verify immutability (no mutation methods exist)
    let _event_clone = event.clone();

    // All access methods return references (immutable)
    let _timestamp_ref = event.timestamp();
    let _type = event.event_type();

    // Cannot mutate event - this won't compile
    // event.timestamp_mut() = ...; // This does not exist
}

#[test]
fn test_event_timestamps_preserved() {
    let timestamp = Utc::now();
    let event = DomainEvent::bead_closed(
        BeadId::parse("bd-abc").expect("valid id"),
        timestamp - chrono::Duration::seconds(10),
        timestamp,
    );

    // Serialize and deserialize
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    if let DomainEvent::BeadClosed(e) = &deserialized {
        assert_eq!(e.closed_at, timestamp - chrono::Duration::seconds(10));
        assert_eq!(e.timestamp, timestamp);
    } else {
        panic!("Expected BeadClosed event");
    }
}

#[test]
fn test_session_name_preserved_in_events() {
    let timestamp = Utc::now();
    let session_name = SessionName::parse("test-session-123").expect("valid name");

    let event = DomainEvent::session_created(
        "session-unicode".to_string(),
        session_name.clone(),
        timestamp,
    );

    // Serialize and deserialize
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    if let DomainEvent::SessionCreated(e) = &deserialized {
        assert_eq!(e.session_name, session_name);
    } else {
        panic!("Expected SessionCreated event");
    }
}

#[test]
fn test_workspace_name_preserved_in_events() {
    let timestamp = Utc::now();
    let workspace_name = WorkspaceName::parse("test-workspace-123").expect("valid name");

    let event = DomainEvent::workspace_created(
        workspace_name.clone(),
        PathBuf::from("/tmp/test-workspace"),
        timestamp,
    );

    // Serialize and deserialize
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    if let DomainEvent::WorkspaceCreated(e) = &deserialized {
        assert_eq!(e.workspace_name, workspace_name);
    } else {
        panic!("Expected WorkspaceCreated event");
    }
}

#[test]
fn test_agent_id_preserved_in_events() {
    use zjj_core::domain::events::ClaimTimestamps;

    let timestamp = Utc::now();
    let agent_id = AgentId::parse("special-agent:123").expect("valid agent");

    let event = DomainEvent::queue_entry_claimed(
        1,
        WorkspaceName::parse("test").expect("valid name"),
        agent_id.clone(),
        ClaimTimestamps::new(timestamp, timestamp, timestamp),
    );

    // Serialize and deserialize
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    if let DomainEvent::QueueEntryClaimed(e) = &deserialized {
        assert_eq!(e.agent, agent_id);
    } else {
        panic!("Expected QueueEntryClaimed event");
    }
}

#[test]
fn test_bead_id_preserved_in_events() {
    let timestamp = Utc::now();
    let bead_id = BeadId::parse("bd-abc123def456").expect("valid id");

    let event = DomainEvent::bead_created(
        bead_id.clone(),
        "Test".to_string(),
        None,
        timestamp,
    );

    // Serialize and deserialize
    let json = serialize_event(&event).expect("serialization failed");
    let deserialized = serde_json::from_str::<DomainEvent>(&json)
        .expect("deserialization failed");

    if let DomainEvent::BeadCreated(e) = &deserialized {
        assert_eq!(e.bead_id, bead_id);
    } else {
        panic!("Expected BeadCreated event");
    }
}

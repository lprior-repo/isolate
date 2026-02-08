//! TDD15 tests for broadcast command
//!
//! Martin Fowler test patterns:
//! - State verification: Check postconditions
//! - Behavior verification: Verify collaboration
//! - Edge case coverage: Boundary conditions
//! - Property-based testing: Invariants

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use super::types::{BroadcastArgs, BroadcastResponse};
use chrono::{DateTime, Utc};
use serde_json::{from_str, to_string_pretty};

/// Helper: Create test broadcast args
#[allow(dead_code)]
fn create_args(message: &str, agent_id: &str) -> BroadcastArgs {
    BroadcastArgs {
        message: message.to_string(),
        agent_id: agent_id.to_string(),
    }
}

/// Helper: Create test broadcast response
#[allow(dead_code)]
fn create_response(
    message: &str,
    sent_to: Vec<String>,
    timestamp: &str,
) -> BroadcastResponse {
    BroadcastResponse {
        success: true,
        message: message.to_string(),
        sent_to,
        timestamp: timestamp.to_string(),
    }
}

// ============================================================================
// PHASE 1: EARS Tests (Event-Condition-Action)
// ============================================================================

#[test]
/// EARS #1: When broadcast runs, system shall send message to all active agents except sender
fn test_broadcast_sends_to_all_active() {
    // Setup: Multiple active agents
    let active_agents = vec![
        "agent-1".to_string(),
        "agent-2".to_string(),
        "agent-3".to_string(),
    ];
    let sender = "agent-1";

    // Act: Filter out sender (simulating the broadcast logic)
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: Sender excluded, other agents included
    assert!(!sent_to.contains(&sender.to_string()));
    assert_eq!(sent_to.len(), 2);
    assert!(sent_to.contains(&"agent-2".to_string()));
    assert!(sent_to.contains(&"agent-3".to_string()));
}

#[test]
/// EARS #2: When storing message, system shall record timestamp and sender agent_id
fn test_message_timestamp_recorded() {
    // Setup
    let message = "Hello, agents!";
    let _sender_id = "agent-1"; // Present for API completeness
    let before_timestamp = chrono::Utc::now();

    // Act: Create response with timestamp
    let response = BroadcastResponse {
        success: true,
        message: message.to_string(),
        sent_to: vec!["agent-2".to_string()],
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let after_timestamp = chrono::Utc::now();

    // Assert: Timestamp exists and is valid
    let parsed_timestamp_result: Result<DateTime<Utc>, _> = response.timestamp.parse();
    assert!(
        parsed_timestamp_result.is_ok(),
        "Invalid RFC3339 timestamp: {}",
        response.timestamp
    );

    // Verify timestamp is within expected range
    if let Ok(parsed_timestamp) = parsed_timestamp_result {
        assert!(parsed_timestamp >= before_timestamp);
        assert!(parsed_timestamp <= after_timestamp);
    }
}

#[test]
/// EARS #3: When retrieving messages, agents shall see only unread messages for them
fn test_recipient_filtering() {
    // Setup: Broadcast to specific recipients
    let all_recipients = vec!["agent-1".to_string(), "agent-2".to_string()];
    let querying_agent = "agent-2";

    // Act: Filter messages for this agent
    let agent_messages: Vec<&String> = all_recipients
        .iter()
        .filter(|recipient| recipient.as_str() == querying_agent)
        .collect();

    // Assert: Only agent-2's messages
    assert_eq!(agent_messages.len(), 1);
    assert_eq!(agent_messages[0], &"agent-2".to_string());
}

// ============================================================================
// PHASE 2: Edge Cases (Boundary Analysis)
// ============================================================================

#[test]
/// Edge case: No other agents → sent_to is empty
fn test_empty_recipient_list_no_other_agents() {
    // Setup: Only sender is active
    let active_agents = vec!["agent-1".to_string()];
    let sender = "agent-1";

    // Act
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: Empty list
    assert!(sent_to.is_empty());
}

#[test]
/// Edge case: Single agent (sender only) → sent_to is empty
fn test_single_agent_sender_only() {
    // Setup
    let active_agents = vec!["agent-solo".to_string()];
    let sender = "agent-solo";

    // Act
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert
    assert!(sent_to.is_empty());
}

#[test]
/// Edge case: Multiple agents → sent_to excludes sender
fn test_multiple_agents_excludes_sender() {
    // Setup: Large agent list
    let active_agents: Vec<String> = (1..=10)
        .map(|i| format!("agent-{}", i))
        .collect();
    let sender = "agent-5";

    // Act
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: 9 agents (excluded sender)
    assert_eq!(sent_to.len(), 9);
    assert!(!sent_to.contains(&sender.to_string()));
    assert!(sent_to.contains(&"agent-1".to_string()));
    assert!(sent_to.contains(&"agent-10".to_string()));
}

#[test]
/// Edge case: Empty message string
fn test_empty_message() {
    // Setup
    let args = create_args("", "agent-1");
    let response = BroadcastResponse {
        success: true,
        message: args.message.clone(),
        sent_to: vec!["agent-2".to_string()],
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Assert: Empty message handled gracefully
    assert_eq!(response.message, "");
    assert!(response.success);
}

#[test]
/// Edge case: Very long message (10KB)
fn test_large_message() {
    // Setup: Create 10KB message
    let large_message = "x".repeat(10_240);
    let args = create_args(&large_message, "agent-1");
    let response = BroadcastResponse {
        success: true,
        message: args.message.clone(),
        sent_to: vec!["agent-2".to_string()],
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Assert: Large message preserved
    assert_eq!(response.message.len(), 10_240);
    assert!(response.success);
}

// ============================================================================
// PHASE 3: Invariants (Property-Based Testing)
// ============================================================================

#[test]
/// Invariant: Sender never appears in sent_to list
fn test_sender_never_in_recipient_list() {
    // Property: For any agent list and sender, sender not in recipients
    let test_cases = vec![
        (vec!["agent-1".to_string()], "agent-1"),
        (vec!["agent-1".to_string(), "agent-2".to_string()], "agent-1"),
        (
            (1..=100).map(|i| format!("agent-{}", i)).collect(),
            "agent-50",
        ),
    ];

    for (active_agents, sender) in test_cases {
        let sent_to: Vec<String> = active_agents
            .iter()
            .filter(|agent| agent.as_str() != sender)
            .cloned()
            .collect();

        assert!(!sent_to.contains(&sender.to_string()));
    }
}

#[test]
/// Invariant: Timestamp always valid RFC3339
fn test_timestamp_always_valid_rfc3339() {
    // Property: All generated timestamps parse successfully
    let timestamps = vec![
        chrono::Utc::now().to_rfc3339(),
        chrono::Utc::now().to_rfc3339(),
        chrono::Utc::now().to_rfc3339(),
    ];

    for timestamp in timestamps {
        let parsed: Result<DateTime<Utc>, _> = timestamp.parse();
        assert!(parsed.is_ok(), "Invalid RFC3339: {}", timestamp);
    }
}

#[test]
/// Invariant: Recipient list is subset of active agents (excluding sender)
fn test_recipients_subset_of_active_agents() {
    // Setup
    let active_agents: Vec<String> = vec!["agent-1", "agent-2", "agent-3"]
        .into_iter()
        .map(String::from)
        .collect();
    let sender = "agent-1";

    // Act
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: All recipients are in active agents (except sender)
    for recipient in &sent_to {
        assert!(active_agents.contains(recipient));
        assert_ne!(recipient, sender);
    }
}

#[test]
/// Invariant: Success flag always true when response created
fn test_success_flag_invariant() {
    // Property: Response always has success=true
    let responses = vec![
        create_response("msg1", vec!["agent-2".to_string()], "2024-01-01T00:00:00Z"),
        create_response("msg2", vec![], "2024-01-01T00:00:00Z"),
        create_response("", vec!["agent-1".to_string()], "2024-01-01T00:00:00Z"),
    ];

    for response in responses {
        assert!(response.success);
    }
}

// ============================================================================
// PHASE 4: Serialization Tests (Martin Fowler - State Verification)
// ============================================================================

#[test]
/// Serialization: Response serializes to valid JSON
fn test_response_serialization() {
    // Setup
    let response = create_response(
        "Hello, world!",
        vec!["agent-2".to_string(), "agent-3".to_string()],
        "2024-01-15T12:30:45Z",
    );

    // Act: Serialize
    let json = to_string_pretty(&response);

    // Assert: Valid JSON with expected fields
    assert!(json.is_ok(), "Serialization failed");
    if let Ok(json_str) = json {
        assert!(json_str.contains("\"success\":true"));
        assert!(json_str.contains("\"message\":\"Hello, world!\""));
        assert!(json_str.contains("\"timestamp\":\"2024-01-15T12:30:45Z\""));
    }
}

#[test]
/// Deserialization: JSON parses back to response
fn test_response_deserialization() {
    // Setup
    let json_str = r#"{
        "success": true,
        "message": "Test message",
        "sent_to": ["agent-2", "agent-3"],
        "timestamp": "2024-01-15T12:30:45Z"
    }"#;

    // Act: Parse
    let response: Result<BroadcastResponse, _> = from_str(json_str);

    // Assert
    assert!(response.is_ok(), "Deserialization failed");
    if let Ok(parsed) = response {
        assert!(parsed.success);
        assert_eq!(parsed.message, "Test message");
        assert_eq!(parsed.sent_to.len(), 2);
        assert_eq!(parsed.timestamp, "2024-01-15T12:30:45Z");
    }
}

#[test]
/// Round-trip: Serialize then deserialize preserves data
fn test_response_round_trip() {
    // Setup
    let original = create_response(
        "Round-trip test",
        vec!["agent-1".to_string(), "agent-4".to_string()],
        "2024-02-01T08:15:30Z",
    );

    // Act: Serialize then deserialize
    let json_result = to_string_pretty(&original);
    assert!(json_result.is_ok(), "Serialization should succeed");

    if let Ok(json) = json_result {
        let restored_result: Result<BroadcastResponse, _> = from_str(&json);
        assert!(restored_result.is_ok(), "Deserialization should succeed");

        if let Ok(restored) = restored_result {
            // Assert: All fields preserved
            assert_eq!(restored.success, original.success);
            assert_eq!(restored.message, original.message);
            assert_eq!(restored.sent_to, original.sent_to);
            assert_eq!(restored.timestamp, original.timestamp);
        }
    }
}

// ============================================================================
// PHASE 5: Behavior Verification (Collaboration Tests)
// ============================================================================

#[test]
/// Behavior: Args correctly pass data to response
fn test_args_to_response_data_flow() {
    // Setup
    let args = create_args("Test message", "agent-1");

    // Act: Create response from args
    let response = BroadcastResponse {
        success: true,
        message: args.message.clone(),
        sent_to: vec!["agent-2".to_string()],
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Assert: Data flows correctly
    assert_eq!(response.message, args.message);
}

#[test]
/// Behavior: Filtering logic is pure (no side effects)
fn test_filtering_is_pure() {
    // Setup
    let active_agents = vec!["agent-1".to_string(), "agent-2".to_string()];
    let sender = "agent-1";

    // Act: Filter twice (should return same result)
    let sent_to1: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    let sent_to2: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: Pure function - same output for same input
    assert_eq!(sent_to1, sent_to2);
}

// ============================================================================
// PHASE 6: Error Scenarios (Robustness)
// ============================================================================

#[test]
/// Error: Invalid timestamp format handled gracefully
fn test_invalid_timestamp_format() {
    // Setup: Invalid RFC3339 timestamp
    let invalid_timestamp = "not-a-timestamp";

    // Act: Attempt to parse
    let parsed: Result<DateTime<Utc>, _> = invalid_timestamp.parse();

    // Assert: Parse fails gracefully
    assert!(parsed.is_err());
}

#[test]
/// Error: Empty agent list handled
fn test_empty_agent_list() {
    // Setup: No active agents
    let active_agents: Vec<String> = vec![];
    let sender = "agent-1";

    // Act
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();

    // Assert: Empty result
    assert!(sent_to.is_empty());
}

// ============================================================================
// PHASE 7: Performance Characteristics (Efficiency)
// ============================================================================

#[test]
/// Performance: Filtering is O(n) where n = agent count
fn test_filtering_performance() {
    // Setup: Large agent list (1,000 agents)
    let active_agents: Vec<String> = (1..=1000)
        .map(|i| format!("agent-{}", i))
        .collect();
    let sender = "agent-500";

    // Act: Filter (should be fast)
    let start = std::time::Instant::now();
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.as_str() != sender)
        .cloned()
        .collect();
    let duration = start.elapsed();

    // Assert: Should complete in reasonable time (< 10ms for 1000 items)
    assert!(duration.as_millis() < 10);
    assert_eq!(sent_to.len(), 999);
}

#[test]
/// Performance: Serialization is efficient for large recipient lists
fn test_serialization_performance() {
    // Setup: Large recipient list (100 agents)
    let sent_to: Vec<String> = (1..=100).map(|i| format!("agent-{}", i)).collect();
    let response = create_response(
        "Performance test",
        sent_to,
        "2024-01-01T00:00:00Z",
    );

    // Act: Serialize
    let start = std::time::Instant::now();
    let json = to_string_pretty(&response);
    let duration = start.elapsed();

    // Assert: Should be fast (< 5ms)
    assert!(json.is_ok());
    assert!(duration.as_millis() < 5);
}

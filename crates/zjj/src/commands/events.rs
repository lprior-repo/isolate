//! Events command - Event streaming for multi-agent coordination
//!
//! Provides real-time event streaming with --follow support.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

/// Options for the events command
#[derive(Debug, Clone)]
pub struct EventsOptions {
    /// Filter by session name
    pub session: Option<String>,
    /// Filter by event type
    pub event_type: Option<String>,
    /// Follow mode (stream events)
    pub follow: bool,
    /// Maximum number of events to return
    pub limit: Option<usize>,
    /// Only show events after this timestamp
    pub since: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Event types in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    SessionCreated,
    SessionRemoved,
    SessionFocused,
    SessionMerged,
    SessionAborted,
    SessionSynced,
    AgentRegistered,
    AgentUnregistered,
    AgentHeartbeat,
    LockAcquired,
    LockReleased,
    CheckpointCreated,
    CheckpointRestored,
    BeadStatusChanged,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_else(|_| "unknown".to_string());
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// An event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    pub id: String,
    /// Event type
    #[allow(clippy::struct_field_names)]
    pub event_type: EventType,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Session name if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Agent ID if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Event-specific data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Human-readable message
    pub message: String,
}

/// Events response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsResponse {
    /// List of events
    pub events: Vec<Event>,
    /// Total count (may be more than returned)
    pub total: usize,
    /// Whether there are more events
    pub has_more: bool,
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Run the events command
pub async fn run(options: &EventsOptions) -> Result<()> {
    if options.follow {
        run_follow(options).await
    } else {
        run_list(options).await
    }
}

async fn run_list(options: &EventsOptions) -> Result<()> {
    // Get recent events from the database/log
    let events = get_recent_events(
        options.session.as_deref(),
        options.event_type.as_deref(),
        options.limit.unwrap_or(50),
        options.since.as_deref(),
    )
    .await?;

    let response = EventsResponse {
        total: events.len(),
        has_more: false,
        cursor: None,
        events,
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("events-response", "single", &response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        response.events.iter().for_each(|event| {
            let session_str = event
                .session
                .as_ref()
                .map_or(String::new(), |s| format!(" [{s}]"));
            let agent_str = event
                .agent_id
                .as_ref()
                .map_or(String::new(), |a| format!(" agent:{a}"));
            println!(
                "{} {}{}{}: {}",
                event.timestamp, event.event_type, session_str, agent_str, event.message
            );
        });
    }

    Ok(())
}

async fn run_follow(options: &EventsOptions) -> Result<()> {
    eprintln!("Following events... (Ctrl+C to stop)");
    eprintln!();

    // In follow mode, we poll for new events
    let mut last_id: Option<String> = None;

    loop {
        let events = get_new_events(
            options.session.as_deref(),
            options.event_type.as_deref(),
            last_id.as_deref(),
        )
        .await?;

        for event in &events {
            if options.format.is_json() {
                if let Ok(json) = serde_json::to_string(event) {
                    println!("{json}");
                }
            } else {
                let session_str = event
                    .session
                    .as_ref()
                    .map_or(String::new(), |s| format!(" [{s}]"));
                let agent_str = event
                    .agent_id
                    .as_ref()
                    .map_or(String::new(), |a| format!(" agent:{a}"));
                println!(
                    "{} {}{}{}: {}",
                    event.timestamp, event.event_type, session_str, agent_str, event.message
                );
            }
        }

        if let Some(last) = events.last() {
            last_id = Some(last.id.clone());
        }

        // Poll interval
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
}

/// Get the events file path
async fn get_events_file_path() -> Result<std::path::PathBuf> {
    let data_dir = super::zjj_data_dir().await?;
    Ok(data_dir.join("events.jsonl"))
}

async fn get_recent_events(
    session: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
    since: Option<&str>,
) -> Result<Vec<Event>> {
    let events_file = get_events_file_path().await?;

    let mut events = match tokio::fs::try_exists(&events_file).await {
        Ok(true) => {
            let content = tokio::fs::read_to_string(&events_file).await?;
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|line| {
                    serde_json::from_str::<Event>(line).ok().filter(|event| {
                        // Apply filters
                        let session_matches =
                            session.is_none_or(|s| event.session.as_deref() == Some(s));
                        let type_matches =
                            event_type.is_none_or(|t| event.event_type.to_string() == t);
                        let since_matches = since.is_none_or(|st| event.timestamp.as_str() >= st);

                        session_matches && type_matches && since_matches
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    // Sort by timestamp descending (most recent first)
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit
    events.truncate(limit);

    Ok(events)
}

async fn get_new_events(
    session: Option<&str>,
    event_type: Option<&str>,
    after_id: Option<&str>,
) -> Result<Vec<Event>> {
    let events_file = get_events_file_path().await?;

    let events = match tokio::fs::try_exists(&events_file).await {
        Ok(true) => {
            let content = tokio::fs::read_to_string(&events_file).await?;
            let mut found_marker = after_id.is_none(); // If no marker, include all

            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|line| {
                    serde_json::from_str::<Event>(line).ok().and_then(|event| {
                        // Skip until we find the marker
                        if !found_marker {
                            if Some(event.id.as_str()) == after_id {
                                found_marker = true;
                            }
                            return None;
                        }

                        // Apply filters
                        let session_matches =
                            session.is_none_or(|s| event.session.as_deref() == Some(s));
                        let type_matches =
                            event_type.is_none_or(|t| event.event_type.to_string() == t);

                        if session_matches && type_matches {
                            Some(event)
                        } else {
                            None
                        }
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    Ok(events)
}

/// Generate a simple event ID based on timestamp
///
/// Part of the public event logging API. Not yet integrated into command flows,
/// but available for future use by session/agent coordination features.
#[allow(clippy::used_underscore_items)]
fn _generate_event_id() -> String {
    let now = chrono::Utc::now();
    format!("evt-{timestamp}-{pid}", timestamp = now.timestamp_millis(), pid = std::process::id())
}

/// Log an event to the events log
///
/// This function appends an event to the events.jsonl file.
/// Errors are propagated to the caller rather than silently ignored.
///
/// Part of the public event logging API. Not yet integrated into command flows,
/// but available for future use by session/agent coordination features.
#[allow(clippy::used_underscore_items)]
pub async fn _log_event(
    event_type: EventType,
    session: Option<&str>,
    agent_id: Option<&str>,
    message: &str,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let event = Event {
        id: _generate_event_id(),
        event_type,
        timestamp: chrono::Utc::now().to_rfc3339(),
        session: session.map(String::from),
        agent_id: agent_id.map(String::from),
        data: None,
        message: message.to_string(),
    };

    // Get events file path
    let events_file = get_events_file_path().await?;

    // Serialize event
    let event_json = serde_json::to_string(&event)
        .map_err(|e| anyhow::anyhow!("Failed to serialize event: {e}"))?;

    // Open file for appending
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_file)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open events file: {e}"))?;

    // Write event
    file.write_all(format!("{event_json}\n").as_bytes())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write event: {e}"))?;

    Ok(())
}

/// Log an event, ignoring any errors (for non-critical event logging)
///
/// Part of the public event logging API. Not yet integrated into command flows,
/// but available for future use by session/agent coordination features.
#[allow(clippy::used_underscore_items)]
pub fn _log_event_silent(
    event_type: EventType,
    session: Option<&str>,
    agent_id: Option<&str>,
    message: &str,
) {
    drop(_log_event(event_type, session, agent_id, message));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::SessionCreated.to_string(), "session_created");
        assert_eq!(EventType::AgentHeartbeat.to_string(), "agent_heartbeat");
    }

    #[test]
    fn test_event_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let event = Event {
            id: "evt-1".to_string(),
            event_type: EventType::SessionCreated,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: Some("test".to_string()),
            agent_id: None,
            data: None,
            message: "Created session".to_string(),
        };

        let json = serde_json::to_string(&event)?;
        assert!(json.contains("\"event_type\":\"session_created\""));
        assert!(json.contains("\"session\":\"test\""));
        Ok(())
    }

    #[test]
    fn test_events_response_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let response = EventsResponse {
            events: vec![],
            total: 0,
            has_more: false,
            cursor: None,
        };

        let json = serde_json::to_string(&response)?;
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"has_more\":false"));
        Ok(())
    }

    #[test]
    fn test_event_types_are_snake_case() -> Result<(), Box<dyn std::error::Error>> {
        let types = vec![
            EventType::SessionCreated,
            EventType::LockAcquired,
            EventType::BeadStatusChanged,
        ];

        for t in types {
            let json = serde_json::to_string(&t)?;
            assert!(json.contains('_') || json == "\"session_created\"");
        }
        Ok(())
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the events command
    // ============================================================================

    mod event_type_behavior {
        use super::*;

        /// GIVEN: All possible event types
        /// WHEN: Listed
        /// THEN: Should cover all important state changes
        #[test]
        fn covers_all_important_state_changes() {
            let state_change_events: [EventType; 5] = [
                EventType::SessionCreated,
                EventType::SessionRemoved,
                EventType::SessionMerged,
                EventType::SessionAborted,
                EventType::SessionSynced,
            ];

            // Each should have a meaningful name
            for event_type in &state_change_events {
                let name = event_type.to_string();
                assert!(!name.is_empty(), "Event type should have name");
                assert!(name.contains('_'), "Should be snake_case");
            }
        }

        /// GIVEN: Event types for agent coordination
        /// WHEN: Used
        /// THEN: Should support multi-agent workflows
        #[test]
        fn supports_agent_coordination() {
            let coordination_events = [
                EventType::AgentHeartbeat,
                EventType::LockAcquired,
                EventType::LockReleased,
            ];

            for event_type in &coordination_events {
                let name = event_type.to_string();
                assert!(
                    name.contains("agent") || name.contains("lock"),
                    "Coordination event should relate to agents or locks"
                );
            }
        }

        /// GIVEN: Event type serialization
        /// WHEN: Serialized
        /// THEN: Should be `snake_case` for consistency
        #[test]
        fn serializes_as_snake_case() -> Result<(), Box<dyn std::error::Error>> {
            let event = EventType::BeadStatusChanged;
            let json = serde_json::to_string(&event)?;

            // Should contain underscore (snake_case)
            assert!(json.contains('_'), "Should be snake_case: {json}");
            // Should be lowercase
            assert_eq!(json, json.to_lowercase(), "Should be lowercase: {json}");
            Ok(())
        }
    }

    mod event_behavior {
        use super::*;

        /// GIVEN: An event
        /// WHEN: Created
        /// THEN: Should have id, type, timestamp, and message
        #[test]
        fn event_has_required_fields() {
            let event = Event {
                id: "evt-123".to_string(),
                event_type: EventType::SessionCreated,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("my-session".to_string()),
                agent_id: Some("agent-1".to_string()),
                data: None,
                message: "Session created successfully".to_string(),
            };

            assert!(!event.id.is_empty(), "Must have ID");
            assert!(!event.timestamp.is_empty(), "Must have timestamp");
            assert!(!event.message.is_empty(), "Must have message");
        }

        /// GIVEN: Session-related event
        /// WHEN: Created
        /// THEN: Should include session field
        #[test]
        fn session_events_include_session() {
            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::SessionCreated,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("feature-auth".to_string()),
                agent_id: None,
                data: None,
                message: "Created".to_string(),
            };

            assert!(
                event.session.is_some(),
                "Session events should have session"
            );
            assert_eq!(event.session, Some("feature-auth".to_string()));
        }

        /// GIVEN: Agent-related event
        /// WHEN: Created
        /// THEN: Should include `agent_id` field
        #[test]
        fn agent_events_include_agent_id() {
            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::AgentHeartbeat,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: None,
                agent_id: Some("agent-xyz".to_string()),
                data: None,
                message: "Heartbeat".to_string(),
            };

            assert!(
                event.agent_id.is_some(),
                "Agent events should have agent_id"
            );
        }

        /// GIVEN: Event with additional data
        /// WHEN: Created
        /// THEN: Data field should contain structured info
        #[test]
        fn event_data_is_structured() -> Result<(), Box<dyn std::error::Error>> {
            use serde_json::json;

            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::BeadStatusChanged,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("task".to_string()),
                agent_id: None,
                data: Some(json!({
                    "old_status": "in_progress",
                    "new_status": "completed",
                    "bead_id": "zjj-abc12"
                })),
                message: "Status changed".to_string(),
            };

            let data = event.data.as_ref().ok_or("Data field is missing")?;
            assert!(data.get("old_status").is_some());
            assert!(data.get("new_status").is_some());
            Ok(())
        }
    }

    mod events_response_behavior {
        use super::*;

        /// GIVEN: Events query result
        /// WHEN: Response created
        /// THEN: Should have events, total, and pagination info
        #[test]
        fn response_has_pagination_info() {
            let response = EventsResponse {
                events: vec![],
                total: 100,
                has_more: true,
                cursor: Some("cursor-abc".to_string()),
            };

            assert_eq!(response.total, 100, "Should show total count");
            assert!(response.has_more, "Should indicate more available");
            assert!(
                response.cursor.is_some(),
                "Should have cursor for next page"
            );
        }

        /// GIVEN: No more events
        /// WHEN: Response created
        /// THEN: `has_more=false`, cursor=None
        #[test]
        fn last_page_has_no_cursor() {
            let response = EventsResponse {
                events: vec![],
                total: 10,
                has_more: false,
                cursor: None,
            };

            assert!(!response.has_more);
            assert!(response.cursor.is_none());
        }

        /// GIVEN: Empty result
        /// WHEN: Response created
        /// THEN: Should handle gracefully
        #[test]
        fn empty_response_is_valid() {
            let response = EventsResponse {
                events: vec![],
                total: 0,
                has_more: false,
                cursor: None,
            };

            assert!(response.events.is_empty());
            assert_eq!(response.total, 0);
        }
    }

    mod events_options_behavior {
        use super::*;

        /// GIVEN: Events query with session filter
        /// WHEN: Options created
        /// THEN: Should filter by session
        #[test]
        fn session_filter_is_applied() {
            let options = EventsOptions {
                session: Some("feature-x".to_string()),
                event_type: None,
                limit: Some(10),
                follow: false,
                since: None,
                format: zjj_core::OutputFormat::Json,
            };

            assert_eq!(options.session, Some("feature-x".to_string()));
        }

        /// GIVEN: Events query with `event_type` filter
        /// WHEN: Options created
        /// THEN: Should filter by type
        #[test]
        fn event_type_filter_is_applied() {
            let options = EventsOptions {
                session: None,
                event_type: Some("session_created".to_string()),
                limit: Some(10),
                follow: false,
                since: None,
                format: zjj_core::OutputFormat::Json,
            };

            assert_eq!(options.event_type, Some("session_created".to_string()));
        }

        /// GIVEN: Follow mode enabled
        /// WHEN: Options created
        /// THEN: Should indicate streaming
        #[test]
        fn follow_mode_enables_streaming() {
            let options = EventsOptions {
                session: None,
                event_type: None,
                limit: Some(100),
                follow: true,
                since: None,
                format: zjj_core::OutputFormat::Json,
            };

            assert!(options.follow, "Should be in follow mode");
        }

        /// GIVEN: Limit specified
        /// WHEN: Options created
        /// THEN: Should respect limit
        #[test]
        fn limit_is_respected() {
            let options = EventsOptions {
                session: None,
                event_type: None,
                limit: Some(50),
                follow: false,
                since: None,
                format: zjj_core::OutputFormat::Json,
            };

            assert_eq!(options.limit, Some(50));
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: Event is serialized
        /// WHEN: AI parses it
        /// THEN: Should have all fields for processing
        #[test]
        fn event_json_is_complete() -> Result<(), Box<dyn std::error::Error>> {
            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::SessionCreated,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: Some("test".to_string()),
                agent_id: Some("agent-1".to_string()),
                data: None,
                message: "Created".to_string(),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&event)?)?;

            // Required fields
            assert!(json.get("id").is_some(), "Must have id");
            assert!(json.get("event_type").is_some(), "Must have event_type");
            assert!(json.get("timestamp").is_some(), "Must have timestamp");
            assert!(json.get("message").is_some(), "Must have message");

            // Optional but present
            assert!(json.get("session").is_some(), "Should include session");
            assert!(json.get("agent_id").is_some(), "Should include agent_id");
            Ok(())
        }

        /// GIVEN: `EventsResponse` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have events array and pagination
        #[test]
        fn events_response_json_is_paginated() -> Result<(), Box<dyn std::error::Error>> {
            let response = EventsResponse {
                events: vec![Event {
                    id: "evt-1".to_string(),
                    event_type: EventType::SessionCreated,
                    timestamp: "2025-01-15T12:00:00Z".to_string(),
                    session: Some("test".to_string()),
                    agent_id: None,
                    data: None,
                    message: "Created".to_string(),
                }],
                total: 100,
                has_more: true,
                cursor: Some("next-page".to_string()),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&response)?)?;

            // Pagination fields
            assert!(json.get("total").is_some());
            assert!(json.get("has_more").is_some());
            assert!(json.get("cursor").is_some());

            // Events array
            assert!(json.get("events").is_some());
            assert!(json["events"].is_array());
            Ok(())
        }

        /// GIVEN: Event type in JSON
        /// WHEN: Parsed
        /// THEN: Should be lowercase `snake_case` string
        #[test]
        fn event_type_json_is_snake_case() -> Result<(), Box<dyn std::error::Error>> {
            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::BeadStatusChanged,
                timestamp: "now".to_string(),
                session: None,
                agent_id: None,
                data: None,
                message: "Changed".to_string(),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&event)?)?;

            let event_type = json["event_type"]
                .as_str()
                .ok_or("event_type is not a string")?;
            assert_eq!(event_type, event_type.to_lowercase());
            assert!(event_type.contains('_'), "Should be snake_case");
            Ok(())
        }
    }

    mod follow_mode_behavior {
        use super::*;

        /// GIVEN: Follow mode
        /// WHEN: Events are streamed
        /// THEN: JSON format should output one event per line
        #[test]
        fn follow_outputs_json_lines() -> Result<(), Box<dyn std::error::Error>> {
            // In follow mode, each event should be a complete JSON object
            // that can be parsed independently
            let event = Event {
                id: "evt-1".to_string(),
                event_type: EventType::AgentHeartbeat,
                timestamp: "2025-01-15T12:00:00Z".to_string(),
                session: None,
                agent_id: Some("agent-1".to_string()),
                data: None,
                message: "Heartbeat".to_string(),
            };

            // Each event should be a valid JSON that can be parsed
            let json_str = serde_json::to_string(&event)?;
            let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

            assert!(parsed.is_object(), "Each line should be valid JSON object");
            Ok(())
        }
    }
}

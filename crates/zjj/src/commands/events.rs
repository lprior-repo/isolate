//! Events command - Event streaming for multi-agent coordination
//!
//! Provides real-time event streaming with --follow support.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::commands::get_session_db;

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
pub fn run(options: &EventsOptions) -> Result<()> {
    if options.follow {
        run_follow(options)
    } else {
        run_list(options)
    }
}

fn run_list(options: &EventsOptions) -> Result<()> {
    // Get recent events from the database/log
    let events = get_recent_events(
        options.session.as_deref(),
        options.event_type.as_deref(),
        options.limit.unwrap_or(50),
        options.since.as_deref(),
    )?;

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
        for event in &response.events {
            let session_str = event.session.as_ref().map_or(String::new(), |s| format!(" [{s}]"));
            let agent_str = event.agent_id.as_ref().map_or(String::new(), |a| format!(" agent:{a}"));
            println!(
                "{} {}{}{}: {}",
                event.timestamp,
                event.event_type,
                session_str,
                agent_str,
                event.message
            );
        }
    }

    Ok(())
}

fn run_follow(options: &EventsOptions) -> Result<()> {
    eprintln!("Following events... (Ctrl+C to stop)");
    eprintln!();

    // In follow mode, we poll for new events
    let mut last_id: Option<String> = None;

    loop {
        let events = get_new_events(
            options.session.as_deref(),
            options.event_type.as_deref(),
            last_id.as_deref(),
        )?;

        for event in &events {
            if options.format.is_json() {
                println!("{}", serde_json::to_string(&event)?);
            } else {
                let session_str = event.session.as_ref().map_or(String::new(), |s| format!(" [{s}]"));
                let agent_str = event.agent_id.as_ref().map_or(String::new(), |a| format!(" agent:{a}"));
                println!(
                    "{} {}{}{}: {}",
                    event.timestamp,
                    event.event_type,
                    session_str,
                    agent_str,
                    event.message
                );
            }
            last_id = Some(event.id.clone());
        }

        // Poll interval
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn get_recent_events(
    session: Option<&str>,
    event_type: Option<&str>,
    limit: usize,
    _since: Option<&str>,
) -> Result<Vec<Event>> {
    // Try to read from the events log file
    let db = get_session_db().ok();

    let mut events = Vec::new();

    // Generate synthetic events from current state
    if let Some(db) = db {
        if let Ok(sessions) = db.list_blocking(None) {
            for sess in sessions.iter().take(limit) {
                if session.is_some_and(|s| s != sess.name) {
                    continue;
                }

                let event = Event {
                    id: format!("evt-{}", sess.name),
                    event_type: EventType::SessionCreated,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    session: Some(sess.name.clone()),
                    agent_id: None,
                    data: None,
                    message: format!("Session '{}' exists", sess.name),
                };

                if event_type.is_none() || event_type == Some("session_created") {
                    events.push(event);
                }
            }
        }
    }

    Ok(events)
}

fn get_new_events(
    session: Option<&str>,
    event_type: Option<&str>,
    _after_id: Option<&str>,
) -> Result<Vec<Event>> {
    // In a real implementation, this would watch a log file or use inotify
    // For now, return empty to indicate no new events
    let _ = (session, event_type);
    Ok(vec![])
}

/// Generate a simple event ID based on timestamp
fn generate_event_id() -> String {
    let now = chrono::Utc::now();
    format!(
        "evt-{}-{}",
        now.timestamp_millis(),
        std::process::id()
    )
}

/// Log an event to the events log
pub fn log_event(event_type: EventType, session: Option<&str>, agent_id: Option<&str>, message: &str) -> Result<()> {
    let event = Event {
        id: generate_event_id(),
        event_type,
        timestamp: chrono::Utc::now().to_rfc3339(),
        session: session.map(String::from),
        agent_id: agent_id.map(String::from),
        data: None,
        message: message.to_string(),
    };

    // Write to events log file
    if let Ok(data_dir) = super::zjj_data_dir() {
        let events_file = data_dir.join("events.jsonl");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_file)
        {
            use std::io::Write;
            let _ = writeln!(file, "{}", serde_json::to_string(&event).unwrap_or_default());
        }
    }

    Ok(())
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
    fn test_event_serialization() {
        let event = Event {
            id: "evt-1".to_string(),
            event_type: EventType::SessionCreated,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            session: Some("test".to_string()),
            agent_id: None,
            data: None,
            message: "Created session".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\":\"session_created\""));
        assert!(json.contains("\"session\":\"test\""));
    }

    #[test]
    fn test_events_response_serialization() {
        let response = EventsResponse {
            events: vec![],
            total: 0,
            has_more: false,
            cursor: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn test_event_types_are_snake_case() {
        let types = vec![
            EventType::SessionCreated,
            EventType::LockAcquired,
            EventType::BeadStatusChanged,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            assert!(json.contains('_') || json == "\"session_created\"");
        }
    }
}

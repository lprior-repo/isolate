//! Events command implementation
//!
//! Provides event listing and streaming for coordination.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use stak_core::{Event, EventType};

/// Events command options
#[derive(Debug, Clone)]
pub struct EventsOptions {
    /// List events
    pub list: bool,
    /// Follow mode (stream events)
    pub follow: bool,
    /// Filter by session
    pub session: Option<String>,
    /// Filter by event type
    pub event_type: Option<String>,
    /// Maximum number of events
    pub limit: usize,
}

/// Events response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsResponse {
    /// List of events
    pub events: Vec<Event>,
    /// Total count
    pub total: usize,
    /// Whether there are more events
    pub has_more: bool,
}

/// Event store (in-memory for now, will be database-backed)
#[derive(Debug, Clone, Default)]
pub struct EventStore {
    events: Vec<Event>,
}

impl EventStore {
    /// Create a new event store
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event
    pub fn add(&mut self, event: Event) {
        self.events.push(event);
    }

    /// List events with optional filters
    #[must_use]
    pub fn list(
        &self,
        session: Option<&str>,
        event_type: Option<&str>,
        limit: usize,
    ) -> Vec<&Event> {
        let mut filtered: Vec<&Event> = self
            .events
            .iter()
            .filter(|e| {
                let session_match = session.is_none_or(|s| e.session.as_deref() == Some(s));
                let type_match = event_type.is_none_or(|t| {
                    e.event_type.to_string().to_lowercase() == t.to_lowercase().replace('-', "_")
                });
                session_match && type_match
            })
            .collect();

        // Sort by timestamp descending (most recent first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        filtered.truncate(limit);
        filtered
    }

    /// Get events after a given timestamp
    #[must_use]
    pub fn after(&self, after: &DateTime<Utc>) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.timestamp > *after)
            .collect()
    }
}

/// Run the events command
///
/// # Errors
///
/// Returns an error if event listing fails.
pub fn run(options: &EventsOptions, store: &EventStore) -> Result<()> {
    if options.follow {
        run_follow(options, store)
    } else {
        run_list(options, store)
    }
}

/// List events
fn run_list(options: &EventsOptions, store: &EventStore) -> Result<()> {
    let events = store.list(
        options.session.as_deref(),
        options.event_type.as_deref(),
        options.limit,
    );

    if events.is_empty() {
        println!("No events found");
        return Ok(());
    }

    println!("Events ({}):", events.len());
    for event in events {
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
            event.timestamp.to_rfc3339(),
            event.event_type,
            session_str,
            agent_str,
            event.message
        );
    }

    Ok(())
}

/// Follow events (streaming mode)
fn run_follow(options: &EventsOptions, _store: &EventStore) -> Result<()> {
    eprintln!("Following events... (Ctrl+C to stop)");
    eprintln!();

    // In follow mode, we would poll for new events
    // For now, just indicate this is a placeholder
    println!("Event streaming is not yet implemented");
    println!("Use 'stak events list' to see recent events");

    let _ = options; // Acknowledge unused parameter

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_store() {
        let mut store = EventStore::new();

        store.add(Event::new(
            EventType::AgentRegistered,
            "Agent registered".to_string(),
        ));
        store.add(Event::new(
            EventType::LockAcquired,
            "Lock acquired".to_string(),
        ));

        let events = store.list(None, None, 10);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_event_filter() {
        let mut store = EventStore::new();

        let event1 = Event::new(EventType::AgentRegistered, "Agent registered".to_string())
            .with_session("session-1");
        let event2 = Event::new(EventType::LockAcquired, "Lock acquired".to_string())
            .with_session("session-2");

        store.add(event1);
        store.add(event2);

        let events = store.list(Some("session-1"), None, 10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].session, Some("session-1".to_string()));
    }
}

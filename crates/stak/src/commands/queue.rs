//! Queue command implementation
//!
//! Manages the merge queue for sequential multi-agent coordination.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use anyhow::Result;
use stak_core::{Queue, QueueEntry, QueueEntryId, QueueStatus};

/// Queue command options
#[derive(Debug, Clone)]
pub struct QueueOptions {
    /// List queue entries
    pub list: bool,
    /// Show queue status
    pub status: bool,
    /// Session to enqueue
    pub enqueue: Option<String>,
    /// Session to dequeue
    pub dequeue: Option<String>,
    /// Process queue entries
    pub process: bool,
    /// Priority for enqueue (lower = higher priority)
    pub priority: u32,
}

/// Run the queue command
///
/// # Errors
///
/// Returns an error if:
/// - Session is required but not provided (enqueue)
/// - Session is already in queue (enqueue)
pub async fn run(options: &QueueOptions, queue: &mut Queue) -> Result<()> {
    if options.enqueue.is_some() {
        handle_enqueue(options, queue).await
    } else if options.dequeue.is_some() {
        handle_dequeue(options, queue).await
    } else if options.process {
        handle_process(options, queue).await
    } else if options.status {
        handle_status(options, queue).await
    } else {
        // Default to list
        handle_list(options, queue).await
    }
}

/// Handle enqueue command
async fn handle_enqueue(options: &QueueOptions, queue: &mut Queue) -> Result<()> {
    let session = options
        .enqueue
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Session required for enqueue"))?;

    // Check if session already in queue
    if queue.find_by_session(&session).is_some() {
        anyhow::bail!("Session '{session}' is already in the queue");
    }

    let id = QueueEntryId::new(format!("q-{}", chrono::Utc::now().timestamp_millis()));
    let entry = QueueEntry::new(id, session.clone(), options.priority);

    queue.enqueue(entry);

    println!(
        "Added session '{session}' to queue with priority {}",
        options.priority
    );

    Ok(())
}

/// Handle dequeue command
async fn handle_dequeue(options: &QueueOptions, queue: &mut Queue) -> Result<()> {
    let session = options
        .dequeue
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Session required for dequeue"))?;

    let entry = queue
        .find_by_session(&session)
        .map(|e| e.id.clone())
        .and_then(|id| queue.dequeue(&id));

    if let Some(removed) = entry {
        println!("Removed session '{}' from queue", removed.session);
        Ok(())
    } else {
        println!("Session '{session}' not found in queue");
        Ok(())
    }
}

/// Handle list command
async fn handle_list(options: &QueueOptions, queue: &Queue) -> Result<()> {
    let entries = queue.entries();

    if entries.is_empty() {
        println!("Queue is empty");
        return Ok(());
    }

    println!("╔═════════════════════════════════════════════════════════════════╗");
    println!("║ MERGE QUEUE                                                     ║");
    println!("╠═════════════════════════════════════════════════════════════════╣");
    println!("║ Session                    │ Status      │ Priority │ Agent    ║");
    println!("╠═════════════════════════════════════════════════════════════════╣");

    for entry in entries {
        let agent = entry.claimed_by.as_deref().map_or("-", |s| s);
        println!(
            "║ {:25} │ {:11} │ {:8} │ {:8} ║",
            truncate(&entry.session, 25),
            entry.status,
            entry.priority,
            truncate(agent, 8)
        );
    }
    println!("╚═════════════════════════════════════════════════════════════════╝");

    let _ = options; // Acknowledge unused parameter

    Ok(())
}

/// Handle status command
async fn handle_status(_options: &QueueOptions, queue: &Queue) -> Result<()> {
    let entries = queue.entries();

    let pending = entries
        .iter()
        .filter(|e| e.status == QueueStatus::Pending)
        .count();
    let claimed = entries
        .iter()
        .filter(|e| e.status == QueueStatus::Claimed)
        .count();
    let merged = entries
        .iter()
        .filter(|e| e.status == QueueStatus::Merged)
        .count();
    let failed = entries.iter().filter(|e| e.status.is_failed()).count();

    println!("Queue Statistics:");
    println!("  Total:      {}", entries.len());
    println!("  Pending:    {pending}");
    println!("  Claimed:    {claimed}");
    println!("  Merged:     {merged}");
    println!("  Failed:     {failed}");

    Ok(())
}

/// Handle process command
async fn handle_process(_options: &QueueOptions, queue: &Queue) -> Result<()> {
    match queue.next_pending() {
        Some(entry) => {
            println!("Next pending session: {}", entry.session);
            println!("  ID: {}", entry.id);
            println!("  Status: {}", entry.status);
            println!("  Priority: {}", entry.priority);
            if let Some(agent) = &entry.claimed_by {
                println!("  Agent: {agent}");
            }
        }
        None => {
            println!("Queue is empty - no pending entries");
        }
    }

    Ok(())
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("very long string", 8), "very lo…");
    }

    #[tokio::test]
    async fn test_enqueue_dequeue() -> Result<()> {
        let mut queue = Queue::new();
        let options = QueueOptions {
            list: false,
            status: false,
            enqueue: Some("test-session".to_string()),
            dequeue: None,
            process: false,
            priority: 5,
        };

        run(&options, &mut queue).await?;
        assert_eq!(queue.len(), 1);

        let dequeue_options = QueueOptions {
            list: false,
            status: false,
            enqueue: None,
            dequeue: Some("test-session".to_string()),
            process: false,
            priority: 0,
        };

        run(&dequeue_options, &mut queue).await?;
        assert!(queue.is_empty());

        Ok(())
    }
}

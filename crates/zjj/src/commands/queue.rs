//! Manage the merge queue for sequential multi-agent coordination
//!
//! The merge queue ensures that multiple agents can coordinate their work
//! by processing workspaces sequentially. This command provides access to:
//! - Adding workspaces to the queue
//! - Listing pending and completed entries
//! - Getting the next entry to process
//! - Removing entries from the queue
//! - Checking overall queue status and statistics

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

/// Response for queue add operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAddOutput {
    pub id: i64,
    pub workspace: String,
    pub bead_id: Option<String>,
    pub priority: i32,
    pub position: usize,
    pub total_pending: usize,
    pub message: String,
}

/// Response for queue list operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueListOutput {
    pub entries: Vec<QueueEntryOutput>,
    pub total: usize,
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Individual queue entry for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryOutput {
    pub id: i64,
    pub workspace: String,
    pub bead_id: Option<String>,
    pub priority: i32,
    pub status: String,
    pub added_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
    pub agent_id: Option<String>,
}

/// Response for queue next operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNextOutput {
    pub entry: Option<QueueEntryOutput>,
    pub message: String,
}

/// Response for queue remove operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRemoveOutput {
    pub workspace: String,
    pub removed: bool,
    pub message: String,
}

/// Response for queue status operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusOutput {
    pub workspace: String,
    pub exists: bool,
    pub id: Option<i64>,
    pub status: Option<String>,
    pub message: String,
}

/// Response for queue stats operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsOutput {
    pub total: usize,
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Options for queue command
#[derive(Debug, Clone)]
pub struct QueueOptions {
    pub format: OutputFormat,
    pub add: Option<String>,
    pub bead_id: Option<String>,
    pub priority: i32,
    pub agent_id: Option<String>,
    pub list: bool,
    pub next: bool,
    pub remove: Option<String>,
    pub status: Option<String>,
    pub stats: bool,
}

/// Get or create the merge queue database
fn get_queue() -> Result<zjj_core::MergeQueue> {
    let queue_db = Path::new(".zjj/queue.db");
    zjj_core::MergeQueue::open(queue_db).context("Failed to open merge queue database")
}

/// Run the queue command with options
pub fn run_with_options(options: &QueueOptions) -> Result<()> {
    let queue = get_queue()?;

    if let Some(workspace) = &options.add {
        handle_add(&queue, workspace, options)?;
    } else if options.list {
        handle_list(&queue, options)?;
    } else if options.next {
        handle_next(&queue, options)?;
    } else if let Some(workspace) = &options.remove {
        handle_remove(&queue, workspace, options)?;
    } else if let Some(workspace) = &options.status {
        handle_status(&queue, workspace, options)?;
    } else if options.stats {
        handle_stats(&queue, options)?;
    } else {
        // Default to showing list
        handle_list(&queue, options)?;
    }

    Ok(())
}

/// Handle the add command
fn handle_add(queue: &zjj_core::MergeQueue, workspace: &str, options: &QueueOptions) -> Result<()> {
    let response = queue.add(
        workspace,
        options.bead_id.as_deref(),
        options.priority,
        options.agent_id.as_deref(),
    )?;

    let message = format!(
        "Added workspace '{}' to queue at position {}/{}",
        workspace, response.position, response.total_pending
    );

    if options.format.is_json() {
        let output = QueueAddOutput {
            id: response.entry.id,
            workspace: response.entry.workspace.clone(),
            bead_id: response.entry.bead_id.clone(),
            priority: response.entry.priority,
            position: response.position,
            total_pending: response.total_pending,
            message,
        };
        print_queue_envelope("queue-add-response", &output)?;
    } else {
        println!("{message}");
    }

    Ok(())
}

/// Handle the list command
fn handle_list(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let entries = queue.list(None)?;
    let stats = queue.stats()?;

    if options.format.is_json() {
        let entries: Vec<QueueEntryOutput> = entries
            .into_iter()
            .map(|e| QueueEntryOutput {
                id: e.id,
                workspace: e.workspace,
                bead_id: e.bead_id,
                priority: e.priority,
                status: e.status.as_str().to_string(),
                added_at: e.added_at,
                started_at: e.started_at,
                completed_at: e.completed_at,
                error_message: e.error_message,
                agent_id: e.agent_id,
            })
            .collect();

        let output = QueueListOutput {
            entries,
            total: stats.total,
            pending: stats.pending,
            processing: stats.processing,
            completed: stats.completed,
            failed: stats.failed,
        };
        print_queue_envelope("queue-list-response", &output)?;
    } else {
        if entries.is_empty() {
            println!("Queue is empty");
        } else {
            println!("╔═════════════════════════════════════════════════════════════════╗");
            println!("║ MERGE QUEUE                                                     ║");
            println!("╠════╦═══════════════════╦═════════════╦═══════════════════════════╣");
            println!("║ ID ║ Workspace         ║ Status      ║ Priority │ Agent         ║");
            println!("╠════╬═══════════════════╬═════════════╬═══════════════════════════╣");

            for entry in &entries {
                let status_str = entry.status.as_str();
                let agent = entry.agent_id.as_deref().unwrap_or("-");
                println!(
                    "║ {:2} ║ {:17} ║ {:11} ║ {:8} │ {:13} ║",
                    entry.id,
                    &entry.workspace[..entry.workspace.len().min(17)],
                    status_str,
                    entry.priority,
                    &agent[..agent.len().min(13)]
                );
            }
            println!("╚════╩═══════════════════╩═════════════╩═══════════════════════════╝");
        }

        println!(
            "\nStats: {} total | {} pending | {} processing | {} completed | {} failed",
            stats.total, stats.pending, stats.processing, stats.completed, stats.failed
        );
    }

    Ok(())
}

/// Handle the next command
fn handle_next(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let entry = queue.next()?;

    if options.format.is_json() {
        let entry_output = entry.map(|e| QueueEntryOutput {
            id: e.id,
            workspace: e.workspace,
            bead_id: e.bead_id,
            priority: e.priority,
            status: e.status.as_str().to_string(),
            added_at: e.added_at,
            started_at: e.started_at,
            completed_at: e.completed_at,
            error_message: e.error_message,
            agent_id: e.agent_id,
        });

        let message = if entry_output.is_some() {
            "Found next pending entry".to_string()
        } else {
            "Queue is empty".to_string()
        };

        let output = QueueNextOutput {
            entry: entry_output,
            message,
        };
        print_queue_envelope("queue-next-response", &output)?;
    } else {
        match entry {
            Some(e) => {
                println!("Next pending workspace: {}", e.workspace);
                println!("  ID: {}", e.id);
                println!("  Status: {}", e.status.as_str());
                println!("  Priority: {}", e.priority);
                if let Some(bead_id) = e.bead_id {
                    println!("  Bead ID: {bead_id}");
                }
                if let Some(agent_id) = e.agent_id {
                    println!("  Agent ID: {agent_id}");
                }
            }
            None => {
                println!("Queue is empty - no pending entries");
            }
        }
    }

    Ok(())
}

/// Handle the remove command
fn handle_remove(
    queue: &zjj_core::MergeQueue,
    workspace: &str,
    options: &QueueOptions,
) -> Result<()> {
    let removed = queue.remove(workspace)?;

    let message = if removed {
        format!("Removed workspace '{workspace}' from queue")
    } else {
        format!("Workspace '{workspace}' not found in queue")
    };

    if options.format.is_json() {
        let output = QueueRemoveOutput {
            workspace: workspace.to_string(),
            removed,
            message,
        };
        print_queue_envelope("queue-remove-response", &output)?;
    } else {
        println!("{message}");
    }

    Ok(())
}

/// Handle the status command
fn handle_status(
    queue: &zjj_core::MergeQueue,
    workspace: &str,
    options: &QueueOptions,
) -> Result<()> {
    let entry = queue.get_by_workspace(workspace)?;

    if options.format.is_json() {
        let (exists, id, status) = entry.as_ref().map_or((false, None, None), |e| {
            (true, Some(e.id), Some(e.status.as_str().to_string()))
        });

        let message = if exists {
            format!("Workspace '{workspace}' is in queue")
        } else {
            format!("Workspace '{workspace}' is not in queue")
        };

        let output = QueueStatusOutput {
            workspace: workspace.to_string(),
            exists,
            id,
            status,
            message,
        };
        print_queue_envelope("queue-status-response", &output)?;
    } else {
        match entry {
            Some(e) => {
                println!("Workspace: {}", e.workspace);
                println!("  ID: {}", e.id);
                println!("  Status: {}", e.status.as_str());
                println!("  Priority: {}", e.priority);
                if let Some(bead_id) = e.bead_id {
                    println!("  Bead ID: {bead_id}");
                }
                if let Some(agent_id) = e.agent_id {
                    println!("  Agent ID: {agent_id}");
                }
                if let Some(started_at) = e.started_at {
                    println!("  Started At: {started_at}");
                }
                if let Some(error_msg) = e.error_message {
                    println!("  Error: {error_msg}");
                }
            }
            None => {
                println!("Workspace '{workspace}' is not in the queue");
            }
        }
    }

    Ok(())
}

/// Handle the stats command
fn handle_stats(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let stats = queue.stats()?;

    if options.format.is_json() {
        let output = QueueStatsOutput {
            total: stats.total,
            pending: stats.pending,
            processing: stats.processing,
            completed: stats.completed,
            failed: stats.failed,
        };
        print_queue_envelope("queue-stats-response", &output)?;
    } else {
        println!("Queue Statistics:");
        println!("  Total:      {}", stats.total);
        println!("  Pending:    {}", stats.pending);
        println!("  Processing: {}", stats.processing);
        println!("  Completed:  {}", stats.completed);
        println!("  Failed:     {}", stats.failed);
    }

    Ok(())
}

fn print_queue_envelope<T: Serialize>(schema_name: &str, payload: &T) -> Result<()> {
    let envelope = SchemaEnvelope::new(schema_name, "single", payload);
    let json_str =
        serde_json::to_string_pretty(&envelope).context("Failed to serialize queue response")?;
    println!("{json_str}");
    Ok(())
}

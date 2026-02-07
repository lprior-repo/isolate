//! Manage the merge queue for sequential multi-agent coordination
//!
//! The merge queue ensures that multiple agents can coordinate their work
//! by processing workspaces sequentially. This command provides access to:
//! - Adding workspaces to the queue
//! - Listing pending and completed entries
//! - Getting the next entry to process
//! - Removing entries from the queue
//! - Checking overall queue status and statistics

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::commands::get_session_db;

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
#[allow(clippy::struct_excessive_bools)]
pub struct QueueOptions {
    pub format: OutputFormat,
    pub add: Option<String>,
    pub bead_id: Option<String>,
    pub priority: i32,
    pub agent_id: Option<String>,
    pub list: bool,
    pub process: bool,
    pub next: bool,
    pub remove: Option<String>,
    pub status: Option<String>,
    pub stats: bool,
}

/// Get or create the merge queue database
async fn get_queue() -> Result<zjj_core::MergeQueue> {
    let queue_db = Path::new(".zjj/queue.db");
    zjj_core::MergeQueue::open(queue_db)
        .await
        .context("Failed to open merge queue database")
}

/// Run the queue command with options
pub async fn run_with_options(options: &QueueOptions) -> Result<()> {
    let queue = get_queue().await?;

    if let Some(workspace) = &options.add {
        handle_add(&queue, workspace, options).await?;
    } else if options.list {
        handle_list(&queue, options).await?;
    } else if options.process {
        handle_process(&queue, options).await?;
    } else if options.next {
        handle_next(&queue, options).await?;
    } else if let Some(workspace) = &options.remove {
        handle_remove(&queue, workspace, options).await?;
    } else if let Some(workspace) = &options.status {
        handle_status(&queue, workspace, options).await?;
    } else if options.stats {
        handle_stats(&queue, options).await?;
    } else {
        // Default to showing list
        handle_list(&queue, options).await?;
    }

    Ok(())
}

/// Handle the add command
async fn handle_add(
    queue: &zjj_core::MergeQueue,
    workspace: &str,
    options: &QueueOptions,
) -> Result<()> {
    let response = queue
        .add(
            workspace,
            options.bead_id.as_deref(),
            options.priority,
            options.agent_id.as_deref(),
        )
        .await?;

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
async fn handle_list(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let entries = queue.list(None).await?;
    let stats = queue.stats().await?;

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
                let agent = entry.agent_id.as_deref().map_or("-", |s| s);
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
async fn handle_next(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let entry = queue.next().await?;

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

/// Handle the process command
async fn handle_process(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let agent_id = resolve_agent_id(options.agent_id.as_deref());
    let entry = queue.next_with_lock(&agent_id).await?;

    let Some(entry) = entry else {
        let lock = queue.get_processing_lock().await?;
        let message = if let Some(lock) = lock {
            format!(
                "Queue is locked by agent '{}' until {}",
                lock.agent_id, lock.expires_at
            )
        } else {
            "Queue is empty - no pending entries".to_string()
        };

        if options.format.is_json() {
            let output = QueueNextOutput {
                entry: None,
                message,
            };
            print_queue_envelope("queue-process-response", &output)?;
        } else {
            println!("{message}");
        }

        return Ok(());
    };

    if !options.format.is_json() {
        println!(
            "Processing queued workspace '{}' (queue id {})",
            entry.workspace, entry.id
        );
    }

    let workspace_path = resolve_workspace_path(&entry.workspace).await?;
    let original_dir = std::env::current_dir().context("Failed to read current directory")?;
    std::env::set_current_dir(&workspace_path)
        .with_context(|| format!("Failed to enter workspace at {}", workspace_path.display()))?;

    let done_options = crate::commands::done::types::DoneOptions {
        message: None,
        keep_workspace: false,
        no_keep: false,
        squash: false,
        dry_run: false,
        detect_conflicts: false,
        no_bead_update: false,
        format: options.format,
    };

    let done_result = crate::commands::done::run_with_options(&done_options).await;

    let _ = std::env::set_current_dir(&original_dir);

    match done_result {
        Ok(()) => {
            let marked = queue.mark_completed(&entry.workspace).await?;
            anyhow::ensure!(
                marked,
                "Failed to mark queue entry '{}' as completed",
                entry.workspace
            );
        }
        Err(err) => {
            let error_msg = err.to_string();
            let status_msg = if error_msg.contains("conflict") {
                "merge conflict"
            } else if error_msg.contains("not in a workspace") {
                "not in workspace"
            } else {
                "done failed"
            };

            let _ = queue.mark_failed(&entry.workspace, status_msg).await;
            let _ = queue.release_processing_lock(&agent_id).await;
            return Err(err);
        }
    }

    let _ = queue.release_processing_lock(&agent_id).await;
    Ok(())
}

/// Handle the remove command
async fn handle_remove(
    queue: &zjj_core::MergeQueue,
    workspace: &str,
    options: &QueueOptions,
) -> Result<()> {
    let removed = queue.remove(workspace).await?;

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
async fn handle_status(
    queue: &zjj_core::MergeQueue,
    workspace: &str,
    options: &QueueOptions,
) -> Result<()> {
    let entry = queue.get_by_workspace(workspace).await?;

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
async fn handle_stats(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let stats = queue.stats().await?;

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

fn resolve_agent_id(agent_id: Option<&str>) -> String {
    agent_id
        .map(String::from)
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .unwrap_or_else(|| format!("pid-{}", std::process::id()))
}

async fn resolve_workspace_path(workspace: &str) -> Result<PathBuf> {
    if let Ok(db) = get_session_db().await {
        if let Some(session) = db.get(workspace).await? {
            let path = PathBuf::from(session.workspace_path);
            if tokio::fs::try_exists(&path)
                .await
                .is_ok_and(|exists| exists)
            {
                return Ok(path);
            }
        }
    }

    let fallback = PathBuf::from(workspace);
    if tokio::fs::try_exists(&fallback)
        .await
        .is_ok_and(|exists| exists)
    {
        Ok(fallback)
    } else {
        Err(anyhow::anyhow!("Workspace '{workspace}' not found"))
    }
}

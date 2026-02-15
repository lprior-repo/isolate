//! Manage the merge queue for sequential multi-agent coordination
//!
//! The merge queue ensures that multiple agents can coordinate their work
//! by processing workspaces sequentially. This command provides access to:
//! - Adding workspaces to the queue
//! - Listing pending and completed entries
//! - Getting the next entry to process
//! - Removing entries from the queue
//! - Checking overall queue status and statistics

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use zjj_core::{json::SchemaEnvelope, OutputFormat, QueueStatus};

use crate::commands::{get_queue_db_path, get_session_db};

fn cli_flag_used(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

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

/// Queue event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEventOutput {
    pub event_id: i64,
    pub event_type: String,
    pub details_json: Option<String>,
    pub created_at: i64,
}

/// Response for queue status-id operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusIdOutput {
    pub entry: QueueEntryOutput,
    pub events: Vec<QueueEventOutput>,
    pub message: String,
}

/// Response for queue retry operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRetryOutput {
    pub entry: QueueEntryOutput,
    pub message: String,
}

/// Response for queue cancel operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueCancelOutput {
    pub entry: QueueEntryOutput,
    pub message: String,
}

/// Response for queue reclaim-stale operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueReclaimStaleOutput {
    pub reclaimed: usize,
    pub threshold_secs: i64,
    pub message: String,
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
    pub status_id: Option<i64>,
    pub retry: Option<i64>,
    pub cancel: Option<i64>,
    pub reclaim_stale: Option<i64>,
}

/// Get or create the merge queue database
async fn get_queue() -> Result<zjj_core::MergeQueue> {
    let queue_db = get_queue_db_path().await?;
    zjj_core::MergeQueue::open(&queue_db)
        .await
        .context("Failed to open merge queue database")
}

/// Run the queue command with options
pub async fn run_with_options(options: &QueueOptions) -> Result<()> {
    let add_only_flags_used_without_add = options.add.is_none()
        && ["--bead", "--priority", "--agent"]
            .into_iter()
            .any(cli_flag_used);

    if add_only_flags_used_without_add {
        anyhow::bail!("--bead, --priority, and --agent require --add");
    }

    let queue = get_queue().await?;

    if let Some(workspace) = &options.add {
        handle_add(&queue, workspace, options).await?;
    } else if let Some(id) = options.retry {
        handle_retry(&queue, id, options).await?;
    } else if let Some(id) = options.cancel {
        handle_cancel(&queue, id, options).await?;
    } else if let Some(threshold) = options.reclaim_stale {
        handle_reclaim_stale(&queue, threshold, options).await?;
    } else if let Some(queue_id) = options.status_id {
        handle_status_id(&queue, queue_id, options).await?;
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
#[allow(clippy::too_many_lines)]
async fn handle_process(queue: &zjj_core::MergeQueue, options: &QueueOptions) -> Result<()> {
    let agent_id = resolve_agent_id(options.agent_id.as_deref());
    if !queue.acquire_processing_lock(&agent_id).await? {
        let lock = queue.get_processing_lock().await?;
        let message = if let Some(lock) = lock {
            format!(
                "Queue is locked by agent '{}' until {}",
                lock.agent_id, lock.expires_at
            )
        } else {
            "Queue is locked by another agent".to_string()
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
    }

    let ready_entries = queue.list(Some(QueueStatus::ReadyToMerge)).await?;
    let Some(entry) = ready_entries.into_iter().next() else {
        let _ = queue.release_processing_lock(&agent_id).await;

        let message = "Queue has no ready-to-merge entries".to_string();
        if options.format.is_json() {
            let output = QueueNextOutput {
                entry: None,
                message,
            };
            print_queue_envelope("queue-process-response", &output)?;
        } else {
            println!("Queue has no ready-to-merge entries");
        }

        return Ok(());
    };

    if !options.format.is_json() {
        println!(
            "Processing queued workspace '{}' (queue id {})",
            entry.workspace, entry.id
        );
    }

    let repo_dir = std::env::current_dir().context("Failed to read current directory")?;
    let current_main_sha = get_main_sha(&repo_dir).await?;
    let is_fresh = queue.is_fresh(&entry.workspace, &current_main_sha).await?;

    if !is_fresh {
        queue
            .return_to_rebasing(&entry.workspace, &current_main_sha)
            .await?;
        let _ = queue.release_processing_lock(&agent_id).await;

        let message = format!(
            "Entry '{}' is stale against main {}, returned to rebasing",
            entry.workspace, current_main_sha
        );
        if options.format.is_json() {
            let output = QueueNextOutput {
                entry: Some(QueueEntryOutput {
                    id: entry.id,
                    workspace: entry.workspace,
                    bead_id: entry.bead_id,
                    priority: entry.priority,
                    status: QueueStatus::Rebasing.as_str().to_string(),
                    added_at: entry.added_at,
                    started_at: entry.started_at,
                    completed_at: entry.completed_at,
                    error_message: entry.error_message,
                    agent_id: entry.agent_id,
                }),
                message,
            };
            print_queue_envelope("queue-process-response", &output)?;
        } else {
            println!(
                "Entry '{}' is stale and was returned to rebasing",
                entry.workspace
            );
        }

        return Ok(());
    }

    queue.begin_merge(&entry.workspace).await?;

    let workspace_path = resolve_workspace_path(&entry.workspace).await?;
    std::env::set_current_dir(&workspace_path)
        .with_context(|| format!("Failed to enter workspace at {}", workspace_path.display()))?;

    let done_options = crate::commands::done::types::DoneOptions {
        workspace: Some(entry.workspace.clone()),
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

    match done_result {
        Ok(()) => {
            std::env::set_current_dir(&repo_dir)
                .context("Failed to restore original working directory")?;

            let merged_sha = get_main_sha(&repo_dir).await?;
            queue.complete_merge(&entry.workspace, &merged_sha).await?;
        }
        Err(err) => {
            let error_msg = err.to_string();

            let _ = std::env::set_current_dir(&repo_dir);
            let is_retryable = is_retryable_merge_failure(&error_msg);
            let _ = queue
                .fail_merge(&entry.workspace, &error_msg, is_retryable)
                .await;
            let _ = queue.release_processing_lock(&agent_id).await;
            return Err(err);
        }
    }

    let _ = queue.release_processing_lock(&agent_id).await;
    Ok(())
}

async fn get_main_sha(repo_dir: &Path) -> Result<String> {
    let output = Command::new("jj")
        .args(["log", "-r", "main", "--no-graph", "-T", "commit_id"])
        .current_dir(repo_dir)
        .output()
        .await
        .context("Failed to query main HEAD via jj")?;

    anyhow::ensure!(
        output.status.success(),
        "Failed to query main HEAD: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    anyhow::ensure!(!sha.is_empty(), "jj log returned empty main SHA");
    Ok(sha)
}

fn is_retryable_merge_failure(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.contains("conflict")
        || lower.contains("database is locked")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("tempor")
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

/// Handle the retry command
async fn handle_retry(queue: &zjj_core::MergeQueue, id: i64, options: &QueueOptions) -> Result<()> {
    let entry = queue
        .retry_entry(id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let message = format!(
        "Retried queue entry {} (workspace '{}') - attempt {}/{}",
        entry.id, entry.workspace, entry.attempt_count, entry.max_attempts
    );

    if options.format.is_json() {
        let entry_output = QueueEntryOutput {
            id: entry.id,
            workspace: entry.workspace,
            bead_id: entry.bead_id,
            priority: entry.priority,
            status: entry.status.as_str().to_string(),
            added_at: entry.added_at,
            started_at: entry.started_at,
            completed_at: entry.completed_at,
            error_message: entry.error_message,
            agent_id: entry.agent_id,
        };
        let output = QueueRetryOutput {
            entry: entry_output,
            message,
        };
        print_queue_envelope("queue-retry-response", &output)?;
    } else {
        println!("{message}");
    }

    Ok(())
}

/// Handle the cancel command
async fn handle_cancel(
    queue: &zjj_core::MergeQueue,
    id: i64,
    options: &QueueOptions,
) -> Result<()> {
    let entry = queue
        .cancel_entry(id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let message = format!(
        "Cancelled queue entry {} (workspace '{}')",
        entry.id, entry.workspace
    );

    if options.format.is_json() {
        let entry_output = QueueEntryOutput {
            id: entry.id,
            workspace: entry.workspace,
            bead_id: entry.bead_id,
            priority: entry.priority,
            status: entry.status.as_str().to_string(),
            added_at: entry.added_at,
            started_at: entry.started_at,
            completed_at: entry.completed_at,
            error_message: entry.error_message,
            agent_id: entry.agent_id,
        };
        let output = QueueCancelOutput {
            entry: entry_output,
            message,
        };
        print_queue_envelope("queue-cancel-response", &output)?;
    } else {
        println!("{message}");
    }

    Ok(())
}

/// Handle the reclaim-stale command
async fn handle_reclaim_stale(
    queue: &zjj_core::MergeQueue,
    threshold_secs: i64,
    options: &QueueOptions,
) -> Result<()> {
    let reclaimed = queue.reclaim_stale(threshold_secs).await?;

    let message = if reclaimed == 0 {
        format!("No stale entries found (threshold: {threshold_secs}s)")
    } else {
        format!(
            "Reclaimed {reclaimed} stale entr{}",
            if reclaimed == 1 { "y" } else { "ies" }
        )
    };

    if options.format.is_json() {
        let output = QueueReclaimStaleOutput {
            reclaimed,
            threshold_secs,
            message,
        };
        print_queue_envelope("queue-reclaim-stale-response", &output)?;
    } else {
        println!("{message}");
    }

    Ok(())
}

async fn handle_status_id(
    queue: &zjj_core::MergeQueue,
    queue_id: i64,
    options: &QueueOptions,
) -> Result<()> {
    let entry = queue.get_by_id(queue_id).await?;

    let Some(entry) = entry else {
        anyhow::bail!("queue entry not found: {queue_id}");
    };

    let events = queue.fetch_events(queue_id).await?;

    let entry_output = QueueEntryOutput {
        id: entry.id,
        workspace: entry.workspace,
        bead_id: entry.bead_id,
        priority: entry.priority,
        status: entry.status.as_str().to_string(),
        added_at: entry.added_at,
        started_at: entry.started_at,
        completed_at: entry.completed_at,
        error_message: entry.error_message,
        agent_id: entry.agent_id,
    };

    let events_output: Vec<QueueEventOutput> = events
        .into_iter()
        .map(|e| QueueEventOutput {
            event_id: e.id,
            event_type: e.event_type.as_str().to_string(),
            details_json: e.details_json,
            created_at: e.created_at,
        })
        .collect();

    let message = format!("Status for queue entry {}", queue_id);

    if options.format.is_json() {
        let output = QueueStatusIdOutput {
            entry: entry_output,
            events: events_output,
            message,
        };
        print_queue_envelope("queue-status-id-response", &output)?;
    } else {
        println!("Queue Entry:");
        println!("  ID: {}", entry_output.id);
        println!("  Workspace: {}", entry_output.workspace);
        println!("  Status: {}", entry_output.status);
        println!("  Priority: {}", entry_output.priority);
        if let Some(bead_id) = entry_output.bead_id {
            println!("  Bead ID: {bead_id}");
        }
        if let Some(agent_id) = entry_output.agent_id {
            println!("  Agent ID: {agent_id}");
        }
        println!(
            "\nEvents ({events_output_len} total):",
            events_output_len = events_output.len()
        );
        for event in &events_output {
            println!(
                "  [{}] {}: {}",
                event.event_type,
                event.created_at,
                event.details_json.as_deref().unwrap_or("")
            );
        }
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

//! Task command - Manage tasks and work items (beads)
//!
//! Provides subcommands for listing, showing, claiming, yielding, starting,
//! and completing tasks. Tasks are represented as beads in the beads database.
//!
//! # Subcommands
//!
//! - `list` - List all tasks
//! - `show` - Show task details
//! - `claim` - Claim a task for work (uses `LockManager`)
//! - `yield` - Release a claimed task
//! - `start` - Start work on a task (creates session)
//! - `done` - Complete a task

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use isolate_core::json::SchemaEnvelope;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    beads::{BeadMetadata, BeadRepository, BeadStatus},
    cli::handlers::json_format::extract_json_flag,
    commands::isolate_project_root,
};

// ═══════════════════════════════════════════════════════════════════════════
// TASK DOMAIN TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Task status matching beads status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is open and ready to be claimed
    Open,
    /// Task has been claimed by an agent
    Claimed,
    /// Task is in progress
    InProgress,
    /// Task is blocked
    Blocked,
    /// Task has been completed
    Completed,
    /// Task was cancelled
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Claimed => write!(f, "claimed"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl From<BeadStatus> for TaskStatus {
    fn from(status: BeadStatus) -> Self {
        match status {
            BeadStatus::Open => Self::Open,
            BeadStatus::InProgress => Self::InProgress,
            BeadStatus::Blocked => Self::Blocked,
            BeadStatus::Deferred => Self::Cancelled,
            BeadStatus::Closed => Self::Completed,
        }
    }
}

/// Task information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Unique task identifier (e.g., "bd-abc123")
    pub id: String,
    /// Task title
    pub title: String,
    /// Current status
    pub status: TaskStatus,
    /// Task description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent that claimed this task (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    /// When the task was claimed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<DateTime<Utc>>,
    /// When the claim expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_expires_at: Option<DateTime<Utc>>,
}

impl From<BeadMetadata> for TaskInfo {
    fn from(bead: BeadMetadata) -> Self {
        Self {
            id: bead.id,
            title: bead.title,
            status: bead.status.into(),
            description: bead.description,
            claimed_by: None,
            claimed_at: None,
            claim_expires_at: None,
        }
    }
}

/// Result of task list operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResult {
    /// List of tasks
    pub tasks: Vec<TaskInfo>,
    /// Total count
    pub total: usize,
}

/// Result of task claim operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClaimResult {
    /// Whether the claim succeeded
    pub claimed: bool,
    /// Task ID
    pub task_id: String,
    /// Agent that now holds the claim
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// When the claim expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Error message if claim failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of task yield operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskYieldResult {
    /// Whether the yield succeeded
    pub yielded: bool,
    /// Task ID
    pub task_id: String,
    /// Error message if yield failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Task lock information stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskLock {
    task_id: String,
    holder: String,
    claimed_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════
// TASK REPOSITORY
// ═══════════════════════════════════════════════════════════════════════════

/// Repository for task operations with locking support
pub struct TaskRepository {
    bead_repo: BeadRepository,
    data_dir: PathBuf,
}

impl TaskRepository {
    /// Create a new task repository
    pub fn new(root: &str) -> Self {
        Self {
            bead_repo: BeadRepository::new(root),
            data_dir: PathBuf::from(root).join(".isolate"),
        }
    }

    /// Get a task by ID
    pub async fn get_task(&self, id: &str) -> Result<Option<TaskInfo>> {
        let bead = self.bead_repo.get_bead(id).await?;
        Ok(bead.map(TaskInfo::from))
    }

    /// List all tasks with optional status filter
    pub async fn list_tasks(&self, status_filter: Option<&str>) -> Result<TaskListResult> {
        let beads = self.bead_repo.list_beads().await?;

        let tasks: Vec<TaskInfo> = beads
            .into_iter()
            .map(TaskInfo::from)
            .filter(|task| {
                status_filter.is_none_or(|filter| {
                    task.status.to_string().to_lowercase() == filter.to_lowercase()
                })
            })
            .collect();

        let total = tasks.len();
        Ok(TaskListResult { tasks, total })
    }

    /// Update task status
    pub async fn update_status(&self, id: &str, status: BeadStatus) -> Result<()> {
        self.bead_repo.update_status(id, status).await
    }

    /// Get task locks directory
    fn locks_dir(&self) -> PathBuf {
        self.data_dir.join("task-locks")
    }

    /// Get lock file path for a task
    fn lock_file(&self, task_id: &str) -> PathBuf {
        self.locks_dir().join(format!("{task_id}.lock"))
    }

    /// Ensure locks directory exists
    async fn ensure_locks_dir(&self) -> Result<()> {
        let dir = self.locks_dir();
        let exists = tokio::fs::try_exists(&dir).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %dir.display(), "Failed to check locks directory existence, assuming missing");
            false
        });
        if !exists {
            tokio::fs::create_dir_all(&dir).await.with_context(|| {
                format!("Failed to create task locks directory: {}", dir.display())
            })?;
        }
        Ok(())
    }

    /// Claim a task for an agent
    #[allow(clippy::too_many_lines)]
    pub async fn claim_task(
        &self,
        task_id: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> Result<TaskClaimResult> {
        self.ensure_locks_dir().await?;

        // First verify the task exists and is claimable
        let task = self.get_task(task_id).await?;
        match task {
            None => {
                return Ok(TaskClaimResult {
                    claimed: false,
                    task_id: task_id.to_string(),
                    holder: None,
                    expires_at: None,
                    error: Some(format!("Task '{task_id}' not found")),
                });
            }
            Some(t) if t.status == TaskStatus::Completed || t.status == TaskStatus::Cancelled => {
                return Ok(TaskClaimResult {
                    claimed: false,
                    task_id: task_id.to_string(),
                    holder: None,
                    expires_at: None,
                    error: Some(format!("Task '{}' is already {}", task_id, t.status)),
                });
            }
            Some(_) => {}
        }

        let lock_path = self.lock_file(task_id);
        let now = Utc::now();
        let ttl_seconds_i64 = i64::try_from(ttl_seconds).unwrap_or_else(|e| {
            tracing::warn!(error = %e, ttl_seconds, "Invalid TTL seconds, using default 300");
            300
        });
        let expires_at = now + chrono::Duration::seconds(ttl_seconds_i64);

        // Try to read existing lock
        let lock_exists = tokio::fs::try_exists(&lock_path).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %lock_path.display(), "Failed to check lock file existence, assuming missing");
            false
        });
        if lock_exists {
            let content = tokio::fs::read_to_string(&lock_path)
                .await
                .with_context(|| "Failed to read task lock file")?;
            let existing_lock: TaskLock =
                serde_json::from_str(&content).with_context(|| "Failed to parse task lock file")?;

            // Check if lock is expired
            if existing_lock.expires_at > now {
                // Lock is still valid
                if existing_lock.holder == agent_id {
                    // Same agent - extend the lock (idempotent)
                    let new_lock = TaskLock {
                        task_id: task_id.to_string(),
                        holder: agent_id.to_string(),
                        claimed_at: now,
                        expires_at,
                    };
                    let content = serde_json::to_string(&new_lock)?;
                    tokio::fs::write(&lock_path, content).await?;

                    info!(
                        task_id = %task_id,
                        agent_id = %agent_id,
                        expires_at = %expires_at.to_rfc3339(),
                        "Task claim extended (idempotent)"
                    );

                    return Ok(TaskClaimResult {
                        claimed: true,
                        task_id: task_id.to_string(),
                        holder: Some(agent_id.to_string()),
                        expires_at: Some(expires_at),
                        error: None,
                    });
                }

                // Different agent holds the lock
                let holder = existing_lock.holder.clone();
                let expires = existing_lock.expires_at;
                return Ok(TaskClaimResult {
                    claimed: false,
                    task_id: task_id.to_string(),
                    holder: Some(holder.clone()),
                    expires_at: Some(expires),
                    error: Some(format!("Task is already claimed by {holder}")),
                });
            }
            // Lock expired - will be replaced below
            warn!(
                task_id = %task_id,
                previous_holder = %existing_lock.holder,
                "Previous task lock expired, allowing new claim"
            );
        }

        // Create new lock
        let new_lock = TaskLock {
            task_id: task_id.to_string(),
            holder: agent_id.to_string(),
            claimed_at: now,
            expires_at,
        };
        let content = serde_json::to_string(&new_lock)?;
        tokio::fs::write(&lock_path, content).await?;

        // Update task status to claimed
        self.update_status(task_id, BeadStatus::InProgress).await?;

        info!(
            task_id = %task_id,
            agent_id = %agent_id,
            expires_at = %expires_at.to_rfc3339(),
            "Task claimed successfully"
        );

        Ok(TaskClaimResult {
            claimed: true,
            task_id: task_id.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: Some(expires_at),
            error: None,
        })
    }

    /// Yield a claimed task
    pub async fn yield_task(&self, task_id: &str, agent_id: &str) -> Result<TaskYieldResult> {
        let lock_path = self.lock_file(task_id);

        // Check if lock exists
        if !tokio::fs::try_exists(&lock_path).await.unwrap_or(false) {
            // Idempotent - already unlocked
            return Ok(TaskYieldResult {
                yielded: true,
                task_id: task_id.to_string(),
                error: None,
            });
        }

        // Read and verify lock
        let content = tokio::fs::read_to_string(&lock_path)
            .await
            .with_context(|| "Failed to read task lock file")?;
        let existing_lock: TaskLock =
            serde_json::from_str(&content).with_context(|| "Failed to parse task lock file")?;

        if existing_lock.holder != agent_id {
            return Ok(TaskYieldResult {
                yielded: false,
                task_id: task_id.to_string(),
                error: Some(format!(
                    "Task is claimed by {}, not you",
                    existing_lock.holder
                )),
            });
        }

        // Remove lock
        tokio::fs::remove_file(&lock_path).await?;

        // Update task status back to open
        self.update_status(task_id, BeadStatus::Open).await?;

        info!(
            task_id = %task_id,
            agent_id = %agent_id,
            "Task yielded successfully"
        );

        Ok(TaskYieldResult {
            yielded: true,
            task_id: task_id.to_string(),
            error: None,
        })
    }

    /// Complete a task
    pub async fn complete_task(&self, task_id: &str, agent_id: &str) -> Result<TaskInfo> {
        // Remove any existing lock
        let lock_path = self.lock_file(task_id);
        if tokio::fs::try_exists(&lock_path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&lock_path).await?;
            if let Ok(existing_lock) = serde_json::from_str::<TaskLock>(&content) {
                if existing_lock.holder != agent_id {
                    anyhow::bail!("Task is claimed by {}, not you", existing_lock.holder);
                }
            }
            let _ = tokio::fs::remove_file(&lock_path).await;
        }

        // Update status to closed
        self.update_status(task_id, BeadStatus::Closed).await?;

        info!(
            task_id = %task_id,
            agent_id = %agent_id,
            "Task completed successfully"
        );

        // Return updated task
        self.get_task(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task disappeared after completion"))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CLI HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/// Get the current agent ID from environment
fn get_agent_id() -> String {
    std::env::var("Isolate_AGENT_ID").unwrap_or_else(|_| format!("agent-{}", std::process::id()))
}

/// Handle task list subcommand
pub async fn handle_task_list(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let status_filter = args.get_one::<String>("state").map(String::as_str);
    let include_all = args.get_flag("all");

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);
    let result = repo
        .list_tasks(if include_all { None } else { status_filter })
        .await?;

    if format.is_json() {
        let envelope = SchemaEnvelope::new("task-list-response", "array", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        if result.tasks.is_empty() {
            println!("No tasks found.");
            return Ok(());
        }

        println!("Tasks ({} total):", result.total);
        println!();
        for task in &result.tasks {
            let status_icon = match task.status {
                TaskStatus::Open => "[ ]",
                TaskStatus::Claimed => "[>]",
                TaskStatus::InProgress => "[*]",
                TaskStatus::Blocked => "[!]",
                TaskStatus::Completed => "[x]",
                TaskStatus::Cancelled => "[-]",
            };
            println!("  {} {} - {}", status_icon, task.id, task.title);
            if let Some(ref desc) = task.description {
                let truncated = if desc.len() > 60 {
                    format!("{}...", &desc[..57])
                } else {
                    desc.clone()
                };
                println!("      {truncated}");
            }
        }
    }

    Ok(())
}

/// Handle task show subcommand
pub async fn handle_task_show(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let task_id = args
        .get_one::<String>("id")
        .context("Task ID is required")?;

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);
    let task = repo.get_task(task_id).await?;

    match task {
        None => {
            if format.is_json() {
                let error_result = serde_json::json!({
                    "error": format!("Task '{}' not found", task_id),
                    "task_id": task_id,
                });
                let envelope = SchemaEnvelope::new("task-show-response", "single", &error_result);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            }
            anyhow::bail!("Task '{task_id}' not found");
        }
        Some(t) => {
            if format.is_json() {
                let envelope = SchemaEnvelope::new("task-show-response", "single", &t);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                println!("Task: {}", t.id);
                println!("  Title: {}", t.title);
                println!("  Status: {}", t.status);
                if let Some(ref desc) = t.description {
                    println!("  Description: {desc}");
                }
                if let Some(ref holder) = t.claimed_by {
                    println!("  Claimed by: {holder}");
                }
            }
        }
    }

    Ok(())
}

/// Handle task claim subcommand
pub async fn handle_task_claim(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let task_id = args
        .get_one::<String>("id")
        .context("Task ID is required")?;

    let agent_id = get_agent_id();
    let ttl_seconds: u64 = std::env::var("Isolate_TASK_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300); // 5 minutes default

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);
    let result = repo.claim_task(task_id, &agent_id, ttl_seconds).await?;

    if format.is_json() {
        let envelope = SchemaEnvelope::new("task-claim-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.claimed {
        println!("Claimed task '{task_id}' (expires in {ttl_seconds}s)");
    } else {
        eprintln!("Failed to claim task '{task_id}'");
        if let Some(ref error) = result.error {
            eprintln!("  Error: {error}");
        }
        anyhow::bail!("Claim failed");
    }

    Ok(())
}

/// Handle task yield subcommand
pub async fn handle_task_yield(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let task_id = args
        .get_one::<String>("id")
        .context("Task ID is required")?;

    let agent_id = get_agent_id();

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);
    let result = repo.yield_task(task_id, &agent_id).await?;

    if format.is_json() {
        let envelope = SchemaEnvelope::new("task-yield-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.yielded {
        println!("Yielded task '{task_id}'");
    } else {
        eprintln!("Failed to yield task '{task_id}'");
        if let Some(ref error) = result.error {
            eprintln!("  Error: {error}");
        }
        anyhow::bail!("Yield failed");
    }

    Ok(())
}

/// Handle task start subcommand (creates session)
pub async fn handle_task_start(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let task_id = args
        .get_one::<String>("id")
        .context("Task ID is required")?;

    let agent_id = get_agent_id();

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);

    // First claim the task
    let claim_result = repo.claim_task(task_id, &agent_id, 3600).await?; // 1 hour TTL
    if !claim_result.claimed {
        if format.is_json() {
            let envelope = SchemaEnvelope::new("task-start-response", "single", &claim_result);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            eprintln!(
                "Cannot start task: {}",
                claim_result.error.as_deref().unwrap_or("unknown error")
            );
        }
        anyhow::bail!("Failed to claim task for start");
    }

    // Now delegate to spawn to create the workspace
    // The spawn command already handles workspace creation for beads
    info!(
        task_id = %task_id,
        "Starting task workspace"
    );

    let result = serde_json::json!({
        "task_id": task_id,
        "status": "started",
        "workspace": format!(".isolate/workspaces/{}", task_id),
    });

    if format.is_json() {
        let envelope = SchemaEnvelope::new("task-start-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Started task '{task_id}'");
        println!("  Workspace: .isolate/workspaces/{task_id}");
        println!();
        println!("Run 'isolate spawn {task_id}' to begin work");
    }

    Ok(())
}

/// Handle task done subcommand
pub async fn handle_task_done(args: &clap::ArgMatches) -> Result<()> {
    let format = extract_json_flag(args);
    let task_id = args.get_one::<String>("id");

    let agent_id = get_agent_id();

    let root = isolate_project_root()
        .await
        .context("Failed to get project root")?;
    let root_str = root.to_string_lossy();

    let repo = TaskRepository::new(&root_str);

    // Get task ID from args or current session
    let task_id = match task_id {
        Some(id) => id.clone(),
        None => {
            // Try to get from environment
            std::env::var("Isolate_BEAD_ID")
                .context("No task ID provided and not in a workspace (Isolate_BEAD_ID not set)")?
        }
    };

    let result = repo.complete_task(&task_id, &agent_id).await?;

    if format.is_json() {
        let envelope = SchemaEnvelope::new("task-done-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Completed task '{task_id}'");
        println!("  Title: {}", result.title);
        println!("  Status: {}", result.status);
    }

    Ok(())
}

/// Main task command dispatcher
pub async fn handle_task(args: &clap::ArgMatches) -> Result<()> {
    match args.subcommand() {
        Some(("list", sub_args)) => handle_task_list(sub_args).await,
        Some(("show", sub_args)) => handle_task_show(sub_args).await,
        Some(("claim", sub_args)) => handle_task_claim(sub_args).await,
        Some(("yield", sub_args)) => handle_task_yield(sub_args).await,
        Some(("start", sub_args)) => handle_task_start(sub_args).await,
        Some(("done", sub_args)) => handle_task_done(sub_args).await,
        _ => {
            // No subcommand - show help
            println!("Task management commands:");
            println!();
            println!("  isolate task list [--all] [--state <STATE>]  List tasks");
            println!("  isolate task show <ID>                        Show task details");
            println!("  isolate task claim <ID>                       Claim a task");
            println!("  isolate task yield <ID>                       Release a claimed task");
            println!("  isolate task start <ID>                       Start work on a task");
            println!("  isolate task done [ID]                        Complete a task");
            println!();
            println!("Run 'isolate task <command> --help' for more information.");
            Ok(())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Open.to_string(), "open");
        assert_eq!(TaskStatus::Claimed.to_string(), "claimed");
        assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
        assert_eq!(TaskStatus::Blocked.to_string(), "blocked");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_task_status_from_bead_status() {
        assert_eq!(TaskStatus::from(BeadStatus::Open), TaskStatus::Open);
        assert_eq!(
            TaskStatus::from(BeadStatus::InProgress),
            TaskStatus::InProgress
        );
        assert_eq!(TaskStatus::from(BeadStatus::Blocked), TaskStatus::Blocked);
        assert_eq!(
            TaskStatus::from(BeadStatus::Deferred),
            TaskStatus::Cancelled
        );
        assert_eq!(TaskStatus::from(BeadStatus::Closed), TaskStatus::Completed);
    }

    #[test]
    fn test_task_info_serialization() {
        let task = TaskInfo {
            id: "bd-test123".to_string(),
            title: "Test task".to_string(),
            status: TaskStatus::Open,
            description: Some("A test task".to_string()),
            claimed_by: None,
            claimed_at: None,
            claim_expires_at: None,
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"id\":\"bd-test123\""));
        assert!(json.contains("\"status\":\"open\""));
    }

    #[test]
    fn test_task_claim_result_serialization() {
        let result = TaskClaimResult {
            claimed: true,
            task_id: "bd-test".to_string(),
            holder: Some("agent-1".to_string()),
            expires_at: Some(Utc::now()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"claimed\":true"));
    }

    #[test]
    fn test_task_yield_result_serialization() {
        let result = TaskYieldResult {
            yielded: true,
            task_id: "bd-test".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"yielded\":true"));
    }

    #[test]
    fn test_task_list_result_serialization() {
        let result = TaskListResult {
            tasks: vec![TaskInfo {
                id: "bd-1".to_string(),
                title: "Task 1".to_string(),
                status: TaskStatus::Open,
                description: None,
                claimed_by: None,
                claimed_at: None,
                claim_expires_at: None,
            }],
            total: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"tasks\""));
    }
}

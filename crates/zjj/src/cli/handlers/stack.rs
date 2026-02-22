//! Stack command handlers for querying stack relationships.

use anyhow::Result;
use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use super::json_format::get_format;
use crate::commands::get_queue_db_path;

/// Response payload for stack status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackStatusPayload {
    pub workspace: String,
    pub in_queue: bool,
    pub depth: i32,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub root: Option<String>,
    pub message: String,
}

/// Custom envelope for stack status that uses explicit "payload" field
/// to match the expected test schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackStatusEnvelope {
    #[serde(rename = "$schema")]
    pub schema: String,
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    pub payload: StackStatusPayload,
}

impl StackStatusEnvelope {
    pub fn new(payload: StackStatusPayload) -> Self {
        Self {
            schema: "zjj://stack-status-response/v1".to_string(),
            schema_version: "1.0".to_string(),
            schema_type: "single".to_string(),
            success: None,
            payload,
        }
    }

    pub const fn with_error(mut self) -> Self {
        self.success = Some(false);
        self
    }
}

/// Handle the stack command with subcommands
pub async fn handle_stack(sub_m: &ArgMatches) -> Result<()> {
    if let Some((subcommand_name, subcommand_matches)) = sub_m.subcommand() {
        return match subcommand_name {
            "status" => handle_stack_status(subcommand_matches).await,
            _ => Err(anyhow::anyhow!(
                "Unknown stack subcommand: {}",
                subcommand_name
            )),
        };
    }

    Err(anyhow::anyhow!(
        "Stack command requires a subcommand. Use 'zjj stack status <workspace>'"
    ))
}

/// Handle the stack status subcommand
pub async fn handle_stack_status(sub_m: &ArgMatches) -> Result<()> {
    let workspace = sub_m
        .get_one::<String>("workspace")
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("workspace argument is required"))?;

    let format = get_format(sub_m);
    let is_json = format.is_json();

    let queue = get_queue().await?;

    // Get the entry for this workspace
    let entry = queue.get_by_workspace(workspace).await?;

    // Check if workspace is in queue
    if entry.is_none() {
        // Workspace not in queue
        if is_json {
            let payload = StackStatusPayload {
                workspace: workspace.to_string(),
                in_queue: false,
                depth: 0,
                parent: None,
                children: vec![],
                root: None,
                message: format!("Workspace '{workspace}' is not in queue"),
            };
            let envelope = StackStatusEnvelope::new(payload).with_error();
            print_json_envelope(&envelope)?;
        } else {
            println!("Workspace '{workspace}' is not in queue");
        }
        // Exit with code 2 (not found) via CommandExit
        return Err(super::CommandExit::new(2).into());
    }

    // Workspace is in queue, compute stack info
    let all_entries = queue.list(None).await?;

    // Calculate depth
    // SAFETY: Stack depth is bounded by reasonable limits, conversion is safe
    #[allow(clippy::cast_possible_wrap)]
    let depth = zjj_core::coordination::calculate_stack_depth(workspace, &all_entries)
        .map(|d| d as i32)
        .map_err(|e| anyhow::anyhow!("Failed to calculate stack depth: {e}"))?;

    // Get parent from the entry
    let entry = queue.get_by_workspace(workspace).await?;
    let entry = entry.ok_or_else(|| anyhow::anyhow!("Entry disappeared during query"))?;
    let parent = entry.parent_workspace.clone();

    // Get children
    let children_entries = queue.get_children(workspace).await?;
    let children: Vec<String> = children_entries.into_iter().map(|e| e.workspace).collect();

    // Find root
    let root = zjj_core::coordination::find_stack_root(workspace, &all_entries)
        .ok()
        .or_else(|| {
            // If the workspace has no parent, it's its own root
            if parent.is_none() {
                Some(workspace.to_string())
            } else {
                None
            }
        });

    if is_json {
        let payload = StackStatusPayload {
            workspace: workspace.to_string(),
            in_queue: true,
            depth,
            parent,
            children,
            root,
            message: format!("Workspace '{workspace}' stack status"),
        };
        let envelope = StackStatusEnvelope::new(payload);
        print_json_envelope(&envelope)?;
    } else {
        print_human_readable(workspace, depth, parent.as_ref(), &children, root.as_ref());
    }

    Ok(())
}

/// Get or create the merge queue database
async fn get_queue() -> Result<zjj_core::MergeQueue> {
    let queue_db = get_queue_db_path().await?;
    zjj_core::MergeQueue::open(&queue_db)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open merge queue database: {e}"))
}

/// Print JSON envelope to stdout
fn print_json_envelope(envelope: &StackStatusEnvelope) -> Result<()> {
    let json_str = serde_json::to_string_pretty(envelope)
        .map_err(|e| anyhow::anyhow!("Failed to serialize response: {e}"))?;
    println!("{json_str}");
    Ok(())
}

/// Print human-readable stack status
fn print_human_readable(
    workspace: &str,
    depth: i32,
    parent: Option<&String>,
    children: &[String],
    root: Option<&String>,
) {
    println!("Stack status for '{workspace}':");
    println!("  Depth: {depth}");

    match parent {
        Some(p) => println!("  Parent: {p}"),
        None => println!("  Parent: (none - root)"),
    }

    if children.is_empty() {
        println!("  Children: (none)");
    } else {
        println!("  Children:");
        for child in children {
            println!("    - {child}");
        }
    }

    match root {
        Some(r) => println!("  Root: {r}"),
        None => println!("  Root: (unknown)"),
    }
}

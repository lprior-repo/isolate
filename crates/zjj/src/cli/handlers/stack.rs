//! Stack command handlers for querying stack relationships.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use clap::ArgMatches;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use zjj_core::output::{emit_stdout, OutputLine, Stack as OutputStack, StackEntryStatus};

use super::json_format::get_format;
use crate::commands::{add, get_queue_db_path};

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

/// Response payload for stack create command (bd-2kj)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackCreatePayload {
    pub name: String,
    pub parent: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub status: String,
    pub created: bool,
}

/// Envelope for stack create response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackCreateEnvelope {
    #[serde(rename = "$schema")]
    pub schema: String,
    #[serde(rename = "_schema_version")]
    pub schema_version: String,
    pub schema_type: String,
    pub success: bool,
    pub payload: StackCreatePayload,
}

impl StackCreateEnvelope {
    pub fn new(payload: StackCreatePayload) -> Self {
        Self {
            schema: "zjj://stack-create-response/v1".to_string(),
            schema_version: "1.0".to_string(),
            schema_type: "single".to_string(),
            success: true,
            payload,
        }
    }
}

/// A stack tree node for building hierarchical output
#[derive(Debug, Clone)]
struct StackNode {
    workspace: String,
    status: zjj_core::coordination::QueueStatus,
    bead_id: Option<String>,
    children: Vec<StackNode>,
}

/// Handle the stack command with subcommands
pub async fn handle_stack(sub_m: &ArgMatches) -> Result<()> {
    if let Some((subcommand_name, subcommand_matches)) = sub_m.subcommand() {
        return match subcommand_name {
            "status" => handle_stack_status(subcommand_matches).await,
            "create" => handle_stack_create(subcommand_matches).await,
            "list" | "ls" => handle_stack_list(subcommand_matches).await,
            _ => Err(anyhow::anyhow!(
                "Unknown stack subcommand: {}",
                subcommand_name
            )),
        };
    }

    Err(anyhow::anyhow!(
        "Stack command requires a subcommand. Use 'zjj stack status <workspace>', 'zjj stack create <name> --parent <parent>', or 'zjj stack list'"
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

/// Handle the stack create subcommand (bd-2kj)
///
/// Creates a new session that is a child of an existing parent session.
/// This wraps the `add --parent` functionality with stack-specific semantics.
pub async fn handle_stack_create(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("name argument is required"))?;

    let parent = sub_m
        .get_one::<String>("parent")
        .ok_or_else(|| anyhow::anyhow!("--parent is required for stack create"))?;

    let format = get_format(sub_m);

    // Build AddOptions with parent relationship
    let options = add::AddOptions {
        name: name.clone(),
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        parent: Some(parent.clone()),
        no_hooks: sub_m.get_flag("no-hooks"),
        template: sub_m.get_one::<String>("template").cloned(),
        no_open: sub_m.get_flag("no-zellij"),
        no_zellij: sub_m.get_flag("no-zellij"),
        format,
        idempotent: false,
        dry_run: false,
    };

    // Call add command to create the session with parent relationship
    // This will create the session with parent_session set in the database
    add::run_with_options(&options).await
}

/// Handle the stack list subcommand
pub async fn handle_stack_list(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let is_json = format.is_json();

    let queue = get_queue().await?;

    // Get all entries from the queue
    let all_entries = queue.list(None).await?;

    if all_entries.is_empty() {
        if is_json {
            // Emit empty summary
            let summary = zjj_core::output::Summary::new(
                zjj_core::output::SummaryType::Info,
                "No workspaces in queue".to_string(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create summary: {e}"))?;
            let line = OutputLine::Summary(summary);
            emit_stdout(&line).map_err(|e| anyhow::anyhow!("Failed to emit output: {e}"))?;
        } else {
            println!("No workspaces in queue.");
        }
        return Ok(());
    }

    // Build stack trees
    let trees = build_stack_trees(&all_entries);

    if is_json {
        // Emit each stack tree as a JSONL Stack line
        for tree in &trees {
            emit_stack_tree(&tree)?;
        }

        // Emit context summary as the last line
        emit_context_summary(&trees)?;
    } else {
        // Print human-readable tree format
        print_stack_trees_human(&trees);
    }

    Ok(())
}

/// Build stack trees from flat list of entries.
///
/// Groups workspaces by their parent relationship and builds
/// tree structures with roots at the top.
fn build_stack_trees(entries: &[zjj_core::coordination::QueueEntry]) -> Vec<StackNode> {
    // Build a map of workspace -> children
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut entry_map: HashMap<String, &zjj_core::coordination::QueueEntry> = HashMap::new();
    let mut roots: Vec<String> = Vec::new();

    for entry in entries {
        entry_map.insert(entry.workspace.clone(), entry);

        match &entry.parent_workspace {
            Some(parent) => {
                children_map
                    .entry(parent.clone())
                    .or_default()
                    .push(entry.workspace.clone());
            }
            None => {
                roots.push(entry.workspace.clone());
            }
        }
    }

    // Build tree nodes recursively for each root
    roots
        .into_iter()
        .sorted()
        .filter_map(|root| build_tree_node(&root, &children_map, &entry_map))
        .collect()
}

/// Recursively build a tree node from the workspace name.
fn build_tree_node(
    workspace: &str,
    children_map: &HashMap<String, Vec<String>>,
    entry_map: &HashMap<String, &zjj_core::coordination::QueueEntry>,
) -> Option<StackNode> {
    let entry = entry_map.get(workspace)?;

    // Get children, sorted alphabetically
    let children: Vec<StackNode> = children_map
        .get(workspace)
        .map(|child_names| {
            child_names
                .iter()
                .sorted()
                .filter_map(|name| build_tree_node(name, children_map, entry_map))
                .collect()
        })
        .unwrap_or_default();

    Some(StackNode {
        workspace: workspace.to_string(),
        status: entry.status,
        bead_id: entry.bead_id.clone(),
        children,
    })
}

/// Emit a stack tree as JSONL output.
fn emit_stack_tree(tree: &StackNode) -> Result<()> {
    let output_stack = stack_node_to_output_stack(tree)?;

    let line = OutputLine::Stack(output_stack);
    emit_stdout(&line).map_err(|e| anyhow::anyhow!("Failed to emit stack output: {e}"))
}

/// Convert a StackNode to the output Stack type.
fn stack_node_to_output_stack(node: &StackNode) -> Result<OutputStack> {
    let mut stack = OutputStack::new(node.workspace.clone(), "main".to_string())
        .map_err(|e| anyhow::anyhow!("Failed to create stack: {e}"))?;

    // Add root entry
    let root_status = queue_status_to_stack_status(node.status);
    stack = stack
        .with_entry(
            node.workspace.clone(),
            PathBuf::from(&node.workspace),
            root_status,
            node.bead_id.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to add stack entry: {e}"))?;

    // Add children recursively
    for child in &node.children {
        stack = add_children_to_stack(stack, child, 1)?;
    }

    Ok(stack)
}

/// Recursively add children to the stack.
fn add_children_to_stack(stack: OutputStack, node: &StackNode, _depth: u32) -> Result<OutputStack> {
    let status = queue_status_to_stack_status(node.status);
    let mut current_stack = stack
        .with_entry(
            node.workspace.clone(),
            PathBuf::from(&node.workspace),
            status,
            node.bead_id.clone(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to add stack entry: {e}"))?;

    for child in &node.children {
        current_stack = add_children_to_stack(current_stack, child, _depth + 1)?;
    }

    Ok(current_stack)
}

/// Convert QueueStatus to StackEntryStatus.
fn queue_status_to_stack_status(status: zjj_core::coordination::QueueStatus) -> StackEntryStatus {
    use zjj_core::coordination::QueueStatus;

    match status {
        QueueStatus::Pending => StackEntryStatus::Pending,
        QueueStatus::Claimed => StackEntryStatus::Pending,
        QueueStatus::Rebasing => StackEntryStatus::Pending,
        QueueStatus::Testing => StackEntryStatus::Ready,
        QueueStatus::ReadyToMerge => StackEntryStatus::Ready,
        QueueStatus::Merging => StackEntryStatus::Merging,
        QueueStatus::Merged => StackEntryStatus::Merged,
        QueueStatus::FailedRetryable => StackEntryStatus::Failed,
        QueueStatus::FailedTerminal => StackEntryStatus::Failed,
        QueueStatus::Cancelled => StackEntryStatus::Failed,
    }
}

/// Emit a context summary line.
fn emit_context_summary(trees: &[StackNode]) -> Result<()> {
    let total_workspaces: usize = trees.iter().map(count_nodes).sum();
    let total_stacks = trees.len();

    let message = format!("Found {total_stacks} stack(s) with {total_workspaces} workspace(s)");

    let summary = zjj_core::output::Summary::new(zjj_core::output::SummaryType::Status, message)
        .map_err(|e| anyhow::anyhow!("Failed to create summary: {e}"))?;

    let line = OutputLine::Summary(summary);
    emit_stdout(&line).map_err(|e| anyhow::anyhow!("Failed to emit summary: {e}"))
}

/// Count total nodes in a tree.
fn count_nodes(node: &StackNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

/// Print stack trees in human-readable format.
fn print_stack_trees_human(trees: &[StackNode]) {
    if trees.is_empty() {
        println!("No stacks found.");
        return;
    }

    let total_workspaces: usize = trees.iter().map(count_nodes).sum();
    let total_stacks = trees.len();

    println!("Stacks ({total_stacks} stacks, {total_workspaces} workspaces):");
    println!();

    for tree in trees {
        print_tree_node(tree, 0);
        println!();
    }
}

/// Print a single tree node with indentation.
fn print_tree_node(node: &StackNode, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let status_str = format_status(node.status);
    let bead_str = node
        .bead_id
        .as_ref()
        .map(|b| format!(" [{b}]"))
        .unwrap_or_default();

    if indent == 0 {
        println!(
            "{}{} {}{}",
            indent_str, node.workspace, status_str, bead_str
        );
    } else {
        println!(
            "{}|- {} {}{}",
            indent_str, node.workspace, status_str, bead_str
        );
    }

    for child in &node.children {
        print_tree_node(child, indent + 1);
    }
}

/// Format queue status for human-readable output.
fn format_status(status: zjj_core::coordination::QueueStatus) -> String {
    use zjj_core::coordination::QueueStatus;

    match status {
        QueueStatus::Pending => "[pending]".to_string(),
        QueueStatus::Claimed => "[claimed]".to_string(),
        QueueStatus::Rebasing => "[rebasing]".to_string(),
        QueueStatus::Testing => "[testing]".to_string(),
        QueueStatus::ReadyToMerge => "[ready]".to_string(),
        QueueStatus::Merging => "[merging]".to_string(),
        QueueStatus::Merged => "[merged]".to_string(),
        QueueStatus::FailedRetryable => "[failed-retryable]".to_string(),
        QueueStatus::FailedTerminal => "[failed]".to_string(),
        QueueStatus::Cancelled => "[cancelled]".to_string(),
    }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use zjj_core::coordination::{
        queue_entities::Dependents, queue_status::StackMergeState, QueueEntry, QueueStatus,
        WorkspaceQueueState,
    };

    use super::*;

    fn create_test_entry(
        workspace: &str,
        parent_workspace: Option<&str>,
        status: QueueStatus,
        bead_id: Option<&str>,
    ) -> QueueEntry {
        QueueEntry {
            id: 1,
            workspace: workspace.to_string(),
            bead_id: bead_id.map(std::string::ToString::to_string),
            priority: 0,
            status,
            added_at: 1_700_000_000,
            started_at: None,
            completed_at: None,
            error_message: None,
            agent_id: None,
            dedupe_key: None,
            workspace_state: WorkspaceQueueState::Created,
            previous_state: None,
            state_changed_at: None,
            head_sha: None,
            tested_against_sha: None,
            attempt_count: 0,
            max_attempts: 3,
            rebase_count: 0,
            last_rebase_at: None,
            parent_workspace: parent_workspace.map(std::string::ToString::to_string),
            stack_depth: 0,
            dependents: Dependents::new(),
            stack_root: None,
            stack_merge_state: StackMergeState::Independent,
        }
    }

    #[test]
    fn test_build_stack_trees_empty() {
        let entries: Vec<QueueEntry> = vec![];
        let trees = build_stack_trees(&entries);
        assert!(trees.is_empty());
    }

    #[test]
    fn test_build_stack_trees_single_root() {
        let entries = vec![create_test_entry("root", None, QueueStatus::Pending, None)];
        let trees = build_stack_trees(&entries);

        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].workspace, "root");
        assert!(trees[0].children.is_empty());
    }

    #[test]
    fn test_build_stack_trees_parent_child() {
        let entries = vec![
            create_test_entry("parent", None, QueueStatus::Pending, None),
            create_test_entry("child", Some("parent"), QueueStatus::Pending, None),
        ];
        let trees = build_stack_trees(&entries);

        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].workspace, "parent");
        assert_eq!(trees[0].children.len(), 1);
        assert_eq!(trees[0].children[0].workspace, "child");
    }

    #[test]
    fn test_build_stack_trees_multi_level() {
        let entries = vec![
            create_test_entry("root", None, QueueStatus::Pending, None),
            create_test_entry("child1", Some("root"), QueueStatus::Pending, None),
            create_test_entry("child2", Some("root"), QueueStatus::Pending, None),
            create_test_entry("grandchild", Some("child1"), QueueStatus::Pending, None),
        ];
        let trees = build_stack_trees(&entries);

        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].workspace, "root");
        assert_eq!(trees[0].children.len(), 2);
        assert_eq!(trees[0].children[0].workspace, "child1");
        assert_eq!(trees[0].children[0].children.len(), 1);
        assert_eq!(trees[0].children[0].children[0].workspace, "grandchild");
    }

    #[test]
    fn test_build_stack_trees_multiple_roots() {
        let entries = vec![
            create_test_entry("root-a", None, QueueStatus::Pending, None),
            create_test_entry("root-b", None, QueueStatus::Pending, None),
            create_test_entry("child-a", Some("root-a"), QueueStatus::Pending, None),
        ];
        let trees = build_stack_trees(&entries);

        assert_eq!(trees.len(), 2);
        // Roots should be sorted alphabetically
        assert_eq!(trees[0].workspace, "root-a");
        assert_eq!(trees[1].workspace, "root-b");
    }

    #[test]
    fn test_build_stack_trees_with_bead_id() {
        let entries = vec![create_test_entry(
            "root",
            None,
            QueueStatus::Pending,
            Some("bd-123"),
        )];
        let trees = build_stack_trees(&entries);

        assert_eq!(trees[0].bead_id, Some("bd-123".to_string()));
    }

    #[test]
    fn test_count_nodes_single() {
        let node = StackNode {
            workspace: "root".to_string(),
            status: QueueStatus::Pending,
            bead_id: None,
            children: vec![],
        };
        assert_eq!(count_nodes(&node), 1);
    }

    #[test]
    fn test_count_nodes_with_children() {
        let node = StackNode {
            workspace: "root".to_string(),
            status: QueueStatus::Pending,
            bead_id: None,
            children: vec![
                StackNode {
                    workspace: "child1".to_string(),
                    status: QueueStatus::Pending,
                    bead_id: None,
                    children: vec![],
                },
                StackNode {
                    workspace: "child2".to_string(),
                    status: QueueStatus::Pending,
                    bead_id: None,
                    children: vec![],
                },
            ],
        };
        assert_eq!(count_nodes(&node), 3);
    }

    #[test]
    fn test_queue_status_to_stack_status() {
        assert_eq!(
            queue_status_to_stack_status(QueueStatus::Pending),
            StackEntryStatus::Pending
        );
        assert_eq!(
            queue_status_to_stack_status(QueueStatus::ReadyToMerge),
            StackEntryStatus::Ready
        );
        assert_eq!(
            queue_status_to_stack_status(QueueStatus::Merging),
            StackEntryStatus::Merging
        );
        assert_eq!(
            queue_status_to_stack_status(QueueStatus::Merged),
            StackEntryStatus::Merged
        );
        assert_eq!(
            queue_status_to_stack_status(QueueStatus::FailedTerminal),
            StackEntryStatus::Failed
        );
    }

    #[test]
    fn test_stack_node_to_output_stack() {
        let node = StackNode {
            workspace: "root".to_string(),
            status: QueueStatus::Pending,
            bead_id: Some("bd-123".to_string()),
            children: vec![StackNode {
                workspace: "child".to_string(),
                status: QueueStatus::ReadyToMerge,
                bead_id: None,
                children: vec![],
            }],
        };

        let result = stack_node_to_output_stack(&node);
        assert!(result.is_ok());

        let stack = result.unwrap();
        assert_eq!(stack.name, "root");
        assert_eq!(stack.entries.len(), 2);
        assert_eq!(stack.entries[0].session, "root");
        assert_eq!(stack.entries[0].bead, Some("bd-123".to_string()));
        assert_eq!(stack.entries[1].session, "child");
    }

    #[test]
    fn test_format_status() {
        assert_eq!(format_status(QueueStatus::Pending), "[pending]");
        assert_eq!(format_status(QueueStatus::Merged), "[merged]");
        assert_eq!(format_status(QueueStatus::FailedTerminal), "[failed]");
    }

    // Tests for bd-2kj: stack create command
    mod stack_create_tests {
        use super::*;

        #[test]
        fn test_stack_create_payload_serialization() {
            let payload = StackCreatePayload {
                name: "feature-auth".to_string(),
                parent: "main".to_string(),
                workspace_path: "/path/to/workspace".to_string(),
                zellij_tab: "zjj:feature-auth".to_string(),
                status: "active".to_string(),
                created: true,
            };

            let json = serde_json::to_string(&payload);
            assert!(json.is_ok());

            let parsed: Result<StackCreatePayload, _> = serde_json::from_str(&json.unwrap());
            assert!(parsed.is_ok());
            let parsed = parsed.unwrap();
            assert_eq!(parsed.name, "feature-auth");
            assert_eq!(parsed.parent, "main");
        }

        #[test]
        fn test_stack_create_envelope_serialization() {
            let payload = StackCreatePayload {
                name: "test".to_string(),
                parent: "parent".to_string(),
                workspace_path: "/path".to_string(),
                zellij_tab: "zjj:test".to_string(),
                status: "active".to_string(),
                created: true,
            };

            let envelope = StackCreateEnvelope::new(payload);
            assert!(envelope.success);

            let json = serde_json::to_string(&envelope);
            assert!(json.is_ok());

            let parsed: Result<StackCreateEnvelope, _> = serde_json::from_str(&json.unwrap());
            assert!(parsed.is_ok());
            let parsed = parsed.unwrap();
            assert!(parsed.success);
            assert_eq!(parsed.schema, "zjj://stack-create-response/v1");
        }
    }
}

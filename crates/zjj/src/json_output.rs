//! JSON output structures for zjj commands

use serde::Serialize;

/// Init command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InitOutput {
    pub success: bool,
    pub message: String,
    pub zjj_dir: String,
    pub config_file: String,
    pub state_db: String,
    pub layouts_dir: String,
}

/// Add command JSON output
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct AddOutput {
    pub success: bool,
    pub session_name: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

/// Remove command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOutput {
    pub success: bool,
    pub session_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<RemoveOperation>>,
    /// Bead ID that was closed via --close-bead flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_bead: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

/// Individual operation performed during removal
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOperation {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab: Option<String>,
}

/// Remove command dry-run JSON output
#[derive(Debug, Serialize)]
pub struct RemoveDryRunOutput<'a> {
    pub success: bool,
    pub dry_run: bool,
    pub plan: &'a RemoveDryRunPlan,
}

/// Planned operations for remove dry-run
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
pub struct RemoveDryRunPlan {
    pub session_name: String,
    pub session_id: i64,
    pub workspace_path: String,
    pub workspace_exists: bool,
    pub zellij_tab: String,
    pub inside_zellij: bool,
    pub would_run_hooks: bool,
    pub would_merge: bool,
    pub planned_operations: Vec<PlannedRemoveOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// A single planned removal operation
#[derive(Debug, Clone, Serialize)]
pub struct PlannedRemoveOperation {
    pub order: u32,
    pub action: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub reversible: bool,
}

/// Focus command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FocusOutput {
    pub success: bool,
    pub session_name: String,
    pub tab: String,
    pub switched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

/// Sync command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncOutput {
    pub success: bool,
    pub session_name: Option<String>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub errors: Vec<SyncError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rebased_commits: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<usize>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub session_name: String,
    pub error: String,
}

/// Sync command dry-run JSON output
#[derive(Debug, Serialize)]
pub struct SyncDryRunOutput<'a> {
    pub success: bool,
    pub dry_run: bool,
    pub plan: &'a SyncDryRunPlan,
}

/// Planned operations for sync dry-run
#[derive(Debug, Serialize)]
pub struct SyncDryRunPlan {
    /// Session name (if syncing specific session)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    /// All sessions that would be synced
    pub sessions_to_sync: Vec<SyncSessionPlan>,
    /// Target branch for rebase
    pub target_branch: String,
    /// How target branch was determined
    pub target_branch_source: String,
    /// Total sessions that would be synced
    pub total_count: usize,
    /// Estimated operations per session
    pub operations_per_session: Vec<String>,
}

/// Plan for syncing a single session
#[derive(Debug, Clone, Serialize)]
pub struct SyncSessionPlan {
    pub name: String,
    pub workspace_path: String,
    pub workspace_exists: bool,
    pub status: String,
    pub can_sync: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

/// Diff command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    pub session_name: String,
    pub base: String,
    pub head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<FileDiffStat>,
}

#[derive(Debug, Serialize)]
pub struct FileDiffStat {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
    pub status: String,
}

/// Config command JSON output - for viewing all config
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConfigViewAllOutput {
    pub success: bool,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Config command JSON output - for getting a specific key
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConfigGetOutput {
    pub success: bool,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Config command JSON output - for setting a value
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ConfigSetOutput {
    pub success: bool,
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// Re-export error types from core for convenience
pub use zjj_core::json::{ErrorDetail, JsonError as ErrorOutput};

// ═══════════════════════════════════════════════════════════════════════════
// HELP JSON OUTPUT (zjj-g80p)
// ═══════════════════════════════════════════════════════════════════════════

/// Machine-readable help output for --help-json
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct HelpOutput {
    /// Command name (e.g., "zjj")
    pub command: String,
    /// Version string
    pub version: String,
    /// Brief description of the tool
    pub description: String,
    /// List of available subcommands
    pub subcommands: Vec<SubcommandHelp>,
    /// Exit code documentation
    pub exit_codes: Vec<ExitCodeHelp>,
    /// Author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Help for a specific subcommand
#[derive(Debug, Serialize)]
pub struct SubcommandHelp {
    /// Subcommand name
    pub name: String,
    /// Brief description
    pub description: String,
    /// Available parameters (flags, options, arguments)
    pub parameters: Vec<ParameterHelp>,
    /// Usage examples
    pub examples: Vec<ExampleHelp>,
}

/// Help for a parameter (flag, option, or positional argument)
#[derive(Debug, Serialize)]
pub struct ParameterHelp {
    /// Parameter name (e.g., "--json", "name")
    pub name: String,
    /// Description of what this parameter does
    pub description: String,
    /// Type of parameter: "flag", "option", "arg"
    pub param_type: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Value type: "string", "bool", "int", "path"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Possible values for enums
    #[serde(skip_serializing_if = "Option::is_none")]
    pub possible_values: Option<Vec<String>>,
}

/// Usage example with description
#[derive(Debug, Serialize)]
pub struct ExampleHelp {
    /// Full command example
    pub command: String,
    /// Explanation of what this example does
    pub description: String,
}

/// Exit code documentation
#[derive(Debug, Serialize)]
pub struct ExitCodeHelp {
    /// Exit code number
    pub code: i32,
    /// Short meaning (e.g., "User error")
    pub meaning: String,
    /// Detailed description
    pub description: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// BATCH OPERATIONS JSON OUTPUT (zjj-xi2j)
// ═══════════════════════════════════════════════════════════════════════════

/// Generic batch operation result with aggregated statistics
#[derive(Debug, Serialize)]
pub struct BatchOperationOutput<T> {
    /// Overall success (true if ALL items succeeded)
    pub success: bool,
    /// Total number of items processed
    pub total_count: usize,
    /// Number of successfully processed items
    pub success_count: usize,
    /// Number of failed items
    pub failure_count: usize,
    /// Individual results for each item
    pub results: Vec<BatchItemResult<T>>,
    /// Optional operation-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Result of processing a single item in a batch operation
#[derive(Debug, Clone, Serialize)]
pub struct BatchItemResult<T> {
    /// Whether this individual item succeeded
    pub success: bool,
    /// Identifier for this item (e.g., session name, bead ID)
    pub item_id: String,
    /// Index in the batch (0-based)
    pub index: usize,
    /// The successful result data (if succeeded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Optional item-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Batch add command JSON output (zjj add-batch)
pub type AddBatchOutput = BatchOperationOutput<AddOutput>;

/// Batch remove command JSON output (zjj remove-batch)
#[allow(dead_code)]
pub type RemoveBatchOutput = BatchOperationOutput<RemoveOutput>;

impl<T> BatchOperationOutput<T> {
    /// Create a new batch operation output from individual results
    pub fn from_results(results: Vec<BatchItemResult<T>>) -> Self {
        let total_count = results.len();
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.iter().filter(|r| !r.success).count();
        let success = failure_count == 0;

        Self {
            success,
            total_count,
            success_count,
            failure_count,
            results,
            metadata: None,
        }
    }

    /// Create with additional metadata
    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl<T> BatchItemResult<T> {
    /// Create a successful item result
    #[allow(clippy::missing_const_for_fn)]
    pub fn success(item_id: String, index: usize, data: T) -> Self {
        Self {
            success: true,
            item_id,
            index,
            data: Some(data),
            error: None,
            metadata: None,
        }
    }

    /// Create a failed item result
    #[allow(clippy::missing_const_for_fn)]
    pub fn failure(item_id: String, index: usize, error: String) -> Self {
        Self {
            success: false,
            item_id,
            index,
            data: None,
            error: Some(error),
            metadata: None,
        }
    }

    /// Add metadata to the result
    #[allow(dead_code)]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AGENT TRACKING JSON OUTPUT (zjj-bq9g)
// ═══════════════════════════════════════════════════════════════════════════

/// Agent metadata information
#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    /// Session name this agent is working in
    pub session_name: String,
    /// Agent identifier (e.g., "claude-code-1234")
    pub agent_id: String,
    /// Task ID the agent is working on (e.g., "zjj-1fei")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Unix timestamp when agent was spawned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawned_at: Option<u64>,
    /// Agent process ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Agent exit code (after completion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Path to agent outputs/artifacts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_path: Option<String>,
}

/// Agent list command JSON output
#[derive(Debug, Serialize)]
pub struct AgentListOutput {
    /// Total number of agents found
    pub total_count: usize,
    /// List of agents with their metadata
    pub agents: Vec<AgentInfo>,
}

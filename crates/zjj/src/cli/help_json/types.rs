//! Type definitions for machine-readable CLI help output

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Complete CLI documentation in machine-readable format
#[derive(Debug, Serialize, Deserialize)]
pub struct CliDocumentation {
    /// Version string (top-level for AI compatibility)
    pub version: String,
    /// Tool metadata
    pub tool: ToolMetadata,
    /// All available commands
    pub commands: HashMap<String, CommandDocumentation>,
    /// Command categories
    pub categories: HashMap<String, Vec<String>>,
    /// Common workflows
    pub workflows: Vec<Workflow>,
    /// Exit codes and their meanings
    pub exit_codes: HashMap<i32, String>,
    /// Prerequisites for using the tool
    pub prerequisites: Vec<Prerequisite>,
}

/// Tool-level metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub purpose: String,
    pub core_concepts: Vec<CoreConcept>,
}

/// Core concept explanation
#[derive(Debug, Serialize, Deserialize)]
pub struct CoreConcept {
    pub name: String,
    pub description: String,
    pub example: Option<String>,
}

/// Complete documentation for a single command
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandDocumentation {
    pub name: String,
    pub aliases: Vec<String>,
    pub category: String,
    pub description: String,
    pub long_description: String,
    pub usage: String,
    pub arguments: Vec<ArgumentDoc>,
    pub options: Vec<OptionDoc>,
    pub examples: Vec<Example>,
    pub prerequisites: Vec<String>,
    pub workflow_position: WorkflowPosition,
    pub related_commands: Vec<String>,
    pub output_formats: Vec<String>,
    pub exit_codes: Vec<i32>,
    pub ai_guidance: String,
    pub state_changes: Vec<StateChange>,
}

/// Argument documentation
#[derive(Debug, Serialize, Deserialize)]
pub struct ArgumentDoc {
    pub name: String,
    pub required: bool,
    pub description: String,
    pub validation_rules: Vec<ValidationRule>,
    pub examples: Vec<String>,
}

/// Option documentation
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionDoc {
    pub name: String,
    pub short: Option<String>,
    pub long: String,
    pub description: String,
    pub value_type: Option<String>,
    pub default: Option<String>,
    pub conflicts_with: Vec<String>,
    pub requires: Vec<String>,
}

/// Example with explanation
#[derive(Debug, Serialize, Deserialize)]
pub struct Example {
    pub command: String,
    pub description: String,
    pub use_case: String,
}

/// Validation rule
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: String,
    pub description: String,
    pub example_valid: String,
    pub example_invalid: String,
}

/// Where this command fits in the workflow
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowPosition {
    pub typical_order: i32,
    pub comes_after: Vec<String>,
    pub comes_before: Vec<String>,
    pub can_run_parallel_with: Vec<String>,
}

/// State change made by a command
#[derive(Debug, Serialize, Deserialize)]
pub struct StateChange {
    pub what: String,
    pub how: String,
    pub reversible: bool,
    pub reverse_command: Option<String>,
}

/// Common workflow pattern
#[derive(Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

/// Single step in a workflow
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub order: i32,
    pub command: String,
    pub description: String,
    pub optional: bool,
}

/// Prerequisite for using the tool
#[derive(Debug, Serialize, Deserialize)]
pub struct Prerequisite {
    pub name: String,
    pub description: String,
    pub install_url: Option<String>,
    pub check_command: Option<String>,
    pub required: bool,
}

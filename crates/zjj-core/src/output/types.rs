+++++++ zsoznxwk c94eee2d (rebased revision)
//! OutputLine types for JSONL streaming output
//!
//! This module provides structured types for AI-first CLI output.
//! Each variant is designed to be emitted as a single JSON line.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Severity level for issues and errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    /// Critical error blocking operation
    Error,
    /// Warning that doesn't block operation
    Warning,
    /// Informational message
    Info,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// Status of an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    /// Action is pending execution
    Pending,
    /// Action is currently running
    Running,
    /// Action completed successfully
    Complete,
    /// Action failed
    Failed,
    /// Action was skipped
    Skipped,
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// Kind of issue detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueKind {
    /// Workspace has conflicts
    Conflict,
    /// Workspace is stale (behind main)
    Stale,
    /// Session is orphaned
    Orphaned,
    /// Workspace has uncommitted changes
    Dirty,
    /// Merge is in progress
    Merging,
}

impl std::fmt::Display for IssueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => write!(f, "conflict"),
            Self::Stale => write!(f, "stale"),
            Self::Orphaned => write!(f, "orphaned"),
            Self::Dirty => write!(f, "dirty"),
            Self::Merging => write!(f, "merging"),
        }
    }
}

/// Kind of result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultKind {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
    /// Operation is pending
    Pending,
}

impl std::fmt::Display for ResultKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
            Self::Pending => write!(f, "pending"),
        }
    }
}

/// Session state for output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Session is active
    Active,
    /// Session is paused
    Paused,
    /// Session is completed
    Completed,
    /// Session has failed
    Failed,
    /// Session is being created
    Creating,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Creating => write!(f, "creating"),
        }
    }
}

/// Summary of sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Total number of sessions
    pub total: usize,
    /// Number of active sessions
    pub active: usize,
    /// Number of stale sessions
    pub stale: usize,
    /// Number of sessions with conflicts
    pub conflict: usize,
    /// Number of orphaned sessions
    pub orphaned: usize,
}

/// Session information for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session name
    pub name: String,
    /// Session state
    pub state: SessionState,
    /// Age of session in days
    pub age_days: u64,
    /// Owner of the session (agent ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
    /// Suggested action for this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Number of changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<usize>,
    /// Workspace path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Bead ID if associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_id: Option<String>,
}

/// Issue detected during operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Kind of issue
    pub kind: IssueKind,
    /// Severity of the issue
    pub severity: ErrorSeverity,
    /// Human-readable message
    pub message: String,
    /// Session this issue relates to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Suggested action to resolve
    pub suggested_action: String,
}

/// Plan for what would be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Command that would be run
    pub command: String,
    /// Whether this would actually execute
    pub would_execute: bool,
}

/// Plan step for multi-step plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step number
    pub step: usize,
    /// Command for this step
    pub command: String,
    /// Description of what this step does
    pub description: String,
}

/// Action being performed or to be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action ID
    pub id: usize,
    /// Action verb (create, remove, sync, etc.)
    pub verb: String,
    /// Target type (session, workspace, etc.)
    pub target: String,
    /// Name of target
    pub name: String,
    /// Reason for this action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Whether this action is safe (reversible)
    pub safe: bool,
    /// Current status of the action
    pub status: ActionStatus,
}

/// Warning about potential issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    /// Warning message
    pub message: String,
    /// Affected sessions/resources
    pub affected: Vec<String>,
    /// Whether this action may cause data loss
    pub data_loss: bool,
}

/// Result of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    /// Status of the result
    pub status: ResultKind,
    /// Number of completed operations
    pub completed: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Total operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Suggestion for how to resolve
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Recovery suggestion for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recovery {
    /// Recovery suggestions
    pub suggestions: Vec<RecoveryAction>,
}

/// A single recovery action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// Description of the recovery action
    pub description: String,
    /// Command to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Whether this is automatic or manual
    pub automatic: bool,
}

/// Assessment of current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Overall health status
    pub healthy: bool,
    /// Number of issues found
    pub issues_count: usize,
    /// Whether intervention is required
    pub intervention_required: bool,
}

// ============================================================================
// CONFLICT RESOLUTION TYPES (bd-36v)
// ============================================================================

/// Type of merge conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// File modified in both workspace and main
    Overlapping,
    /// File has existing JJ conflicts
    Existing,
    /// File was deleted in one branch, modified in another
    DeleteModify,
    /// File was renamed in one branch, modified in another
    RenameModify,
    /// Binary file conflict
    Binary,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overlapping => write!(f, "overlapping"),
            Self::Existing => write!(f, "existing"),
            Self::DeleteModify => write!(f, "delete_modify"),
            Self::RenameModify => write!(f, "rename_modify"),
            Self::Binary => write!(f, "binary"),
        }
    }
}

/// Resolution strategy for a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    /// Accept our version (workspace changes)
    AcceptOurs,
    /// Accept their version (main/trunk changes)
    AcceptTheirs,
    /// Manually merge the conflict
    ManualMerge,
    /// Skip this file (keep base version)
    Skip,
    /// Use jj resolve command
    JjResolve,
    /// Rebase onto trunk first
    Rebase,
    /// Abort the merge operation
    Abort,
}

impl std::fmt::Display for ResolutionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptOurs => write!(f, "accept_ours"),
            Self::AcceptTheirs => write!(f, "accept_theirs"),
            Self::ManualMerge => write!(f, "manual_merge"),
            Self::Skip => write!(f, "skip"),
            Self::JjResolve => write!(f, "jj_resolve"),
            Self::Rebase => write!(f, "rebase"),
            Self::Abort => write!(f, "abort"),
        }
    }
}

/// Risk level of a resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionRisk {
    /// Safe resolution - no data loss
    Safe,
    /// Moderate risk - may need review
    Moderate,
    /// High risk - likely data loss without careful review
    Risky,
    /// Destructive - will lose changes
    Destructive,
}

impl std::fmt::Display for ResolutionRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::Moderate => write!(f, "moderate"),
            Self::Risky => write!(f, "risky"),
            Self::Destructive => write!(f, "destructive"),
        }
    }
}

/// A single conflict resolution option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionOption {
    /// Strategy to use
    pub strategy: ResolutionStrategy,
    /// Human-readable description of what this does
    pub description: String,
    /// Risk level of this resolution
    pub risk: ResolutionRisk,
    /// Whether this resolution can be automated
    pub automatic: bool,
    /// Command to execute this resolution (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Additional context or notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Details about a single conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetail {
    /// File path with the conflict
    pub file: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Lines added in workspace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_additions: Option<usize>,
    /// Lines removed in workspace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_deletions: Option<usize>,
    /// Lines added in main (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_additions: Option<usize>,
    /// Lines removed in main (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_deletions: Option<usize>,
    /// Resolution options for this conflict
    pub resolutions: Vec<ResolutionOption>,
    /// Recommended resolution strategy
    pub recommended: ResolutionStrategy,
}

/// Complete conflict analysis with resolution options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    /// Session/workspace name
    pub session: String,
    /// Whether merge is safe (no conflicts)
    pub merge_safe: bool,
    /// Total number of conflicts
    pub total_conflicts: usize,
    /// Files with existing JJ conflicts
    pub existing_conflicts: Vec<String>,
    /// Files with overlapping changes
    pub overlapping_files: Vec<String>,
    /// Detailed conflict information
    pub conflicts: Vec<ConflictDetail>,
    /// Merge base commit (common ancestor)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_base: Option<String>,
    /// Analysis timestamp (ISO 8601)
    pub timestamp: String,
    /// Time taken for analysis in milliseconds
    pub analysis_time_ms: u64,
}

/// Context information for human readers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Human-readable summary
    pub text: String,
    /// Whether this is meant for human consumption
    pub for_human: bool,
}

/// OutputLine is the main discriminated union for JSONL output.
/// Each variant represents a single line of output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutputLine {
    /// Summary of sessions
    Summary(Summary),
    /// Session information
    Session(Session),
    /// Issue detected
    Issue(Issue),
    /// Plan of what would be executed
    Plan(Plan),
    /// Plan step for multi-step plans
    PlanStep(PlanStep),
    /// Action being performed
    Action(Action),
    /// Warning about potential issues
    Warning(Warning),
    /// Result of an operation
    Result(Result),
    /// Error information
    Error(Error),
    /// Recovery suggestions
    Recovery(Recovery),
    /// Assessment of current state
    Assessment(Assessment),
    /// Context for human readers
    Context(Context),
    /// Conflict analysis with resolution options
    ConflictAnalysis(ConflictAnalysis),
    /// Single conflict detail with resolution options
    ConflictDetail(ConflictDetail),
}

impl OutputLine {
    /// Create a Session output line
    #[must_use]
    pub fn session(name: impl Into<String>, state: SessionState, age_days: u64) -> Self {
        Self::Session(Session {
            name: name.into(),
            state,
            age_days,
            owned_by: None,
            action: None,
            branch: None,
            changes: None,
            workspace_path: None,
            bead_id: None,
        })
    }

    /// Create a Context output line
    #[must_use]
    pub fn context(text: impl Into<String>) -> Self {
        Self::Context(Context {
            text: text.into(),
            for_human: true,
        })
    }

    /// Create an Error output line
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(Error {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestion: None,
        })
    }

    /// Create a Summary output line
    #[must_use]
    pub fn summary(
        total: usize,
        active: usize,
        stale: usize,
        conflict: usize,
        orphaned: usize,
    ) -> Self {
        Self::Summary(Summary {
            total,
            active,
            stale,
            conflict,
            orphaned,
        })
    }

    /// Create a Result output line
    #[must_use]
    pub fn result(status: ResultKind, completed: usize, failed: usize) -> Self {
        Self::Result(Result {
            status,
            completed,
            failed,
            total: None,
        })
    }

    /// Create a ConflictAnalysis output line
    #[must_use]
    pub fn conflict_analysis(
        session: impl Into<String>,
        merge_safe: bool,
        conflicts: Vec<ConflictDetail>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let existing_conflicts: Vec<String> = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Existing)
            .map(|c| c.file.clone())
            .collect();
        let overlapping_files: Vec<String> = conflicts
            .iter()
            .filter(|c| c.conflict_type == ConflictType::Overlapping)
            .map(|c| c.file.clone())
            .collect();
        Self::ConflictAnalysis(ConflictAnalysis {
            session: session.into(),
            merge_safe,
            total_conflicts: conflicts.len(),
            existing_conflicts,
            overlapping_files,
            conflicts,
            merge_base: None,
            timestamp: now,
            analysis_time_ms: 0,
        })
    }

    /// Create a ConflictDetail output line
    #[must_use]
    pub fn conflict_detail(
        file: impl Into<String>,
        conflict_type: ConflictType,
        resolutions: Vec<ResolutionOption>,
        recommended: ResolutionStrategy,
    ) -> Self {
        Self::ConflictDetail(ConflictDetail {
            file: file.into(),
            conflict_type,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions,
            recommended,
        })
    }
}

// ============================================================================
// CONFLICT RESOLUTION HELPERS
// ============================================================================

impl ResolutionOption {
    /// Create a resolution option for accepting workspace changes
    #[must_use]
    pub fn accept_ours() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptOurs,
            description: "Keep workspace changes, discard trunk changes".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj resolve --restore @".to_string()),
            notes: Some("May lose important changes from trunk".to_string()),
        }
    }

    /// Create a resolution option for accepting trunk changes
    #[must_use]
    pub fn accept_theirs() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptTheirs,
            description: "Keep trunk changes, discard workspace changes".to_string(),
            risk: ResolutionRisk::Destructive,
            automatic: true,
            command: Some("jj diff --from trunk() --to @ | jj restore --from trunk()".to_string()),
            notes: Some("Will lose all workspace changes for this file".to_string()),
        }
    }

    /// Create a resolution option for manual merge
    #[must_use]
    pub fn manual_merge() -> Self {
        Self {
            strategy: ResolutionStrategy::ManualMerge,
            description: "Manually edit file to resolve conflict".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: false,
            command: None,
            notes: Some("Open file in editor and resolve markers".to_string()),
        }
    }

    /// Create a resolution option for jj resolve
    #[must_use]
    pub fn jj_resolve(file: &str) -> Self {
        Self {
            strategy: ResolutionStrategy::JjResolve,
            description: "Use JJ's interactive resolve tool".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: false,
            command: Some(format!("jj resolve {file}")),
            notes: Some("Launches merge tool configured in jj".to_string()),
        }
    }

    /// Create a resolution option for rebase
    #[must_use]
    pub fn rebase() -> Self {
        Self {
            strategy: ResolutionStrategy::Rebase,
            description: "Rebase workspace onto trunk to resolve conflicts".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj rebase -d trunk()".to_string()),
            notes: Some("May introduce additional merge conflicts".to_string()),
        }
    }

    /// Create a resolution option for aborting
    #[must_use]
    pub fn abort() -> Self {
        Self {
            strategy: ResolutionStrategy::Abort,
            description: "Abort the merge operation".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: Some("jj abandon".to_string()),
            notes: None,
        }
    }

    /// Create a skip resolution option
    #[must_use]
    pub fn skip() -> Self {
        Self {
            strategy: ResolutionStrategy::Skip,
            description: "Skip this file and keep base version".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj restore --from trunk()".to_string()),
            notes: Some("Both changes will be discarded".to_string()),
        }
    }
}

impl ConflictDetail {
    /// Create a conflict detail for an overlapping file
    #[must_use]
    pub fn overlapping(file: impl Into<String>) -> Self {
        let file_str = file.into();
        Self {
            file: file_str.clone(),
            conflict_type: ConflictType::Overlapping,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(&file_str),
                ResolutionOption::manual_merge(),
                ResolutionOption::accept_ours(),
                ResolutionOption::accept_theirs(),
                ResolutionOption::rebase(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }

    /// Create a conflict detail for an existing JJ conflict
    #[must_use]
    pub fn existing(file: impl Into<String>) -> Self {
        let file_str = file.into();
        Self {
            file: file_str.clone(),
            conflict_type: ConflictType::Existing,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(&file_str),
                ResolutionOption::manual_merge(),
                ResolutionOption::abort(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_line_session_serializes_with_type() {
        let line = OutputLine::session("test-session", SessionState::Active, 5);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"session"#));
        assert!(json.contains(r#""name":"test-session"#));
        assert!(json.contains(r#""state":"active"#));
        assert!(json.contains(r#""age_days":5"#));
    }

    #[test]
    fn test_output_line_context_serializes_with_type() {
        let line = OutputLine::context("All operations completed");
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"context"#));
        assert!(json.contains(r#""text":"All operations completed"#));
        assert!(json.contains(r#""for_human":true"#));
    }

    #[test]
    fn test_output_line_error_serializes_with_type() {
        let line = OutputLine::error("SYNC_FAILED", "Failed to sync workspace");
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"error"#));
        assert!(json.contains(r#""code":"SYNC_FAILED"#));
        assert!(json.contains(r#""message":"Failed to sync workspace"#));
    }

    #[test]
    fn test_output_line_summary_serializes_with_type() {
        let line = OutputLine::summary(10, 5, 2, 1, 0);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"summary"#));
        assert!(json.contains(r#""total":10"#));
        assert!(json.contains(r#""active":5"#));
        assert!(json.contains(r#""stale":2"#));
        assert!(json.contains(r#""conflict":1"#));
        assert!(json.contains(r#""orphaned":0"#));
    }

    #[test]
    fn test_severity_enum_serializes_to_string() {
        let severity = ErrorSeverity::Warning;
        let json = serde_json::to_string(&severity);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""warning""#);
    }

    #[test]
    fn test_action_status_enum_serializes_to_string() {
        let status = ActionStatus::Complete;
        let json = serde_json::to_string(&status);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""complete""#);
    }

    #[test]
    fn test_result_kind_enum_serializes_to_string() {
        let kind = ResultKind::Success;
        let json = serde_json::to_string(&kind);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""success""#);
    }

    #[test]
    fn test_session_with_all_fields() {
        let session = Session {
            name: "my-feature".to_string(),
            state: SessionState::Active,
            age_days: 3,
            owned_by: Some("agent-1".to_string()),
            action: Some("sync".to_string()),
            branch: Some("feature-branch".to_string()),
            changes: Some(5),
            workspace_path: Some("/workspaces/my-feature".to_string()),
            bead_id: Some("bd-123".to_string()),
        };
        let line = OutputLine::Session(session);
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains(r#""owned_by":"agent-1"#));
        assert!(json.contains(r#""branch":"feature-branch"#));
        assert!(json.contains(r#""changes":5"#));
    }

    #[test]
    fn test_session_skips_none_fields() {
        let session = Session {
            name: "test".to_string(),
            state: SessionState::Active,
            age_days: 0,
            owned_by: None,
            action: None,
            branch: None,
            changes: None,
            workspace_path: None,
            bead_id: None,
        };
        let line = OutputLine::Session(session);
        let json = serde_json::to_string(&line).unwrap();
        assert!(!json.contains("owned_by"));
        assert!(!json.contains("branch"));
        assert!(!json.contains("changes"));
    }

    #[test]
    fn test_deserialize_session_output_line() {
        let json = r#"{"type":"session","name":"test","state":"active","age_days":5}"#;
        let line: OutputLine = serde_json::from_str(json).unwrap();
        match line {
            OutputLine::Session(s) => {
                assert_eq!(s.name, "test");
                assert_eq!(s.state, SessionState::Active);
                assert_eq!(s.age_days, 5);
            }
            _ => panic!("Expected Session variant"),
        }
    }

    #[test]
    fn test_issue_output_line() {
        let issue = Issue {
            kind: IssueKind::Conflict,
            severity: ErrorSeverity::Error,
            message: "Workspace has conflicts".to_string(),
            session: Some("feature-x".to_string()),
            suggested_action: "Run 'jj resolve' to resolve conflicts".to_string(),
        };
        let line = OutputLine::Issue(issue);
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains(r#""type":"issue"#));
        assert!(json.contains(r#""kind":"conflict"#));
        assert!(json.contains(r#""severity":"error"#));
    }

    #[test]
    fn test_action_output_line() {
        let action = Action {
            id: 1,
            verb: "remove".to_string(),
            target: "session".to_string(),
            name: "old-feature".to_string(),
            reason: Some("Stale workspace".to_string()),
            safe: false,
            status: ActionStatus::Pending,
        };
        let line = OutputLine::Action(action);
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains(r#""type":"action"#));
        assert!(json.contains(r#""verb":"remove"#));
        assert!(json.contains(r#""safe":false"#));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // CONFLICT RESOLUTION TYPE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_conflict_type_serializes_to_snake_case() {
        let conflict_type = ConflictType::DeleteModify;
        let json = serde_json::to_string(&conflict_type);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""delete_modify""#);
    }

    #[test]
    fn test_resolution_strategy_serializes_to_snake_case() {
        let strategy = ResolutionStrategy::AcceptOurs;
        let json = serde_json::to_string(&strategy);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""accept_ours""#);
    }

    #[test]
    fn test_resolution_risk_serializes_to_lowercase() {
        let risk = ResolutionRisk::Moderate;
        let json = serde_json::to_string(&risk);
        assert!(json.is_ok());
        assert_eq!(json.unwrap(), r#""moderate""#);
    }

    #[test]
    fn test_resolution_option_helpers() {
        let ours = ResolutionOption::accept_ours();
        assert_eq!(ours.strategy, ResolutionStrategy::AcceptOurs);
        assert_eq!(ours.risk, ResolutionRisk::Moderate);
        assert!(ours.automatic);
        assert!(ours.command.is_some());

        let theirs = ResolutionOption::accept_theirs();
        assert_eq!(theirs.strategy, ResolutionStrategy::AcceptTheirs);
        assert_eq!(theirs.risk, ResolutionRisk::Destructive);

        let manual = ResolutionOption::manual_merge();
        assert_eq!(manual.strategy, ResolutionStrategy::ManualMerge);
        assert_eq!(manual.risk, ResolutionRisk::Safe);
        assert!(!manual.automatic);
        assert!(manual.command.is_none());

        let jj_resolve = ResolutionOption::jj_resolve("src/main.rs");
        assert_eq!(jj_resolve.strategy, ResolutionStrategy::JjResolve);
        assert!(jj_resolve
            .command
            .as_ref()
            .map_or(false, |c| c.contains("src/main.rs")));

        let rebase = ResolutionOption::rebase();
        assert_eq!(rebase.strategy, ResolutionStrategy::Rebase);

        let abort = ResolutionOption::abort();
        assert_eq!(abort.strategy, ResolutionStrategy::Abort);

        let skip = ResolutionOption::skip();
        assert_eq!(skip.strategy, ResolutionStrategy::Skip);
    }

    #[test]
    fn test_conflict_detail_overlapping() {
        let detail = ConflictDetail::overlapping("src/lib.rs");
        assert_eq!(detail.file, "src/lib.rs");
        assert_eq!(detail.conflict_type, ConflictType::Overlapping);
        assert_eq!(detail.recommended, ResolutionStrategy::JjResolve);
        assert!(!detail.resolutions.is_empty());
    }

    #[test]
    fn test_conflict_detail_existing() {
        let detail = ConflictDetail::existing("src/conflict.rs");
        assert_eq!(detail.file, "src/conflict.rs");
        assert_eq!(detail.conflict_type, ConflictType::Existing);
        assert_eq!(detail.recommended, ResolutionStrategy::JjResolve);
        assert!(!detail.resolutions.is_empty());
    }

    #[test]
    fn test_output_line_conflict_detail() {
        let detail = ConflictDetail::overlapping("src/test.rs");
        let line = OutputLine::ConflictDetail(detail);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"conflictdetail"#));
        assert!(json.contains(r#""file":"src/test.rs"#));
        assert!(json.contains(r#""conflict_type":"overlapping"#));
        assert!(json.contains(r#""resolutions""#));
    }

    #[test]
    fn test_output_line_conflict_analysis() {
        let conflicts = vec![
            ConflictDetail::overlapping("src/lib.rs"),
            ConflictDetail::existing("src/conflict.rs"),
        ];
        let line = OutputLine::conflict_analysis("my-session", false, conflicts);
        let json = serde_json::to_string(&line);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert!(json.contains(r#""type":"conflictanalysis"#));
        assert!(json.contains(r#""session":"my-session"#));
        assert!(json.contains(r#""merge_safe":false"#));
        assert!(json.contains(r#""total_conflicts":2"#));
        assert!(json.contains(r#""existing_conflicts""#));
        assert!(json.contains(r#""overlapping_files""#));
        assert!(json.contains(r#""timestamp""#));
    }

    #[test]
    fn test_conflict_analysis_safe_merge() {
        let conflicts: Vec<ConflictDetail> = vec![];
        let line = OutputLine::conflict_analysis("safe-session", true, conflicts);
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains(r#""merge_safe":true"#));
        assert!(json.contains(r#""total_conflicts":0"#));
    }

    #[test]
    fn test_conflict_analysis_categorizes_conflicts() {
        let conflicts = vec![
            ConflictDetail::overlapping("src/a.rs"),
            ConflictDetail::existing("src/b.rs"),
            ConflictDetail::overlapping("src/c.rs"),
        ];
        let line = OutputLine::conflict_analysis("test", false, conflicts);
        let json = serde_json::to_string(&line).unwrap();

        // Parse to verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let existing = parsed.get("existing_conflicts").and_then(|v| v.as_array());
        let overlapping = parsed.get("overlapping_files").and_then(|v| v.as_array());

        assert!(existing.is_some());
        assert!(overlapping.is_some());
        if let (Some(existing), Some(overlapping)) = (existing, overlapping) {
            assert_eq!(existing.len(), 1);
            assert_eq!(overlapping.len(), 2);
        }
    }

    #[test]
    fn test_resolution_option_serialization() {
        let option = ResolutionOption {
            strategy: ResolutionStrategy::JjResolve,
            description: "Use jj resolve".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: false,
            command: Some("jj resolve file.rs".to_string()),
            notes: Some("Launches merge tool".to_string()),
        };
        let json = serde_json::to_string(&option).unwrap();
        assert!(json.contains(r#""strategy":"jj_resolve""#));
        assert!(json.contains(r#""risk":"safe""#));
        assert!(json.contains(r#""automatic":false"#));
        assert!(json.contains(r#""command":"jj resolve file.rs""#));
    }

    #[test]
    fn test_conflict_detail_with_line_stats() {
        let detail = ConflictDetail {
            file: "src/main.rs".to_string(),
            conflict_type: ConflictType::Overlapping,
            workspace_additions: Some(10),
            workspace_deletions: Some(5),
            main_additions: Some(8),
            main_deletions: Some(3),
            resolutions: vec![ResolutionOption::manual_merge()],
            recommended: ResolutionStrategy::ManualMerge,
        };
        let line = OutputLine::ConflictDetail(detail);
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.contains(r#""workspace_additions":10"#));
        assert!(json.contains(r#""workspace_deletions":5"#));
        assert!(json.contains(r#""main_additions":8"#));
        assert!(json.contains(r#""main_deletions":3"#));
    }
}

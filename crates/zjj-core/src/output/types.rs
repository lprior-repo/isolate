//! JSONL output types for AI-first CLI design
//!
//! Each output line is a self-describing JSON object with a "type" field.
//! This enables streaming JSONL output where each line can be parsed independently.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{types::SessionStatus, WorkspaceState};

#[derive(Debug, Clone, Error)]
pub enum OutputLineError {
    #[error("message is required but was empty")]
    EmptyMessage,
    #[error("title is required but was empty")]
    EmptyTitle,
    #[error("description is required but was empty")]
    EmptyDescription,
    #[error("session name is required but was empty")]
    EmptySessionName,
    #[error("at least one action is required")]
    NoActions,
    #[error("plan step count exceeds u32::MAX")]
    PlanStepOverflow,
    #[error("recovery action count exceeds u32::MAX")]
    RecoveryActionOverflow,
    #[error("stack entry count exceeds u32::MAX")]
    StackEntryOverflow,
    #[error("train step count exceeds u32::MAX")]
    TrainStepOverflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputLine {
    Summary(Summary),
    Session(SessionOutput),
    Issue(Issue),
    Plan(Plan),
    Action(Action),
    Warning(Warning),
    Result(ResultOutput),
    Stack(Stack),
    QueueSummary(QueueSummary),
    QueueEntry(QueueEntry),
    Train(Train),
}

impl OutputLine {
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Summary(_) => "summary",
            Self::Session(_) => "session",
            Self::Issue(_) => "issue",
            Self::Plan(_) => "plan",
            Self::Action(_) => "action",
            Self::Warning(_) => "warning",
            Self::Result(_) => "result",
            Self::Stack(_) => "stack",
            Self::QueueSummary(_) => "queue_summary",
            Self::QueueEntry(_) => "queue_entry",
            Self::Train(_) => "train",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
    #[serde(rename = "type")]
    pub type_field: SummaryType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryType {
    Status,
    Count,
    Info,
}

impl Summary {
    pub fn new(type_field: SummaryType, message: String) -> Result<Self, OutputLineError> {
        if message.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self {
            type_field,
            message,
            details: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_details(self, details: String) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOutput {
    pub name: String,
    pub status: SessionStatus,
    pub state: WorkspaceState,
    pub workspace_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

impl SessionOutput {
    pub fn new(
        name: String,
        status: SessionStatus,
        state: WorkspaceState,
        workspace_path: PathBuf,
    ) -> Result<Self, OutputLineError> {
        if name.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        let now = Utc::now();
        Ok(Self {
            name,
            status,
            state,
            workspace_path,
            branch: None,
            created_at: now,
            updated_at: now,
        })
    }

    #[must_use]
    pub fn with_branch(self, branch: String) -> Self {
        Self {
            branch: Some(branch),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub kind: IssueKind,
    pub severity: IssueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    Validation,
    StateConflict,
    ResourceNotFound,
    PermissionDenied,
    Timeout,
    Configuration,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Hint,
    Warning,
    Error,
    Critical,
}

impl Issue {
    pub fn new(
        id: String,
        title: String,
        kind: IssueKind,
        severity: IssueSeverity,
    ) -> Result<Self, OutputLineError> {
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self {
            id,
            title,
            kind,
            severity,
            session: None,
            suggestion: None,
        })
    }

    #[must_use]
    pub fn with_session(self, session: String) -> Self {
        Self {
            session: Some(session),
            ..self
        }
    }

    #[must_use]
    pub fn with_suggestion(self, suggestion: String) -> Self {
        Self {
            suggestion: Some(suggestion),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStep {
    pub order: u32,
    pub description: String,
    pub status: ActionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl Plan {
    pub fn new(title: String, description: String) -> Result<Self, OutputLineError> {
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        if description.trim().is_empty() {
            return Err(OutputLineError::EmptyDescription);
        }
        Ok(Self {
            title,
            description,
            steps: Vec::new(),
            created_at: Utc::now(),
        })
    }

    /// Append a step to this plan.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::PlanStepOverflow` when the number of steps
    /// cannot be represented as `u32`.
    pub fn with_step(
        self,
        description: String,
        status: ActionStatus,
    ) -> Result<Self, OutputLineError> {
        let order =
            u32::try_from(self.steps.len()).map_err(|_| OutputLineError::PlanStepOverflow)?;
        Ok(Self {
            steps: self
                .steps
                .into_iter()
                .chain(std::iter::once(PlanStep {
                    order,
                    description,
                    status,
                }))
                .collect(),
            ..self
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub verb: String,
    pub target: String,
    pub status: ActionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

impl Action {
    #[must_use]
    pub fn new(verb: String, target: String, status: ActionStatus) -> Self {
        Self {
            verb,
            target,
            status,
            result: None,
            timestamp: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_result(self, result: String) -> Self {
        Self {
            result: Some(result),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Warning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub session: String,
    pub workspace: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<serde_json::Value>,
}

impl Warning {
    pub fn new(code: String, message: String) -> Result<Self, OutputLineError> {
        if message.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self {
            code,
            message,
            context: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_context(self, session: String, workspace: PathBuf) -> Self {
        Self {
            context: Some(Context {
                session,
                workspace,
                additional: None,
            }),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultOutput {
    pub kind: ResultKind,
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Command,
    Operation,
    Assessment,
    Recovery,
}

impl ResultOutput {
    pub fn success(kind: ResultKind, message: String) -> Result<Self, OutputLineError> {
        if message.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self {
            kind,
            success: true,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    pub fn failure(kind: ResultKind, message: String) -> Result<Self, OutputLineError> {
        if message.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self {
            kind,
            success: false,
            message,
            data: None,
            timestamp: Utc::now(),
        })
    }

    #[must_use]
    pub fn with_data(self, data: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recovery {
    pub issue_id: String,
    pub assessment: Assessment,
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Assessment {
    pub severity: ErrorSeverity,
    pub recoverable: bool,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryAction {
    pub order: u32,
    pub description: String,
    pub command: Option<String>,
    pub automatic: bool,
}

impl Recovery {
    #[must_use]
    pub fn new(issue_id: String, assessment: Assessment) -> Self {
        Self {
            issue_id,
            assessment,
            actions: Vec::new(),
        }
    }

    /// Append a recovery action.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::RecoveryActionOverflow` when the number of
    /// actions cannot be represented as `u32`.
    pub fn with_action(
        self,
        description: String,
        command: Option<String>,
        automatic: bool,
    ) -> Result<Self, OutputLineError> {
        let order = u32::try_from(self.actions.len())
            .map_err(|_| OutputLineError::RecoveryActionOverflow)?;
        Ok(Self {
            actions: self
                .actions
                .into_iter()
                .chain(std::iter::once(RecoveryAction {
                    order,
                    description,
                    command,
                    automatic,
                }))
                .collect(),
            ..self
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stack {
    pub name: String,
    pub base_ref: String,
    pub entries: Vec<StackEntry>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StackEntry {
    pub order: u32,
    pub session: String,
    pub workspace: PathBuf,
    pub status: StackEntryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StackEntryStatus {
    Pending,
    Ready,
    Merging,
    Merged,
    Failed,
}

impl Stack {
    /// Create a new stack.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptySessionName` if the name is empty.
    pub fn new(name: String, base_ref: String) -> Result<Self, OutputLineError> {
        if name.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        Ok(Self {
            name,
            base_ref,
            entries: Vec::new(),
            updated_at: Utc::now(),
        })
    }

    /// Append an entry to this stack.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::StackEntryOverflow` when the number of
    /// entries cannot be represented as `u32`.
    pub fn with_entry(
        self,
        session: String,
        workspace: PathBuf,
        status: StackEntryStatus,
        bead: Option<String>,
    ) -> Result<Self, OutputLineError> {
        let order =
            u32::try_from(self.entries.len()).map_err(|_| OutputLineError::StackEntryOverflow)?;
        Ok(Self {
            entries: self
                .entries
                .into_iter()
                .chain(std::iter::once(StackEntry {
                    order,
                    session,
                    workspace,
                    status,
                    bead,
                }))
                .collect(),
            updated_at: Utc::now(),
            ..self
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueSummary {
    pub total: u32,
    pub pending: u32,
    pub ready: u32,
    pub blocked: u32,
    pub in_progress: u32,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

impl QueueSummary {
    #[must_use]
    pub fn new() -> Self {
        Self {
            total: 0,
            pending: 0,
            ready: 0,
            blocked: 0,
            in_progress: 0,
            updated_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_counts(
        self,
        total: u32,
        pending: u32,
        ready: u32,
        blocked: u32,
        in_progress: u32,
    ) -> Self {
        Self {
            total,
            pending,
            ready,
            blocked,
            in_progress,
            ..self
        }
    }

    #[must_use]
    pub fn has_blockers(&self) -> bool {
        self.blocked > 0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl Default for QueueSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueEntry {
    pub id: String,
    pub session: String,
    pub priority: u8,
    pub status: QueueEntryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueEntryStatus {
    Pending,
    Ready,
    Claimed,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

impl QueueEntry {
    /// Create a new queue entry.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptySessionName` if the session is empty.
    pub fn new(id: String, session: String, priority: u8) -> Result<Self, OutputLineError> {
        if session.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        let now = Utc::now();
        Ok(Self {
            id,
            session,
            priority,
            status: QueueEntryStatus::Pending,
            bead: None,
            agent: None,
            created_at: now,
            updated_at: now,
        })
    }

    #[must_use]
    pub fn with_bead(self, bead: String) -> Self {
        Self {
            bead: Some(bead),
            ..self
        }
    }

    #[must_use]
    pub fn with_agent(self, agent: String) -> Self {
        Self {
            agent: Some(agent),
            ..self
        }
    }

    #[must_use]
    pub fn with_status(self, status: QueueEntryStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Train {
    pub id: String,
    pub name: String,
    pub steps: Vec<TrainStep>,
    pub status: TrainStatus,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrainStep {
    pub order: u32,
    pub session: String,
    pub action: TrainAction,
    pub status: TrainStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainAction {
    Sync,
    Rebase,
    Merge,
    Push,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainStepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

impl Train {
    /// Create a new train.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptySessionName` if the name is empty.
    pub fn new(id: String, name: String) -> Result<Self, OutputLineError> {
        if name.trim().is_empty() {
            return Err(OutputLineError::EmptySessionName);
        }
        let now = Utc::now();
        Ok(Self {
            id,
            name,
            steps: Vec::new(),
            status: TrainStatus::Pending,
            created_at: now,
            updated_at: now,
        })
    }

    /// Append a step to this train.
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::TrainStepOverflow` when the number of steps
    /// cannot be represented as `u32`.
    pub fn with_step(
        self,
        session: String,
        action: TrainAction,
        status: TrainStepStatus,
    ) -> Result<Self, OutputLineError> {
        let order =
            u32::try_from(self.steps.len()).map_err(|_| OutputLineError::TrainStepOverflow)?;
        Ok(Self {
            steps: self
                .steps
                .into_iter()
                .chain(std::iter::once(TrainStep {
                    order,
                    session,
                    action,
                    status,
                    error: None,
                }))
                .collect(),
            updated_at: Utc::now(),
            ..self
        })
    }

    #[must_use]
    pub fn with_status(self, status: TrainStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }
}

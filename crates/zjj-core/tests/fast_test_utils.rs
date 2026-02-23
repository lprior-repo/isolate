//! Fast in-memory test utilities - no subprocess spawning
//!
//! This module provides test helpers that operate entirely in-memory,
//! avoiding the overhead of subprocess spawning for unit-level tests.
//!
//! # Design Principles
//!
//! - Zero subprocess spawning - all operations are in-memory
//! - Zero unwrap - functional error handling throughout
//! - Pure functions where possible - deterministic and testable
//! - Pre-built fixtures - common test scenarios ready to use
//!
//! # Usage
//!
//! ```rust,ignore
//! use zjj_core::tests::fast_test_utils::*;
//!
//! // Create an in-memory test harness
//! let harness = FastTestHarness::new();
//!
//! // Add sessions without spawning jj
//! let session = harness.add_session("feature-auth")?;
//!
//! // Run commands in-memory
//! let output = harness.run_command("list")?;
//!
//! // Validate output
//! assert!(output.contains_session("feature-auth"));
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use zjj_core::output::{
    domain_types::{
        ActionTarget, ActionVerb, BaseRef, BeadAttachment, BeadId, IssueId, IssueTitle,
        Message, Outcome, PlanDescription, PlanTitle, QueueEntryId, SessionName, TrainId,
        WarningCode,
    },
    Action, ActionStatus, Issue, IssueKind, IssueSeverity,
    OutputLine, Plan, QueueEntry, QueueEntryStatus, QueueSummary, ResultOutput,
    SessionOutput, Stack, StackEntryStatus, Summary, SummaryType, Train,
    TrainAction, TrainStepStatus, Warning,
};
use zjj_core::types::SessionStatus;
use zjj_core::WorkspaceState;

// ═══════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Error type for fast test utilities
#[derive(Debug, Clone, Error)]
pub enum FastTestError {
    /// Session name is invalid
    #[error("invalid session name: {0}")]
    InvalidSessionName(String),

    /// Session already exists
    #[error("session already exists: {0}")]
    SessionAlreadyExists(String),

    /// Session not found
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Command not recognized
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    /// Invalid argument for command
    #[error("invalid argument for {command}: {details}")]
    InvalidArgument {
        /// The command that failed
        command: String,
        /// Details about the invalid argument
        details: String,
    },

    /// Output line construction failed
    #[error("failed to construct output: {0}")]
    OutputConstruction(String),
}

// ═══════════════════════════════════════════════════════════════════════════
// IN-MEMORY DATA TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// In-memory session data (no file system required)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemorySession {
    /// Session name
    pub name: SessionName,
    /// Session status
    pub status: SessionStatus,
    /// Workspace state
    pub state: WorkspaceState,
    /// Workspace path (mocked, does not need to exist)
    pub workspace_path: PathBuf,
    /// Optional branch name
    pub branch: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Parent session (for stacked sessions)
    pub parent: Option<String>,
    /// Stack depth (0 for top-level)
    pub depth: u32,
}

impl InMemorySession {
    /// Create a new in-memory session with defaults
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::InvalidSessionName` if the name is invalid.
    pub fn new(name: &str) -> Result<Self, FastTestError> {
        let session_name = SessionName::parse(name)
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;
        let now = Utc::now();
        Ok(Self {
            name: session_name,
            status: SessionStatus::Active,
            state: WorkspaceState::Created,
            workspace_path: PathBuf::from(format!("/mock/workspaces/{name}")),
            branch: None,
            created_at: now,
            updated_at: now,
            parent: None,
            depth: 0,
        })
    }

    /// Convert to `SessionOutput` for JSON serialization
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::OutputConstruction` if conversion fails.
    pub fn to_session_output(&self) -> Result<SessionOutput, FastTestError> {
        SessionOutput::new(
            self.name.as_str().to_string(),
            self.status,
            self.state,
            self.workspace_path.clone(),
        )
        .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a stacked session on top of this one
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::InvalidSessionName` if the name is invalid.
    pub fn create_stacked(&self, name: &str) -> Result<Self, FastTestError> {
        let session_name = SessionName::parse(name)
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;
        let now = Utc::now();
        Ok(Self {
            name: session_name,
            status: SessionStatus::Active,
            state: WorkspaceState::Created,
            workspace_path: PathBuf::from(format!("/mock/workspaces/{name}")),
            branch: None,
            created_at: now,
            updated_at: now,
            parent: Some(self.name.as_str().to_string()),
            depth: self.depth + 1,
        })
    }

    /// Set the branch name
    #[must_use]
    pub fn with_branch(self, branch: impl Into<String>) -> Self {
        Self {
            branch: Some(branch.into()),
            ..self
        }
    }

    /// Set the workspace state
    #[must_use]
    pub fn with_state(self, state: WorkspaceState) -> Self {
        Self {
            state,
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the status
    #[must_use]
    pub fn with_status(self, status: SessionStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }
}

/// In-memory queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryQueueEntry {
    /// Entry ID
    pub id: QueueEntryId,
    /// Session name
    pub session: SessionName,
    /// Priority (lower is higher priority)
    pub priority: u8,
    /// Entry status
    pub status: QueueEntryStatus,
    /// Optional bead attachment
    pub bead: Option<String>,
    /// Optional agent assignment
    pub agent: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl InMemoryQueueEntry {
    /// Create a new queue entry
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::InvalidSessionName` if the session name is invalid.
    pub fn new(id: i64, session: &str, priority: u8) -> Result<Self, FastTestError> {
        let entry_id = QueueEntryId::new(id)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let session_name = SessionName::parse(session)
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;
        let now = Utc::now();
        Ok(Self {
            id: entry_id,
            session: session_name,
            priority,
            status: QueueEntryStatus::Pending,
            bead: None,
            agent: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Convert to `QueueEntry` for output
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::OutputConstruction` if conversion fails.
    pub fn to_queue_entry(&self) -> Result<QueueEntry, FastTestError> {
        let mut entry = QueueEntry::new(self.id, self.session.clone(), self.priority)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_status(self.status);

        if let Some(ref bead_id) = self.bead {
            if let Ok(bead) = BeadId::parse(bead_id) {
                entry = entry.with_bead(bead);
            }
        }

        if let Some(ref agent_id) = self.agent {
            entry = entry.with_agent(agent_id.clone());
        }

        Ok(entry)
    }

    /// Set the status
    #[must_use]
    pub fn with_status(self, status: QueueEntryStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the agent
    #[must_use]
    pub fn with_agent(self, agent: impl Into<String>) -> Self {
        Self {
            agent: Some(agent.into()),
            updated_at: Utc::now(),
            ..self
        }
    }

    /// Set the bead
    #[must_use]
    pub fn with_bead(self, bead: impl Into<String>) -> Self {
        Self {
            bead: Some(bead.into()),
            updated_at: Utc::now(),
            ..self
        }
    }
}

/// In-memory workspace data
#[derive(Debug, Clone, Default)]
pub struct InMemoryWorkspace {
    /// Workspace name
    pub name: String,
    /// Whether the workspace exists (simulated)
    pub exists: bool,
    /// Whether there are uncommitted changes
    pub has_changes: bool,
    /// Whether there are conflicts
    pub has_conflicts: bool,
    /// Current branch
    pub current_branch: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// FAST TEST HARNESS
// ═══════════════════════════════════════════════════════════════════════════

/// Fast in-memory test harness
///
/// Provides integration-level testing without subprocess spawning.
/// All state is held in memory, making tests extremely fast.
///
/// # Example
///
/// ```rust,ignore
/// let harness = FastTestHarness::new();
///
/// // Add a session
/// let session = harness.add_session("feature-auth")?;
///
/// // Simulate a command
/// let output = harness.run_command("list")?;
///
/// // Check results
/// assert_eq!(output.sessions.len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct FastTestHarness {
    /// Sessions indexed by name
    sessions: HashMap<String, InMemorySession>,
    /// Queue entries indexed by ID
    queue_entries: HashMap<i64, InMemoryQueueEntry>,
    /// Workspaces indexed by name
    workspaces: HashMap<String, InMemoryWorkspace>,
    /// Next queue entry ID
    next_queue_id: i64,
    /// Current session (focused)
    current_session: Option<String>,
}

impl FastTestHarness {
    /// Create a new empty test harness
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            queue_entries: HashMap::new(),
            workspaces: HashMap::new(),
            next_queue_id: 1,
            current_session: None,
        }
    }

    /// Create a harness with pre-built fixtures
    ///
    /// # Errors
    ///
    /// Returns an error if fixture creation fails.
    pub fn with_fixtures() -> Result<Self, FastTestError> {
        let harness = Self::new();
        harness.with_default_sessions()
    }

    /// Add default session fixtures
    fn with_default_sessions(self) -> Result<Self, FastTestError> {
        let harness = self
            .add_session("feature-auth")?
            .add_session("bugfix-timeout")?
            .add_session("refactor-config")?;
        Ok(harness)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SESSION MANAGEMENT
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Add a session without spawning jj
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::SessionAlreadyExists` if the session already exists.
    /// Returns `FastTestError::InvalidSessionName` if the name is invalid.
    pub fn add_session(&self, name: &str) -> Result<Self, FastTestError> {
        if self.sessions.contains_key(name) {
            return Err(FastTestError::SessionAlreadyExists(name.to_string()));
        }

        let session = InMemorySession::new(name)?;
        let mut new_sessions = self.sessions.clone();
        new_sessions.insert(name.to_string(), session);

        let mut new_workspaces = self.workspaces.clone();
        new_workspaces.insert(
            name.to_string(),
            InMemoryWorkspace {
                name: name.to_string(),
                exists: true,
                has_changes: false,
                has_conflicts: false,
                current_branch: Some(format!("refs/heads/{name}")),
            },
        );

        Ok(Self {
            sessions: new_sessions,
            workspaces: new_workspaces,
            ..self.clone()
        })
    }

    /// Add a stacked session on top of an existing one
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::SessionNotFound` if the parent does not exist.
    /// Returns `FastTestError::SessionAlreadyExists` if the child already exists.
    pub fn add_stacked_session(
        &self,
        parent: &str,
        child: &str,
    ) -> Result<Self, FastTestError> {
        let parent_session = self
            .sessions
            .get(parent)
            .ok_or_else(|| FastTestError::SessionNotFound(parent.to_string()))?;

        if self.sessions.contains_key(child) {
            return Err(FastTestError::SessionAlreadyExists(child.to_string()));
        }

        let stacked = parent_session.create_stacked(child)?;
        let mut new_sessions = self.sessions.clone();
        new_sessions.insert(child.to_string(), stacked);

        Ok(Self {
            sessions: new_sessions,
            ..self.clone()
        })
    }

    /// Remove a session
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::SessionNotFound` if the session does not exist.
    pub fn remove_session(&self, name: &str) -> Result<Self, FastTestError> {
        if !self.sessions.contains_key(name) {
            return Err(FastTestError::SessionNotFound(name.to_string()));
        }

        let mut new_sessions = self.sessions.clone();
        new_sessions.remove(name);

        let mut new_workspaces = self.workspaces.clone();
        new_workspaces.remove(name);

        let new_current = if self.current_session.as_deref() == Some(name) {
            None
        } else {
            self.current_session.clone()
        };

        Ok(Self {
            sessions: new_sessions,
            workspaces: new_workspaces,
            current_session: new_current,
            ..self.clone()
        })
    }

    /// Focus a session
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::SessionNotFound` if the session does not exist.
    pub fn focus_session(&self, name: &str) -> Result<Self, FastTestError> {
        if !self.sessions.contains_key(name) {
            return Err(FastTestError::SessionNotFound(name.to_string()));
        }

        Ok(Self {
            current_session: Some(name.to_string()),
            ..self.clone()
        })
    }

    /// Get a session by name
    #[must_use]
    pub fn get_session(&self, name: &str) -> Option<&InMemorySession> {
        self.sessions.get(name)
    }

    /// Get all sessions
    #[must_use]
    pub fn all_sessions(&self) -> Vec<&InMemorySession> {
        self.sessions.values().collect()
    }

    /// Get session count
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // QUEUE MANAGEMENT
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Add a queue entry
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::InvalidSessionName` if the session name is invalid.
    pub fn add_queue_entry(&self, session: &str, priority: u8) -> Result<Self, FastTestError> {
        let id = self.next_queue_id;
        let entry = InMemoryQueueEntry::new(id, session, priority)?;

        let mut new_entries = self.queue_entries.clone();
        new_entries.insert(id, entry);

        Ok(Self {
            queue_entries: new_entries,
            next_queue_id: id + 1,
            ..self.clone()
        })
    }

    /// Update queue entry status
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::SessionNotFound` if the entry does not exist.
    pub fn update_queue_status(
        &self,
        id: i64,
        status: QueueEntryStatus,
    ) -> Result<Self, FastTestError> {
        let entry = self
            .queue_entries
            .get(&id)
            .ok_or_else(|| FastTestError::SessionNotFound(format!("queue entry {id}")))?;

        let updated = entry.clone().with_status(status);
        let mut new_entries = self.queue_entries.clone();
        new_entries.insert(id, updated);

        Ok(Self {
            queue_entries: new_entries,
            ..self.clone()
        })
    }

    /// Get all queue entries
    #[must_use]
    pub fn all_queue_entries(&self) -> Vec<&InMemoryQueueEntry> {
        self.queue_entries.values().sorted_by_key(|e| e.priority).collect()
    }

    /// Get queue summary
    #[must_use]
    pub fn queue_summary(&self) -> QueueSummary {
        let entries: Vec<_> = self.queue_entries.values().collect();

        let pending = entries
            .iter()
            .filter(|e| matches!(e.status, QueueEntryStatus::Pending))
            .count();
        let ready = entries
            .iter()
            .filter(|e| matches!(e.status, QueueEntryStatus::Ready))
            .count();
        let blocked = entries
            .iter()
            .filter(|e| matches!(e.status, QueueEntryStatus::Blocked))
            .count();
        let in_progress = entries
            .iter()
            .filter(|e| matches!(e.status, QueueEntryStatus::InProgress))
            .count();

        QueueSummary::new().with_counts(zjj_core::output::QueueCounts {
            total: u32::try_from(entries.len()).unwrap_or(u32::MAX),
            pending: u32::try_from(pending).unwrap_or(u32::MAX),
            ready: u32::try_from(ready).unwrap_or(u32::MAX),
            blocked: u32::try_from(blocked).unwrap_or(u32::MAX),
            in_progress: u32::try_from(in_progress).unwrap_or(u32::MAX),
        })
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // COMMAND SIMULATION
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Run a command in-memory and return the output
    ///
    /// # Errors
    ///
    /// Returns `FastTestError::UnknownCommand` if the command is not recognized.
    pub fn run_command(&self, command: &str) -> Result<CommandOutput, FastTestError> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        match parts.first().copied() {
            Some("list" | "status") => self.run_list_command(),
            Some("add") => self.run_add_command(&parts[1..]),
            Some("remove") => self.run_remove_command(&parts[1..]),
            Some("focus") => self.run_focus_command(&parts[1..]),
            Some("queue") => self.run_queue_command(&parts[1..]),
            Some("stack") => self.run_stack_command(&parts[1..]),
            Some(cmd) => Err(FastTestError::UnknownCommand(cmd.to_string())),
            None => Err(FastTestError::UnknownCommand(String::new())),
        }
    }

    fn run_list_command(&self) -> Result<CommandOutput, FastTestError> {
        let sessions: Vec<SessionOutput> = self
            .sessions
            .values()
            .filter_map(|s| s.to_session_output().ok())
            .collect();

        let lines: Vec<OutputLine> = sessions
            .into_iter()
            .map(OutputLine::Session)
            .collect();

        Ok(CommandOutput::new(lines))
    }

    fn run_add_command(&self, args: &[&str]) -> Result<CommandOutput, FastTestError> {
        let name = args.first().copied().unwrap_or("");

        let new_harness = self.add_session(name)?;

        let session = new_harness
            .get_session(name)
            .ok_or_else(|| FastTestError::SessionNotFound(name.to_string()))?;

        let output = session.to_session_output()?;

        let message = Message::new(format!("Created session: {name}"))
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        let summary = Summary::new(SummaryType::Info, message)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Ok(CommandOutput::new(vec![
            OutputLine::Session(output),
            OutputLine::Summary(summary),
        ]))
    }

    fn run_remove_command(&self, args: &[&str]) -> Result<CommandOutput, FastTestError> {
        let name = args.first().copied().unwrap_or("");

        let _ = self.remove_session(name)?;

        let message = Message::new(format!("Removed session: {name}"))
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        let result = ResultOutput::success(
            zjj_core::output::ResultKind::Command,
            message,
        )
        .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Ok(CommandOutput::new(vec![OutputLine::Result(result)]))
    }

    fn run_focus_command(&self, args: &[&str]) -> Result<CommandOutput, FastTestError> {
        let name = args.first().copied().unwrap_or("");

        let session = self
            .sessions
            .get(name)
            .ok_or_else(|| FastTestError::SessionNotFound(name.to_string()))?;

        let output = session.to_session_output()?;

        let message = Message::new(format!("Focused session: {name}"))
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        let summary = Summary::new(SummaryType::Info, message)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Ok(CommandOutput::new(vec![
            OutputLine::Session(output),
            OutputLine::Summary(summary),
        ]))
    }

    fn run_queue_command(&self, args: &[&str]) -> Result<CommandOutput, FastTestError> {
        match args.first().copied() {
            Some("list") => {
                let entries: Vec<QueueEntry> = self
                    .queue_entries
                    .values()
                    .filter_map(|e| e.to_queue_entry().ok())
                    .collect();

                let summary = self.queue_summary();

                let lines: Vec<OutputLine> = std::iter::once(OutputLine::QueueSummary(summary))
                    .chain(entries.into_iter().map(OutputLine::QueueEntry))
                    .collect();

                Ok(CommandOutput::new(lines))
            }
            Some("summary") => {
                let summary = self.queue_summary();
                Ok(CommandOutput::new(vec![OutputLine::QueueSummary(
                    summary,
                )]))
            }
            _ => Err(FastTestError::InvalidArgument {
                command: "queue".to_string(),
                details: "expected 'list' or 'summary'".to_string(),
            }),
        }
    }

    fn run_stack_command(&self, args: &[&str]) -> Result<CommandOutput, FastTestError> {
        let name = args.first().copied().unwrap_or("");

        // Find sessions in the stack (this session and its children)
        let stack_sessions: Vec<&InMemorySession> = if name.is_empty() {
            self.sessions.values().collect()
        } else {
            self.sessions
                .values()
                .filter(|s| s.name.as_str() == name || s.parent.as_deref() == Some(name))
                .collect()
        };

        if stack_sessions.is_empty() && !name.is_empty() {
            return Err(FastTestError::SessionNotFound(name.to_string()));
        }

        let first = stack_sessions.first().copied();
        let base_ref = BaseRef::new("main");

        let stack_result = if let Some(first_session) = first {
            let mut stack = Stack::new(first_session.name.clone(), base_ref)
                .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

            for (order, session) in stack_sessions.iter().enumerate() {
                let status = if session.state == WorkspaceState::Conflict {
                    StackEntryStatus::Failed
                } else {
                    StackEntryStatus::Ready
                };

                stack = stack
                    .with_entry(
                        session.name.clone(),
                        session.workspace_path.clone(),
                        status,
                        BeadAttachment::None,
                    )
                    .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

                // Add order check - we only want one entry per session
                if order > 0 {
                    break;
                }
            }

            stack
        } else {
            Stack::new(
                SessionName::parse("empty-stack")
                    .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?,
                base_ref,
            )
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
        };

        Ok(CommandOutput::new(vec![OutputLine::Stack(stack_result)]))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════════════════════

/// Output from a simulated command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Output lines
    pub lines: Vec<OutputLine>,
}

impl CommandOutput {
    /// Create new command output
    #[must_use]
    pub fn new(lines: Vec<OutputLine>) -> Self {
        Self { lines }
    }

    /// Create empty output
    #[must_use]
    pub fn empty() -> Self {
        Self { lines: Vec::new() }
    }

    /// Check if output contains a session with the given name
    #[must_use]
    pub fn contains_session(&self, name: &str) -> bool {
        self.lines.iter().any(|line| {
            if let OutputLine::Session(session) = line {
                session.name == name
            } else {
                false
            }
        })
    }

    /// Get all sessions in the output
    #[must_use]
    pub fn sessions(&self) -> Vec<&SessionOutput> {
        self.lines
            .iter()
            .filter_map(|line| {
                if let OutputLine::Session(session) = line {
                    Some(session)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the session count
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions().len()
    }

    /// Check if output contains a queue entry
    #[must_use]
    pub fn contains_queue_entry(&self, session: &str) -> bool {
        self.lines.iter().any(|line| {
            if let OutputLine::QueueEntry(entry) = line {
                entry.session.as_str() == session
            } else {
                false
            }
        })
    }

    /// Get queue summary if present
    #[must_use]
    pub fn queue_summary(&self) -> Option<&QueueSummary> {
        self.lines.iter().find_map(|line| {
            if let OutputLine::QueueSummary(summary) = line {
                Some(summary)
            } else {
                None
            }
        })
    }

    /// Convert to JSONL string
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        self.lines
            .iter()
            .filter_map(|line| serde_json::to_string(line).ok())
            .join("\n")
    }

    /// Parse from JSONL string
    ///
    /// # Errors
    ///
    /// Returns an error if JSON parsing fails.
    pub fn from_jsonl(jsonl: &str) -> Result<Self, serde_json::Error> {
        let lines: Result<Vec<OutputLine>, _> = jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect();

        lines.map(Self::new)
    }

    /// Check if output indicates success
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.lines.iter().all(|line| {
            if let OutputLine::Result(result) = line {
                matches!(result.outcome, Outcome::Success)
            } else {
                true
            }
        })
    }
}

impl Default for CommandOutput {
    fn default() -> Self {
        Self::empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON VALIDATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// JSON validation helpers for fast testing
pub struct JsonValidator;

impl JsonValidator {
    /// Validate that a string is valid JSON
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid JSON.
    pub fn is_valid_json(s: &str) -> Result<(), serde_json::Error> {
        let _: serde_json::Value = serde_json::from_str(s)?;
        Ok(())
    }

    /// Validate that a string is valid JSONL (JSON Lines)
    ///
    /// # Errors
    ///
    /// Returns an error if any line is not valid JSON.
    pub fn is_valid_jsonl(s: &str) -> Result<Vec<serde_json::Value>, serde_json::Error> {
        s.lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect()
    }

    /// Check if JSON has a specific field
    #[must_use]
    pub fn has_field(json: &serde_json::Value, field: &str) -> bool {
        json.get(field).is_some()
    }

    /// Check if JSON has the "type" field expected in OutputLine
    #[must_use]
    pub fn has_type_field(json: &serde_json::Value) -> bool {
        // OutputLine is serialized as an internally tagged enum
        json.get("session").is_some()
            || json.get("summary").is_some()
            || json.get("queue_entry").is_some()
            || json.get("queue_summary").is_some()
            || json.get("stack").is_some()
            || json.get("result").is_some()
            || json.get("action").is_some()
            || json.get("warning").is_some()
            || json.get("issue").is_some()
            || json.get("plan").is_some()
            || json.get("train").is_some()
    }

    /// Parse JSONL and extract all session names
    #[must_use]
    pub fn extract_session_names(jsonl: &str) -> Vec<String> {
        jsonl
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|json| {
                json.get("session")
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str())
                    .map(String::from)
            })
            .collect()
    }

    /// Validate session output structure
    #[must_use]
    pub fn is_valid_session_output(json: &serde_json::Value) -> bool {
        json.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| !n.is_empty())
            && json.get("status").is_some()
            && json.get("workspace_path").is_some()
    }

    /// Validate queue entry structure
    #[must_use]
    pub fn is_valid_queue_entry(json: &serde_json::Value) -> bool {
        json.get("id").is_some()
            && json
                .get("session")
                .and_then(|s| s.as_str())
                .is_some_and(|s| !s.is_empty())
            && json.get("status").is_some()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PRE-BUILT TEST FIXTURES
// ═══════════════════════════════════════════════════════════════════════════

/// Pre-built test fixtures for common scenarios
pub struct Fixtures;

impl Fixtures {
    /// Create a minimal harness with one session
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub fn minimal_harness() -> Result<FastTestHarness, FastTestError> {
        FastTestHarness::new().add_session("test")
    }

    /// Create a harness with multiple sessions
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub fn multi_session_harness() -> Result<FastTestHarness, FastTestError> {
        FastTestHarness::new()
            .add_session("feature-auth")?
            .add_session("feature-db")?
            .add_session("bugfix-timeout")?
            .add_session("refactor-config")
    }

    /// Create a harness with a stacked session
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub fn stacked_harness() -> Result<FastTestHarness, FastTestError> {
        FastTestHarness::new()
            .add_session("feature-auth")?
            .add_stacked_session("feature-auth", "feature-auth-tests")
    }

    /// Create a harness with queue entries
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn queue_harness() -> Result<FastTestHarness, FastTestError> {
        FastTestHarness::new()
            .add_session("task-1")?
            .add_session("task-2")?
            .add_session("task-3")?
            .add_queue_entry("task-1", 1)?
            .add_queue_entry("task-2", 5)?
            .add_queue_entry("task-3", 10)
    }

    /// Create a harness with a conflicted session
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub fn conflicted_harness() -> Result<FastTestHarness, FastTestError> {
        let harness = FastTestHarness::new().add_session("conflicted")?;

        // Update the session to have conflict state
        if let Some(session) = harness.get_session("conflicted") {
            let mut new_sessions = harness.sessions.clone();
            let conflicted = session.clone().with_state(WorkspaceState::Conflict);
            new_sessions.insert("conflicted".to_string(), conflicted);

            Ok(FastTestHarness {
                sessions: new_sessions,
                ..harness
            })
        } else {
            Ok(harness)
        }
    }

    /// Create a sample session output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_session_output() -> Result<SessionOutput, FastTestError> {
        SessionOutput::new(
            "feature-test".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            PathBuf::from("/mock/workspaces/feature-test"),
        )
        .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample queue entry output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_queue_entry() -> Result<QueueEntry, FastTestError> {
        let id = QueueEntryId::new(1).map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let session = SessionName::parse("task-1")
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;

        QueueEntry::new(id, session, 5).map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample stack output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_stack() -> Result<Stack, FastTestError> {
        let session = SessionName::parse("feature-auth")
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;
        let base_ref = BaseRef::new("main");

        Stack::new(session.clone(), base_ref)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_entry(
                session,
                PathBuf::from("/mock/workspaces/feature-auth"),
                StackEntryStatus::Ready,
                BeadAttachment::None,
            )
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample plan output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_plan() -> Result<Plan, FastTestError> {
        let title = PlanTitle::new("Merge Plan").map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let description = PlanDescription::new("Steps to merge the feature branch")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Plan::new(title, description)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_step("Fetch latest main".to_string(), ActionStatus::Completed)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_step("Rebase onto main".to_string(), ActionStatus::InProgress)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample action output
    #[must_use]
    pub fn sample_action() -> Action {
        let verb = ActionVerb::new("merge").unwrap_or(ActionVerb::Merge);
        let target = ActionTarget::new("feature-auth").unwrap_or_else(|_| ActionTarget::new("target").unwrap_or(ActionTarget::new("unknown").unwrap()));

        Action::new(verb, target, ActionStatus::InProgress)
    }

    /// Create a sample warning output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_warning() -> Result<Warning, FastTestError> {
        let code = WarningCode::new("MERGE_CONFLICT")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let message = Message::new("Conflicts detected in src/main.rs")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Warning::new(code, message).map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample issue output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_issue() -> Result<Issue, FastTestError> {
        let id = IssueId::new("ISSUE-001").map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let title = IssueTitle::new("Merge conflict in authentication module")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        Issue::new(id, title, IssueKind::StateConflict, IssueSeverity::Warning)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample train output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_train() -> Result<Train, FastTestError> {
        let id = TrainId::new("train-001").map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;
        let name = SessionName::parse("merge-train")
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;

        let session1 = SessionName::parse("feature-auth")
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;
        let session2 = SessionName::parse("feature-db")
            .map_err(|e| FastTestError::InvalidSessionName(e.to_string()))?;

        Train::new(id, name)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_step(session1, TrainAction::Rebase, TrainStepStatus::Success)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?
            .with_step(session2, TrainAction::Rebase, TrainStepStatus::Running)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample result output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_success_result() -> Result<ResultOutput, FastTestError> {
        let message = Message::new("Operation completed successfully")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        ResultOutput::success(zjj_core::output::ResultKind::Command, message)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }

    /// Create a sample failure result output
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails.
    pub fn sample_failure_result() -> Result<ResultOutput, FastTestError> {
        let message = Message::new("Operation failed: conflict detected")
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))?;

        ResultOutput::failure(zjj_core::output::ResultKind::Command, message)
            .map_err(|e| FastTestError::OutputConstruction(e.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // FAST TEST HARNESS TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_harness_creation() {
        let harness = FastTestHarness::new();
        assert_eq!(harness.session_count(), 0);
    }

    #[test]
    fn test_harness_with_fixtures() {
        let harness = FastTestHarness::with_fixtures().expect("fixtures should work");
        assert!(harness.session_count() >= 3);
    }

    #[test]
    fn test_add_session() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test-session").expect("add should work");

        assert_eq!(harness.session_count(), 1);
        assert!(harness.get_session("test-session").is_some());
    }

    #[test]
    fn test_add_duplicate_session() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let result = harness.add_session("test");
        assert!(matches!(result, Err(FastTestError::SessionAlreadyExists(_))));
    }

    #[test]
    fn test_add_session_invalid_name() {
        let harness = FastTestHarness::new();
        let result = harness.add_session("123-invalid");

        assert!(matches!(result, Err(FastTestError::InvalidSessionName(_))));
    }

    #[test]
    fn test_remove_session() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let harness = harness.remove_session("test").expect("remove should work");
        assert_eq!(harness.session_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent_session() {
        let harness = FastTestHarness::new();
        let result = harness.remove_session("nonexistent");

        assert!(matches!(result, Err(FastTestError::SessionNotFound(_))));
    }

    #[test]
    fn test_add_stacked_session() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("parent").expect("add should work");
        let harness = harness
            .add_stacked_session("parent", "child")
            .expect("stacked should work");

        let child = harness.get_session("child").expect("child should exist");
        assert_eq!(child.parent, Some("parent".to_string()));
        assert_eq!(child.depth, 1);
    }

    #[test]
    fn test_focus_session() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let harness = harness.focus_session("test").expect("focus should work");
        assert_eq!(harness.current_session, Some("test".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QUEUE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_queue_entry() {
        let harness = FastTestHarness::new();
        let harness = harness
            .add_queue_entry("task-1", 5)
            .expect("add queue should work");

        let entries = harness.all_queue_entries();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_queue_summary() {
        let harness = FastTestHarness::new()
            .add_session("t1").expect("add session")
            .add_queue_entry("t1", 1).expect("add queue")
            .add_session("t2").expect("add session")
            .add_queue_entry("t2", 5).expect("add queue");

        let summary = harness.queue_summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.pending, 2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // COMMAND SIMULATION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_run_list_command() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let output = harness.run_command("list").expect("list should work");
        assert_eq!(output.session_count(), 1);
        assert!(output.contains_session("test"));
    }

    #[test]
    fn test_run_add_command() {
        let harness = FastTestHarness::new();

        let output = harness.run_command("add new-session").expect("add should work");
        assert!(output.contains_session("new-session"));
    }

    #[test]
    fn test_run_remove_command() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let output = harness.run_command("remove test").expect("remove should work");
        assert!(output.is_success());
    }

    #[test]
    fn test_run_queue_list_command() {
        let harness = FastTestHarness::new()
            .add_session("t1").expect("add session")
            .add_queue_entry("t1", 1).expect("add queue");

        let output = harness.run_command("queue list").expect("queue list should work");
        assert!(output.contains_queue_entry("t1"));
        assert!(output.queue_summary().is_some());
    }

    #[test]
    fn test_unknown_command() {
        let harness = FastTestHarness::new();
        let result = harness.run_command("unknown");

        assert!(matches!(result, Err(FastTestError::UnknownCommand(_))));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JSON VALIDATION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_is_valid_json() {
        assert!(JsonValidator::is_valid_json(r#"{"test": true}"#).is_ok());
        assert!(JsonValidator::is_valid_json("not json").is_err());
    }

    #[test]
    fn test_is_valid_jsonl() {
        let jsonl = r#"{"session": {"name": "test"}}
{"summary": {"message": "done"}}"#;

        let result = JsonValidator::is_valid_jsonl(jsonl);
        assert!(result.is_ok());
        let values = result.expect("parsed values");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_has_field() {
        let json: serde_json::Value = serde_json::json!({"name": "test", "status": "active"});

        assert!(JsonValidator::has_field(&json, "name"));
        assert!(JsonValidator::has_field(&json, "status"));
        assert!(!JsonValidator::has_field(&json, "missing"));
    }

    #[test]
    fn test_extract_session_names() {
        let jsonl = r#"{"session": {"name": "test1", "status": "active"}}
{"session": {"name": "test2", "status": "paused"}}
{"summary": {"message": "done"}}"#;

        let names = JsonValidator::extract_session_names(jsonl);
        assert_eq!(names, vec!["test1", "test2"]);
    }

    #[test]
    fn test_is_valid_session_output() {
        let valid = serde_json::json!({
            "name": "test",
            "status": "active",
            "workspace_path": "/tmp/test"
        });
        let invalid = serde_json::json!({
            "status": "active"
        });

        assert!(JsonValidator::is_valid_session_output(&valid));
        assert!(!JsonValidator::is_valid_session_output(&invalid));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FIXTURE TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_minimal_harness_fixture() {
        let harness = Fixtures::minimal_harness().expect("fixture should work");
        assert_eq!(harness.session_count(), 1);
    }

    #[test]
    fn test_multi_session_harness_fixture() {
        let harness = Fixtures::multi_session_harness().expect("fixture should work");
        assert!(harness.session_count() >= 4);
    }

    #[test]
    fn test_stacked_harness_fixture() {
        let harness = Fixtures::stacked_harness().expect("fixture should work");

        let child = harness.get_session("feature-auth-tests").expect("child exists");
        assert_eq!(child.parent, Some("feature-auth".to_string()));
    }

    #[test]
    fn test_queue_harness_fixture() {
        let harness = Fixtures::queue_harness().expect("fixture should work");

        let entries = harness.all_queue_entries();
        assert_eq!(entries.len(), 3);

        // Should be sorted by priority
        assert_eq!(entries[0].priority, 1);
        assert_eq!(entries[1].priority, 5);
        assert_eq!(entries[2].priority, 10);
    }

    #[test]
    fn test_conflicted_harness_fixture() {
        let harness = Fixtures::conflicted_harness().expect("fixture should work");

        let session = harness.get_session("conflicted").expect("session exists");
        assert_eq!(session.state, WorkspaceState::Conflict);
    }

    #[test]
    fn test_sample_session_output_fixture() {
        let output = Fixtures::sample_session_output().expect("fixture should work");
        assert_eq!(output.name, "feature-test");
    }

    #[test]
    fn test_sample_queue_entry_fixture() {
        let entry = Fixtures::sample_queue_entry().expect("fixture should work");
        assert_eq!(entry.session.as_str(), "task-1");
    }

    #[test]
    fn test_sample_stack_fixture() {
        let stack = Fixtures::sample_stack().expect("fixture should work");
        assert_eq!(stack.name.as_str(), "feature-auth");
    }

    #[test]
    fn test_sample_plan_fixture() {
        let plan = Fixtures::sample_plan().expect("fixture should work");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn test_sample_action_fixture() {
        let action = Fixtures::sample_action();
        assert_eq!(action.status, ActionStatus::InProgress);
    }

    #[test]
    fn test_sample_warning_fixture() {
        let warning = Fixtures::sample_warning().expect("fixture should work");
        assert_eq!(warning.code.as_str(), "MERGE_CONFLICT");
    }

    #[test]
    fn test_sample_issue_fixture() {
        let issue = Fixtures::sample_issue().expect("fixture should work");
        assert_eq!(issue.kind, IssueKind::StateConflict);
    }

    #[test]
    fn test_sample_train_fixture() {
        let train = Fixtures::sample_train().expect("fixture should work");
        assert_eq!(train.steps.len(), 2);
    }

    #[test]
    fn test_sample_result_fixtures() {
        let success = Fixtures::sample_success_result().expect("fixture should work");
        assert!(matches!(success.outcome, Outcome::Success));

        let failure = Fixtures::sample_failure_result().expect("fixture should work");
        assert!(matches!(failure.outcome, Outcome::Failure));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // COMMAND OUTPUT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_command_output_to_jsonl() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let output = harness.run_command("list").expect("list should work");
        let jsonl = output.to_jsonl();

        assert!(jsonl.contains("test"));
        assert!(JsonValidator::is_valid_jsonl(&jsonl).is_ok());
    }

    #[test]
    fn test_command_output_from_jsonl() {
        // Use proper JSON format - timestamps are milliseconds since epoch
        let jsonl = r#"{"session": {"name": "test", "status": "active", "state": "created", "workspace_path": "/tmp/test", "created_at": 1234567890000, "updated_at": 1234567890000}}"#;

        let output = CommandOutput::from_jsonl(jsonl).expect("parse should work");
        assert_eq!(output.session_count(), 1);
    }

    #[test]
    fn test_command_output_is_success() {
        let harness = FastTestHarness::new();
        let harness = harness.add_session("test").expect("add should work");

        let output = harness.run_command("remove test").expect("remove should work");
        assert!(output.is_success());
    }
}

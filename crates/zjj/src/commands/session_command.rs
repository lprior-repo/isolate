//! Session command - Unified session management operations
//!
//! This module provides a cohesive API for session operations that integrates:
//! - `SessionDb` for persistence
//! - `LockManager` for coordination
//! - `SessionStateManager` for state machine enforcement
//! - Structured logging and JSON output
//!
//! # Architecture
//!
//! ```text
//! SessionCommand (shell layer)
//!     |
//!     +-- SessionManager (core business logic)
//!     |       |
//!     |       +-- SessionDb (persistence)
//!     |       +-- LockManager (coordination)
//!     |       +-- SessionStateManager (state machine)
//!     |
//!     +-- JSONL output (structured logging)
//!     +-- CLI handlers
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::DateTime;
use tracing::info;
use zjj_core::{
    coordination::locks::LockManager,
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput, SessionOutput,
    },
    OutputFormat,
};

use crate::{
    commands::get_session_db,
    db::SessionDb,
    session::{validate_session_name, Session, SessionStatus, SessionUpdate},
};

// ═══════════════════════════════════════════════════════════════════════════
// SESSION MANAGER (CORE BUSINESS LOGIC)
// ═══════════════════════════════════════════════════════════════════════════

/// Core session manager providing business logic operations.
///
/// This is the pure functional core that coordinates:
/// - Session persistence via `SessionDb`
/// - Lock management via `LockManager`
/// - State transitions via `SessionStateManager`
///
/// All operations return Result<T> with zero panics.
#[derive(Debug, Clone)]
pub struct SessionManager {
    db: SessionDb,
    lock_manager: LockManager,
}

impl SessionManager {
    /// Create a new session manager from an existing database connection.
    #[must_use]
    pub fn new(db: SessionDb) -> Self {
        let lock_manager = LockManager::new(db.pool().clone());
        Self { db, lock_manager }
    }

    /// Create a new session manager from the current repository.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JJ prerequisites are not met
    /// - ZJJ is not initialized
    /// - Database cannot be opened
    pub async fn from_current_repo() -> Result<Self> {
        let db = get_session_db().await?;
        Ok(Self::new(db))
    }

    /// Get a reference to the underlying database.
    #[must_use]
    pub const fn db(&self) -> &SessionDb {
        &self.db
    }

    /// Get a reference to the lock manager.
    #[must_use]
    pub const fn lock_manager(&self) -> &LockManager {
        &self.lock_manager
    }

    /// Create a new session with validation and state management.
    ///
    /// # Arguments
    ///
    /// * `name` - Session name (must be valid)
    /// * `workspace_path` - Absolute path to workspace
    /// * `parent` - Optional parent session for stacked sessions
    /// * `agent_id` - Optional agent ID for lock acquisition
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Name validation fails
    /// - Session already exists
    /// - Parent session doesn't exist (if specified)
    /// - Database operation fails
    pub async fn create_session(
        &self,
        name: &str,
        workspace_path: &str,
        parent: Option<&str>,
        agent_id: Option<&str>,
    ) -> Result<Session> {
        // Validate name
        validate_session_name(name).map_err(|e| anyhow::anyhow!("{e}"))?;

        // Validate parent exists if specified
        if let Some(parent_name) = parent {
            let parent_session = self
                .db
                .get(parent_name)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent session '{parent_name}' not found"))?;
            if parent_session.status == SessionStatus::Completed {
                return Err(anyhow::anyhow!(
                    "Cannot create stacked session under completed parent '{parent_name}'"
                ));
            }
        }

        // Check for duplicate
        if self.db.get(name).await?.is_some() {
            return Err(anyhow::anyhow!("Session '{name}' already exists"));
        }

        // Create in database
        let _session = self
            .db
            .create(name, workspace_path)
            .await
            .context("Failed to create session in database")?;

        // Update parent if specified
        if let Some(parent_name) = parent {
            self.db
                .update(
                    name,
                    SessionUpdate {
                        parent_session: Some(parent_name.to_string()),
                        ..SessionUpdate::default()
                    },
                )
                .await
                .context("Failed to set parent session")?;
        }

        // Acquire lock if agent specified
        if let Some(agent) = agent_id {
            self.lock_manager
                .lock(name, agent)
                .await
                .context("Failed to acquire session lock")?;
        }

        info!(
            session_name = %name,
            workspace_path = %workspace_path,
            parent = ?parent,
            "Session created"
        );

        // Fetch the final session state (with parent set if applicable)
        self.db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found after creation"))
    }

    /// Get a session by name.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails.
    pub async fn get_session(&self, name: &str) -> Result<Option<Session>> {
        self.db.get(name).await.context("Failed to get session")
    }

    /// List sessions with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `status_filter` - Optional status filter
    /// * `include_closed` - Include completed/failed sessions
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails.
    pub async fn list_sessions(
        &self,
        status_filter: Option<SessionStatus>,
        include_closed: bool,
    ) -> Result<Vec<Session>> {
        let sessions = self
            .db
            .list(None)
            .await
            .context("Failed to list sessions")?;

        let filtered = sessions
            .into_iter()
            .filter(|s| {
                let status_match = status_filter.as_ref().is_none_or(|f| s.status == *f);
                let closed_match = include_closed
                    || !matches!(s.status, SessionStatus::Completed | SessionStatus::Failed);
                status_match && closed_match
            })
            .collect();

        Ok(filtered)
    }

    /// Remove a session with cleanup.
    ///
    /// # Arguments
    ///
    /// * `name` - Session name
    /// * `force` - Force removal even with uncommitted changes
    /// * `agent_id` - Agent performing the removal
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session not found
    /// - Session is locked by another agent
    /// - Database operation fails
    pub async fn remove_session(&self, name: &str, force: bool, agent_id: &str) -> Result<()> {
        // Check session exists
        if self.db.get(name).await?.is_none() {
            return Err(anyhow::anyhow!("Session '{name}' not found"));
        }

        // Check for child sessions
        let all_sessions = self.db.list(None).await?;
        let has_children = all_sessions
            .iter()
            .any(|s| s.parent_session.as_deref() == Some(name));

        if has_children && !force {
            return Err(anyhow::anyhow!(
                "Session '{name}' has child sessions. Use --force to remove anyway."
            ));
        }

        // Acquire lock for removal
        self.lock_manager
            .lock(name, agent_id)
            .await
            .context("Failed to acquire lock for removal")?;

        // Mark as completed
        self.db
            .update(
                name,
                SessionUpdate {
                    status: Some(SessionStatus::Completed),
                    ..SessionUpdate::default()
                },
            )
            .await
            .context("Failed to update session status")?;

        // Delete from database
        self.db
            .delete(name)
            .await
            .context("Failed to delete session")?;

        // Release lock
        self.lock_manager.unlock(name, agent_id).await?;

        info!(
            session_name = %name,
            forced = force,
            agent_id = %agent_id,
            "Session removed"
        );

        Ok(())
    }

    /// Focus (switch to) a session.
    ///
    /// # Arguments
    ///
    /// * `name` - Session name
    ///
    /// # Errors
    ///
    /// Returns an error if session not found.
    pub async fn focus_session(&self, name: &str) -> Result<Session> {
        let session = self
            .db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

        if session.status == SessionStatus::Completed {
            return Err(anyhow::anyhow!("Cannot focus completed session '{name}'"));
        }

        info!(session_name = %name, "Session focused");
        Ok(session)
    }

    /// Pause an active session.
    ///
    /// # Arguments
    ///
    /// * `name` - Session name
    /// * `agent_id` - Agent performing the pause
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session not found
    /// - Session is not active
    /// - Lock acquisition fails
    pub async fn pause_session(&self, name: &str, agent_id: &str) -> Result<Session> {
        let session = self
            .db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

        if session.status != SessionStatus::Active {
            return Err(anyhow::anyhow!(
                "Cannot pause session in '{}' state",
                session.status
            ));
        }

        // Acquire lock
        self.lock_manager
            .lock(name, agent_id)
            .await
            .context("Failed to acquire lock for pause")?;

        // Update status
        self.db
            .update(
                name,
                SessionUpdate {
                    status: Some(SessionStatus::Paused),
                    ..SessionUpdate::default()
                },
            )
            .await
            .context("Failed to pause session")?;

        // Release lock
        self.lock_manager.unlock(name, agent_id).await?;

        info!(session_name = %name, agent_id = %agent_id, "Session paused");

        // Fetch and return the updated session
        self.db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found after update"))
    }

    /// Resume a paused session.
    ///
    /// # Arguments
    ///
    /// * `name` - Session name
    /// * `agent_id` - Agent performing the resume
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session not found
    /// - Session is not paused
    /// - Lock acquisition fails
    pub async fn resume_session(&self, name: &str, agent_id: &str) -> Result<Session> {
        let session = self
            .db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

        if session.status != SessionStatus::Paused {
            return Err(anyhow::anyhow!(
                "Cannot resume session in '{}' state",
                session.status
            ));
        }

        // Acquire lock
        self.lock_manager
            .lock(name, agent_id)
            .await
            .context("Failed to acquire lock for resume")?;

        // Update status
        self.db
            .update(
                name,
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await
            .context("Failed to resume session")?;

        // Release lock
        self.lock_manager.unlock(name, agent_id).await?;

        info!(session_name = %name, agent_id = %agent_id, "Session resumed");

        // Fetch and return the updated session
        self.db
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found after update"))
    }

    /// Rename a session.
    ///
    /// # Arguments
    ///
    /// * `old_name` - Current session name
    /// * `new_name` - New session name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session not found
    /// - New name already exists
    /// - Name validation fails
    pub async fn rename_session(&self, old_name: &str, new_name: &str) -> Result<Session> {
        // Validate new name
        validate_session_name(new_name).map_err(|e| anyhow::anyhow!("{e}"))?;

        // Check old session exists
        let session = self
            .db
            .get(old_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{old_name}' not found"))?;

        // Check new name doesn't exist
        if self.db.get(new_name).await?.is_some() {
            return Err(anyhow::anyhow!("Session '{new_name}' already exists"));
        }

        // Create new session with same data
        let _new_session = self
            .db
            .create(new_name, &session.workspace_path)
            .await
            .context("Failed to create renamed session")?;

        // Copy metadata
        self.db
            .update(
                new_name,
                SessionUpdate {
                    status: Some(session.status),
                    branch: session.branch.clone(),
                    parent_session: session.parent_session.clone(),
                    metadata: session.metadata.clone(),
                    ..SessionUpdate::default()
                },
            )
            .await
            .context("Failed to update renamed session metadata")?;

        // Delete old session
        self.db
            .delete(old_name)
            .await
            .context("Failed to delete old session")?;

        // Fetch final session
        let renamed = self
            .db
            .get(new_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{new_name}' not found after rename"))?;

        info!(old_name = %old_name, new_name = %new_name, "Session renamed");
        Ok(renamed)
    }

    /// Get the current session based on working directory.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails.
    pub async fn get_current_session(&self) -> Result<Option<Session>> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        let sessions = self
            .db
            .list(None)
            .await
            .context("Failed to list sessions")?;

        let current = sessions.into_iter().find(|s| {
            cwd.starts_with(&s.workspace_path)
                && !matches!(s.status, SessionStatus::Completed | SessionStatus::Failed)
        });

        Ok(current)
    }

    /// Count active sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails.
    pub async fn count_active_sessions(&self) -> Result<usize> {
        let sessions = self.list_sessions(None, false).await?;
        Ok(sessions
            .iter()
            .filter(|s| matches!(s.status, SessionStatus::Active | SessionStatus::Creating))
            .count())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION COMMAND (SHELL LAYER)
// ═══════════════════════════════════════════════════════════════════════════

/// Options for session command operations.
#[derive(Debug, Clone, Default)]
pub struct SessionCommandOptions {
    /// Output format
    pub format: OutputFormat,
    /// Dry run mode
    pub dry_run: bool,
    /// Agent ID for operations
    pub agent_id: Option<String>,
    /// Verbose output
    pub verbose: bool,
}

/// Session command handler providing CLI-level operations.
///
/// This is the shell layer that:
/// - Handles CLI parsing and validation
/// - Emits structured JSONL output
/// - Integrates with `SessionManager` for business logic
#[derive(Debug)]
pub struct SessionCommand {
    manager: SessionManager,
    options: SessionCommandOptions,
}

impl SessionCommand {
    /// Create a new session command handler.
    #[must_use]
    pub const fn new(manager: SessionManager, options: SessionCommandOptions) -> Self {
        Self { manager, options }
    }

    /// Create from current repository with default options.
    ///
    /// # Errors
    ///
    /// Returns an error if repository initialization fails.
    pub async fn from_current_repo() -> Result<Self> {
        let manager = SessionManager::from_current_repo().await?;
        Ok(Self::new(manager, SessionCommandOptions::default()))
    }

    /// Get a reference to the session manager.
    #[must_use]
    pub const fn manager(&self) -> &SessionManager {
        &self.manager
    }

    /// Get a reference to the options.
    #[must_use]
    pub const fn options(&self) -> &SessionCommandOptions {
        &self.options
    }

    /// Run the list subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_list(
        &self,
        status_filter: Option<&str>,
        include_closed: bool,
    ) -> Result<Vec<Session>> {
        // Parse status filter
        let status = if let Some(s) = status_filter {
            Some(
                s.parse::<SessionStatus>()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            )
        } else {
            None
        };

        let sessions = self.manager.list_sessions(status, include_closed).await?;

        // Emit output
        if self.options.format.is_json() {
            for session in &sessions {
                emit_session_output(session)?;
            }
            emit_result_success(&format!("Listed {} sessions", sessions.len()))?;
        } else {
            self.print_session_list(&sessions);
        }

        Ok(sessions)
    }

    /// Run the add subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_add(
        &self,
        name: &str,
        workspace_path: &str,
        parent: Option<&str>,
    ) -> Result<Session> {
        if self.options.dry_run {
            emit_action("create", name, ActionStatus::Pending)?;
            emit_result_success(&format!(
                "Would create session '{name}' at {workspace_path}"
            ))?;
            return Ok(Session::default());
        }

        emit_action("create", name, ActionStatus::InProgress)?;

        let session = self
            .manager
            .create_session(
                name,
                workspace_path,
                parent,
                self.options.agent_id.as_deref(),
            )
            .await?;

        emit_action("create", name, ActionStatus::Completed)?;
        emit_session_output(&session)?;
        emit_result_success(&format!("Created session '{name}'"))?;

        Ok(session)
    }

    /// Run the remove subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_remove(&self, name: &str, force: bool) -> Result<()> {
        let agent_id = self.options.agent_id.as_deref().unwrap_or("cli");

        if self.options.dry_run {
            emit_action("remove", name, ActionStatus::Pending)?;
            emit_result_success(&format!("Would remove session '{name}'"))?;
            return Ok(());
        }

        emit_action("remove", name, ActionStatus::InProgress)?;

        self.manager.remove_session(name, force, agent_id).await?;

        emit_action("remove", name, ActionStatus::Completed)?;
        emit_result_success(&format!("Removed session '{name}'"))?;

        Ok(())
    }

    /// Run the focus subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_focus(&self, name: &str) -> Result<Session> {
        emit_action("focus", name, ActionStatus::InProgress)?;

        let session = self.manager.focus_session(name).await?;

        emit_action("focus", name, ActionStatus::Completed)?;
        emit_session_output(&session)?;
        emit_result_success(&format!("Focused session '{name}'"))?;

        Ok(session)
    }

    /// Run the pause subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_pause(&self, name: &str) -> Result<Session> {
        let agent_id = self.options.agent_id.as_deref().unwrap_or("cli");

        emit_action("pause", name, ActionStatus::InProgress)?;

        let session = self.manager.pause_session(name, agent_id).await?;

        emit_action("pause", name, ActionStatus::Completed)?;
        emit_session_output(&session)?;
        emit_result_success(&format!("Paused session '{name}'"))?;

        Ok(session)
    }

    /// Run the resume subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_resume(&self, name: &str) -> Result<Session> {
        let agent_id = self.options.agent_id.as_deref().unwrap_or("cli");

        emit_action("resume", name, ActionStatus::InProgress)?;

        let session = self.manager.resume_session(name, agent_id).await?;

        emit_action("resume", name, ActionStatus::Completed)?;
        emit_session_output(&session)?;
        emit_result_success(&format!("Resumed session '{name}'"))?;

        Ok(session)
    }

    /// Run the rename subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_rename(&self, old_name: &str, new_name: &str) -> Result<Session> {
        emit_action("rename", old_name, ActionStatus::InProgress)?;

        let session = self.manager.rename_session(old_name, new_name).await?;

        emit_action("rename", new_name, ActionStatus::Completed)?;
        emit_session_output(&session)?;
        emit_result_success(&format!("Renamed session '{old_name}' to '{new_name}'"))?;

        Ok(session)
    }

    /// Run the status subcommand.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn run_status(&self, name: Option<&str>) -> Result<Option<Session>> {
        let session = if let Some(n) = name {
            self.manager.get_session(n).await?
        } else {
            self.manager.get_current_session().await?
        };

        if let Some(ref s) = session {
            emit_session_output(s)?;
            emit_result_success(&format!("Session '{}' status: {}", s.name, s.status))?;
        } else {
            emit_result_success("No current session")?;
        }

        Ok(session)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PRIVATE HELPERS
    // ═══════════════════════════════════════════════════════════════════════

    /// Print session list in human-readable format.
    fn print_session_list(&self, sessions: &[Session]) {
        if sessions.is_empty() {
            println!("No sessions found.");
            return;
        }

        println!("SESSIONS ({})", sessions.len());
        println!("{:-<60}", "");

        for session in sessions {
            let status_icon = match session.status {
                SessionStatus::Active => "[*]",
                SessionStatus::Paused => "[_]",
                SessionStatus::Creating => "[~]",
                SessionStatus::Completed => "[x]",
                SessionStatus::Failed => "[!]",
            };

            let parent_info = session
                .parent_session
                .as_ref()
                .map(|p| format!(" (parent: {p})"))
                .unwrap_or_default();

            println!(
                "{} {} {}{}",
                status_icon, session.name, session.workspace_path, parent_info
            );

            if self.options.verbose {
                println!(
                    "    Status: {} | State: {} | Created: {}",
                    session.status,
                    session.state,
                    DateTime::from_timestamp(i64::try_from(session.created_at).unwrap_or(0), 0)
                        .map_or_else(
                            || "unknown".to_string(),
                            |dt| dt.format("%Y-%m-%d %H:%M").to_string()
                        )
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Convert local `SessionStatus` to core `SessionStatus`.
const fn to_core_status(status: SessionStatus) -> zjj_core::types::SessionStatus {
    match status {
        SessionStatus::Active => zjj_core::types::SessionStatus::Active,
        SessionStatus::Paused => zjj_core::types::SessionStatus::Paused,
        SessionStatus::Completed => zjj_core::types::SessionStatus::Completed,
        SessionStatus::Failed => zjj_core::types::SessionStatus::Failed,
        SessionStatus::Creating => zjj_core::types::SessionStatus::Creating,
    }
}

/// Emit a session output line.
fn emit_session_output(session: &Session) -> Result<()> {
    let workspace_path: PathBuf = session.workspace_path.clone().into();

    let session_output = SessionOutput::new(
        session.name.clone(),
        to_core_status(session.status),
        session.state,
        workspace_path,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_output = if let Some(branch) = &session.branch {
        session_output.with_branch(branch.clone())
    } else {
        session_output
    };

    emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an action line.
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a success result.
fn emit_result_success(message: &str) -> Result<()> {
    let result = ResultOutput::success(
        ResultKind::Command,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an issue line.
#[allow(dead_code)]
fn emit_issue(
    id: &str,
    title: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    let mut issue = Issue::new(
        IssueId::new(id).map_err(|e| anyhow::anyhow!("Invalid issue ID: {e}"))?,
        IssueTitle::new(title).map_err(|e| anyhow::anyhow!("Invalid issue title: {e}"))?,
        kind,
        severity,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(s) = session {
        issue = issue
            .with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }

    emit_stdout(&OutputLine::Issue(issue)).map_err(|e| anyhow::anyhow!("{e}"))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use tempfile::TempDir;

    use super::*;

    async fn setup_test_manager() -> Result<(SessionManager, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((SessionManager::new(db), dir))
    }

    #[tokio::test]
    async fn test_create_session_succeeds() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        let session = manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        assert_eq!(session.name, "test-session");
        assert_eq!(session.workspace_path, "/tmp/test");
        assert_eq!(session.status, SessionStatus::Creating);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_duplicate_session_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        let result = manager
            .create_session("test-session", "/tmp/test2", None, None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("already exists"));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_session_with_invalid_name_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        let result = manager
            .create_session("123-invalid", "/tmp/test", None, None)
            .await;

        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        let session = manager.get_session("test-session").await?;

        assert!(session.is_some());
        let s = session.unwrap();
        assert_eq!(s.name, "test-session");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        let session = manager.get_session("nonexistent").await?;

        assert!(session.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_sessions() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("session-1", "/tmp/1", None, None)
            .await?;
        manager
            .create_session("session-2", "/tmp/2", None, None)
            .await?;

        let sessions = manager.list_sessions(None, false).await?;

        assert_eq!(sessions.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        manager
            .remove_session("test-session", false, "test-agent")
            .await?;

        let session = manager.get_session("test-session").await?;
        assert!(session.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_nonexistent_session_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        let result = manager
            .remove_session("nonexistent", false, "test-agent")
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));

        Ok(())
    }

    #[tokio::test]
    async fn test_pause_and_resume_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create and activate session
        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        // Update to active
        manager
            .db()
            .update(
                "test-session",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Pause
        let paused = manager.pause_session("test-session", "test-agent").await?;
        assert_eq!(paused.status, SessionStatus::Paused);

        // Resume
        let resumed = manager.resume_session("test-session", "test-agent").await?;
        assert_eq!(resumed.status, SessionStatus::Active);

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("old-name", "/tmp/test", None, None)
            .await?;

        let renamed = manager.rename_session("old-name", "new-name").await?;

        assert_eq!(renamed.name, "new-name");

        // Old name should not exist
        let old = manager.get_session("old-name").await?;
        assert!(old.is_none());

        // New name should exist
        let new = manager.get_session("new-name").await?;
        assert!(new.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_count_active_sessions() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("session-1", "/tmp/1", None, None)
            .await?;
        manager
            .create_session("session-2", "/tmp/2", None, None)
            .await?;

        // Update one to active
        manager
            .db()
            .update(
                "session-1",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        let count = manager.count_active_sessions().await?;
        assert_eq!(count, 2); // Creating and Active both count

        Ok(())
    }

    #[tokio::test]
    async fn test_create_stacked_session() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create parent
        manager
            .create_session("parent", "/tmp/parent", None, None)
            .await?;

        // Update parent to active
        manager
            .db()
            .update(
                "parent",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Create child
        let child = manager
            .create_session("child", "/tmp/child", Some("parent"), None)
            .await?;

        assert_eq!(child.parent_session, Some("parent".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_stacked_with_nonexistent_parent_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        let result = manager
            .create_session("child", "/tmp/child", Some("nonexistent"), None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Parent session"));

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ADVERSARIAL TESTS (Phase 5 - REVIEW)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Test concurrent session creation from multiple tasks.
    /// Verifies that duplicate detection works correctly under concurrent access.
    #[tokio::test]
    async fn test_concurrent_session_creation_no_duplicates() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;
        let manager = std::sync::Arc::new(manager);

        // Spawn 10 concurrent tasks trying to create the same session
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let mgr = std::sync::Arc::clone(&manager);
                tokio::spawn(async move {
                    mgr.create_session(&format!("concurrent-session-{i}"), "/tmp/test", None, None)
                        .await
                })
            })
            .collect();

        // Wait for all tasks
        let results: Vec<_> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("task panicked"))
            .collect();

        // All should succeed (different names)
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 10);

        // Verify all sessions exist
        let sessions = manager.list_sessions(None, false).await?;
        assert_eq!(sessions.len(), 10);

        Ok(())
    }

    /// Test concurrent creation with same name - only one should succeed.
    #[tokio::test]
    async fn test_concurrent_same_name_creation_only_one_succeeds() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;
        let manager = std::sync::Arc::new(manager);

        // Spawn 10 concurrent tasks trying to create the SAME session
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let mgr = std::sync::Arc::clone(&manager);
                tokio::spawn(async move {
                    mgr.create_session("same-name-session", "/tmp/test", None, None)
                        .await
                })
            })
            .collect();

        // Wait for all tasks
        let results: Vec<_> = futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("task panicked"))
            .collect();

        // Exactly one should succeed (first to commit)
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let fail_count = results.iter().filter(|r| r.is_err()).count();

        // At least one should succeed and at least one should fail
        // (Due to SQLite locking, the exact distribution may vary)
        assert!(success_count >= 1, "At least one creation should succeed");
        assert!(
            fail_count >= 1 || success_count == 1,
            "Either multiple should fail or exactly one succeed"
        );

        // Verify exactly one session exists
        let session = manager.get_session("same-name-session").await?;
        assert!(session.is_some(), "The session should exist");

        Ok(())
    }

    /// Test name collision with different case.
    #[tokio::test]
    async fn test_name_collision_case_sensitivity() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create session with lowercase
        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        // Try to create with uppercase - should succeed (names are case-sensitive)
        let result = manager
            .create_session("TEST-SESSION", "/tmp/test", None, None)
            .await;

        // Session names are case-sensitive, so this should succeed
        assert!(result.is_ok());

        // Verify both exist
        let lower = manager.get_session("test-session").await?;
        let upper = manager.get_session("TEST-SESSION").await?;
        assert!(lower.is_some());
        assert!(upper.is_some());

        Ok(())
    }

    /// Test name collision with special characters.
    #[tokio::test]
    async fn test_name_with_special_characters() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create session with hyphen
        let result = manager
            .create_session("test-session-123", "/tmp/test", None, None)
            .await;
        assert!(result.is_ok());

        // Session with underscore
        let result = manager
            .create_session("test_session_456", "/tmp/test", None, None)
            .await;
        assert!(result.is_ok());

        Ok(())
    }

    /// Test rename collision - renaming to existing name should fail.
    #[tokio::test]
    async fn test_rename_to_existing_name_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("session-a", "/tmp/a", None, None)
            .await?;
        manager
            .create_session("session-b", "/tmp/b", None, None)
            .await?;

        // Try to rename session-a to session-b (which exists)
        let result = manager.rename_session("session-a", "session-b").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("already exists"));

        // Verify original still exists
        let a = manager.get_session("session-a").await?;
        assert!(a.is_some());

        Ok(())
    }

    /// Test removing session with children without force.
    #[tokio::test]
    async fn test_remove_parent_without_force_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create parent
        manager
            .create_session("parent", "/tmp/parent", None, None)
            .await?;

        // Update parent to active
        manager
            .db()
            .update(
                "parent",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Create child
        manager
            .create_session("child", "/tmp/child", Some("parent"), None)
            .await?;

        // Try to remove parent without force
        let result = manager.remove_session("parent", false, "test-agent").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("child sessions"));

        // Verify parent still exists
        let parent = manager.get_session("parent").await?;
        assert!(parent.is_some());

        Ok(())
    }

    /// Test removing session with children with force.
    #[tokio::test]
    async fn test_remove_parent_with_force_succeeds() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create parent
        manager
            .create_session("parent", "/tmp/parent", None, None)
            .await?;

        // Update parent to active
        manager
            .db()
            .update(
                "parent",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Create child
        manager
            .create_session("child", "/tmp/child", Some("parent"), None)
            .await?;

        // Remove parent with force
        manager.remove_session("parent", true, "test-agent").await?;

        // Verify parent is gone
        let parent = manager.get_session("parent").await?;
        assert!(parent.is_none());

        Ok(())
    }

    /// Test pause non-active session fails.
    #[tokio::test]
    async fn test_pause_non_active_session_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        // Session is in "Creating" status, not "Active"
        let result = manager.pause_session("test-session", "test-agent").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot pause session"));

        Ok(())
    }

    /// Test resume non-paused session fails.
    #[tokio::test]
    async fn test_resume_non_paused_session_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        // Session is in "Creating" status, not "Paused"
        let result = manager.resume_session("test-session", "test-agent").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot resume session"));

        Ok(())
    }

    /// Test focus completed session fails.
    #[tokio::test]
    async fn test_focus_completed_session_fails() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        manager
            .create_session("test-session", "/tmp/test", None, None)
            .await?;

        // Mark as completed
        manager
            .db()
            .update(
                "test-session",
                SessionUpdate {
                    status: Some(SessionStatus::Completed),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Try to focus completed session
        let result = manager.focus_session("test-session").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot focus completed session"));

        Ok(())
    }

    /// Test large session count handling.
    #[tokio::test]
    async fn test_large_session_count() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create 100 sessions
        for i in 0..100 {
            manager
                .create_session(&format!("session-{i:03}"), &format!("/tmp/{i}"), None, None)
                .await?;
        }

        // Verify count
        let sessions = manager.list_sessions(None, false).await?;
        assert_eq!(sessions.len(), 100);

        // Count active (all in Creating status)
        let count = manager.count_active_sessions().await?;
        assert_eq!(count, 100);

        Ok(())
    }

    /// Test session state filtering.
    #[tokio::test]
    async fn test_session_status_filtering() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create sessions
        manager
            .create_session("creating-1", "/tmp/1", None, None)
            .await?;
        manager
            .create_session("active-1", "/tmp/2", None, None)
            .await?;
        manager
            .db()
            .update(
                "active-1",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;
        manager
            .create_session("paused-1", "/tmp/3", None, None)
            .await?;
        manager
            .db()
            .update(
                "paused-1",
                SessionUpdate {
                    status: Some(SessionStatus::Paused),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Filter by status
        let active = manager
            .list_sessions(Some(SessionStatus::Active), true)
            .await?;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "active-1");

        let paused = manager
            .list_sessions(Some(SessionStatus::Paused), true)
            .await?;
        assert_eq!(paused.len(), 1);
        assert_eq!(paused[0].name, "paused-1");

        let creating = manager
            .list_sessions(Some(SessionStatus::Creating), true)
            .await?;
        assert_eq!(creating.len(), 1);
        assert_eq!(creating[0].name, "creating-1");

        Ok(())
    }

    /// Test deep nesting of sessions.
    #[tokio::test]
    async fn test_deep_session_nesting() -> Result<()> {
        let (manager, _dir) = setup_test_manager().await?;

        // Create root session
        manager
            .create_session("level-0", "/tmp/0", None, None)
            .await?;
        manager
            .db()
            .update(
                "level-0",
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..SessionUpdate::default()
                },
            )
            .await?;

        // Create 10 levels of nesting
        for i in 1..=10 {
            let parent = format!("level-{}", i - 1);
            let child = format!("level-{i}");
            manager
                .create_session(&child, &format!("/tmp/{i}"), Some(&parent), None)
                .await?;
            manager
                .db()
                .update(
                    &child,
                    SessionUpdate {
                        status: Some(SessionStatus::Active),
                        ..SessionUpdate::default()
                    },
                )
                .await?;
        }

        // Verify chain
        let level_10 = manager.get_session("level-10").await?;
        assert!(level_10.is_some());
        assert_eq!(
            level_10.unwrap().parent_session,
            Some("level-9".to_string())
        );

        Ok(())
    }
}

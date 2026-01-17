//! Agent tracking command
//!
//! This module provides functionality to track and query AI agents working in sessions.
//! Agents can be spawned in sessions and their metadata (ID, task, artifacts, etc.)
//! is stored in the session metadata field.

pub mod list;

use anyhow::Result;

/// Agent command options
#[derive(Debug)]
#[allow(dead_code)]
pub struct AgentOptions {
    pub json: bool,
}

/// Run the agent list command
pub async fn run_list(session: Option<&str>, json: bool) -> Result<()> {
    list::run(session, json).await
}

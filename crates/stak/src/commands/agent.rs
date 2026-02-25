//! Agent command implementation
//!
//! Manages agent registration, heartbeats, and status.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::struct_excessive_bools)]

use anyhow::Result;
use chrono::Utc;
use stak_core::{Agent, AgentId, AgentStatus};

/// Agent command options
#[derive(Debug, Clone)]
pub struct AgentOptions {
    /// List agents
    pub list: bool,
    /// Register as agent
    pub register: bool,
    /// Send heartbeat
    pub heartbeat: bool,
    /// Unregister agent
    pub unregister: bool,
    /// Show agent status
    pub status: bool,
    /// Agent ID (optional, auto-generated if not provided)
    pub agent_id: Option<String>,
    /// Session to associate with agent
    pub session: Option<String>,
    /// Command being executed (for heartbeat)
    pub command: Option<String>,
    /// Include stale agents in list
    pub all: bool,
}

/// Agent registry (in-memory for now, will be database-backed)
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: Vec<Agent>,
}

impl AgentRegistry {
    /// Create a new agent registry
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new agent
    ///
    /// Returns a reference to the newly registered agent.
    pub fn register(&mut self, agent_id: AgentId) -> &Agent {
        let agent = Agent::new(agent_id);
        self.agents.push(agent);
        // SAFETY: We just pushed an agent, so last() is guaranteed to return Some
        self.agents
            .last()
            .unwrap_or_else(|| unreachable!("Agent was just pushed to the vector"))
    }

    /// Get an agent by ID
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id.as_str() == id)
    }

    /// Get mutable agent by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id.as_str() == id)
    }

    /// Unregister an agent
    pub fn unregister(&mut self, id: &str) -> bool {
        let initial_len = self.agents.len();
        self.agents.retain(|a| a.id.as_str() != id);
        self.agents.len() != initial_len
    }

    /// List all agents
    #[must_use]
    pub fn list(&self, include_stale: bool) -> Vec<&Agent> {
        if include_stale {
            self.agents.iter().collect()
        } else {
            self.agents.iter().filter(|a| a.is_active()).collect()
        }
    }

    /// Send heartbeat for an agent
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub fn heartbeat(&mut self, id: &str, command: Option<&str>) -> Result<()> {
        let agent = self
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Agent '{id}' not found"))?;

        agent.last_seen = Utc::now();
        agent.actions_count = agent.actions_count.saturating_add(1);
        if let Some(cmd) = command {
            agent.current_command = Some(cmd.to_string());
        }

        Ok(())
    }
}

/// Run the agent command
///
/// # Errors
///
/// Returns an error if:
/// - Agent registration fails
/// - Heartbeat fails (agent not found)
/// - Agent ID is invalid
pub fn run(options: &AgentOptions, registry: &mut AgentRegistry) -> Result<()> {
    if options.register {
        handle_register(options, registry)
    } else if options.heartbeat {
        handle_heartbeat(options, registry)
    } else if options.unregister {
        handle_unregister(options, registry)
    } else if options.status {
        handle_status(options, registry)
    } else {
        // Default to list
        handle_list(options, registry)
    }
}

/// Handle register command
fn handle_register(options: &AgentOptions, registry: &mut AgentRegistry) -> Result<()> {
    let agent_id = match &options.agent_id {
        Some(id) => AgentId::new(id),
        None => generate_agent_id(),
    };

    // Check if already registered
    if registry.get(agent_id.as_str()).is_some() {
        println!("Agent '{agent_id}' already registered");
        return Ok(());
    }

    registry.register(agent_id.clone());

    // Set environment variable
    std::env::set_var("STAK_AGENT_ID", agent_id.as_str());

    println!("Registered agent '{agent_id}'");
    if let Some(ref session) = options.session {
        println!("  Session: {session}");
    }
    println!("\nSet STAK_AGENT_ID={agent_id} in your environment");

    Ok(())
}

/// Handle heartbeat command
fn handle_heartbeat(options: &AgentOptions, registry: &mut AgentRegistry) -> Result<()> {
    let agent_id = std::env::var("STAK_AGENT_ID").map_err(|_| {
        anyhow::anyhow!("No agent registered. Set STAK_AGENT_ID or run 'stak agent register'")
    })?;

    registry.heartbeat(&agent_id, options.command.as_deref())?;

    println!("Heartbeat sent for agent '{agent_id}'");

    Ok(())
}

/// Handle unregister command
fn handle_unregister(options: &AgentOptions, registry: &mut AgentRegistry) -> Result<()> {
    let agent_id = options
        .agent_id
        .clone()
        .or_else(|| std::env::var("STAK_AGENT_ID").ok())
        .ok_or_else(|| anyhow::anyhow!("No agent ID provided. Set STAK_AGENT_ID or use --id"))?;

    if registry.unregister(&agent_id) {
        // Clear environment variable
        std::env::remove_var("STAK_AGENT_ID");
        println!("Unregistered agent '{agent_id}'");
    } else {
        println!("Agent '{agent_id}' not found");
    }

    Ok(())
}

/// Handle status command
fn handle_status(options: &AgentOptions, registry: &AgentRegistry) -> Result<()> {
    let agent_id = std::env::var("STAK_AGENT_ID").ok();

    match agent_id {
        Some(id) => match registry.get(&id) {
            Some(agent) => {
                println!("Agent Status:");
                println!("  ID: {}", agent.id);
                println!("  Status: {}", agent.status());
                println!(
                    "  Session: {}",
                    agent.current_session.as_deref().map_or("none", |s| s)
                );
                println!("  Actions: {}", agent.actions_count);
                println!("  Last seen: {}", agent.last_seen.to_rfc3339());
            }
            None => {
                println!("Agent '{id}' not found in registry");
            }
        },
        None => {
            println!("No agent registered (STAK_AGENT_ID not set)");
        }
    }

    let _ = options; // Acknowledge unused parameter

    Ok(())
}

/// Handle list command
fn handle_list(options: &AgentOptions, registry: &AgentRegistry) -> Result<()> {
    let agents = registry.list(options.all);

    if agents.is_empty() {
        println!("No active agents");
        return Ok(());
    }

    println!("Active Agents ({}):", agents.len());

    for agent in agents {
        let status_str = match agent.status() {
            AgentStatus::Active => "active",
            AgentStatus::Stale => "STALE",
        };

        print!("  {} ", agent.id);

        if let Some(ref session) = agent.current_session {
            print!("(on {session}) ");
        }

        if let Some(ref command) = agent.current_command {
            print!("running {command} ");
        }

        println!("[{status_str}]");
        println!("    Actions: {}", agent.actions_count);
        println!("    Last seen: {}", agent.last_seen.to_rfc3339());
    }

    Ok(())
}

/// Generate a unique agent ID
fn generate_agent_id() -> AgentId {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let pid = std::process::id();
    AgentId::new(format!("agent-{timestamp:08x}-{pid:04x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_agent() -> Result<()> {
        let mut registry = AgentRegistry::new();
        let options = AgentOptions {
            list: false,
            register: true,
            heartbeat: false,
            unregister: false,
            status: false,
            agent_id: Some("test-agent".to_string()),
            session: None,
            command: None,
            all: false,
        };

        run(&options, &mut registry).await?;
        assert!(registry.get("test-agent").is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_unregister_agent() -> Result<()> {
        let mut registry = AgentRegistry::new();
        registry.register(AgentId::new("test-agent"));

        let options = AgentOptions {
            list: false,
            register: false,
            heartbeat: false,
            unregister: true,
            status: false,
            agent_id: Some("test-agent".to_string()),
            session: None,
            command: None,
            all: false,
        };

        run(&options, &mut registry).await?;
        assert!(registry.get("test-agent").is_none());

        Ok(())
    }
}

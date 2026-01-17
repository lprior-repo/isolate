//! Output logic - formatting and displaying agent list results

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{cli::is_tty, json_output::AgentInfo, json_output::AgentListOutput};

/// Output agents in requested format
///
/// Handles both JSON and human-readable table output formats.
/// If no agents found, displays appropriate message.
///
/// # Arguments
///
/// * `agents` - List of agents to output
/// * `json` - Whether to output as JSON
/// * `session` - Optional session name filter (used for empty message)
pub fn output_agents(
    agents: Vec<AgentInfo>,
    json: bool,
    session: Option<&str>,
) -> anyhow::Result<()> {
    if agents.is_empty() {
        output_empty(json, session);
        return Ok(());
    }

    if json {
        output_json(&agents)?;
    } else {
        output_table(&agents);
    }

    Ok(())
}

/// Output empty result message
fn output_empty(json: bool, session: Option<&str>) {
    if json {
        let output = AgentListOutput {
            agents: Vec::new(),
            total_count: 0,
        };
        // Safe to unwrap since we're formatting empty JSON
        let _ = println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else if is_tty() {
        if session.is_some() {
            println!("No agent found for session.");
        } else {
            println!("No agents found in any session.");
            println!("Agents can be registered by updating session metadata.");
        }
    }
}

/// Output agents as JSON
fn output_json(agents: &[AgentInfo]) -> anyhow::Result<()> {
    let output = AgentListOutput {
        total_count: agents.len(),
        agents: agents.to_vec(),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output agents as a human-readable table
fn output_table(agents: &[AgentInfo]) {
    println!("\nAgents Working in Sessions:");
    println!("{}", "=".repeat(80));

    for agent in agents {
        println!("\nSession:    {}", agent.session_name);
        println!("Agent ID:   {}", agent.agent_id);

        if let Some(task) = &agent.task_id {
            println!("Task:       {task}");
        }

        if let Some(spawned) = agent.spawned_at {
            if let Some(formatted_time) = format_spawned_time(spawned) {
                println!("Spawned:    {formatted_time}");
            }
        }

        if let Some(pid) = agent.pid {
            println!("PID:        {pid}");
        }

        if let Some(code) = agent.exit_code {
            println!("Exit Code:  {code}");
        }

        if let Some(artifacts) = &agent.artifacts_path {
            println!("Artifacts:  {artifacts}");
        }

        println!("{}", "-".repeat(80));
    }

    println!("\nTotal: {} agent(s)\n", agents.len());
}

/// Format spawned timestamp as human-readable relative time
fn format_spawned_time(spawned: u64) -> Option<String> {
    let time = UNIX_EPOCH.checked_add(Duration::from_secs(spawned))?;
    let duration = SystemTime::now().duration_since(time).ok()?;

    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() % 3600) / 60;

    if hours > 0 {
        Some(format!("{hours}h {minutes}m ago"))
    } else {
        Some(format!("{minutes}m ago"))
    }
}

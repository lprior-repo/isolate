//! Agents command - List all active agents and their locks

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod types;

#[cfg(test)]
mod tests;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::SqlitePool;
use zjj_core::{coordination::locks::LockManager, json::SchemaEnvelope, OutputFormat};

use self::types::{
    AgentInfo, AgentStatusOutput, AgentsArgs, AgentsOutput, HeartbeatArgs, HeartbeatOutput,
    LockSummary, RegisterArgs, RegisterOutput, UnregisterArgs, UnregisterOutput,
};

/// Run the agents command
///
/// # Errors
///
/// Returns error if:
/// - Database cannot be accessed
/// - Agents table does not exist
/// - Locks table does not exist
/// - Query fails
pub async fn run(args: &AgentsArgs, format: OutputFormat) -> Result<()> {
    // Get database connection
    let pool = get_db_pool().await?;

    // Get agents (with optional filtering)
    let agents = get_agents(&pool, args).await?;

    // Get all active locks
    let lock_mgr = LockManager::new(pool.clone());
    let locks = get_locks(&lock_mgr).await?;

    // Compute counts
    let total_active = agents.iter().filter(|a| !a.stale).count();
    let total_stale = agents.iter().filter(|a| a.stale).count();

    // Build output
    let output = AgentsOutput {
        agents,
        locks,
        total_active,
        total_stale,
    };

    // Output based on format
    if format.is_json() {
        let envelope = SchemaEnvelope::new("agents-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_human_readable(&output);
    }

    Ok(())
}

/// Get database pool from session database
async fn get_db_pool() -> Result<SqlitePool> {
    let db_path = crate::commands::get_db_path().await?;

    // Check if database exists
    if !db_path.exists() {
        anyhow::bail!("ZJJ not initialized. Run 'zjj init' first.");
    }

    let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {e}"))?;

    Ok(pool)
}

/// Get agents from the database
///
/// This function:
/// 1. Fetches all agents from the agents table
/// 2. Computes staleness based on `last_seen` timestamp
/// 3. Filters by --all flag and --session filter
async fn get_agents(pool: &SqlitePool, args: &AgentsArgs) -> Result<Vec<AgentInfo>> {
    let cutoff = Utc::now() - chrono::Duration::seconds(60);

    // Build query with optional session filter
    let query = if args.session.is_some() {
        "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
         FROM agents
         WHERE current_session = ?"
    } else {
        "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
         FROM agents"
    };

    let mut query_builder =
        sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, i64)>(query);

    if let Some(ref session) = args.session {
        query_builder = query_builder.bind(session);
    }

    let rows = match query_builder.fetch_all(pool).await {
        Ok(rows) => rows,
        Err(e) => {
            // If agents table doesn't exist yet, return empty list
            if e.to_string().contains("no such table") {
                return Ok(Vec::new());
            }
            return Err(anyhow::anyhow!("Failed to query agents: {e}"));
        }
    };

    // Transform rows into AgentInfo, computing staleness
    let agents: Result<Vec<AgentInfo>, _> = rows
        .into_iter()
        .map(
            |(
                agent_id,
                registered_at,
                last_seen,
                current_session,
                current_command,
                actions_count,
            )|
             -> Result<AgentInfo, anyhow::Error> {
                let registered_at = parse_timestamp(&registered_at)?;
                let last_seen = parse_timestamp(&last_seen)?;
                let stale = last_seen < cutoff;

                Ok(AgentInfo {
                    agent_id,
                    registered_at,
                    last_seen,
                    current_session,
                    current_command,
                    actions_count: u64::try_from(actions_count).unwrap_or(0),
                    stale,
                })
            },
        )
        .collect();

    let mut agents = agents?;

    // Filter stale agents if --all not specified
    if !args.all {
        agents.retain(|a| !a.stale);
    }

    Ok(agents)
}

/// Get all active locks from the lock manager
async fn get_locks(lock_mgr: &LockManager) -> Result<Vec<LockSummary>> {
    let locks = match lock_mgr.get_all_locks().await {
        Ok(locks) => locks,
        Err(e) => {
            // If locks table doesn't exist yet, return empty list
            if e.to_string().contains("no such table") {
                return Ok(Vec::new());
            }
            return Err(anyhow::anyhow!("Failed to query locks: {e}"));
        }
    };

    Ok(locks
        .into_iter()
        .map(|l| LockSummary {
            session: l.session,
            holder: l.agent_id,
            expires_at: l.expires_at,
        })
        .collect())
}

/// Parse an RFC3339 timestamp
fn parse_timestamp(ts: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt: DateTime<FixedOffset>| dt.with_timezone(&Utc))
        .map_err(|e| anyhow::anyhow!("Invalid timestamp '{ts}': {e}"))
}

/// Print human-readable output
fn print_human_readable(output: &AgentsOutput) {
    println!("Active Agents ({}):", output.total_active);

    if output.agents.is_empty() {
        println!("  No active agents");
    } else {
        output.agents.iter().for_each(|agent| {
            print!("  {} ", agent.agent_id);

            if let Some(ref session) = agent.current_session {
                print!("(on {session}) ");
            }

            if let Some(ref command) = agent.current_command {
                print!("running {command} ");
            }

            println!();

            println!("    Actions: {}", agent.actions_count);
            println!("    Last seen: {}", agent.last_seen.to_rfc3339());
        });
    }

    if output.total_stale > 0 {
        println!();
        println!("Stale Agents ({}):", output.total_stale);
        output.agents.iter().filter(|a| a.stale).for_each(|agent| {
            println!(
                "  {} (last seen: {})",
                agent.agent_id,
                agent.last_seen.to_rfc3339()
            );
        });
    }

    println!();

    if output.locks.is_empty() {
        println!("No active locks");
    } else {
        println!("Active Locks ({}):", output.locks.len());
        output.locks.iter().for_each(|lock| {
            println!(
                "  {} held by {} (expires: {})",
                lock.session,
                lock.holder,
                lock.expires_at.to_rfc3339()
            );
        });
    }
}

// ============================================================================
// Agent Self-Management Subcommands
// ============================================================================

/// Reserved keywords that cannot be used as agent IDs
const RESERVED_AGENT_IDS: &[&str] = &["null", "undefined", "true", "false", "none", "nil", "void"];

/// Validate that an agent ID is non-empty and not just whitespace
///
/// # Errors
///
/// Returns error if:
/// - Agent ID is empty or consists only of whitespace
/// - Agent ID contains spaces or newlines (breaks shell quoting)
/// - Agent ID is a reserved keyword
pub fn validate_agent_id(agent_id: &str) -> Result<()> {
    let trimmed = agent_id.trim();

    if trimmed.is_empty() {
        anyhow::bail!("Agent ID cannot be empty or whitespace-only");
    }

    // Check for spaces and newlines (breaks shell quoting)
    if agent_id.chars().any(char::is_whitespace) {
        anyhow::bail!("Agent ID cannot contain whitespace characters (spaces, tabs, newlines)");
    }

    // Check for reserved keywords (case-insensitive)
    let lower = trimmed.to_lowercase();
    if RESERVED_AGENT_IDS.iter().any(|&keyword| keyword == lower) {
        anyhow::bail!("Agent ID '{trimmed}' is a reserved keyword and cannot be used");
    }

    // If the trimmed version differs from the original, warn but accept it
    if trimmed.len() != agent_id.len() {
        // This is informational - we accept the trimmed version
        tracing::warn!("Agent ID contained leading/trailing whitespace; using trimmed value");
    }

    Ok(())
}

/// Register a new agent
///
/// # Errors
///
/// Returns error if database access fails or agent ID is invalid
pub async fn run_register(args: &RegisterArgs, format: OutputFormat) -> Result<()> {
    let pool = get_db_pool().await?;

    // Generate or validate agent ID
    let agent_id = match args.agent_id.clone() {
        Some(id) => {
            // Validate user-provided agent ID
            validate_agent_id(&id)?;
            id.trim().to_string()
        }
        None => generate_agent_id(),
    };

    // Set the environment variable so subsequent commands can use it
    std::env::set_var("ZJJ_AGENT_ID", &agent_id);

    let now = Utc::now().to_rfc3339();

    // Insert or update agent record
    sqlx::query(
        "INSERT INTO agents (agent_id, registered_at, last_seen, current_session, current_command, actions_count)
         VALUES (?, ?, ?, ?, NULL, 0)
         ON CONFLICT(agent_id) DO UPDATE SET last_seen = ?, current_session = ?"
    )
    .bind(&agent_id)
    .bind(&now)
    .bind(&now)
    .bind(&args.session)
    .bind(&now)
    .bind(&args.session)
    .execute(&pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to register agent: {e}"))?;

    let output = RegisterOutput {
        agent_id: agent_id.clone(),
        session: args.session.clone(),
        message: format!("Registered agent '{agent_id}'"),
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("agent-register-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{}", output.message);
        println!("  Agent ID: {}", output.agent_id);
        if let Some(ref session) = output.session {
            println!("  Session: {session}");
        }
        println!("\nSet ZJJ_AGENT_ID={} in your environment", output.agent_id);
    }

    Ok(())
}

/// Send a heartbeat for the current agent
///
/// # Errors
///
/// Returns error if:
/// - No agent ID set in environment
/// - Agent ID not found in database (agent was unregistered)
/// - Database access fails
pub async fn run_heartbeat(args: &HeartbeatArgs, format: OutputFormat) -> Result<()> {
    let agent_id = std::env::var("ZJJ_AGENT_ID").map_err(|_| {
        anyhow::anyhow!("No agent registered. Set ZJJ_AGENT_ID or run 'zjj agent register'")
    })?;

    let pool = get_db_pool().await?;
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    // Update last_seen and optionally current_command
    let result = if let Some(ref command) = args.command {
        sqlx::query(
            "UPDATE agents SET last_seen = ?, current_command = ?, actions_count = actions_count + 1 WHERE agent_id = ?"
        )
        .bind(&now_str)
        .bind(command)
        .bind(&agent_id)
        .execute(&pool)
        .await
    } else {
        sqlx::query(
            "UPDATE agents SET last_seen = ?, actions_count = actions_count + 1 WHERE agent_id = ?",
        )
        .bind(&now_str)
        .bind(&agent_id)
        .execute(&pool)
        .await
    };

    let result = result.map_err(|e| anyhow::anyhow!("Failed to send heartbeat: {e}"))?;

    // Check if agent exists - 0 rows affected means agent was unregistered
    if result.rows_affected() == 0 {
        anyhow::bail!("Agent '{agent_id}' not found. Agent may have been unregistered. Please run 'zjj agent register' to re-register.");
    }

    let output = HeartbeatOutput {
        agent_id: agent_id.clone(),
        timestamp: now_str,
        message: format!("Heartbeat sent for agent '{agent_id}'"),
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("agent-heartbeat-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{}", output.message);
    }

    Ok(())
}

/// Get status of the current agent
///
/// # Errors
///
/// Returns error if database access fails
pub async fn run_status(format: OutputFormat) -> Result<()> {
    let agent_id = std::env::var("ZJJ_AGENT_ID").ok();

    let output = if let Some(ref id) = agent_id {
        let pool = get_db_pool().await?;
        let cutoff = Utc::now() - chrono::Duration::seconds(60);

        let row: Option<(String, String, String, Option<String>, Option<String>, i64)> = sqlx::query_as(
            "SELECT agent_id, registered_at, last_seen, current_session, current_command, actions_count
             FROM agents WHERE agent_id = ?"
        )
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query agent: {e}"))?;

        if let Some((
            agent_id,
            registered_at,
            last_seen,
            current_session,
            current_command,
            actions_count,
        )) = row
        {
            let registered_at = parse_timestamp(&registered_at)?;
            let last_seen_dt = parse_timestamp(&last_seen)?;
            let stale = last_seen_dt < cutoff;

            let agent = AgentInfo {
                agent_id,
                registered_at,
                last_seen: last_seen_dt,
                current_session,
                current_command,
                actions_count: u64::try_from(actions_count).unwrap_or(0),
                stale,
            };

            AgentStatusOutput {
                registered: true,
                agent: Some(agent),
                message: format!("Agent '{id}' is registered"),
            }
        } else {
            AgentStatusOutput {
                registered: false,
                agent: None,
                message: format!("Agent '{id}' not found in database"),
            }
        }
    } else {
        AgentStatusOutput {
            registered: false,
            agent: None,
            message: "No agent registered (ZJJ_AGENT_ID not set)".to_string(),
        }
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("agent-status-response", "single", &output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{}", output.message);
        if let Some(ref agent) = output.agent {
            println!("  Agent ID: {}", agent.agent_id);
            println!(
                "  Session: {}",
                agent.current_session.as_deref().unwrap_or("none")
            );
            println!("  Actions: {}", agent.actions_count);
            println!("  Last seen: {}", agent.last_seen.to_rfc3339());
            println!("  Status: {}", if agent.stale { "STALE" } else { "active" });
        }
    }

    Ok(())
}

/// Unregister an agent
///
/// # Errors
///
/// Returns error if no agent ID, invalid agent ID, or database access fails
pub async fn run_unregister(args: &UnregisterArgs, format: OutputFormat) -> Result<()> {
    let agent_id = args
        .agent_id
        .clone()
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .ok_or_else(|| anyhow::anyhow!("No agent ID provided. Set ZJJ_AGENT_ID or use --id"))?;

    // Validate agent ID
    validate_agent_id(&agent_id)?;

    let pool = get_db_pool().await?;

    // Delete agent record
    sqlx::query("DELETE FROM agents WHERE agent_id = ?")
        .bind(&agent_id)
        .execute(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to unregister agent: {e}"))?;

    // Clear the environment variable
    std::env::remove_var("ZJJ_AGENT_ID");

    let output = UnregisterOutput {
        agent_id: agent_id.clone(),
        message: format!("Unregistered agent '{agent_id}'"),
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("agent-unregister-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{}", output.message);
    }

    Ok(())
}

/// Generate a unique agent ID
fn generate_agent_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_or(0u128, |v| v);

    // Use PID to make IDs unique across concurrent processes
    let pid = std::process::id();

    let timestamp_u32 = u32::try_from(timestamp).map_or(u32::MAX, |v| v);
    let pid_u16 = u16::try_from(pid).map_or(u16::MAX, |v| v);

    format!("agent-{timestamp_u32:08x}-{pid_u16:04x}")
}

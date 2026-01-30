//! Agents command - List all active agents and their locks

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod types;

#[cfg(test)]
mod tests;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::SqlitePool;
use zjj_core::{coordination::locks::LockManager, json::SchemaEnvelope, OutputFormat};

use self::types::{AgentInfo, AgentsArgs, AgentsOutput, LockSummary};

/// Run the agents command
///
/// # Errors
///
/// Returns error if:
/// - Database cannot be accessed
/// - Agents table does not exist
/// - Locks table does not exist
/// - Query fails
pub fn run(args: &AgentsArgs, format: OutputFormat) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { run_async(args, format).await })
}

/// Async implementation of the agents command
async fn run_async(args: &AgentsArgs, format: OutputFormat) -> Result<()> {
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
    let data_dir = crate::commands::zjj_data_dir()?;
    let db_path = data_dir.join("state.db");

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

    let rows = query_builder
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query agents: {e}"))?;

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
    let locks = lock_mgr
        .get_all_locks()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query locks: {e}"))?;

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
        for agent in &output.agents {
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
        }
    }

    if output.total_stale > 0 {
        println!();
        println!("Stale Agents ({}):", output.total_stale);
        for agent in output.agents.iter().filter(|a| a.stale) {
            println!(
                "  {} (last seen: {})",
                agent.agent_id,
                agent.last_seen.to_rfc3339()
            );
        }
    }

    println!();

    if output.locks.is_empty() {
        println!("No active locks");
    } else {
        println!("Active Locks ({}):", output.locks.len());
        for lock in &output.locks {
            println!(
                "  {} held by {} (expires: {})",
                lock.session,
                lock.holder,
                lock.expires_at.to_rfc3339()
            );
        }
    }
}

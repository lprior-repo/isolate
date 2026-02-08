//! Broadcast command - Send messages to all active agents

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

#[cfg(test)]
mod tests;
pub mod types;

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use zjj_core::{agents::registry::AgentRegistry, json::SchemaEnvelope, OutputFormat};

use self::types::{BroadcastArgs, BroadcastResponse};

/// Run the broadcast command
///
/// # Errors
///
/// Returns error if:
/// - Database cannot be accessed
/// - Agent ID is invalid
/// - Message storage fails
pub async fn run(args: &BroadcastArgs, format: OutputFormat) -> Result<()> {
    // Get database connection
    let pool = get_db_pool().await?;

    // Create agent registry with default timeout (60 seconds)
    let registry = AgentRegistry::new(pool.clone(), 60).await?;

    // Get all active agents
    let active_agents = registry.get_active().await?;

    // Filter out the sender from the recipient list
    let sent_to: Vec<String> = active_agents
        .iter()
        .filter(|agent| agent.agent_id != args.agent_id)
        .map(|agent| agent.agent_id.clone())
        .collect();

    // Store the broadcast message in the database
    store_broadcast(&pool, &args.message, &args.agent_id, &sent_to).await?;

    // Build response
    let response = BroadcastResponse {
        success: true,
        message: args.message.clone(),
        sent_to: sent_to.clone(),
        timestamp: Utc::now().to_rfc3339(),
    };

    // Output based on format
    if format.is_json() {
        let envelope = SchemaEnvelope::new("broadcast-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_human_readable(&response);
    }

    Ok(())
}

/// Get database pool from session database
async fn get_db_pool() -> Result<SqlitePool> {
    let data_dir = crate::commands::zjj_data_dir().await?;
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

/// Store a broadcast message in the database
///
/// Creates the broadcasts table if it doesn't exist, then inserts the message.
async fn store_broadcast(
    pool: &SqlitePool,
    message: &str,
    sender_id: &str,
    sent_to: &[String],
) -> Result<()> {
    // Create broadcasts table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS broadcasts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message TEXT NOT NULL,
            sender_id TEXT NOT NULL,
            sent_to TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create broadcasts table: {e}"))?;

    // Serialize sent_to list as JSON
    let sent_to_json = serde_json::to_string(sent_to)
        .map_err(|e| anyhow::anyhow!("Failed to serialize recipient list: {e}"))?;

    let timestamp = Utc::now().to_rfc3339();

    // Insert the broadcast message
    sqlx::query(
        "INSERT INTO broadcasts (message, sender_id, sent_to, timestamp)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(message)
    .bind(sender_id)
    .bind(&sent_to_json)
    .bind(&timestamp)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to store broadcast message: {e}"))?;

    Ok(())
}

/// Print human-readable output
fn print_human_readable(response: &BroadcastResponse) {
    println!("Broadcast sent successfully");
    println!("  Message: {}", response.message);
    println!("  Sent to: {} agents", response.sent_to.len());
    println!("  Timestamp: {}", response.timestamp);

    if response.sent_to.is_empty() {
        println!("  No other active agents");
    } else {
        println!("  Recipients:");
        for agent_id in &response.sent_to {
            println!("    - {agent_id}");
        }
    }
}

//! Coordination handlers: claim, yield, lock, unlock

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::{claim, get_session_db};

pub async fn handle_claim(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let resource = sub_m
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow::anyhow!("Resource is required"))?
        .clone();
    let timeout: u64 = sub_m
        .get_one::<String>("timeout")
        .and_then(|s| s.parse().ok())
        .map_or(30, |v| v);
    let options = claim::ClaimOptions {
        resource,
        timeout,
        format,
    };
    claim::run_claim(&options).await
}

pub async fn handle_yield(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let resource = sub_m
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow::anyhow!("Resource is required"))?
        .clone();
    let options = claim::YieldOptions { resource, format };
    claim::run_yield(&options).await
}

pub async fn handle_lock(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("Session is required"))?
        .clone();
    let agent_id = sub_m.get_one::<String>("agent-id").cloned();
    let ttl = sub_m.get_one::<u64>("ttl").map_or(0, |value| *value);

    let args = crate::commands::lock::types::LockArgs {
        session,
        agent_id,
        ttl,
    };

    let db = get_session_db().await?;
    let mgr = isolate_core::coordination::locks::LockManager::new(db.pool().clone());

    let output = crate::commands::lock::run_lock_async(&args, &mgr).await?;
    if format.is_json() {
        let envelope = isolate_core::SchemaEnvelope::new("lock-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "✓ Locked session '{}' for agent '{}'",
            output.session, output.holder
        );
        if let Some(expires) = output.expires_at {
            let expires: chrono::DateTime<chrono::Utc> = expires;
            println!("  Expires at: {}", expires.to_rfc3339());
        }
    }
    Ok(())
}

pub async fn handle_unlock(sub_m: &ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("session")
        .ok_or_else(|| anyhow::anyhow!("Session is required"))?
        .clone();
    let agent_id = sub_m.get_one::<String>("agent-id").cloned();

    let args = crate::commands::lock::types::UnlockArgs { session, agent_id };

    let db = get_session_db().await?;
    let mgr = isolate_core::coordination::locks::LockManager::new(db.pool().clone());

    let output = crate::commands::lock::run_unlock_async(&args, &mgr).await?;
    if format.is_json() {
        let envelope = isolate_core::SchemaEnvelope::new("unlock-response", "single", output);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("✓ Unlocked session '{}'", output.session);
    }
    Ok(())
}

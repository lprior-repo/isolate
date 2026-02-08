//! Lock and unlock commands for session coordination

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod types;

#[cfg(test)]
mod tests;

use anyhow::Result;
use zjj_core::coordination::locks::LockManager;

use self::types::{LockArgs, LockOutput, UnlockArgs, UnlockOutput};

pub async fn run_lock_async(args: &LockArgs, mgr: &LockManager) -> Result<LockOutput> {
    let agent_id = args
        .agent_id
        .clone()
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("No agent ID provided. Set ZJJ_AGENT_ID or use --agent-id")
        })?;

    // If a custom TTL is provided, create a new manager with that TTL
    let mgr = if args.ttl > 0 {
        LockManager::with_ttl(
            mgr.pool().clone(),
            chrono::Duration::seconds(i64::try_from(args.ttl).unwrap_or(300)),
        )
    } else {
        mgr.clone()
    };

    match mgr.lock(&args.session, &agent_id).await {
        Ok(lock) => Ok(LockOutput {
            success: true,
            locked: true,
            lock_id: Some(lock.lock_id),
            session: lock.session,
            holder: lock.agent_id,
            expires_at: Some(lock.expires_at),
            ttl_seconds: args.ttl,
        }),
        Err(zjj_core::Error::SessionLocked { holder, .. }) => {
            anyhow::bail!("SESSION_LOCKED: Resource locked by {holder}")
        }
        Err(e) => anyhow::bail!("Failed to acquire lock: {e}"),
    }
}

pub async fn run_unlock_async(args: &UnlockArgs, mgr: &LockManager) -> Result<UnlockOutput> {
    let agent_id = args
        .agent_id
        .clone()
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("No agent ID provided. Set ZJJ_AGENT_ID or use --agent-id")
        })?;

    match mgr.unlock(&args.session, &agent_id).await {
        Ok(()) => Ok(UnlockOutput {
            success: true,
            released: true,
            session: args.session.clone(),
        }),
        Err(zjj_core::Error::NotLockHolder { .. }) => {
            anyhow::bail!("NOT_LOCK_HOLDER: You are not the lock holder for this session")
        }
        Err(e) => anyhow::bail!("Failed to release lock: {e}"),
    }
}

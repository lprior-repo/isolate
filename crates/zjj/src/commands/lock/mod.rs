//! Lock and unlock commands for session coordination

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod types;

#[cfg(test)]
mod tests;

use anyhow::Result;
use zjj_core::coordination::locks::LockManager;

use self::types::{LockArgs, LockOutput, UnlockArgs, UnlockOutput};
use crate::commands::get_session_db;

pub async fn run_lock_async(args: &LockArgs, mgr: &LockManager) -> Result<LockOutput> {
    let agent_id = args
        .agent_id
        .clone()
        .or_else(|| std::env::var("ZJJ_AGENT_ID").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("No agent ID provided. Set ZJJ_AGENT_ID or use --agent-id")
        })?;

    let lock_result = if args.ttl == 0 {
        mgr.lock(&args.session, &agent_id).await
    } else {
        mgr.lock_with_ttl(&args.session, &agent_id, args.ttl).await
    };

    match lock_result {
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
    // Check if session exists before attempting to unlock
    let db = get_session_db().await?;
    let session_exists = db.get(&args.session).await?.is_some();

    if !session_exists {
        anyhow::bail!(
            "SESSION_NOT_FOUND: Session '{}' does not exist",
            args.session
        );
    }

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

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

#[allow(dead_code, clippy::unused_async)]
pub async fn run_lock_async(_args: &LockArgs, _mgr: &LockManager) -> Result<LockOutput> {
    anyhow::bail!("Not yet implemented")
}

#[allow(dead_code, clippy::unused_async)]
pub async fn run_unlock_async(_args: &UnlockArgs, _mgr: &LockManager) -> Result<UnlockOutput> {
    anyhow::bail!("Not yet implemented")
}

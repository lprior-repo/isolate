//! Types for lock/unlock commands

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LockArgs {
    pub session: String,
    pub agent_id: Option<String>,
    pub ttl: u64,
}

#[derive(Debug, Clone)]
pub struct UnlockArgs {
    pub session: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockOutput {
    pub success: bool,
    pub session: String,
    pub locked: bool,
    pub lock_id: Option<String>,
    pub holder: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockOutput {
    pub success: bool,
    pub session: String,
    pub released: bool,
}

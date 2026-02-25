//! Database layer for stak
//!
//! Uses SQLite for queue and agent state.

use sqlx::SqlitePool;

#[allow(dead_code)]
pub struct StakDb {
    pool: SqlitePool,
}

impl StakDb {
    pub async fn connect(path: &str) -> anyhow::Result<Self> {
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", path)).await?;
        Ok(Self { pool })
    }
}

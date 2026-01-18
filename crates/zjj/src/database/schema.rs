//! Database schema definitions and initialization

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use zjj_core::{Error, Result};

/// Database schema as SQL string - executed once on init
pub(crate) const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_name ON sessions(name);

CREATE TRIGGER IF NOT EXISTS update_timestamp
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;
";

/// Create `SQLite` connection pool
pub(crate) async fn create_connection_pool(db_url: &str) -> Result<SqlitePool> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect(db_url)
        .await
        .map_err(|e| Error::database_error(format!("Failed to connect to database: {e}")))
}

/// Initialize database schema
pub(crate) async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| Error::database_error(format!("Failed to initialize schema: {e}")))
}

/// Initialize database schema (transaction version)
pub(crate) async fn init_schema_tx(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<()> {
    sqlx::query(SCHEMA)
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| Error::database_error(format!("Failed to initialize schema: {e}")))
}

/// Drop existing database schema (transaction version)
pub(crate) async fn drop_existing_schema_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS sessions")
        .execute(&mut **tx)
        .await
        .map_err(|e| Error::database_error(format!("Failed to drop sessions table: {e}")))?;

    sqlx::query("DROP TRIGGER IF EXISTS update_timestamp")
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| Error::database_error(format!("Failed to drop update trigger: {e}")))
}

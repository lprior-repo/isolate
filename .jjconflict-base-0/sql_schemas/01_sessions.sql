-- ZJJ Sessions Table Schema
--
-- This table stores all session information for zjj workspace management.
-- Each session represents a JJ workspace with associated Zellij session.

CREATE TABLE IF NOT EXISTS sessions (
    -- Primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Session identifier (unique human-readable name)
    name TEXT UNIQUE NOT NULL,

    -- Session lifecycle status
    -- Valid values: 'creating', 'active', 'paused', 'completed', 'failed'
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),

    -- File system path to the JJ workspace
    workspace_path TEXT NOT NULL,

    -- JJ branch name (optional)
    branch TEXT,

    -- Timestamps (Unix epoch seconds)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,

    -- JSON metadata for extensible session properties
    metadata TEXT
);

-- Index for status-based filtering
-- Used by: list command, dashboard, clean operations
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Index for name-based lookups
-- Used by: get, update, delete operations
CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name);

-- Index for created_at ordering
-- Used by: list command (default ordering)
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);

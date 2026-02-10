# ZJJ SQL Schemas

This directory contains all SQL schema definitions for the zjj database.

## Schema Files

### `01_sessions.sql`
Main sessions table storing all zjj session state.

**Columns:**
- `id` - Auto-increment primary key
- `name` - Unique session identifier
- `status` - Session lifecycle (creating/active/paused/completed/failed)
- `state` - Workspace lifecycle state (created/working/ready/merged/abandoned/conflict)
- `workspace_path` - Path to JJ workspace
- `branch` - Current JJ branch (optional)
- `created_at` - Unix timestamp of session creation
- `updated_at` - Unix timestamp of last update (auto-managed)
- `last_synced` - Unix timestamp of last sync with JJ
- `metadata` - JSON extensible metadata

**Indexes:**
- `idx_sessions_status` - Fast status filtering
- `idx_sessions_state` - Fast workspace state filtering
- `idx_sessions_name` - Fast name lookups
- `idx_sessions_created_at` - Ordered listing

### `02_session_locks.sql`
Concurrency control for multi-agent scenarios (DRQ Round 4).

**Purpose:** Prevent race conditions when multiple AI agents operate on the same session.

**Columns:**
- `session_name` - Session being locked
- `operation` - Type of operation (sync/remove/modify/spawn)
- `agent_id` - Optional agent identifier
- `acquired_at` - Lock acquisition time
- `expires_at` - TTL-based auto-release

**Indexes:**
- `idx_session_locks_expires_at` - Expired lock cleanup
- `idx_session_locks_agent_id` - Agent-based queries
- `idx_session_locks_acquired_at` - Chronological ordering

### `03_triggers.sql`
Automatic timestamp management.

**Triggers:**
- `update_sessions_timestamp` - Auto-updates `updated_at` on row modification

## Usage with sqlx

```rust
// Load and execute schema file
let schema = std::fs::read_to_string("sql_schemas/01_sessions.sql")?;
sqlx::query(&schema).execute(&pool).await?;
```

## Migration Order

1. `01_sessions.sql` - Base table
2. `02_session_locks.sql` - Concurrency control
3. `03_triggers.sql` - Auto-updates

### `state_transitions`

The `state_transitions` table is created by `01_sessions.sql` and records
workspace lifecycle transitions for audit and analysis.

## DRQ Alignment

These schemas support the Dynamic Revaluation of Quality testing methodology:

- **State Consistency**: Explicit status enum prevents invalid states
- **Concurrency Safety**: Lock table enables multi-agent coordination
- **Recovery**: Timestamps enable orphan detection
- **Observability**: Agent_id field enables debugging multi-agent scenarios

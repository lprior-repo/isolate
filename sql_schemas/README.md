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

### `04_conflict_resolutions.sql`
Append-only audit trail for conflict resolution decisions (bd-2gj).

**Purpose:** Track AI vs human conflict resolution decisions for debugging and accountability.

**Columns:**
- `id` - Auto-increment primary key
- `timestamp` - ISO 8601 timestamp of resolution
- `session` - Session name where conflict occurred
- `file` - File path with conflict
- `strategy` - Resolution strategy used (accept_theirs, accept_ours, manual_merge, skip)
- `reason` - Human-readable reason for resolution (optional)
- `confidence` - Confidence score for AI decisions (optional)
- `decider` - Who made the decision ('ai' or 'human', CHECK constraint enforced)

**Design Principles:**
1. **Append-Only**: No UPDATE or DELETE operations allowed
2. **AI vs Human Tracking**: Every resolution records the decider type
3. **Transparency**: Full audit trail for debugging and accountability
4. **Performance**: Optimized for inserts and queries with indexes

**Indexes:**
- `idx_conflict_resolutions_session` - Session-based queries
- `idx_conflict_resolutions_timestamp` - Time-based queries
- `idx_conflict_resolutions_decider` - Decider-based queries
- `idx_conflict_resolutions_session_timestamp` - Composite index for session+time queries

**Constraints:**
- `CHECK(decider IN ('ai', 'human'))` - Ensures only valid decider types

### `05_queue_tables.sql`
Queue management tables for merge train integration (bd-39j).

**Purpose:** Three tables for merge queue lifecycle management with state machine enforcement.

**Tables:**

#### `merge_queue`
Queue entries with full state machine lifecycle tracking.

**Columns:**
- `id` - Auto-increment primary key
- `workspace` - Workspace identifier (unique, FK to sessions.name)
- `bead_id` - Optional bead ID for tracking work items
- `priority` - Queue ordering priority (higher = more urgent)
- `status` - Queue status with CHECK constraint (pending/claimed/rebasing/testing/ready_to_merge/merging/merged/failed_retryable/failed_terminal/cancelled)
- `added_at` - Unix timestamp when added to queue
- `started_at` - Unix timestamp when processing started (optional)
- `completed_at` - Unix timestamp when processing completed (optional)
- `error_message` - Error information for failed entries (optional)
- `agent_id` - Agent tracking for multi-agent scenarios (optional)
- `dedupe_key` - Deduplication key for idempotent submissions (unique, optional)
- `workspace_state` - Workspace state (created/working/ready/merged/abandoned/conflict)
- `previous_state` - Previous state for rollback/recovery (optional)
- `state_changed_at` - State change timestamp (optional)
- `head_sha` - Git SHA for rebase validation (optional)
- `tested_against_sha` - SHA tested against (optional)
- `attempt_count` - Current attempt count (default 0)
- `max_attempts` - Maximum retry attempts (default 3)
- `rebase_count` - Number of rebase attempts (default 0)
- `last_rebase_at` - Last rebase timestamp (optional)

**Indexes:**
- `idx_merge_queue_status` - Status-based filtering
- `idx_merge_queue_status_priority` - Priority ordering within status
- `idx_merge_queue_workspace` - Workspace lookups
- `idx_merge_queue_agent_id` - Agent-based queries
- `idx_merge_queue_dedupe_key` - Idempotent submission checks
- `idx_merge_queue_added_at` - FIFO ordering

#### `queue_processing_lock`
Single-row lock for serialized queue processing with TTL-based auto-release.

**Columns:**
- `agent_id` - Agent holding the lock
- `acquired_at` - Lock acquisition timestamp
- `expires_at` - Lock expiration timestamp (TTL-based)

**Note:** No indexes needed - table has at most one row.

#### `queue_events`
Append-only audit trail for queue entry lifecycle.

**Columns:**
- `id` - Auto-increment primary key (monotonically increasing)
- `queue_id` - Reference to merge_queue entry (FK)
- `event_type` - Event type with CHECK constraint (created/claimed/transitioned/failed/retried/cancelled/merged/rebased/heartbeat)
- `details_json` - Optional JSON details for event context
- `created_at` - Event timestamp

**Design Principles:**
1. **State Machine**: Valid status transitions enforced by CHECK constraints
2. **Lock Safety**: TTL-based auto-release prevents stale locks
3. **Audit Trail**: Append-only events for debugging and recovery
4. **Performance**: Indexes optimized for common query patterns

**Indexes:**
- `idx_queue_events_queue_id` - Queue entry lookups
- `idx_queue_events_created_at` - Time-based queries
- `idx_queue_events_queue_id_created_at` - Composite for entry history
- `idx_queue_events_event_type` - Event type filtering

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
4. `04_conflict_resolutions.sql` - Conflict resolution audit trail
5. `05_queue_tables.sql` - Merge queue management (merge_queue, queue_processing_lock, queue_events)

### `state_transitions`

The `state_transitions` table is created by `01_sessions.sql` and records
workspace lifecycle transitions for audit and analysis.

## DRQ Alignment

These schemas support the Dynamic Revaluation of Quality testing methodology:

- **State Consistency**: Explicit status enum prevents invalid states
- **Concurrency Safety**: Lock table enables multi-agent coordination
- **Recovery**: Timestamps enable orphan detection
- **Observability**: Agent_id field enables debugging multi-agent scenarios

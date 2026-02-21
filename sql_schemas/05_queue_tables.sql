-- ZJJ Queue Tables Schema (bd-39j)
--
-- Three tables for merge queue management:
-- 1. merge_queue - Queue entries with state machine lifecycle
-- 2. queue_processing_lock - Single-worker processing lock
-- 3. queue_events - Append-only audit trail
--
-- Design Principles:
-- 1. State Machine: Valid status transitions enforced by CHECK constraints
-- 2. Lock Safety: TTL-based auto-release prevents stale locks
-- 3. Audit Trail: Append-only events for debugging and recovery
-- 4. Performance: Indexes optimized for common query patterns

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- MERGE QUEUE TABLE
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- Queue entries with full state machine lifecycle tracking.
--
-- State Machine:
-- pending -> claimed -> rebasing -> testing -> ready_to_merge -> merging -> merged
--     |          |          |           |              |            |
--     v          v          v           v              v            v
-- cancelled  failed_retryable/failed_terminal/cancelled (from each state)
--
-- Terminal states: merged, failed_terminal, cancelled
CREATE TABLE IF NOT EXISTS merge_queue (
    -- Primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Workspace identifier (foreign key to sessions.name)
    workspace TEXT NOT NULL UNIQUE,

    -- Optional bead ID for tracking work items
    bead_id TEXT,

    -- Priority for queue ordering (higher = more urgent)
    priority INTEGER NOT NULL DEFAULT 0,

    -- Queue entry status (state machine)
    -- Valid values: 'pending', 'claimed', 'rebasing', 'testing', 'ready_to_merge',
    --                'merging', 'merged', 'failed_retryable', 'failed_terminal', 'cancelled'
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending', 'claimed', 'rebasing', 'testing', 'ready_to_merge',
                         'merging', 'merged', 'failed_retryable', 'failed_terminal', 'cancelled')),

    -- Timestamps (Unix epoch seconds)
    added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    started_at INTEGER,
    completed_at INTEGER,

    -- Error information for failed entries
    error_message TEXT,

    -- Agent tracking for multi-agent scenarios
    agent_id TEXT,

    -- Deduplication key for idempotent submissions
    dedupe_key TEXT UNIQUE,

    -- Workspace state tracking
    -- Valid values: 'created', 'working', 'ready', 'merged', 'abandoned', 'conflict'
    workspace_state TEXT NOT NULL DEFAULT 'created'
        CHECK(workspace_state IN ('created', 'working', 'ready', 'merged', 'abandoned', 'conflict')),

    -- Previous state for rollback/recovery
    previous_state TEXT,

    -- State change timestamp
    state_changed_at INTEGER,

    -- Git SHA tracking for rebase validation
    head_sha TEXT,
    tested_against_sha TEXT,

    -- Retry management
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,

    -- Rebase tracking
    rebase_count INTEGER NOT NULL DEFAULT 0,
    last_rebase_at INTEGER,

    -- Stack parent reference (for stacked PRs)
    parent_workspace TEXT,

    FOREIGN KEY (workspace) REFERENCES sessions(name) ON DELETE CASCADE
);

-- Index for status-based filtering (most common query pattern)
-- Used by: claim_next(), get_by_status(), list queue
CREATE INDEX IF NOT EXISTS idx_merge_queue_status ON merge_queue(status);

-- Index for priority ordering within status
-- Used by: claim_next() - find highest priority pending item
CREATE INDEX IF NOT EXISTS idx_merge_queue_status_priority ON merge_queue(status, priority DESC);

-- Index for workspace lookups
-- Used by: get_by_workspace(), update operations
CREATE INDEX IF NOT EXISTS idx_merge_queue_workspace ON merge_queue(workspace);

-- Index for agent-based queries
-- Used by: get_agent_work(), cleanup orphaned work
CREATE INDEX IF NOT EXISTS idx_merge_queue_agent_id ON merge_queue(agent_id);

-- Index for dedupe key lookups
-- Used by: idempotent submission checks
CREATE INDEX IF NOT EXISTS idx_merge_queue_dedupe_key ON merge_queue(dedupe_key);

-- Index for time-based ordering
-- Used by: FIFO ordering within same priority
CREATE INDEX IF NOT EXISTS idx_merge_queue_added_at ON merge_queue(added_at);

-- Index for parent_workspace lookups (stack queries)
-- Used by: finding children in a stack
CREATE INDEX IF NOT EXISTS idx_merge_queue_parent_workspace ON merge_queue(parent_workspace);

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- QUEUE PROCESSING LOCK TABLE
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- Single-row lock for serialized queue processing.
--
-- This table has at most ONE row, ensuring only one agent processes
-- the queue at a time. The lock is TTL-based for automatic recovery
-- from crashed workers.
--
-- Lock Lifecycle:
-- 1. Worker calls acquire_lock() -> INSERT if empty or UPDATE if expired
-- 2. Worker processes queue items
-- 3. Worker calls release_lock() -> DELETE
-- 4. Expired locks are auto-releasable on next acquire attempt
CREATE TABLE IF NOT EXISTS queue_processing_lock (
    -- Agent holding the lock
    agent_id TEXT NOT NULL,

    -- Lock acquisition timestamp (Unix epoch seconds)
    acquired_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Lock expiration timestamp (TTL-based auto-release)
    -- Default: 5 minutes (300 seconds)
    expires_at INTEGER NOT NULL
);

-- Note: No indexes needed - this table has at most one row
-- The single-row constraint is enforced by the application layer

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- QUEUE EVENTS TABLE (Append-Only Audit Trail)
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- Append-only audit trail for queue entry lifecycle.
--
-- Events are never updated or deleted, providing a complete history
-- for debugging, recovery, and accountability.
--
-- Event Types:
-- - created: Entry added to queue
-- - claimed: Entry claimed by worker
-- - transitioned: State transition occurred
-- - failed: Entry failed processing
-- - retried: Entry retry initiated
-- - cancelled: Entry cancelled
-- - merged: Entry successfully merged
-- - rebased: Rebase operation completed
-- - heartbeat: Worker heartbeat for long-running operations
CREATE TABLE IF NOT EXISTS queue_events (
    -- Primary key (monotonically increasing)
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Reference to queue entry
    queue_id INTEGER NOT NULL,

    -- Event type
    -- Valid values: 'created', 'claimed', 'transitioned', 'failed', 'retried',
    --                'cancelled', 'merged', 'rebased', 'heartbeat'
    event_type TEXT NOT NULL
        CHECK(event_type IN ('created', 'claimed', 'transitioned', 'failed', 'retried',
                              'cancelled', 'merged', 'rebased', 'heartbeat')),

    -- Optional JSON details for event context
    details_json TEXT,

    -- Event timestamp (Unix epoch seconds)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (queue_id) REFERENCES merge_queue(id) ON DELETE CASCADE
);

-- Index for queue_id lookups (most common query)
-- Used by: get_events_for_entry(), event history
CREATE INDEX IF NOT EXISTS idx_queue_events_queue_id ON queue_events(queue_id);

-- Index for time-based queries
-- Used by: get_events_by_time_range(), recent activity
CREATE INDEX IF NOT EXISTS idx_queue_events_created_at ON queue_events(created_at);

-- Composite index for queue_id + time (common pattern)
-- Used by: get_recent_events_for_entry()
CREATE INDEX IF NOT EXISTS idx_queue_events_queue_id_created_at ON queue_events(queue_id, created_at);

-- Index for event type filtering
-- Used by: get_events_by_type(), failure analysis
CREATE INDEX IF NOT EXISTS idx_queue_events_event_type ON queue_events(event_type);

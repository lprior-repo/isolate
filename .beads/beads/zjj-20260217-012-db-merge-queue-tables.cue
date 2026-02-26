package beads


// Bead ID: isolate-20260217-012-db-merge-queue-tables
// Section: 1 of 16
bead_id: "isolate-20260217-012-db-merge-queue-tables"

// Section 2: Intent
intent: {
    // What: Create merge queue tables in state.db with zjj_queue prefix
    what: "Create zjj_queue_merge_queue, zjj_queue_processing_lock, zjj_queue_events tables"
    // Why: Merge queue needs dedicated tables; consolidate from separate queue.db
    why: "Current queue.db is separate; single database simplifies architecture"
    // Value: Merge queue state in main database with namespaced tables
    value: "Enables merge queue with single database architecture"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create zjj_queue_merge_queue table",
        "Create zjj_queue_processing_lock table",
        "Create zjj_queue_events table",
        "Add all necessary columns and indexes",
        "Add foreign key constraints",
        "Use zjj_queue prefix to namespace tables",
    ]
    // Out: What we will NOT do
    out: [
        "Update MergeQueue to use new tables (Bead 013)",
        "Remove queue.db (Bead 015)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: []  // No dependencies - foundation bead
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-013-db-update-mergequeue", "isolate-20260217-014-db-conflict-resolutions-table", "isolate-20260217-015-db-remove-queue-path"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        state_db_exists: bool
        database_accessible: bool
    }
    // Output: Produced state/outputs
    output: {
        merge_queue_table_exists: bool
        processing_lock_table_exists: bool
        events_table_exists: bool
        indexes_created: bool
        foreign_keys_created: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All tables have zjj_queue prefix",
        "Foreign key constraints are enforced",
        "Indexes are maintained",
        "Event log is append-only",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read existing queue.db schema if exists",
        "Create sql_schemas/02_merge_queue.sql",
        "CREATE TABLE zjj_queue_merge_queue with all columns",
        "CREATE TABLE zjj_queue_processing_lock",
        "CREATE TABLE zjj_queue_events",
        "Add indexes on session_name, position, status",
        "Add foreign key constraints",
        "Write migration script",
        "Test migration on empty database",
        "Test with sample data",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        merge_queue_table: #"""
            CREATE TABLE zjj_queue_merge_queue (
                id INTEGER PRIMARY KEY,
                session_name TEXT NOT NULL REFERENCES sessions(name),
                position INTEGER NOT NULL UNIQUE,
                status TEXT NOT NULL,
                tests_status TEXT DEFAULT 'pending',
                conflict_check_status TEXT DEFAULT 'pending',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            """#

        processing_lock_table: #"""
            CREATE TABLE zjj_queue_processing_lock (
                id INTEGER PRIMARY KEY,
                locked_at INTEGER NOT NULL,
                locked_by TEXT NOT NULL,
                heartbeat_at INTEGER NOT NULL
            );
            """#

        events_table: #"""
            CREATE TABLE zjj_queue_events (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                session_name TEXT NOT NULL,
                event_type TEXT NOT NULL,
                details TEXT
            );
            """#
    }
    // State: State mutations
    state: {
        tables_created: ["zjj_queue_merge_queue", "zjj_queue_processing_lock", "zjj_queue_events"]
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub async fn migrate_merge_queue_tables(db: &SqlitePool) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn create_merge_queue_table(db: &SqlitePool) -> Result<()>",
        "async fn create_processing_lock_table(db: &SqlitePool) -> Result<()>",
        "async fn create_events_table(db: &SqlitePool) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Table already exists",
        "Foreign key constraint violated",
        "Duplicate position in merge_queue",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Check if table exists before creating",
        "Return descriptive error for constraint violations",
        "Use INSERT OR FAIL for events",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_merge_queue_table_created",
        "test_processing_lock_table_created",
        "test_events_table_created",
        "test_foreign_key_constraints",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_migration_creates_all_tables",
        "test_can_insert_and_query_merge_queue",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "SQL injection in session names",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Use parameterized queries",
        "Validate session names",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Migration completes in < 5 seconds",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Create indexes after data migration",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log table creation",
        "Log migration progress",
    ]
    // Metrics: What to measure
    metrics: [
        "merge_queue_migration_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document all tables and columns",
        "Document foreign key relationships",
    ]
    // External: External docs needed
    external: [
        "Update database schema documentation",
        "Add ER diagram",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Migration may fail on existing databases",
        "Table names may conflict",
    ]
    // Operational: Operational risks
    operational: [
        "Existing queue.db needs migration",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "zjj_queue_merge_queue table exists",
        "zjj_queue_processing_lock table exists",
        "zjj_queue_events table exists",
        "All indexes created",
        "Foreign key constraints exist",
        "Migration runs successfully",
        "Unit tests pass",
        "No unwrap() or panic() in migration code",
    ]
    // Should: Nice to have
    should: [
        "Migration is idempotent",
        "ER diagram updated",
    ]
}

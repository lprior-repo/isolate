package beads


// Bead ID: isolate-20260217-011-db-add-columns
// Section: 1 of 16
bead_id: "isolate-20260217-011-db-add-columns"

// Section 2: Intent
intent: {
    // What: Add parent_session and queue_status columns to sessions table
    what: "ALTER TABLE sessions ADD COLUMN parent_session, queue_status"
    // Why: Graphite-style stacking and merge queue need parent tracking and queue state
    why: "Current schema has no concept of stacked sessions or queue participation"
    // Value: Enables stacked sessions and merge queue features
    value: "Supports parent-child relationships and queue state tracking"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add parent_session TEXT column to sessions table",
        "Add queue_status TEXT DEFAULT 'draft' column to sessions table",
        "Create ALTER TABLE statements",
        "Add foreign key constraint: parent_session REFERENCES sessions(name)",
        "Add indexes on parent_session and queue_status",
        "Update Session struct in code",
        "Create migration script",
    ]
    // Out: What we will NOT do
    out: [
        "Create merge_queue table (Bead 012)",
        "Create stack commands (Bead 017-019)",
        "Create merge queue commands (Bead 023-026)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: []  // No dependencies - foundation bead
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-017-stack-create-cmd", "isolate-20260217-018-stack-list-cmd", "isolate-20260217-019-stack-restack-cmd", "isolate-20260217-023-queue-submit-cmd", "isolate-20260217-024-merge-train-logic", "isolate-20260217-025-train-failure-auto-rebase", "isolate-20260217-026-queue-new-states"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        sessions_table_exists: bool
        database_accessible: bool
    }
    // Output: Produced state/outputs
    output: {
        parent_session_column_exists: bool
        queue_status_column_exists: bool
        foreign_key_constraint_exists: bool
        indexes_created: bool
        session_struct_updated: bool
        migration_runs_successfully: bool
    }
    // Invariants: Must remain true
    invariants: [
        "parent_session must reference valid session name or NULL",
        "queue_status must be valid QueueStatus value or NULL",
        "Foreign key constraint is enforced",
        "Indexes are maintained on new columns",
        "Existing data is not lost during migration",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read sql_schemas/01_sessions.sql",
        "Create ALTER TABLE statements for parent_session",
        "Create ALTER TABLE statements for queue_status",
        "Add foreign key constraint on parent_session",
        "Add index: CREATE INDEX idx_sessions_parent ON sessions(parent_session)",
        "Add index: CREATE INDEX idx_sessions_queue_status ON sessions(queue_status)",
        "Update schema version",
        "Update Session struct in crates/isolate-core/src/session_state.rs",
        "Add parent_session: Option<String> field",
        "Add queue_status: Option<String> field",
        "Write migration test",
        "Run migration on empty database",
        "Run migration on database with existing data",
        "Verify foreign key constraint works",
        "Verify indexes are used in queries",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        sessions_table_migration: #"""
            ALTER TABLE sessions ADD COLUMN parent_session TEXT REFERENCES sessions(name);
            ALTER TABLE sessions ADD COLUMN queue_status TEXT DEFAULT 'draft';
            CREATE INDEX idx_sessions_parent ON sessions(parent_session);
            CREATE INDEX idx_sessions_queue_status ON sessions(queue_status);
            """#

        Session_struct: #"""
            pub struct Session {
                // ... existing fields ...
                pub parent_session: Option<String>,
                pub queue_status: Option<String>,
            }
            """#
    }
    // State: State mutations
    state: {
        sessions_table_columns_added: ["parent_session", "queue_status"]
        indexes_added: 2
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "impl Session { pub parent_session: Option<String>, pub queue_status: Option<String> }",
        "pub async fn migrate_add_parent_and_queue_status(db: &SqlitePool) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn add_parent_session_column(db: &SqlitePool) -> Result<()>",
        "async fn add_queue_status_column(db: &SqlitePool) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Column already exists (migration already run)",
        "Foreign key constraint violated (invalid parent_session)",
        "Index creation fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Check if column exists before adding (idempotent migration)",
        "Return descriptive error for constraint violations",
        "Rollback on error",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_add_parent_session_column_is_idempotent",
        "test_add_queue_status_column_is_idempotent",
        "test_parent_session_foreign_key_constraint_enforced",
        "test_queue_status_accepts_valid_values",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_migration_from_old_schema_to_new",
        "test_stacked_sessions_with_parent_child",
        "test_indexes_improve_query_performance",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Foreign key constraints leak session existence",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Session names are not sensitive",
        "Use parameterized queries",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Migration completes in < 5 seconds for 1000 sessions",
        "Index lookups are O(log n)",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Create indexes after data migration",
        "Use covering indexes for common queries",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log migration start/end",
        "Log each schema change",
    ]
    // Metrics: What to measure
    metrics: [
        "database_migration_duration_seconds",
        "foreign_key_constraint_violations_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document new columns",
        "Document foreign key relationship",
        "Document valid queue_status values",
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
        "Foreign key constraints may break existing code",
    ]
    // Operational: Operational risks
    operational: [
        "Existing installations need manual migration",
        "Rolling back schema changes is difficult",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "parent_session column exists",
        "queue_status column exists with DEFAULT 'draft'",
        "Foreign key constraint exists",
        "Indexes exist on both columns",
        "Session struct updated with new fields",
        "Migration runs successfully on empty database",
        "Migration runs successfully on database with data",
        "Unit tests pass",
        "No unwrap() or panic() in migration code",
    ]
    // Should: Nice to have
    should: [
        "Migration is idempotent",
        "Performance tests show indexes improve queries",
    ]
}

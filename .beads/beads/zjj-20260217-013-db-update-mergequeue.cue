package beads


// Bead ID: isolate-20260217-013-db-update-mergequeue
// Section: 1 of 16
bead_id: "isolate-20260217-013-db-update-mergequeue"

// Section 2: Intent
intent: {
    // What: Update MergeQueue code to use main database instead of queue.db
    what: "Change MergeQueue to query state.db instead of separate queue.db"
    // Why: Single database architecture; queue tables now in state.db
    why: "Separate queue.db is unnecessary complexity; tables migrated to state.db"
    // Value: Simplifies deployment and backup
    value: "Single database for all state; no need to manage multiple files"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update crates/isolate-core/src/coordination/queue.rs",
        "Update crates/isolate-core/src/coordination/queue_entities.rs",
        "Change database queries to use state.db",
        "Remove queue.db path logic",
        "Update queue table names to use zjj_queue prefix",
        "Update MergeQueue struct methods",
    ]
    // Out: What we will NOT do
    out: [
        "Create merge queue tables (Bead 012)",
        "Remove get_queue_db_path function (Bead 015)",
        "Create queue submit command (Bead 023)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-012-db-merge-queue-tables"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-015-db-remove-queue-path", "isolate-20260217-020-self-heal-stale-locks", "isolate-20260217-021-self-heal-orphans", "isolate-20260217-022-self-heal-thresholds", "isolate-20260217-023-queue-submit-cmd", "isolate-20260217-024-merge-train-logic", "isolate-20260217-025-train-failure-auto-rebase", "isolate-20260217-026-queue-new-states"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        merge_queue_tables_exist: bool
        state_db_accessible: bool
    }
    // Output: Produced state/outputs
    output: {
        merge_queue_uses_state_db: bool
        no_queue_db_references: bool
        all_queries_updated: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All queue queries go to state.db",
        "Queue tables use zjj_queue prefix",
        "No references to queue.db",
        "All existing functionality still works",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/isolate-core/src/coordination/queue.rs",
        "Read crates/isolate-core/src/coordination/queue_entities.rs",
        "Grep for references to queue.db or queue_db_path",
        "Update MergeQueue::new() to take state.db pool",
        "Update all queries to use zjj_queue table names",
        "Update table references: merge_queue -> zjj_queue_merge_queue",
        "Update table references: processing_lock -> zjj_queue_processing_lock",
        "Update table references: events -> zjj_queue_events",
        "Remove queue_db_path parameter from constructors",
        "Test all MergeQueue methods",
        "Run cargo test",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        MergeQueue: #"""
            pub struct MergeQueue {
                db: SqlitePool,
            }

            impl MergeQueue {
                pub fn new(db: SqlitePool) -> Self {
                    Self { db }
                }
            }
            """#
    }
    // State: State mutations
    state: {
        // No state mutations - updating queries only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "impl MergeQueue",
        "pub fn new(db: SqlitePool) -> Self",
        "pub async fn add_session(&self, session: &str) -> Result<i64>",
        "pub async fn get_position(&self, session: &str) -> Result<usize>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn query_queue_table(&self) -> &str",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Query fails due to table not found",
        "Query fails due to missing columns",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return descriptive error",
        "Ensure tables exist before querying",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_merge_queue_uses_state_db",
        "test_add_session_works",
        "test_get_position_works",
        "test_all_methods_use_correct_tables",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_merge_queue_end_to_end",
        "test_queue_operations_persist_in_state_db",
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
        "Queries complete in < 100ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use existing indexes",
        "Prepared statements",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log all queue operations",
        "Log query failures",
    ]
    // Metrics: What to measure
    metrics: [
        "merge_queue_query_duration_seconds",
        "merge_queue_operations_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Update MergeQueue documentation",
        "Document that queue is in state.db",
    ]
    // External: External docs needed
    external: [
        "Update architecture documentation",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "May miss some queue.db references",
        "Table name mismatches",
    ]
    // Operational: Operational risks
    operational: [
        "Existing queue.db data needs migration",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "MergeQueue uses state.db",
        "All queries use zjj_queue table names",
        "No references to queue.db in code",
        "No references to queue_db_path",
        "All MergeQueue methods work correctly",
        "Unit tests pass",
        "Integration tests pass",
        "No unwrap() or panic() in updated code",
    ]
    // Should: Nice to have
    should: [
        "Performance tests show no regression",
    ]
}

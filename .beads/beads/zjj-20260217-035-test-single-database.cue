package beads


// Bead ID: zjj-20260217-035-test-single-database
// Section: 1 of 16
bead_id: "zjj-20260217-035-test-single-database"

// Section 2: Intent
intent: {
    // What: Integration tests for single database architecture
    what: "Create end-to-end tests verifying state.db consolidation"
    // Why: Single database migration is complex; needs verification
    why: "Need to ensure queue and session data coexist correctly"
    // Value: Confidence in database consolidation
    value: "Prevents regressions in critical data layer"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create crates/zjj/tests/test_single_database.rs",
        "Test sessions table has parent_session and queue_status",
        "Test merge_queue table exists and works",
        "Test conflict_resolutions table exists and works",
        "Test queue operations use state.db",
        "Test no queue.db file is created",
        "Test foreign key constraints",
        "Test data isolation between tables",
    ]
    // Out: What we will NOT do
    out: [
        "Add columns (Bead 011)",
        "Create merge queue tables (Bead 012)",
        "Update MergeQueue (Bead 013)",
        "Remove queue path (Bead 015)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-015-db-remove-queue-path"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        single_database_implemented: bool
    }
    // Output: Produced state/outputs
    output: {
        test_file_created: bool
        tests_pass: bool
        coverage_adequate: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All tests pass",
        "Tests verify single database",
        "Tests verify no queue.db",
        "Tests verify foreign keys",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create test_single_database.rs",
        "Test: sessions table has parent_session column",
        "Test: sessions table has queue_status column",
        "Test: merge_queue table exists",
        "Test: conflict_resolutions table exists",
        "Test: can insert and query merge_queue",
        "Test: can insert and query conflict_resolutions",
        "Test: queue operations use state.db",
        "Test: no queue.db file created",
        "Test: foreign key constraints enforced",
        "Test: parent_session references valid session",
        "Test: merge_queue.session_name references valid session",
        "Run tests with cargo test",
        "Verify all pass",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - tests only
    }
    // State: State mutations
    state: {
        // Test creates and destroys database
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        // No API - tests only
    ]
    // Internal: Internal helpers
    internal: [
        "async fn get_test_db_path() -> PathBuf",
        "async fn verify_table_exists(db: &SqlitePool, table: &str) -> bool",
        "async fn verify_column_exists(db: &SqlitePool, table: &str, column: &str) -> bool",
        "async fn verify_no_queue_db() -> bool",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Test setup fails",
        "Test assertions fail",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Clean up test database",
        "Report failure",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_sessions_has_parent_session_column",
        "test_sessions_has_queue_status_column",
        "test_merge_queue_table_exists",
        "test_conflict_resolutions_table_exists",
        "test_merge_queue_insert_and_query",
        "test_conflict_resolutions_insert_and_query",
        "test_queue_uses_state_db",
        "test_no_queue_db_created",
        "test_parent_session_foreign_key",
        "test_merge_queue_foreign_key",
        "test_data_isolation",
    ]
    // Integration: Integration scenarios
    integration: [
        // This file IS the integration tests
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - tests only",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "N/A",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Tests run in < 1 minute",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use in-memory database for tests",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "No logging in tests",
    ]
    // Metrics: What to measure
    metrics: [
        "test_duration_seconds",
        "test_pass_rate",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document each test",
        "Document test helpers",
    ]
    // External: External docs needed
    external: [
        "No external docs for tests",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Tests may be flaky",
        "Tests may not catch all bugs",
    ]
    // Operational: Operational risks
    operational: [
        "None - tests only",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "test_single_database.rs exists",
        "Tests verify sessions columns",
        "Tests verify queue tables",
        "Tests verify queue uses state.db",
        "Tests verify no queue.db",
        "Tests verify foreign keys",
        "All tests pass",
        "No unwrap() or panic() in tests (use ?)",
    ]
    // Should: Nice to have
    should: [
        "Tests use in-memory database",
        "Tests run quickly",
        "Tests verify data isolation",
    ]
}

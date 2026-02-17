package beads


// Bead ID: zjj-20260217-015-db-remove-queue-path
// Section: 1 of 16
bead_id: "zjj-20260217-015-db-remove-queue-path"

// Section 2: Intent
intent: {
    // What: Remove get_queue_db_path function and queue.db references
    what: "Delete function that returns queue.db path; no longer needed"
    // Why: Merge queue now uses state.db; queue.db no longer exists
    why: "Single database architecture eliminates need for separate queue.db"
    // Value: Removes dead code and simplifies architecture
    value: "Cleaner codebase with single database path"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Remove get_queue_db_path() function",
        "Remove queue.db path logic",
        "Remove any remaining queue.db file references",
        "Remove queue.db initialization code",
        "Update all callers to use state.db",
    ]
    // Out: What we will NOT do
    out: [
        "Update MergeQueue queries (Bead 013)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-013-db-update-mergequeue"]
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-035-test-single-database"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        merge_queue_uses_state_db: bool
    }
    // Output: Produced state/outputs
    output: {
        no_queue_db_path_function: bool
        no_queue_db_references: bool
    }
    // Invariants: Must remain true
    invariants: [
        "No code references queue.db",
        "All database operations use state.db",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Grep for get_queue_db_path",
        "Grep for queue.db",
        "Remove get_queue_db_path() function",
        "Remove queue_db_path variables",
        "Remove queue.db initialization code",
        "Run cargo check",
        "Run cargo test",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - removing code only
    }
    // State: State mutations
    state: {
        // No state mutations - removing code only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        // No API - removing function
    ]
    // Internal: Internal helpers
    internal: [
        // No internal helpers - removing function
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Code still references get_queue_db_path",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Remove all references",
        "Update callers",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_no_queue_db_path_function",
        "test_no_queue_db_references",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_all_database_operations_use_state_db",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - removing code only",
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
        "No performance impact - removing code",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "N/A",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "No logging needed",
    ]
    // Metrics: What to measure
    metrics: [
        "lines_of_code_removed",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "No inline docs - removing code only",
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
        "May miss some references",
    ]
    // Operational: Operational risks
    operational: [
        "None - removing dead code",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "get_queue_db_path function removed",
        "No queue.db references in code",
        "cargo check passes",
        "cargo test passes",
        "Grep finds no queue.db references",
    ]
    // Should: Nice to have
    should: [
        "No dead code left behind",
    ]
}

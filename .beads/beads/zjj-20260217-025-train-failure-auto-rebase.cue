package beads


// Bead ID: isolate-20260217-025-train-failure-auto-rebase
// Section: 1 of 16
bead_id: "isolate-20260217-025-train-failure-auto-rebase"

// Section 2: Intent
intent: {
    // What: Handle train failure and auto-rebase remaining sessions
    what: "When merge fails, rebase subsequent sessions onto latest base"
    // Why: Merge failures invalidate train; need to recover
    why: "Graphite-style trains auto-rebase on failure to recover"
    // Value: Self-healing merge train
    value: "AI agents don't need to manually rebase after failures"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Detect merge failure in train",
        "Kick failed session from queue",
        "Rebase subsequent sessions",
        "Update positions",
        "Emit TrainResult with kicked list",
        "Emit JSONL output",
    ]
    // Out: What we will NOT do
    out: [
        "Process merge train (Bead 024)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-024-merge-train-logic"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        failed_session: string
        remaining_sessions: []
    }
    // Output: Produced state/outputs
    output: {
        failed_session_kicked: bool
        remaining_sessions_rebased: bool
        positions_updated: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Failed session is removed from queue",
        "Remaining sessions are rebased",
        "Positions are sequential",
        "TrainResult includes kicked list",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "In train processing, catch merge failures",
        "On failure:",
        "  - Mark session as 'kicked'",
        "  - Remove from merge_queue",
        "  - For each subsequent session:",
        "    - Rebase onto latest base",
        "    - Update position",
        "  - Emit TrainResult with kicked list",
        "  - Restart train",
        "Handle rebase failures",
        "Test with merge failure",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // Uses existing TrainResult type
    }
    // State: State mutations
    state: {
        sessions_kicked: []
        sessions_rebased: []
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub async fn handle_train_failure(failed_session: &str) -> Result<TrainResult>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn kick_session(db: &SqlitePool, session: &str) -> Result<()>",
        "async fn rebase_subsequent_sessions(db: &SqlitePool, from_position: i64) -> Result<Vec<String>>",
        "async fn update_positions(db: &SqlitePool) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Rebase fails",
        "Position update fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Mark sessions as 'blocked'",
        "Emit TrainResult with errors",
        "Don't restart train",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_handle_train_failure_kicks_session",
        "test_handle_train_failure_rebases_remaining",
        "test_handle_train_failure_updates_positions",
        "test_handle_train_failure_emits_kicked_list",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_train_failure_recovery",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - rebase only",
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
        "Rebase completes in < 2 minutes per session",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Parallel rebase",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log train failure",
        "Log kick",
        "Log rebase operations",
    ]
    // Metrics: What to measure
    metrics: [
        "train_failures_total",
        "sessions_kicked_total",
        "auto_rebase_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document failure handling",
        "Document auto-rebase",
    ]
    // External: External docs needed
    external: [
        "Add documentation",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Rebase may fail",
        "May kick too many sessions",
    ]
    // Operational: Operational risks
    operational: [
        "May lose work",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "Failure detection works",
        "Kick works",
        "Rebase works",
        "Position update works",
        "Emits TrainResult with kicked",
        "Handles rebase failures",
        "Unit tests pass",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Configurable rebase strategy",
    ]
}

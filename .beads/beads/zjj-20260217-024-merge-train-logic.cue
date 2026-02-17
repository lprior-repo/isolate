package beads


// Bead ID: zjj-20260217-024-merge-train-logic
// Section: 1 of 16
bead_id: "zjj-20260217-024-merge-train-logic"

// Section 2: Intent
intent: {
    // What: Implement merge train processing
    what: "Create train processing logic that merges sessions in order"
    // Why: Graphite-style merge train processes queue sequentially
    why: "Need to process merge queue with automatic testing and merging"
    // Value: Automated merge pipeline
    value: "AI agents can submit to queue and auto-merge"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create crates/zjj-core/src/coordination/train.rs",
        "Implement train processing loop",
        "For each entry in queue:",
        "  - Update status to 'checking'",
        "  - Emit TrainStep line",
        "  - Run tests",
        "  - Check for conflicts",
        "  - Update status to 'mergeable' or 'blocked'",
        "  - Merge if mergeable",
        "  - Emit TrainResult line",
        "Handle train failures",
        "Emit JSONL output",
    ]
    // Out: What we will NOT do
    out: [
        "Handle train failure and auto-rebase (Bead 025)",
        "Add new queue states (Bead 026)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-023-queue-submit-cmd"]
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-025-train-failure-auto-rebase", "zjj-20260217-033-test-merge-train"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        queue_entries: []
    }
    // Output: Produced state/outputs
    output: {
        train_processed: bool
        steps_emitted: bool
        results_emitted: bool
        sessions_merged: int
    }
    // Invariants: Must remain true
    invariants: [
        "Train processes entries in position order",
        "Each step emits TrainStep line",
        "Each result emits TrainResult line",
        "Status updates persisted to database",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create train.rs module",
        "Implement process_train() function",
        "Query queue entries ordered by position",
        "For each entry:",
        "  - Update status to 'checking'",
        "  - Emit TrainStep line",
        "  - Run tests",
        "  - Check conflicts",
        "  - If pass: merge, update status to 'merged'",
        "  - If fail: update status to 'blocked'",
        "  - Emit TrainResult line",
        "Emit final Train line",
        "Handle errors",
        "Test with sample queue",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        TrainProcessor: #"""
            pub struct TrainProcessor {
                db: SqlitePool,
            }

            impl TrainProcessor {
                pub async fn process_train(&self) -> Result<TrainResult> {
                    // Process all entries
                }
            }
            """#
    }
    // State: State mutations
    state: {
        entries_processed: int
        sessions_merged: int
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub struct TrainProcessor",
        "pub async fn process_train(&self) -> Result<TrainResult>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn process_entry(&self, entry: &QueueEntry) -> Result<TrainStep>",
        "async fn run_tests(&self, session: &str) -> Result<bool>",
        "async fn check_conflicts(&self, session: &str) -> Result<bool>",
        "async fn merge_session(&self, session: &str) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Test fails",
        "Conflict detected",
        "Merge fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Mark entry as 'blocked'",
        "Emit TrainResult with error",
        "Continue with next entry",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_process_train_processes_entries",
        "test_process_train_emits_steps",
        "test_process_train_emits_results",
        "test_process_train_handles_failures",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_train_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - processing queue",
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
        "Each entry processes in < 5 minutes",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Parallel tests",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log train start/end",
        "Log each step",
    ]
    // Metrics: What to measure
    metrics: [
        "train_duration_seconds",
        "train_entries_processed",
        "train_failures_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document train processing",
        "Document status transitions",
    ]
    // External: External docs needed
    external: [
        "Add documentation",
        "Add examples",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Train may get stuck",
        "Tests may hang",
    ]
    // Operational: Operational risks
    operational: [
        "Slow train blocks queue",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "train.rs module exists",
        "process_train() function exists",
        "Processes entries in order",
        "Emits TrainStep for each entry",
        "Emits TrainResult for each entry",
        "Updates status in database",
        "Handles failures gracefully",
        "Unit tests pass",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Configurable timeout",
    ]
}

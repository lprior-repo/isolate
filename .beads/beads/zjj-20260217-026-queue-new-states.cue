package beads


// Bead ID: zjj-20260217-026-queue-new-states
// Section: 1 of 16
bead_id: "zjj-20260217-026-queue-new-states"

// Section 2: Intent
intent: {
    // What: Add new queue states (draft, blocked, checking, mergeable)
    what: "Extend QueueStatus enum with new states for train processing"
    // Why: Current states don't capture merge train workflow
    why: "Need granular states for merge train processing stages"
    // Value: Better visibility into queue processing
    value: "AI agents can track merge progress in detail"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update QueueStatus enum",
        "Add Draft state (initial)",
        "Add Blocked state (conflicts/failures)",
        "Add Checking state (tests running)",
        "Add Mergeable state (ready to merge)",
        "Update state transitions in train logic",
        "Update JSONL output types",
    ]
    // Out: What we will NOT do
    out: [
        "Process merge train (Bead 024)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-023-queue-submit-cmd"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        existing_queue_status: string
    }
    // Output: Produced state/outputs
    output: {
        new_states_added: bool
        state_transitions_defined: bool
        jsonl_output_updated: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All states are valid QueueStatus values",
        "State transitions are valid",
        "States serialize to strings",
        "JSONL output includes new states",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read QueueStatus enum",
        "Add Draft variant",
        "Add Blocked variant",
        "Add Checking variant",
        "Add Mergeable variant",
        "Define valid transitions:",
        "  - draft -> checking",
        "  - checking -> mergeable | blocked",
        "  - mergeable -> merged",
        "Update train logic to use new states",
        "Update JSONL serialization",
        "Write unit tests",
        "Test state transitions",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        QueueStatus_extended: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum QueueStatus {
                Draft,      // Initial state
                Checking,   // Tests/conflict checks running
                Mergeable,  // Ready to merge
                Blocked,    // Conflicts or failures
                Merged,     // Successfully merged
                Kicked,     // Removed from queue
            }
            """#
    }
    // State: State mutations
    state: {
        // No state mutations - enum definition only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub enum QueueStatus { Draft, Checking, Mergeable, Blocked, Merged, Kicked }",
        "impl QueueStatus { pub fn can_transition_to(&self, other: &Self) -> bool }",
    ]
    // Internal: Internal helpers
    internal: [
        "fn validate_transition(from: &QueueStatus, to: &QueueStatus) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Invalid state transition",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error",
        "Don't update state",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_draft_can_transition_to_checking",
        "test_checking_can_transition_to_mergeable_or_blocked",
        "test_mergeable_can_transition_to_merged",
        "test_invalid_transition_fails",
        "test_all_states_serialize_to_strings",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_state_transitions_in_train",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - enum values",
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
        "No performance impact - enum values",
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
        "Log state transitions",
    ]
    // Metrics: What to measure
    metrics: [
        "queue_state_transitions_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document all states",
        "Document valid transitions",
    ]
    // External: External docs needed
    external: [
        "Add state machine diagram",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Invalid transitions may break logic",
    ]
    // Operational: Operational risks
    operational: [
        "None - adding states only",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "Draft state exists",
        "Checking state exists",
        "Mergeable state exists",
        "Blocked state exists",
        "Merged state exists",
        "Kicked state exists",
        "Transition validation exists",
        "States serialize to strings",
        "Unit tests pass",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "State machine diagram",
    ]
}

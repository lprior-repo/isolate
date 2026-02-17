package beads


// Bead ID: zjj-20260217-002-jsonl-stack-queue-types
// Section: 1 of 16
bead_id: "zjj-20260217-002-jsonl-stack-queue-types"

// Section 2: Intent
intent: {
    // What: Add stack and queue output types to OutputLine enum
    what: "Add Stack, QueueSummary, QueueEntry, Train, TrainStep, TrainResult variants to OutputLine"
    // Why: Merge queue and stack commands need structured output for AI agents
    why: "Core types don't include merge train or stack visualization data"
    // Value: Enables AI agents to understand merge queue state and train processing
    value: "AI agents can monitor merge train progress and queue state via structured JSON"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add Stack variant to OutputLine enum",
        "Add QueueSummary variant to OutputLine enum",
        "Add QueueEntry variant to OutputLine enum",
        "Add Train variant to OutputLine enum",
        "Add TrainStep variant to OutputLine enum",
        "Add TrainResult variant to OutputLine enum",
        "Define QueueStatus, TrainStatus, TrainAction, StepStatus enums",
        "Add relevant fields to each variant",
    ]
    // Out: What we will NOT do
    out: [
        "Create JsonlWriter (Bead 003)",
        "Implement stack commands (Bead 017-019)",
        "Implement merge train logic (Bead 024)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-001-jsonl-core-types"]
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-004-jsonl-tests"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        output_line_enum_exists: bool
    }
    // Output: Produced state/outputs
    output: {
        stack_variant_exists: bool
        queue_variants_count: int  // Should be 2 (QueueSummary, QueueEntry)
        train_variants_count: int  // Should be 3 (Train, TrainStep, TrainResult)
        all_variants_serializable: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All new variants have 'type' discriminator field",
        "QueueStatus enum values match database states",
        "TrainStatus enum values match processing states",
        "All variants are serializable to JSON",
        "Stack variant includes parent and children fields",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/zjj-core/src/output/types.rs",
        "Define QueueStatus enum (Draft, Ready, Blocked, Checking, Mergeable, Merged, Kicked)",
        "Define TrainStatus enum (Idle, Running, Paused, Complete, Failed)",
        "Define TrainAction enum (Merge, Rebase, Skip, Kick)",
        "Define StepStatus enum (Pending, Running, Complete, Failed, Skipped)",
        "Add Stack variant with name, parent, children, base fields",
        "Add QueueSummary variant with total, ready, blocked, draft fields",
        "Add QueueEntry variant with position, session, status, blocked_by fields",
        "Add Train variant with status, sessions fields",
        "Add TrainStep variant with session, action, status fields",
        "Add TrainResult variant with status, merged, failed, kicked fields",
        "Add #[serde(tag = \"type\")] to all new variants",
        "Write unit tests for all new variants",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        OutputLine_extensions: #"""
            // Extensions to OutputLine enum
            Stack {
                name: String,
                parent: Option<String>,
                children: Vec<String>,
                base: String,
            },
            QueueSummary {
                total: usize,
                ready: usize,
                blocked: usize,
                draft: usize,
            },
            QueueEntry {
                position: usize,
                session: String,
                status: String,  // QueueStatus serialized
                blocked_by: Vec<String>,
            },
            Train {
                status: String,  // TrainStatus serialized
                sessions: Vec<String>,
            },
            TrainStep {
                session: String,
                action: String,  // TrainAction serialized
                status: String,  // StepStatus serialized
            },
            TrainResult {
                status: String,  // Status serialized
                merged: usize,
                failed: usize,
                kicked: Vec<String>,
            },
            """#

        QueueStatus: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum QueueStatus {
                Draft,
                Ready,
                Blocked,
                Checking,
                Mergeable,
                Merged,
                Kicked,
            }
            """#

        TrainStatus: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum TrainStatus {
                Idle,
                Running,
                Paused,
                Complete,
                Failed,
            }
            """#

        TrainAction: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum TrainAction {
                Merge,
                Rebase,
                Skip,
                Kick,
            }
            """#

        StepStatus: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum StepStatus {
                Pending,
                Running,
                Complete,
                Failed,
                Skipped,
            }
            """#
    }
    // State: State mutations
    state: {
        // No state mutations - defining types only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub enum QueueStatus",
        "pub enum TrainStatus",
        "pub enum TrainAction",
        "pub enum StepStatus",
        "// OutputLine enum with new variants",
    ]
    // Internal: Internal helpers
    internal: [
        // No internal helpers - pure data types
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Serialization fails on new variants",
        "Invalid QueueStatus value from database",
        "Empty Vec fields cause validation issues",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Use String representation for enums to handle unknown values",
        "Default to Draft for unknown QueueStatus",
        "Return error from serialization, never panic",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_stack_variant_serializes_with_parent_and_children",
        "test_queue_summary_variant_serializes_with_counts",
        "test_queue_entry_variant_serializes_with_position_and_status",
        "test_train_variant_serializes_with_status_and_sessions",
        "test_train_step_variant_serializes_with_action_and_status",
        "test_train_result_variant_serializes_with_kicked_list",
        "test_queue_status_enum_serializes_to_lowercase",
        "test_train_status_enum_serializes_to_lowercase",
        "test_all_stack_queue_variants_have_type_field",
    ]
    // Integration: Integration scenarios
    integration: [
        // No integration tests - types only
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Queue entries may expose session names",
        "Train results may reveal merge failures",
        "Blocked sessions may leak sensitive info",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Session names are already visible in status command",
        "No additional sensitive data in new variants",
        "Document that queue data is not secret",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Serialization should complete in < 1ms per variant",
        "Memory usage should be minimal for enum variants",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use Vec<String> for blocked_by and kicked (efficient for small lists)",
        "Avoid unnecessary allocations in variant fields",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        // No logging - pure data types
    ]
    // Metrics: What to measure
    metrics: [
        // No metrics - pure data types
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document all new OutputLine variants with examples",
        "Document QueueStatus enum values and transitions",
        "Document TrainStatus enum values and lifecycle",
        "Document TrainAction enum values",
        "Document that Stack shows parent-child relationships",
    ]
    // External: External docs needed
    external: [
        "Update OutputLine schema documentation",
        "Document merge queue output format",
        "Document stack visualization format",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "QueueStatus values may not match database schema",
        "TrainStatus values may not match processing logic",
        "Empty vectors may serialize inconsistently",
    ]
    // Operational: Operational risks
    operational: [
        "Adding new variants breaks existing parsers",
        "Changing enum values breaks backward compatibility",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "Stack variant exists with name, parent, children, base fields",
        "QueueSummary variant exists with total, ready, blocked, draft fields",
        "QueueEntry variant exists with position, session, status, blocked_by fields",
        "Train variant exists with status, sessions fields",
        "TrainStep variant exists with session, action, status fields",
        "TrainResult variant exists with status, merged, failed, kicked fields",
        "QueueStatus, TrainStatus, TrainAction, StepStatus enums exist",
        "All new variants have #[serde(tag = \"type\")] attribute",
        "All new variants serialize to valid JSON with 'type' field",
        "Unit tests pass for all new variants",
        "No unwrap() or panic() in type definitions",
    ]
    // Should: Nice to have
    should: [
        "All variants have comprehensive documentation comments",
        "Examples of serialized JSON for each variant",
        "Enums use #[serde(rename_all = \"lowercase\")] for consistency",
    ]
}

package beads


// Bead ID: zjj-20260217-001-jsonl-core-types
// Section: 1 of 16
bead_id: "zjj-20260217-001-jsonl-core-types"

// Section 2: Intent
intent: {
    // What: Create OutputLine enum with core assessment types
    what: "Create OutputLine enum with Summary, Session, Issue, Plan, Action, Warning, Result, Error, Recovery, Context variants"
    // Why: AI agents need structured, streaming output - JSONL format requires type discrimination
    why: "Current output system lacks structured types for machine consumption"
    // Value: Foundation for all JSONL output across the CLI
    value: "Enables AI agents to parse command results, progress, and errors via structured JSON stream"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create OutputLine enum in crates/zjj-core/src/output/types.rs",
        "Add core variants: Summary, Session, Issue, Plan, Action, Warning, Result, Error, Recovery, Context",
        "Add #[serde(tag = \"type\")] for JSON discrimination",
        "Define relevant fields for each variant",
        "Add Severity, Status, ActionStatus enums",
    ]
    // Out: What we will NOT do
    out: [
        "Add stack/queue variants (Bead 002)",
        "Create JsonlWriter (Bead 003)",
        "Remove human output format (Bead 005)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: []  // No dependencies - foundation bead
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-002-jsonl-stack-queue-types", "zjj-20260217-003-jsonl-writer-emit"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        // No inputs - creating new types
    }
    // Output: Produced state/outputs
    output: {
        output_line_enum_exists: bool
        core_variants_count: int  // Should be 10
        all_variants_have_type_field: bool
        serializable_to_json: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Every OutputLine variant has 'type' discriminator field",
        "All variants are serializable to JSON",
        "Context variant exists and is always emitted last",
        "Error variant includes code and message fields",
        "All fields are public for JSON serialization",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create crates/zjj-core/src/output/types.rs",
        "Define Severity enum (Error, Warning, Info)",
        "Define Status enum (Success, Failure, Pending)",
        "Define ActionStatus enum (Pending, Running, Complete, Failed, Skipped)",
        "Create OutputLine enum with #[serde(tag = \"type\")]",
        "Add Summary variant with total, active, stale, conflict, orphaned fields",
        "Add Session variant with name, state, age_days, owned_by, action fields",
        "Add Issue variant with severity, message, session, suggested_action fields",
        "Add Plan variant with command, would_execute fields",
        "Add Action variant with id, verb, target, name, reason, safe, status fields",
        "Add Warning variant with message, affected, data_loss fields",
        "Add Result variant with status, completed, failed fields",
        "Add Error variant with code, message, details fields",
        "Add Recovery variant with suggestions field",
        "Add Context variant with for_human, text fields",
        "Add #[derive(Debug, Clone, Serialize, Deserialize)] to all types",
        "Write unit tests for all variant serialization",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        OutputLine: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(tag = "type")]
            pub enum OutputLine {
                Summary {
                    total: usize,
                    active: usize,
                    stale: usize,
                    conflict: usize,
                    orphaned: usize,
                },
                Session {
                    name: String,
                    state: String,  // WorkspaceState serialized
                    age_days: u64,
                    owned_by: Option<String>,
                    action: Option<String>,
                },
                Issue {
                    severity: String,  // Severity serialized
                    message: String,
                    session: Option<String>,
                    suggested_action: String,
                },
                Plan {
                    command: String,
                    would_execute: bool,
                },
                Action {
                    id: usize,
                    verb: String,
                    target: String,
                    name: String,
                    reason: Option<String>,
                    safe: bool,
                    status: String,  // ActionStatus serialized
                },
                Warning {
                    message: String,
                    affected: Vec<String>,
                    data_loss: bool,
                },
                Result {
                    status: String,  // Status serialized
                    completed: usize,
                    failed: usize,
                },
                Error {
                    code: String,
                    message: String,
                    details: Option<serde_json::Value>,
                },
                Recovery {
                    suggestions: Vec<String>,
                },
                Context {
                    for_human: String,
                    text: String,
                },
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
        "pub enum OutputLine",
        "pub enum Severity",
        "pub enum Status",
        "pub enum ActionStatus",
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
        "Serialization fails (non-serializable data in variants)",
        "Invalid enum values for Status/Severity",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Use only serializable types in OutputLine variants",
        "String-based enum serialization for compatibility",
        "Return error from serialization, never panic",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_summary_variant_serializes_with_type_field",
        "test_session_variant_serializes_with_type_field",
        "test_issue_variant_serializes_with_type_field",
        "test_error_variant_serializes_with_code_and_message",
        "test_context_variant_serializes_with_for_human_and_text",
        "test_all_core_variants_serialize_to_valid_json",
        "test_severity_enum_serializes_to_string",
        "test_status_enum_serializes_to_string",
    ]
    // Integration: Integration scenarios
    integration: [
        // No integration tests - types only, tested in Bead 003
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Session names may leak sensitive info",
        "Error messages may contain file paths",
        "Details field in Error may contain secrets",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Document that sensitive data should be redacted before creating OutputLine",
        "Error code should be generic, not revealing internals",
        "Details field should be optional and used sparingly",
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
        "Use String instead of Cow<'_, str> for simplicity",
        "Avoid unnecessary allocations in variant fields",
        "Benchmark serialization with criterion",
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
        "Document all OutputLine variants with examples",
        "Document each field's purpose and valid values",
        "Add module-level documentation explaining JSONL format",
        "Document that Context should always be emitted last",
    ]
    // External: External docs needed
    external: [
        "Add OutputLine schema to API documentation",
        "Document JSONL output format for AI agents",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Serde may fail on complex nested types",
        "Enum string values may change, breaking parsers",
        "Missing fields in variants may cause serialization errors",
    ]
    // Operational: Operational risks
    operational: [
        "Adding new variants requires updating all parsers",
        "Changing field names breaks backward compatibility",
        "Large outputs may consume excessive memory",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "OutputLine enum exists with all 10 core variants",
        "All variants have #[serde(tag = \"type\")] attribute",
        "All core variants serialize to valid JSON with 'type' field",
        "Severity, Status, ActionStatus enums exist and serialize to strings",
        "Unit tests pass for all variants",
        "No unwrap() or panic() in type definitions",
    ]
    // Should: Nice to have
    should: [
        "All variants have comprehensive documentation comments",
        "Examples of serialized JSON for each variant",
        "Types are re-exported from zjj-core crate root",
    ]
}

package beads


// Bead ID: isolate-20260217-028-conflict-resolve-cmd
// Section: 1 of 16
bead_id: "isolate-20260217-028-conflict-resolve-cmd"

// Section 2: Intent
intent: {
    // What: Implement `zjj conflict resolve <session> --decision '<json>'` command
    what: "Apply conflict resolution decision and update audit log"
    // Why: AI agents need to apply resolutions after analysis
    why: "Analysis is useless without ability to apply resolutions"
    // Value: Complete conflict resolution workflow
    value: "AI agents can resolve conflicts automatically"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add `zjj conflict resolve <session> --decision '<json>'` subcommand",
        "Parse decision JSON (file, strategy, reason)",
        "Validate decision",
        "Apply resolution to file",
        "Mark conflict as resolved",
        "Insert into conflict_resolutions audit table",
        "Emit JSONL output",
    ]
    // Out: What we will NOT do
    out: [
        "Add quality signals (Bead 029)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-027-conflict-analyze-cmd"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-034-test-conflict-resolution"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        session: string
        decision: string  // JSON
    }
    // Output: Produced state/outputs
    output: {
        resolution_applied: bool
        audit_log_inserted: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Decision JSON is valid",
        "Resolution is applied to file",
        "Audit log entry created",
        "Decider is recorded (ai/human)",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add resolve subcommand",
        "Args: <session>, --decision '<json>'",
        "Parse decision JSON",
        "Validate decision structure",
        "Check config autonomy level",
        "Apply resolution to file",
        "Mark conflict resolved",
        "Insert into conflict_resolutions",
        "Emit Result line",
        "Emit Context line",
        "Handle errors",
        "Test with resolution",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        ResolutionDecision: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct ResolutionDecision {
                pub file: String,
                pub strategy: String,  // accept_ours, accept_theirs, manual
                pub reason: String,
                pub confidence: Option<String>,
            }
            """#
    }
    // State: State mutations
    state: {
        resolution_applied: bool
        audit_id: int
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn conflict_resolve(session: &str, decision: &str) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn apply_resolution(session: &str, decision: &ResolutionDecision) -> Result<()>",
        "async fn log_resolution(db: &SqlitePool, resolution: &ConflictResolution) -> Result<i64>",
        "fn validate_decision(decision: &ResolutionDecision) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Invalid decision JSON",
        "File not found",
        "Apply fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Emit Error line",
        "Return error",
        "Don't apply if validation fails",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_conflict_resolve_applies_resolution",
        "test_conflict_resolve_logs_to_audit",
        "test_conflict_resolve_validates_decision",
        "test_conflict_resolve_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_conflict_resolve_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Auto-resolving security conflicts is dangerous",
        "JSON injection in decision",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Check security keywords",
        "Respect config autonomy level",
        "Validate JSON structure",
        "Sanitize file paths",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Resolution applies in < 5 seconds",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "N/A - simple file operations",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log resolution applied",
        "Log audit entry",
    ]
    // Metrics: What to measure
    metrics: [
        "conflicts_resolved_total",
        "conflict_resolve_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document resolve command",
        "Document decision JSON format",
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
        "May apply wrong resolution",
    ]
    // Operational: Operational risks
    operational: [
        "May break code",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "resolve subcommand exists",
        "Parses decision JSON",
        "Validates decision",
        "Applies resolution",
        "Logs to audit",
        "Emits JSONL output",
        "Handles errors",
        "Unit tests pass",
        "Manual test with resolution",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Respects autonomy config",
    ]
}

package beads


// Bead ID: zjj-20260217-027-conflict-analyze-cmd
// Section: 1 of 16
bead_id: "zjj-20260217-027-conflict-analyze-cmd"

// Section 2: Intent
intent: {
    // What: Implement `zjj conflict analyze <session>` command
    what: "Analyze merge conflicts and emit structured resolution options"
    // Why: AI agents need to understand conflicts to resolve them
    why: "Manual conflict resolution doesn't scale for AI agents"
    // Value: Automated conflict analysis and resolution
    value: "AI agents can analyze and resolve conflicts automatically"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create conflict command in crates/zjj/src/commands/conflict.rs",
        "Add `zjj conflict analyze <session>` subcommand",
        "Detect merge conflicts",
        "Analyze conflict markers",
        "Identify conflicting files",
        "Generate resolution options (accept_ours, accept_theirs, manual)",
        "Emit JSONL output with Issue lines for each conflict",
        "Emit Context line last",
    ]
    // Out: What we will NOT do
    out: [
        "Resolve conflicts (Bead 028)",
        "Add quality signals (Bead 029)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-014-db-conflict-resolutions-table", "zjj-20260217-016-config-conflict-resolution"]
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-028-conflict-resolve-cmd", "zjj-20260217-029-conflict-quality-signals"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        session: string
    }
    // Output: Produced state/outputs
    output: {
        conflicts_detected: int
        resolution_options_generated: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All conflicts are detected",
        "Each conflict has resolution options",
        "Security keywords checked",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create conflict.rs",
        "Add analyze subcommand",
        "Detect merge conflicts in session",
        "For each conflict:",
        "  - Parse conflict markers",
        "  - Check for security keywords",
        "  - Generate resolution options",
        "  - Emit Issue line with conflict details",
        "Emit Result line with conflict count",
        "Emit Context line",
        "Handle errors",
        "Test with conflict",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        Conflict: #"""
            struct Conflict {
                file: PathBuf,
                line_start: usize,
                line_end: usize,
                ours: String,
                theirs: String,
                has_security_keywords: bool,
            }
            """#

        ResolutionOption: #"""
            struct ResolutionOption {
                strategy: String,  // accept_ours, accept_theirs, manual
                reason: String,
                safe: bool,
            }
            """#
    }
    // State: State mutations
    state: {
        conflicts_found: int
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn conflict_analyze(session: &str) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn detect_conflicts(session: &str) -> Result<Vec<Conflict>>",
        "fn check_security_keywords(text: &str, keywords: &[String]) -> bool",
        "fn generate_resolution_options(conflict: &Conflict) -> Vec<ResolutionOption>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Session not in conflict",
        "Parse fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Emit Error line",
        "Return error",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_conflict_analyze_detects_conflicts",
        "test_conflict_analyze_checks_security_keywords",
        "test_conflict_analyze_generates_options",
        "test_conflict_analyze_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_conflict_analyze_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Auto-resolving security-sensitive code is dangerous",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Check for security keywords",
        "Mark conflicts with keywords as unsafe",
        "Require manual resolution for unsafe conflicts",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Analysis completes in < 10 seconds",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Parallel file parsing",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log conflicts found",
        "Log security keyword matches",
    ]
    // Metrics: What to measure
    metrics: [
        "conflicts_detected_total",
        "conflicts_analyze_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document conflict analyze",
        "Document resolution options",
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
        "May miss conflicts",
        "May incorrectly mark safe",
    ]
    // Operational: Operational risks
    operational: [
        "Auto-resolve may break code",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "conflict analyze command exists",
        "Detects conflicts",
        "Checks security keywords",
        "Generates resolution options",
        "Emits JSONL output",
        "Handles no conflicts",
        "Unit tests pass",
        "Manual test with conflict",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Configurable security keywords",
    ]
}

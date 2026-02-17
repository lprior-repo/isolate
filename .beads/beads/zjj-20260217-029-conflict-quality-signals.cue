package beads


// Bead ID: zjj-20260217-029-conflict-quality-signals
// Section: 1 of 16
bead_id: "zjj-20260217-029-conflict-quality-signals"

// Section 2: Intent
intent: {
    // What: Add quality signals to conflict analysis
    what: "Include context, risk, and complexity metrics in conflict analysis"
    // Why: Basic conflict analysis lacks context for safe auto-resolution
    why: "AI agents need more information to make good decisions"
    // Value: Better conflict resolution decisions
    value: "AI agents can assess risk and complexity"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add quality signal calculation to conflict analysis",
        "Calculate conflict size (lines affected)",
        "Calculate conflict complexity (nested conflicts)",
        "Calculate risk score (based on security keywords, file type)",
        "Include context (surrounding code)",
        "Add signals to Issue output",
        "Update audit log with signals",
    ]
    // Out: What we will NOT do
    out: [
        "Analyze conflicts (Bead 027)",
        "Resolve conflicts (Bead 028)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-027-conflict-analyze-cmd"]
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-034-test-conflict-resolution"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        conflict: Conflict
    }
    // Output: Produced state/outputs
    output: {
        quality_signals_calculated: bool
        risk_score_assigned: bool
        complexity_score_assigned: bool
        context_extracted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Quality signals are calculated",
        "Risk score is 0-100",
        "Complexity score is 0-100",
        "Context is included",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add QualitySignals struct",
        "Calculate conflict size (line_end - line_start)",
        "Calculate complexity (nested markers)",
        "Calculate risk (keywords, file type, size)",
        "Extract context (surrounding lines)",
        "Add signals to conflict analysis",
        "Include signals in Issue output",
        "Add signals to audit log",
        "Test with various conflicts",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        QualitySignals: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct QualitySignals {
                pub size_lines: usize,
                pub complexity_score: u8,  // 0-100
                pub risk_score: u8,  // 0-100
                pub has_security_keywords: bool,
                pub file_type_risk: u8,  // 0-100
                pub context_lines: Vec<String>,
            }
            """#
    }
    // State: State mutations
    state: {
        // No state mutations - analysis only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn calculate_quality_signals(conflict: &Conflict) -> QualitySignals",
    ]
    // Internal: Internal helpers
    internal: [
        "fn calculate_size(conflict: &Conflict) -> usize",
        "fn calculate_complexity(conflict: &Conflict) -> u8",
        "fn calculate_risk(conflict: &Conflict, keywords: &[String]) -> u8",
        "fn extract_context(file: &Path, line: usize) -> Vec<String>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "File read fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error",
        "Use empty context",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_calculate_quality_signals",
        "test_size_calculation",
        "test_complexity_calculation",
        "test_risk_calculation",
        "test_context_extraction",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_quality_signals_in_analysis",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - analysis only",
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
        "Calculation completes in < 1 second",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Lazy context extraction",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log quality signals",
    ]
    // Metrics: What to measure
    metrics: [
        "quality_signals_calculated_total",
        "average_risk_score",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document quality signals",
        "Document risk scoring",
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
        "Risk scores may be inaccurate",
    ]
    // Operational: Operational risks
    operational: [
        "May lead to bad decisions",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "QualitySignals struct exists",
        "Size calculation works",
        "Complexity calculation works",
        "Risk calculation works",
        "Context extraction works",
        "Signals in Issue output",
        "Signals in audit log",
        "Unit tests pass",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Configurable risk weights",
    ]
}

package beads


// Bead ID: isolate-20260217-005-remove-outputformat-human
// Section: 1 of 16
bead_id: "isolate-20260217-005-remove-outputformat-human"

// Section 2: Intent
intent: {
    // What: Remove OutputFormat::Human variant from the codebase
    what: "Remove Human variant from OutputFormat enum, keep only Json"
    // Why: AI-first control plane has no human-readable output mode
    why: "Dual-mode output adds complexity; AI agents only need JSON"
    // Value: Simplifies codebase and removes unused code path
    value: "Eliminates conditional logic for human output; single output mode"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Remove Human variant from OutputFormat enum in crates/isolate-core/src/output_format.rs",
        "Keep only Json variant",
        "Update all uses of OutputFormat::Human to OutputFormat::Json or remove",
        "Remove --output-format flag from CLI (always JSON now)",
        "Remove conditional logic based on output format",
        "Update commands that check output format",
    ]
    // Out: What we will NOT do
    out: [
        "Update commands to use JSONL output (Bead 006-009)",
        "Remove color dependencies (Bead 010)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: []  // No dependencies - can be done in parallel with Bead 001
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-006-status-jsonl-only", "isolate-20260217-007-list-jsonl-only", "isolate-20260217-008-queue-jsonl-only", "isolate-20260217-009-other-commands-jsonl", "isolate-20260217-010-remove-color-deps"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        output_format_enum_exists: bool
        human_variant_exists: bool
    }
    // Output: Produced state/outputs
    output: {
        output_format_has_only_json: bool
        no_references_to_human_variant: bool
        no_output_format_flag_in_cli: bool
        all_commands_output_json: bool
    }
    // Invariants: Must remain true
    invariants: [
        "OutputFormat enum has exactly one variant: Json",
        "No code checks OutputFormat::Human",
        "All commands always output JSON",
        "No conditional logic based on output format",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Grep for all uses of OutputFormat::Human",
        "Grep for all uses of OutputFormat enum",
        "Read crates/isolate-core/src/output_format.rs",
        "Remove Human variant from OutputFormat enum",
        "Keep only Json variant",
        "Remove --output-format flag from CLI definition",
        "Update commands that match on OutputFormat to use Json directly",
        "Remove conditional logic: if output_format == Human { ... } else { ... }",
        "Replace with direct JSON output code",
        "Run cargo check to verify no compilation errors",
        "Run cargo test to verify no test failures",
        "Grep again to ensure no references to Human remain",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        OutputFormat_before: #"""
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
            pub enum OutputFormat {
                Json,
                Human,  // REMOVE THIS
            }
            """#

        OutputFormat_after: #"""
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
            pub enum OutputFormat {
                Json,
            }
            """#
    }
    // State: State mutations
    state: {
        output_format_enum_variants: 2  // Before
        output_format_enum_variants_after: 1  // After
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub enum OutputFormat { Json }",
    ]
    // Internal: Internal helpers
    internal: [
        // No internal helpers - simple enum removal
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Code still references OutputFormat::Human",
        "Tests fail because they expect Human variant",
        "CLI parsing fails due to missing --output-format flag",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Replace all Human references with Json",
        "Update tests to use Json variant",
        "Remove --output-format from CLI argument definition",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_output_format_has_only_json_variant",
        "test_output_format_serializes_to_json",
        "test_no_human_variant_exists",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_all_commands_output_json_without_flag",
        "test_cli_does_not_accept_output_format_flag",
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
        "Removing conditional logic may slightly improve performance",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "No logging needed for code removal",
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
        "Update OutputFormat documentation to reflect single variant",
        "Remove references to human output format",
    ]
    // External: External docs needed
    external: [
        "Update user guide to remove --output-format flag",
        "Update examples to remove --output-format",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "May break existing scripts that use --output-format flag",
        "Tests may fail if they reference Human variant",
    ]
    // Operational: Operational risks
    operational: [
        "Users may be confused by removal of flag",
        "Breaking change for existing workflows",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "OutputFormat enum has only Json variant",
        "No references to OutputFormat::Human in codebase",
        "No --output-format flag in CLI",
        "All commands output JSON (no conditional logic)",
        "cargo check passes",
        "cargo test passes",
        "Grep for 'Human' finds no references in output_format.rs",
    ]
    // Should: Nice to have
    should: [
        "Documentation updated",
        "git diff shows clean removal",
        "No dead code left behind",
    ]
}

package beads


// Bead ID: isolate-20260217-030-remove-confirm-remove
// Section: 1 of 16
bead_id: "isolate-20260217-030-remove-confirm-remove"

// Section 2: Intent
intent: {
    // What: Remove confirm() from remove command
    what: "Delete interactive confirmation from remove command"
    // Why: AI-first control plane has no interactive prompts
    why: "Interactive confirmations block AI agent workflows"
    // Value: Non-interactive, scriptable remove command
    value: "AI agents can remove sessions without prompts"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Remove confirm() call from remove.rs",
        "Remove --yes/--force flags (now default)",
        "Make removal immediate",
        "Emit JSONL output with Action lines",
        "Emit Result line",
    ]
    // Out: What we will NOT do
    out: [
        "Remove confirm from clean (Bead 031)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-003-jsonl-writer-emit"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-032-remove-force-flags"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        session: string
    }
    // Output: Produced state/outputs
    output: {
        confirm_removed: bool
        removal_immediate: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "No confirm() call",
        "Removal is immediate",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/zjj/src/commands/remove.rs",
        "Find confirm() call",
        "Remove confirm() call",
        "Remove --yes/--force flags",
        "Add emit_jsonl() calls",
        "Emit Action line for each step",
        "Emit Result line",
        "Emit Context line",
        "Test remove",
        "Verify no prompt",
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
        "pub fn remove_command(session: &str) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        // No internal helpers - simple command
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Session doesn't exist",
        "Remove fails",
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
        "test_remove_has_no_confirm",
        "test_remove_emits_jsonl",
        "test_remove_is_immediate",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_remove_works_without_prompt",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Accidental removal is easier",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "AI agents should double-check before removal",
        "Emit Action lines showing what will be removed",
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
        "No logging changes",
    ]
    // Metrics: What to measure
    metrics: [
        "remove_command_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Update remove command docs",
    ]
    // External: External docs needed
    external: [
        "Update documentation",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "May increase accidental removals",
    ]
    // Operational: Operational risks
    operational: [
        "Users may be surprised",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "confirm() removed from remove",
        "--yes/--force flags removed",
        "Removal is immediate",
        "Emits JSONL output",
        "Unit tests pass",
        "Manual test: zjj remove test-session (no prompt)",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Action lines show what will be removed",
    ]
}

package beads


// Bead ID: isolate-20260217-031-remove-confirm-clean
// Section: 1 of 16
bead_id: "isolate-20260217-031-remove-confirm-clean"

// Section 2: Intent
intent: {
    // What: Remove confirm() from clean command
    what: "Delete interactive confirmation from clean command"
    // Why: AI-first control plane has no interactive prompts
    why: "Interactive confirmations block AI agent workflows"
    // Value: Non-interactive, scriptable clean command
    value: "AI agents can clean workspaces without prompts"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Remove confirm() call from clean/mod.rs",
        "Remove --yes/--force flags (now default)",
        "Make cleanup immediate",
        "Add --dry-run flag for safety",
        "Emit JSONL output with Action lines",
        "Emit Result line",
    ]
    // Out: What we will NOT do
    out: [
        "Remove confirm from remove (Bead 030)",
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
        args: []  // may include --dry-run
    }
    // Output: Produced state/outputs
    output: {
        confirm_removed: bool
        cleanup_immediate: bool
        dry_run_mode_works: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "No confirm() call",
        "Cleanup is immediate (unless --dry-run)",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/isolate/src/commands/clean/mod.rs",
        "Find confirm() call",
        "Remove confirm() call",
        "Remove --yes/--force flags",
        "Add --dry-run flag",
        "Check dry_run flag before actual cleanup",
        "Add emit_jsonl() calls",
        "Emit Action line for each step",
        "Emit Result line",
        "Emit Context line",
        "Test clean",
        "Verify no prompt",
        "Test --dry-run doesn't clean",
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
        "pub fn clean_command(dry_run: bool) -> Result<()>",
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
        "Cleanup fails",
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
        "test_clean_has_no_confirm",
        "test_clean_emits_jsonl",
        "test_clean_is_immediate",
        "test_clean_dry_run_doesnt_clean",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_clean_works_without_prompt",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Accidental cleanup is easier",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Add --dry-run flag",
        "Emit Action lines showing what will be cleaned",
        "AI agents should use --dry-run first",
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
        "clean_command_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Update clean command docs",
        "Document --dry-run flag",
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
        "May increase accidental cleanups",
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
        "confirm() removed from clean",
        "--yes/--force flags removed",
        "--dry-run flag added",
        "Cleanup is immediate (unless --dry-run)",
        "Emits JSONL output",
        "Unit tests pass",
        "Manual test: isolate clean (no prompt)",
        "Manual test: isolate clean --dry-run (doesn't clean)",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Action lines show what will be cleaned",
    ]
}

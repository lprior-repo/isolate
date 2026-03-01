package beads


// Bead ID: isolate-20260217-009-other-commands-jsonl
// Section: 1 of 16
bead_id: "isolate-20260217-009-other-commands-jsonl"

// Section 2: Intent
intent: {
    // What: Update remaining commands to emit JSONL output only
    what: "Replace add, remove, sync, focus, and other commands output with emit_jsonl() calls"
    // Why: AI agents need structured output from all commands
    why: "Remaining commands still have human-readable output; not parseable"
    // Value: All commands emit consistent structured JSONL
    value: "Complete JSONL migration across entire CLI"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update crates/isolate/src/commands/add.rs",
        "Update crates/isolate/src/commands/remove.rs",
        "Update crates/isolate/src/commands/sync.rs",
        "Update crates/isolate/src/commands/focus.rs",
        "Update other commands with output",
        "Add emit_jsonl() calls for appropriate OutputLine variants",
        "Emit Context line last",
    ]
    // Out: What we will NOT do
    out: [
        "Update status, list, queue (Bead 006-008)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-003-jsonl-writer-emit", "isolate-20260217-005-remove-outputformat-human"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        emit_jsonl_function_exists: bool
        output_line_types_exist: bool
    }
    // Output: Produced state/outputs
    output: {
        all_commands_emit_jsonl: bool
        context_emitted_last: bool
        no_human_output: bool
    }
    // Invariants: Must remain true
    invariants: [
        "All commands emit JSONL output",
        "All commands emit Context as last line",
        "All output is valid JSONL",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "List all commands with output: add, remove, sync, focus, etc.",
        "For each command:",
        "  Read command file",
        "  Import emit_jsonl",
        "  Remove human formatting",
        "  Add emit_jsonl() calls",
        "  Emit Context last",
        "Test each command: isolate <cmd> | jq -c .",
        "Verify all output is valid JSON",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - uses OutputLine from Bead 001-003
    }
    // State: State mutations
    state: {
        // No state mutations - updating command output only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn add_command() -> Result<()>",
        "pub fn remove_command() -> Result<()>",
        "pub fn sync_command() -> Result<()>",
        "pub fn focus_command() -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn emit_result_line(result: &Result) -> OutputLine",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Command fails",
        "Serialization fails",
        "emit_jsonl() fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error from command",
        "Emit Error line before returning",
        "Never panic - always return Result",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_add_emits_result_and_context",
        "test_remove_emits_result_and_context",
        "test_sync_emits_result_and_context",
        "test_focus_emits_result_and_context",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_all_commands_output_parsable_by_jq",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No new security concerns - changing output only",
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
        "All commands should complete in < 1 second",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "N/A - output changes don't affect performance",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log command execution",
    ]
    // Metrics: What to measure
    metrics: [
        "command_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document JSONL output format for each command",
    ]
    // External: External docs needed
    external: [
        "Update all command documentation",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "May miss some commands",
        "Output format inconsistencies",
    ]
    // Operational: Operational risks
    operational: [
        "Output format changes break AI parsers",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "All commands emit JSONL output",
        "All commands emit Context last",
        "All output is valid JSON",
        "No human-readable output remains",
        "Unit tests pass",
        "Manual test: all commands | jq -c .",
    ]
    // Should: Nice to have
    should: [
        "Consistent output format across all commands",
    ]
}

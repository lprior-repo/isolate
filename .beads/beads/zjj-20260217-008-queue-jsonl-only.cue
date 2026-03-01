package beads


// Bead ID: isolate-20260217-008-queue-jsonl-only
// Section: 1 of 16
bead_id: "isolate-20260217-008-queue-jsonl-only"

// Section 2: Intent
intent: {
    // What: Update queue command to emit JSONL output only
    what: "Replace queue command output with emit_jsonl() calls"
    // Why: AI agents need structured merge queue state
    why: "Current queue command has human-readable output; not parseable"
    // Value: AI agents can monitor merge queue via structured JSON
    value: "Enables AI agents to understand queue state and entries"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update crates/isolate/src/commands/queue.rs",
        "Remove human-readable formatting",
        "Add emit_jsonl() calls for QueueSummary, QueueEntry lines",
        "Emit one QueueSummary line",
        "Emit one QueueEntry line per queue entry",
        "Emit Context line last",
    ]
    // Out: What we will NOT do
    out: [
        "Create OutputLine types (Bead 002)",
        "Remove OutputFormat::Human (Bead 005)",
        "Implement queue submit (Bead 023)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-003-jsonl-writer-emit", "isolate-20260217-005-remove-outputformat-human"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-023-queue-submit-cmd", "isolate-20260217-024-merge-train-logic", "isolate-20260217-025-train-failure-auto-rebase", "isolate-20260217-026-queue-new-states"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        emit_jsonl_function_exists: bool
        queue_output_types_exist: bool
    }
    // Output: Produced state/outputs
    output: {
        queue_emits_jsonl: bool
        queue_summary_emitted: bool
        queue_entry_lines_emitted: bool
        context_emitted_last: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Queue emits exactly one QueueSummary line",
        "Queue emits one QueueEntry line per entry",
        "Queue always emits Context as last line",
        "All output is valid JSONL",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/isolate/src/commands/queue.rs",
        "Import emit_jsonl from isolate_core::output::jsonl",
        "Import OutputLine types including QueueSummary, QueueEntry",
        "Remove human formatting code",
        "Query merge queue from database",
        "Emit OutputLine::QueueSummary with totals",
        "For each entry: emit OutputLine::QueueEntry",
        "Emit OutputLine::Context last",
        "Test: isolate queue | jq -c .",
        "Verify each line is valid JSON",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - uses OutputLine from Bead 002
    }
    // State: State mutations
    state: {
        // No state mutations - read-only command
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn queue_command() -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn emit_queue_summary(summary: &QueueSummary) -> OutputLine",
        "fn emit_queue_entry(entry: &QueueEntry) -> OutputLine",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Database query fails",
        "Serialization fails",
        "emit_jsonl() fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error from queue command",
        "Emit Error line before returning",
        "Never panic - always return Result",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_queue_emits_queue_summary_line",
        "test_queue_emits_queue_entry_line_for_each_entry",
        "test_queue_emits_context_last",
        "test_queue_handles_empty_queue",
        "test_queue_handles_database_error",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_queue_output_parsable_by_jq",
        "test_queue_shows_correct_entry_count",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Queue may expose session names",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Session names are not sensitive",
        "No secrets in queue output",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Queue should complete in < 500ms for 100 entries",
        "Each emit() call < 10ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Stream entries, don't load all into memory",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log queue command start",
        "Log entry count",
    ]
    // Metrics: What to measure
    metrics: [
        "queue_command_duration_seconds",
        "queue_entries_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document queue command JSONL output format",
    ]
    // External: External docs needed
    external: [
        "Update queue command documentation",
        "Add examples of parsing queue output",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Large entry counts may produce slow output",
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
        "Queue command emits JSONL output",
        "QueueSummary line is emitted first",
        "QueueEntry lines emitted for each entry",
        "Context line emitted last",
        "All output lines are valid JSON",
        "Output can be parsed by jq -c",
        "No human-readable formatting",
        "Unit tests pass",
        "Manual test: isolate queue | jq -c .",
    ]
    // Should: Nice to have
    should: [
        "Queue completes in < 500ms for 100 entries",
    ]
}

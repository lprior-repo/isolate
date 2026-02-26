package beads


// Bead ID: isolate-20260217-007-list-jsonl-only
// Section: 1 of 16
bead_id: "isolate-20260217-007-list-jsonl-only"

// Section 2: Intent
intent: {
    // What: Update list command to emit JSONL output only
    what: "Replace list command output with emit_jsonl() calls"
    // Why: AI agents need structured session listing
    why: "Current list command has human-readable output; not parseable"
    // Value: AI agents can query and parse session lists
    value: "Enables AI agents to discover and filter sessions via JSON"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update crates/zjj/src/commands/list.rs",
        "Remove human-readable formatting",
        "Add emit_jsonl() calls for Session lines",
        "Emit one Session line per session",
        "Emit Context line last",
    ]
    // Out: What we will NOT do
    out: [
        "Create OutputLine types (Bead 001-003)",
        "Remove OutputFormat::Human (Bead 005)",
        "Update other commands (Bead 006, 008-009)",
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
        list_emits_jsonl: bool
        session_lines_emitted: bool
        one_line_per_session: bool
        context_emitted_last: bool
    }
    // Invariants: Must remain true
    invariants: [
        "List emits exactly one Session line per session",
        "List always emits Context as last line",
        "All output is valid JSONL",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/zjj/src/commands/list.rs",
        "Import emit_jsonl from zjj_core::output::jsonl",
        "Import OutputLine types",
        "Remove human formatting code",
        "Query sessions from database",
        "For each session: emit OutputLine::Session",
        "Emit OutputLine::Context last",
        "Test: zjj list | jq -c .",
        "Verify each line is valid JSON",
        "Verify Context is last line",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - uses OutputLine from Bead 001
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
        "pub fn list_command() -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn emit_session_line(session: &Session) -> OutputLine",
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
        "Return error from list command",
        "Emit Error line before returning",
        "Never panic - always return Result",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_list_emits_session_line_for_each_session",
        "test_list_emits_context_last",
        "test_list_handles_empty_database",
        "test_list_handles_database_error",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_list_output_parsable_by_jq",
        "test_list_shows_all_sessions",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "List may expose session names",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Session names are not sensitive",
        "No secrets in list output",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "List should complete in < 500ms for 1000 sessions",
        "Each emit() call < 10ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Stream sessions, don't load all into memory",
        "Use cursor-based pagination for large datasets",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log list command start",
        "Log session count",
    ]
    // Metrics: What to measure
    metrics: [
        "list_command_duration_seconds",
        "list_sessions_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document list command JSONL output format",
    ]
    // External: External docs needed
    external: [
        "Update list command documentation",
        "Add examples of parsing list output",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Large session counts may produce slow output",
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
        "List command emits JSONL output",
        "Session lines emitted for each session",
        "Context line emitted last",
        "All output lines are valid JSON",
        "Output can be parsed by jq -c",
        "No human-readable formatting",
        "Unit tests pass",
        "Manual test: zjj list | jq -c .",
    ]
    // Should: Nice to have
    should: [
        "List completes in < 500ms for 1000 sessions",
    ]
}

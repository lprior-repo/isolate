package beads


// Bead ID: zjj-20260217-006-status-jsonl-only
// Section: 1 of 16
bead_id: "zjj-20260217-006-status-jsonl-only"

// Section 2: Intent
intent: {
    // What: Update status command to emit JSONL output only
    what: "Replace status command output with emit_jsonl() calls for structured output"
    // Why: AI agents need structured session status data
    why: "Current status command has human-readable tables; not parseable"
    // Value: AI agents can query session status and parse results
    value: "Enables AI agents to understand workspace state via structured JSON"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Update crates/zjj/src/commands/status.rs",
        "Remove table/box-drawing output",
        "Remove color output",
        "Add emit_jsonl() calls for Summary, Session, Issue, Context",
        "Emit one Summary line",
        "Emit one Session line per active session",
        "Emit one Issue line per problem found",
        "Emit Context line last",
    ]
    // Out: What we will NOT do
    out: [
        "Create OutputLine types (Bead 001-003)",
        "Remove OutputFormat::Human (Bead 005)",
        "Update other commands (Bead 007-009)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-003-jsonl-writer-emit", "zjj-20260217-005-remove-outputformat-human"]
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
        status_emits_jsonl: bool
        summary_line_emitted: bool
        session_lines_emitted: bool
        issue_lines_emitted: bool
        context_emitted_last: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Status always emits at least one Summary line",
        "Status always emits Context as last line",
        "Each session gets exactly one Session line",
        "Each issue gets exactly one Issue line",
        "All output is valid JSONL",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/zjj/src/commands/status.rs",
        "Import emit_jsonl from zjj_core::output::jsonl",
        "Import OutputLine types",
        "Remove table/box-drawing code",
        "Remove color code",
        "Query sessions from database",
        "Emit OutputLine::Summary with totals",
        "For each session: emit OutputLine::Session",
        "For each issue: emit OutputLine::Issue",
        "Emit OutputLine::Context last",
        "Test: zjj status | jq -c .",
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
        "pub fn status_command() -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn emit_session_summary(session: &Session) -> OutputLine",
        "fn emit_session_issue(issue: &Issue) -> OutputLine",
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
        "Return error from status command",
        "Emit Error line before returning",
        "Never panic - always return Result",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_status_emits_summary_line",
        "test_status_emits_session_line_for_each_session",
        "test_status_emits_issue_line_for_each_issue",
        "test_status_emits_context_last",
        "test_status_handles_empty_database",
        "test_status_handles_database_error",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_status_output_parsable_by_jq",
        "test_status_shows_correct_session_count",
        "test_status_shows_correct_issue_count",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Status may expose session names",
        "Status may expose file paths in issues",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Session names are not sensitive",
        "File paths are relative to workspace",
        "No secrets in status output",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Status should complete in < 500ms for 100 sessions",
        "Each emit() call < 10ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Batch database queries",
        "Avoid N+1 queries",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log status command start",
        "Log session count",
        "Log issue count",
    ]
    // Metrics: What to measure
    metrics: [
        "status_command_duration_seconds",
        "status_sessions_total",
        "status_issues_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document status command JSONL output format",
        "Document each OutputLine variant emitted",
    ]
    // External: External docs needed
    external: [
        "Update status command documentation",
        "Add examples of parsing status output",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Large session counts may produce slow output",
        "Serialization failures may crash command",
    ]
    // Operational: Operational risks
    operational: [
        "Output format changes break AI parsers",
        "Users may miss human-readable status",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "Status command emits JSONL output",
        "Summary line is emitted first",
        "Session lines emitted for each session",
        "Issue lines emitted for each issue",
        "Context line emitted last",
        "All output lines are valid JSON",
        "Output can be parsed by jq -c",
        "No table/box-drawing output",
        "No color output",
        "Unit tests pass",
        "Manual test: zjj status | jq -c .",
    ]
    // Should: Nice to have
    should: [
        "Status completes in < 500ms for 100 sessions",
        "Output is readable when pretty-printed",
    ]
}

package beads


// Bead ID: isolate-20260217-003-jsonl-writer-emit
// Section: 1 of 16
bead_id: "isolate-20260217-003-jsonl-writer-emit"

// Section 2: Intent
intent: {
    // What: Create JsonlWriter and emit function for streaming JSONL output
    what: "Implement JsonlWriter struct and emit() function for stdout streaming"
    // Why: Commands need to write JSONL output to stdout for AI agents
    why: "OutputLine types exist but no mechanism to write them to stdout"
    // Value: All commands can emit structured output via emit() function
    value: "Enables real-time streaming of command results to AI agents"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create JsonlWriter struct in crates/isolate-core/src/output/mod.rs",
        "Create jsonl.rs with emit() function",
        "Implement JsonlWriter::new() for stdout",
        "Implement JsonlWriter::emit() method",
        "Create public emit_jsonl() function",
        "Add flush() call after each write",
        "Handle serialization errors gracefully",
    ]
    // Out: What we will NOT do
    out: [
        "Create OutputLine types (Bead 001-002)",
        "Update commands to use emit() (Bead 006-009)",
        "Remove human output format (Bead 005)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-001-jsonl-core-types"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-006-status-jsonl-only", "isolate-20260217-007-list-jsonl-only", "isolate-20260217-008-queue-jsonl-only", "isolate-20260217-009-other-commands-jsonl", "isolate-20260217-030-remove-confirm-remove", "isolate-20260217-031-remove-confirm-clean"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        output_line_types_exist: bool
        stdout_handle: bool  // Stdout is available
    }
    // Output: Produced state/outputs
    output: {
        jsonl_writer_exists: bool
        emit_function_exists: bool
        emit_writes_to_stdout: bool
        emit_flushes_after_write: bool
        handles_serialization_errors: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Each emit() call writes one line to stdout",
        "Each line is valid JSON",
        "Flush is called after each write",
        "Errors are returned, never panicked",
        "Output is always UTF-8 encoded",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create crates/isolate-core/src/output/mod.rs with pub exports",
        "Create crates/isolate-core/src/output/jsonl.rs",
        "Import OutputLine types and serde_json",
        "Define JsonlWriter<W: Write> struct with writer field",
        "Implement JsonlWriter::new(writer: W) -> Self",
        "Implement JsonlWriter::emit(&mut self, line: &OutputLine) -> Result<()>",
        "In emit(): serialize line to JSON, write to writer, write newline, flush",
        "Handle serde_json::Error and io::Error",
        "Create pub fn emit_jsonl(line: &OutputLine) -> Result<()> for stdout",
        "Add unit tests for emit() function",
        "Add integration test for stdout streaming",
        "Test error handling for invalid data",
        "Verify flush happens after each write",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        JsonlWriter: #"""
            use std::io::Write;
            use crate::output::types::OutputLine;

            #[derive(Debug)]
            pub struct JsonlWriter<W: Write> {
                writer: W,
            }

            impl<W: Write> JsonlWriter<W> {
                pub fn new(writer: W) -> Self {
                    Self { writer }
                }

                pub fn emit(&mut self, line: &OutputLine) -> Result<()> {
                    let json = serde_json::to_string(line)?;
                    writeln!(self.writer, "{}", json)?;
                    self.writer.flush()?;
                    Ok(())
                }
            }

            impl JsonlWriter<Stdout> {
                pub fn stdout() -> Self {
                    Self::new(std::io::stdout())
                }
            }
            """#

        emit_function: #"""
            use crate::output::types::OutputLine;

            pub fn emit_jsonl(line: &OutputLine) -> Result<()> {
                let json = serde_json::to_string(line)?;
                println!("{}", json);
                Ok(())
            }
            """#
    }
    // State: State mutations
    state: {
        // No state mutations - output is write-only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub struct JsonlWriter<W: Write>",
        "impl<W: Write> JsonlWriter<W>",
        "pub fn emit_jsonl(line: &OutputLine) -> Result<()>",
        "pub mod output { pub mod jsonl; pub mod types; }",
    ]
    // Internal: Internal helpers
    internal: [
        "fn serialize_and_write<W: Write>(writer: &mut W, line: &OutputLine) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Serialization fails (OutputLine contains non-serializable data)",
        "Stdout write fails (pipe broken, process killed)",
        "Flush fails (I/O error)",
        "Invalid UTF-8 in output strings",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return Result::Err from emit() - let caller decide",
        "Log error to stderr before propagating",
        "Ensure partial output is flushed before error return",
        "Never panic - always return Result",
        "Map serde_json::Error and io::Error to crate::Error",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_jsonl_writer_emit_writes_valid_json_line",
        "test_jsonl_writer_emit_adds_newline",
        "test_jsonl_writer_emit_flushes_after_write",
        "test_emit_jsonl_function_writes_to_stdout",
        "test_jsonl_writer_handles_serialization_error",
        "test_jsonl_writer_handles_io_error",
        "test_emit_with_context_variant_emits_last",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_multiple_emit_calls_produce_multiple_lines",
        "test_output_can_be_parsed_by_json_lines_decoder",
        "test_pipe_to_jq_parses_all_lines",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Output may contain sensitive data (secrets, tokens)",
        "Structured output may leak internal state",
        "Error messages may contain file paths",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Redact sensitive fields before emitting (caller's responsibility)",
        "Sanitize file paths in error messages",
        "Never output raw environment variables",
        "Document security considerations",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Emit must complete in < 10ms per line",
        "No buffering beyond single line",
        "Memory usage must be O(1) per emit call",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use serde_json::to_string instead of to_vec",
        "Reuse line buffer across emit calls (future optimization)",
        "Avoid string cloning in serialization",
        "Profile with criterion for baseline",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log every emit() call at DEBUG level with variant type",
        "Log serialization failures at ERROR level",
        "Log write failures at ERROR level",
    ]
    // Metrics: What to measure
    metrics: [
        "output_lines_emitted_total",
        "output_serialization_errors_total",
        "output_write_errors_total",
        "output_bytes_emitted_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document JsonlWriter struct and lifetime",
        "Document emit() method with usage example",
        "Document emit_jsonl() function",
        "Add module-level documentation explaining JSONL format",
        "Document error handling and return types",
    ]
    // External: External docs needed
    external: [
        "Add JSONL output format to user guide",
        "Document OutputLine schema for AI agents",
        "Add examples of parsing JSONL output",
        "Document how to pipe output to jq",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Serde serialization may fail on complex nested types",
        "Stdout may block if reader is slow",
        "Flush on every line may be slow for high-volume output",
    ]
    // Operational: Operational risks
    operational: [
        "Output format changes break AI parsers",
        "Large outputs may consume excessive memory",
        "Pipe buffer overflow if reader is too slow",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "JsonlWriter struct exists with generic Write parameter",
        "JsonlWriter::new() creates instance from writer",
        "JsonlWriter::emit() writes JSON + newline + flush",
        "emit_jsonl() function writes to stdout",
        "emit() returns Result::Err on serialization failure",
        "emit() returns Result::Err on I/O failure",
        "Unit tests verify newline after each line",
        "Unit tests verify flush after each write",
        "Integration test verifies output can be parsed by jq",
        "No unwrap() or panic() in emit code",
    ]
    // Should: Nice to have
    should: [
        "Performance benchmarks show < 10ms per emit()",
        "Module has comprehensive documentation",
        "Examples show piping to jq and parsing output",
        "Error messages are descriptive and actionable",
    ]
}

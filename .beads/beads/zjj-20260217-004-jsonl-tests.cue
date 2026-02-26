package beads


// Bead ID: isolate-20260217-004-jsonl-tests
// Section: 1 of 16
bead_id: "isolate-20260217-004-jsonl-tests"

// Section 2: Intent
intent: {
    // What: Unit tests for JSONL output types and writer
    what: "Create comprehensive unit tests for all OutputLine variants and JsonlWriter"
    // Why: Ensure JSONL output is valid, parseable, and reliable
    why: "JSONL output is critical for AI agents; bugs break all downstream parsing"
    // Value: Confidence that JSONL output is correct and stable
    value: "Prevents regressions in output format; ensures AI agents can parse results"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create crates/isolate-core/src/output/tests.rs",
        "Test all 16 OutputLine variants serialize correctly",
        "Test all enum types (Severity, Status, ActionStatus, QueueStatus, TrainStatus, TrainAction, StepStatus)",
        "Test JsonlWriter emit() function",
        "Test emit_jsonl() function",
        "Test Context is always emitted last",
        "Test error handling for serialization failures",
        "Test JSON validation (parse output back to OutputLine)",
    ]
    // Out: What we will NOT do
    out: [
        "Integration tests for commands (Bead 033-035)",
        "Performance benchmarks (future work)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-002-jsonl-stack-queue-types"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything - validation bead
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        all_output_line_variants_exist: bool
        jsonl_writer_exists: bool
    }
    // Output: Produced state/outputs
    output: {
        test_coverage_percentage: float  // Target: 100%
        all_variants_tested: bool
        all_enums_tested: bool
        emit_function_tested: bool
        json_validation_tests_pass: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Every OutputLine variant has at least one test",
        "Every enum has serialization test",
        "Every test is deterministic (no randomness)",
        "Tests use assert_eq! for clear failure messages",
        "Tests run in < 5 seconds total",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create crates/isolate-core/src/output/tests.rs",
        "Add #[cfg(test)] mod tests",
        "Write test_summary_serializes()",
        "Write test_session_serializes()",
        "Write test_issue_serializes()",
        "Write test_plan_serializes()",
        "Write test_action_serializes()",
        "Write test_warning_serializes()",
        "Write test_result_serializes()",
        "Write test_error_serializes()",
        "Write test_recovery_serializes()",
        "Write test_context_serializes()",
        "Write test_stack_serializes()",
        "Write test_queue_summary_serializes()",
        "Write test_queue_entry_serializes()",
        "Write test_train_serializes()",
        "Write test_train_step_serializes()",
        "Write test_train_result_serializes()",
        "Write test_all_enums_serialize_to_string()",
        "Write test_jsonl_writer_emit()",
        "Write test_emit_jsonl_function()",
        "Write test_json_roundtrip() for each variant",
        "Write test_type_field_exists() for each variant",
        "Run cargo test --lib output::tests",
        "Verify all tests pass",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // No new types - tests only
    }
    // State: State mutations
    state: {
        // No state mutations - tests only
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        // No public API - tests only
    ]
    // Internal: Internal helpers
    internal: [
        "fn assert_valid_json(json: &str)",
        "fn assert_has_type_field(json: &str)",
        "fn serialize_and_parse<T>(value: &T) -> T where T: Serialize + DeserializeOwned",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Test fails due to invalid JSON",
        "Test fails due to missing 'type' field",
        "Test fails due to wrong enum value",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Fix OutputLine variant definition",
        "Fix serde attributes",
        "Fix enum serialization",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_summary_type_field_is_summary",
        "test_summary_all_fields_serialize",
        "test_session_type_field_is_session",
        "test_session_all_fields_serialize",
        "test_issue_type_field_is_issue",
        "test_issue_severity_serializes",
        "test_plan_type_field_is_plan",
        "test_action_type_field_is_action",
        "test_action_status_serializes",
        "test_warning_type_field_is_warning",
        "test_result_type_field_is_result",
        "test_error_type_field_is_error",
        "test_error_code_and_message_serialize",
        "test_recovery_type_field_is_recovery",
        "test_context_type_field_is_context",
        "test_context_for_human_and_text_serialize",
        "test_stack_type_field_is_stack",
        "test_stack_parent_and_children_serialize",
        "test_queue_summary_type_field_is_queue_summary",
        "test_queue_entry_type_field_is_queue_entry",
        "test_queue_entry_status_serializes",
        "test_train_type_field_is_train",
        "test_train_step_type_field_is_train_step",
        "test_train_result_type_field_is_train_result",
        "test_severity_enum_serializes_to_string",
        "test_status_enum_serializes_to_string",
        "test_action_status_enum_serializes_to_string",
        "test_queue_status_enum_serializes_to_lowercase",
        "test_train_status_enum_serializes_to_lowercase",
        "test_train_action_enum_serializes_to_lowercase",
        "test_step_status_enum_serializes_to_lowercase",
        "test_jsonl_writer_emits_valid_json",
        "test_jsonl_writer_adds_newline",
        "test_jsonl_writer_flushes",
        "test_json_roundtrip_for_all_variants",
    ]
    // Integration: Integration scenarios
    integration: [
        // No integration tests - unit tests only
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Tests may expose sensitive test data",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Use fake data in tests",
        "No real secrets in test fixtures",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "All tests run in < 5 seconds",
        "Each test runs in < 100ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Avoid expensive setup in tests",
        "Use minimal test data",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        // No logging in tests
    ]
    // Metrics: What to measure
    metrics: [
        "test_execution_time",
        "test_pass_rate",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document each test's purpose",
        "Add comments explaining what each test validates",
    ]
    // External: External docs needed
    external: [
        // No external docs for tests
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Tests may be brittle and fail on valid changes",
        "Tests may not catch all bugs",
    ]
    // Operational: Operational risks
    operational: [
        "Slow tests discourage running them",
        "Flaky tests reduce confidence",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "tests.rs file exists in output module",
        "All 16 OutputLine variants have at least one test",
        "All 7 enum types have serialization tests",
        "JsonlWriter emit() has tests",
        "emit_jsonl() function has tests",
        "Every test verifies 'type' field exists",
        "Every test can serialize and parse back (roundtrip)",
        "All tests pass with `cargo test --lib output::tests`",
        "No unwrap() or expect() in tests (use ? or assert macros)",
        "Tests run in < 5 seconds",
    ]
    // Should: Nice to have
    should: [
        "Test coverage is 100% for output module",
        "Tests are well-documented with comments",
        "Tests use helper functions to reduce duplication",
    ]
}

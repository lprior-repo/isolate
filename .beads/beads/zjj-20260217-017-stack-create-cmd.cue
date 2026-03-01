package beads


// Bead ID: isolate-20260217-017-stack-create-cmd
// Section: 1 of 16
bead_id: "isolate-20260217-017-stack-create-cmd"

// Section 2: Intent
intent: {
    // What: Implement `isolate stack <name> --on <parent>` command
    what: "Create command to create stacked sessions with parent relationship"
    // Why: Graphite-style stacking requires parent-child session tracking
    why: "Need to create sessions that depend on parent sessions"
    // Value: Enables stacked development workflow
    value: "AI agents can create and manage stacked sessions"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Create stack command in crates/isolate/src/commands/stack.rs",
        "Add `isolate stack <name> --on <parent>` subcommand",
        "Validate parent exists",
        "Create session with parent_session set",
        "Emit JSONL output",
        "Handle errors gracefully",
    ]
    // Out: What we will NOT do
    out: [
        "Create stack list (Bead 018)",
        "Create stack restack (Bead 019)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-011-db-add-columns"]
    // Blocks: Blocks until this completes
    blocks: ["isolate-20260217-019-stack-restack-cmd"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        parent_session_column_exists: bool
        command_args: [name: string, parent: string]
    }
    // Output: Produced state/outputs
    output: {
        session_created: bool
        parent_session_set: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Parent session must exist",
        "parent_session must be set in database",
        "Session is created in workspace",
        "JSONL output is emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Create crates/isolate/src/commands/stack.rs",
        "Add CLI args: name, --on <parent>",
        "Validate parent session exists",
        "Create session with parent_session set to parent",
        "Initialize workspace",
        "Emit Result line on success",
        "Emit Context line last",
        "Handle errors: emit Error line",
        "Test: isolate stack feature-a --on main",
        "Verify parent_session is set in database",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // Uses existing Session struct with parent_session field
    }
    // State: State mutations
    state: {
        session_created: bool
        parent_session: string
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn stack_create(name: &str, parent: &str) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn validate_parent_exists(parent: &str) -> Result<bool>",
        "fn create_session_with_parent(name: &str, parent: &str) -> Result<Session>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Parent session doesn't exist",
        "Session creation fails",
        "Workspace init fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Emit Error line with descriptive message",
        "Return error exit code",
        "Never panic - always return Result",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_stack_create_creates_session",
        "test_stack_create_sets_parent_session",
        "test_stack_create_fails_if_parent_missing",
        "test_stack_create_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_stack_create_end_to_end",
        "test_parent_relationship_in_database",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Session names may be malicious",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Validate session names",
        "Sanitize input",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Command completes in < 1 second",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "N/A - simple command",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log stack creation",
        "Log parent validation",
    ]
    // Metrics: What to measure
    metrics: [
        "stack_create_total",
        "stack_create_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document stack command",
        "Document --on flag",
    ]
    // External: External docs needed
    external: [
        "Add stack command documentation",
        "Add examples",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Circular dependencies in stacks",
    ]
    // Operational: Operational risks
    operational: [
        "Users may create invalid stacks",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "stack command exists",
        "--on flag exists",
        "Parent validation works",
        "Session created with parent_session set",
        "JSONL output emitted",
        "Error handling works",
        "Unit tests pass",
        "Manual test: isolate stack feature-a --on main",
        "No unwrap() or panic() in command",
    ]
    // Should: Nice to have
    should: [
        "Prevent circular dependencies",
    ]
}

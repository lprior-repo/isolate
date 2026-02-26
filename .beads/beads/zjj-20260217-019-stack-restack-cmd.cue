package beads


// Bead ID: isolate-20260217-019-stack-restack-cmd
// Section: 1 of 16
bead_id: "isolate-20260217-019-stack-restack-cmd"

// Section 2: Intent
intent: {
    // What: Implement `zjj stack restack` command
    what: "Restack session onto new parent, updating children"
    // Why: Need to move sessions in stack hierarchy
    why: "Stacks change; need to reparent sessions"
    // Value: Flexible stack management
    value: "AI agents can reorganize stacks dynamically"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add `zjj stack restack <session> --on <new-parent>`",
        "Validate new parent exists",
        "Update session's parent_session",
        "Rebase session if needed",
        "Update children to track new structure",
        "Emit JSONL output",
    ]
    // Out: What we will NOT do
    out: [
        "Auto-rebase children (future work)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["isolate-20260217-017-stack-create-cmd"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        session: string
        new_parent: string
    }
    // Output: Produced state/outputs
    output: {
        session_reparented: bool
        session_rebased: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "New parent must exist",
        "Session's parent_session is updated",
        "No circular dependencies",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add stack restack subcommand",
        "Args: session, --on <new-parent>",
        "Validate session exists",
        "Validate new_parent exists",
        "Check for circular dependencies",
        "Update session's parent_session in database",
        "Rebase session onto new parent",
        "Emit Result line",
        "Emit Context line",
        "Handle errors",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // Uses existing Session struct
    }
    // State: State mutations
    state: {
        session_updated: bool
        parent_session_changed: string  // old -> new
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn stack_restack(session: &str, new_parent: &str) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn validate_no_circular_deps(session: &str, new_parent: &str) -> Result<()>",
        "fn update_parent_session(session: &str, new_parent: &str) -> Result<()>",
        "fn rebase_session(session: &str, new_parent: &str) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Session doesn't exist",
        "New parent doesn't exist",
        "Circular dependency detected",
        "Rebase fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Emit Error line",
        "Return error",
        "Rollback database changes on failure",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_stack_restack_updates_parent",
        "test_stack_restack_detects_circular_deps",
        "test_stack_restack_rebases_session",
        "test_stack_restack_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_stack_restack_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - updating metadata",
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
        "Command completes in < 5 seconds (includes rebase)",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use indexes",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log restack operation",
        "Log rebase progress",
    ]
    // Metrics: What to measure
    metrics: [
        "stack_restack_total",
        "stack_restack_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document stack restack",
    ]
    // External: External docs needed
    external: [
        "Add documentation",
        "Add examples",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Rebase may fail",
        "Children may need rebasing",
    ]
    // Operational: Operational risks
    operational: [
        "Users may break stacks",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "stack restack subcommand exists",
        "Updates parent_session",
        "Rebases session",
        "Detects circular deps",
        "Emits JSONL output",
        "Unit tests pass",
        "Manual test: zjj stack restack feature-a --on main",
        "No unwrap() or panic() in command",
    ]
    // Should: Nice to have
        "Warns if children need restacking",
    ]
}

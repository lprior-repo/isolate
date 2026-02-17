package beads


// Bead ID: zjj-20260217-018-stack-list-cmd
// Section: 1 of 16
bead_id: "zjj-20260217-018-stack-list-cmd"

// Section 2: Intent
intent: {
    // What: Implement `zjj stack ls` command
    what: "List all stacks with parent-child relationships"
    // Why: AI agents need to discover stack structure
    why: "Need to visualize and query stack hierarchy"
    // Value: Enables stack exploration and management
    value: "AI agents can understand session dependencies"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add `zjj stack ls` subcommand",
        "Query sessions with parent_session",
        "Build tree structure",
        "Emit Stack lines for each stack",
        "Emit Context line last",
        "Handle orphan sessions (no parent)",
    ]
    // Out: What we will NOT do
    out: [
        "Create stack restack (Bead 019)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-011-db-add-columns"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        parent_session_column_exists: bool
    }
    // Output: Produced state/outputs
    output: {
        stack_lines_emitted: bool
        tree_structure_built: bool
        context_emitted_last: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Each stack emits one Stack line",
        "Root sessions have parent: null",
        "Children listed in parent's Stack line",
        "Context is last line",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add stack ls subcommand",
        "Query all sessions from database",
        "Group by parent_session",
        "Build tree structure (roots first, then children)",
        "For each root: emit Stack line with children",
        "Emit Context line last",
        "Test: zjj stack ls",
        "Verify tree structure is correct",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        StackTree: #"""
            struct StackTree {
                name: String,
                parent: Option<String>,
                children: Vec<String>,
                base: String,
            }
            """#
    }
    // State: State mutations
    state: {
        // Read-only command
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn stack_list() -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn build_stack_tree(sessions: Vec<Session>) -> Vec<StackTree>",
        "fn find_root_sessions(sessions: &[Session]) -> Vec<&Session>",
        "fn find_children(sessions: &[Session], parent: &str) -> Vec<String>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Query fails",
        "Tree building fails (cycles)",
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
        "test_stack_list_emits_stack_lines",
        "test_stack_list_builds_tree",
        "test_stack_list_handles_orphans",
        "test_stack_list_emits_context_last",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_stack_list_with_real_stacks",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - read-only",
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
        "Command completes in < 500ms for 100 sessions",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use indexes on parent_session",
        "Efficient tree building",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log stack list",
    ]
    // Metrics: What to measure
    metrics: [
        "stack_list_total",
        "stack_list_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document stack ls output format",
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
        "Cycles may cause infinite loops",
    ]
    // Operational: Operational risks
    operational: [
        "Large stacks may be slow to build",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "stack ls subcommand exists",
        "Emits Stack lines",
        "Builds tree correctly",
        "Handles orphans",
        "Emits Context last",
        "Unit tests pass",
        "Manual test: zjj stack ls",
        "No unwrap() or panic() in command",
    ]
    // Should: Nice to have
    should: [
        "Detects cycles",
    ]
}

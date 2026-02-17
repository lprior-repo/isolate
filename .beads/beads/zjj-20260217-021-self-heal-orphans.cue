package beads


// Bead ID: zjj-20260217-021-self-heal-orphans
// Section: 1 of 16
bead_id: "zjj-20260217-021-self-heal-orphans"

// Section 2: Intent
intent: {
    // What: Auto-cleanup orphaned workspaces
    what: "Detect and clean up workspaces with no database session"
    // Why: Crashes may leave workspace directories without database entries
    why: "Orphaned workspaces consume disk space and cause confusion"
    // Value: Self-healing system maintains clean state
    value: "AI agents don't need to manually clean up orphans"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Scan workspace directory",
        "Detect workspaces not in database",
        "Add --cleanup-orphans flag to clean command",
        "Delete orphan workspace directories",
        "Emit JSONL output",
        "Dry-run mode for safety",
    ]
    // Out: What we will NOT do
    out: [
        "Enforce thresholds (Bead 022)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: ["zjj-20260217-013-db-update-mergequeue"]
    // Blocks: Blocks until this completes
    blocks: []  // Doesn't block anything
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        workspace_directory: string
        database_sessions: []
    }
    // Output: Produced state/outputs
    output: {
        orphans_detected: int
        orphans_cleaned: int
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Orphans are workspaces not in database",
        "Cleanup only happens with explicit flag",
        "Dry-run mode doesn't delete",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add --cleanup-orphans flag to clean",
        "Add --dry-run flag",
        "List all workspace directories",
        "Query all sessions from database",
        "Find workspaces not in database",
        "For each orphan: emit Warning line",
        "If not dry-run: delete workspace",
        "Emit Result line",
        "Emit Context line",
        "Test with orphan workspace",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        OrphanWorkspace: #"""
            struct OrphanWorkspace {
                name: String,
                path: PathBuf,
            }
            """#
    }
    // State: State mutations
    state: {
        orphans_deleted: int
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn cleanup_orphans(dry_run: bool) -> Result<usize>",
        "pub fn detect_orphans() -> Result<Vec<OrphanWorkspace>>",
    ]
    // Internal: Internal helpers
    internal: [
        "fn list_workspace_dirs() -> Result<Vec<PathBuf>>",
        "fn is_orphan(workspace: &str, sessions: &[Session]) -> bool",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Directory scan fails",
        "Delete fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error",
        "Emit Error line",
        "Continue with other orphans",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_detect_orphans",
        "test_cleanup_orphans",
        "test_dry_run_doesnt_delete",
        "test_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_cleanup_orphans_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "May delete valid workspaces if bug",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Dry-run mode default",
        "Explicit flag required",
        "Warning emitted before delete",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Cleanup completes in < 5 seconds",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Parallel deletion",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log detected orphans",
        "Log deletions",
    ]
    // Metrics: What to measure
    metrics: [
        "orphans_detected_total",
        "orphans_cleaned_total",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document orphan detection",
        "Document --cleanup-orphans flag",
    ]
    // External: External docs needed
    external: [
        "Add documentation",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "May delete valid workspaces",
    ]
    // Operational: Operational risks
    operational: [
        "Users may lose work if bug",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "Orphan detection works",
        "Cleanup works",
        "Dry-run mode works",
        "--cleanup-orphans flag exists",
        "Emits JSONL output",
        "Unit tests pass",
        "Manual test with orphan",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Warnings emitted",
    ]
}

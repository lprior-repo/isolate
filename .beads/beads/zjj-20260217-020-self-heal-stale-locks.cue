package beads


// Bead ID: zjj-20260217-020-self-heal-stale-locks
// Section: 1 of 16
bead_id: "zjj-20260217-020-self-heal-stale-locks"

// Section 2: Intent
intent: {
    // What: Auto-reclaim stale locks on TTL expiry
    what: "Implement self-healing for stale processing locks"
    // Why: Locks may be held by crashed processes
    why: "Need automatic recovery from abandoned locks"
    // Value: System self-heals without manual intervention
    value: "AI agents don't need to manually clean up locks"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add TTL check to processing lock",
        "Auto-reclaim locks older than TTL",
        "Add heartbeat mechanism",
        "Add --heal-locks flag to doctor command",
        "Emit JSONL output for healing operations",
    ]
    // Out: What we will NOT do
    out: [
        "Auto-cleanup orphans (Bead 021)",
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
        processing_lock_table_exists: bool
        ttl_seconds: int
    }
    // Output: Produced state/outputs
    output: {
        stale_locks_reclaimed: bool
        heartbeat_updated: bool
        jsonl_output_emitted: bool
    }
    // Invariants: Must remain true
    invariants: [
        "Locks older than TTL are reclaimed",
        "Active locks have heartbeat < TTL",
        "Healing is logged",
        "JSONL output emitted",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Add heartbeat_at column to processing_lock",
        "Query locks where heartbeat_at < now - TTL",
        "Delete stale locks",
        "Add heartbeat update to lock acquisition",
        "Add --heal-locks to doctor command",
        "Emit Action lines for each reclaimed lock",
        "Emit Result line",
        "Emit Context line",
        "Test with stale lock",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        // Uses existing processing_lock table
    }
    // State: State mutations
    state: {
        locks_reclaimed: int
        ttl_seconds: int
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub fn heal_stale_locks(ttl_seconds: i64) -> Result<usize>",
        "pub fn update_lock_heartbeat(lock_id: i64) -> Result<()>",
    ]
    // Internal: Internal helpers
    internal: [
        "async fn delete_stale_locks(db: &SqlitePool, ttl: i64) -> Result<usize>",
        "async fn is_lock_stale(heartbeat: i64, ttl: i64) -> bool",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Query fails",
        "Delete fails",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return error",
        "Emit Error line",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_stale_locks_reclaimed",
        "test_active_locks_not_reclaimed",
        "test_heartbeat_updated",
        "test_heal_emits_jsonl",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_heal_stale_locks_end_to_end",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "No security concerns - cleanup only",
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
        "Healing completes in < 1 second",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Use index on heartbeat_at",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log reclaimed locks",
    ]
    // Metrics: What to measure
    metrics: [
        "locks_reclaimed_total",
        "heal_stale_locks_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document TTL mechanism",
        "Document heartbeat",
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
        "May reclaim active locks if clock skewed",
    ]
    // Operational: Operational risks
    operational: [
        "TTL too short may cause issues",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "heartbeat_at column exists",
        "Stale lock reclaim works",
        "Heartbeat update works",
        "doctor --heal-locks exists",
        "Emits JSONL output",
        "Unit tests pass",
        "Manual test with stale lock",
        "No unwrap() or panic()",
    ]
    // Should: Nice to have
    should: [
        "Configurable TTL",
    ]
}

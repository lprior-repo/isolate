package beads


// Bead ID: zjj-20260217-016-config-conflict-resolution
// Section: 1 of 16
bead_id: "zjj-20260217-016-config-conflict-resolution"

// Section 2: Intent
intent: {
    // What: Add ConflictResolutionConfig to config system
    what: "Add conflict resolution configuration (mode, autonomy, keywords, logging)"
    // Why: Need configurable conflict resolution behavior
    why: "Different environments have different conflict resolution needs"
    // Value: Flexible conflict resolution policy
    value: "Enables customizing how conflicts are resolved"
}

// Section 3: Scope
scope: {
    // In: What we WILL do
    in: [
        "Add ConflictResolutionConfig struct to config.rs",
        "Add mode field (auto, manual, hybrid)",
        "Add autonomy field (0-100)",
        "Add security_keywords field (list of strings)",
        "Add log_resolutions field (bool)",
        "Add to main Config struct",
        "Add validation logic",
    ]
    // Out: What we will NOT do
    out: [
        "Create conflict resolution commands (Bead 027-029)",
    ]
}

// Section 4: Dependencies
dependencies: {
    // Requires: Must complete before this bead
    requires: []  // No dependencies - foundation bead
    // Blocks: Blocks until this completes
    blocks: ["zjj-20260217-027-conflict-analyze-cmd", "zjj-20260217-028-conflict-resolve-cmd", "zjj-20260217-029-conflict-quality-signals"]
}

// Section 5: Contract
contract: {
    // Input: Required state/inputs
    input: {
        config_file_exists: bool
    }
    // Output: Produced state/outputs
    output: {
        conflict_resolution_config_exists: bool
        mode_field_exists: bool
        autonomy_field_exists: bool
        security_keywords_field_exists: bool
        log_resolutions_field_exists: bool
        validation_exists: bool
    }
    // Invariants: Must remain true
    invariants: [
        "mode must be valid (auto, manual, hybrid)",
        "autonomy must be 0-100",
        "security_keywords must not be empty",
        "log_resolutions must be bool",
    ]
}

// Section 6: Algorithm
algorithm: {
    // Steps: Ordered implementation steps
    steps: [
        "Read crates/zjj-core/src/config.rs",
        "Create ConflictResolutionConfig struct",
        "Add mode: ConflictMode field",
        "Add autonomy: u8 field (0-100)",
        "Add security_keywords: Vec<String> field",
        "Add log_resolutions: bool field",
        "Add validation function",
        "Add to main Config struct",
        "Add default values",
        "Add serde serialization",
        "Write unit tests",
        "Test validation",
    ]
}

// Section 7: Data Model
data_model: {
    // Types: Type definitions
    types: {
        ConflictResolutionConfig: #"""
            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct ConflictResolutionConfig {
                pub mode: ConflictMode,
                pub autonomy: u8,  // 0-100
                pub security_keywords: Vec<String>,
                pub log_resolutions: bool,
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(rename_all = "lowercase")]
            pub enum ConflictMode {
                Auto,
                Manual,
                Hybrid,
            }
            """#
    }
    // State: State mutations
    state: {
        // Config is read-only after load
    }
}

// Section 8: API
api: {
    // Public: Public interfaces
    public: [
        "pub struct ConflictResolutionConfig { ... }",
        "pub enum ConflictMode { Auto, Manual, Hybrid }",
        "impl ConflictResolutionConfig { pub fn validate(&self) -> Result<()> }",
    ]
    // Internal: Internal helpers
    internal: [
        "fn validate_autonomy(autonomy: u8) -> Result<()>",
        "fn validate_security_keywords(keywords: &[String]) -> Result<()>",
    ]
}

// Section 9: Error Handling
error_handling: {
    // Cases: Error scenarios
    cases: [
        "Invalid mode",
        "Autonomy out of range",
        "Empty security_keywords",
    ]
    // Recovery: Recovery strategies
    recovery: [
        "Return validation error",
        "Use default values",
    ]
}

// Section 10: Testing
testing: {
    // Unit: Unit test cases
    unit: [
        "test_conflict_resolution_config_validation",
        "test_autonomy_in_range",
        "test_security_keywords_not_empty",
        "test_mode_is_valid",
        "test_default_values",
    ]
    // Integration: Integration scenarios
    integration: [
        "test_config_load_from_file",
        "test_config_validation_on_load",
    ]
}

// Section 11: Security
security: {
    // Concerns: Security considerations
    concerns: [
        "Config may expose security policies",
    ]
    // Mitigations: Mitigation strategies
    mitigations: [
        "Config is local file",
        "No secrets in config",
    ]
}

// Section 12: Performance
performance: {
    // Constraints: Performance requirements
    constraints: [
        "Config load < 100ms",
    ]
    // Optimizations: Optimization strategies
    optimizations: [
        "Lazy loading",
    ]
}

// Section 13: Observability
observability: {
    // Logging: What to log
    logging: [
        "Log config load",
        "Log validation failures",
    ]
    // Metrics: What to measure
    metrics: [
        "config_load_duration_seconds",
    ]
}

// Section 14: Documentation
documentation: {
    // Inline: Inline docs needed
    inline: [
        "Document ConflictResolutionConfig fields",
        "Document validation rules",
    ]
    // External: External docs needed
    external: [
        "Add config documentation",
        "Add example config file",
    ]
}

// Section 15: Risks
risks: {
    // Technical: Technical risks
    technical: [
        "Invalid config may break conflict resolution",
    ]
    // Operational: Operational risks
    operational: [
        "Users may misconfigure autonomy",
    ]
}

// Section 16: Acceptance Criteria
acceptance_criteria: {
    // Must: Required for completion
    must: [
        "ConflictResolutionConfig struct exists",
        "All fields exist",
        "Validation function exists",
        "Added to main Config",
        "Unit tests pass",
        "No unwrap() or panic() in validation",
    ]
    // Should: Nice to have
    should: [
        "Default values are sensible",
        "Error messages are helpful",
    ]
}

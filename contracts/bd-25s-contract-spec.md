# Contract Specification: Conflict Resolution Configuration (bd-25s)

**Bead ID:** bd-25s
**Title:** Add conflict resolution configuration (mode, autonomy, keywords, logging)
**Version:** 1.0.0
**Status:** Draft

---

## 1. Overview

This document specifies the Design by Contract requirements for the conflict resolution configuration system in zjj. The configuration enables flexible, environment-specific conflict resolution behavior with security safeguards.

### 1.1 Scope

The contract covers:
- Configuration structure for conflict resolution behavior
- Mode selection (auto, manual, hybrid)
- Autonomy level (0-100) for AI-controlled resolution
- Security keyword detection for sensitive conflicts
- Resolution logging and audit trail
- Validation and enforcement of configuration constraints
- Integration with existing config hierarchy (defaults → global → project → env vars)

### 1.2 Security Context

This system has security implications because:
1. **Autonomous Resolution:** High autonomy levels allow AI to resolve conflicts without human review
2. **Security Bypass:** Incorrect keyword configuration could expose sensitive data
3. **Audit Trail:** Missing resolution logging makes debugging impossible
4. **Configuration Validity:** Invalid modes or autonomy levels can break conflict resolution
5. **Environment Override:** Environment variables must respect validation rules

---

## 2. Type Definitions

### 2.1 ConflictResolutionConfig

```rust
/// Configuration for conflict resolution behavior
///
/// This configuration controls how conflicts are detected, analyzed,
/// and resolved across different environments (development, CI, production).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictResolutionConfig {
    /// Resolution mode: auto, manual, or hybrid
    pub mode: ConflictMode,

    /// Autonomy level for AI-controlled resolution (0-100)
    ///
    /// - 0: Fully manual - all conflicts require human approval
    /// - 1-49: Conservative AI - suggest resolutions, require approval
    /// - 50-89: Balanced AI - auto-resolve safe conflicts, prompt for risky ones
    /// - 90-99: Aggressive AI - auto-resolve most conflicts, prompt only for security
    /// - 100: Fully autonomous - AI resolves all conflicts without prompting
    pub autonomy: u8,

    /// Security keywords that trigger human review
    ///
    /// Conflicts in files containing these keywords require human approval
    /// regardless of autonomy level. Examples: "password", "token", "secret",
    /// "api_key", "private_key", "credential", etc.
    pub security_keywords: Vec<String>,

    /// Whether to log all conflict resolutions to audit trail
    ///
    /// When true, all resolutions (auto and manual) are logged to the
    /// conflict_resolutions table with timestamp, agent, file, strategy, and reason.
    pub log_resolutions: bool,
}

impl ConflictResolutionConfig {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns `Error::ValidationError` if:
    /// - autonomy is not in range 0-100
    /// - security_keywords is empty
    /// - mode is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate autonomy range
        if self.autonomy > 100 {
            return Err(Error::ValidationError {
                message: format!(
                    "autonomy must be 0-100, got {}",
                    self.autonomy
                ),
                field: Some("autonomy".to_string()),
                value: Some(self.autonomy.to_string()),
                constraints: vec!["0 <= autonomy <= 100".to_string()],
            });
        }

        // Validate security_keywords is non-empty
        if self.security_keywords.is_empty() {
            return Err(Error::ValidationError {
                message: "security_keywords must not be empty".to_string(),
                field: Some("security_keywords".to_string()),
                value: None,
                constraints: vec!["security_keywords.len() > 0".to_string()],
            });
        }

        // Validate mode (already enforced by enum, but check for completeness)
        match self.mode {
            ConflictMode::Auto | ConflictMode::Manual | ConflictMode::Hybrid => Ok(()),
        }
    }

    /// Check if a file requires human review based on security keywords
    ///
    /// Returns true if the file path contains any security keyword.
    pub fn requires_human_review(&self, file_path: &str) -> bool {
        let file_path_lower = file_path.to_lowercase();
        self.security_keywords.iter().any(|keyword| {
            file_path_lower.contains(&keyword.to_lowercase())
        })
    }

    /// Check if resolution can proceed automatically based on autonomy
    ///
    /// Returns true if:
    /// - mode is Auto, OR
    /// - mode is Hybrid AND autonomy >= threshold AND file doesn't contain security keywords
    pub fn can_auto_resolve(&self, file_path: Option<&str>) -> bool {
        match self.mode {
            ConflictMode::Auto => true,
            ConflictMode::Manual => false,
            ConflictMode::Hybrid => {
                // In hybrid mode, check autonomy and security keywords
                if let Some(path) = file_path {
                    !self.requires_human_review(path) && self.autonomy >= 50
                } else {
                    self.autonomy >= 50
                }
            }
        }
    }
}
```

### 2.2 ConflictMode

```rust
/// Conflict resolution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictMode {
    /// Fully automatic resolution
    ///
    /// All conflicts are resolved by AI without human intervention.
    /// Use with caution - recommended only for CI environments with tests.
    Auto,

    /// Fully manual resolution
    ///
    /// All conflicts require human intervention. AI may suggest resolutions,
    /// but humans must approve them.
    Manual,

    /// Hybrid mode
    ///
    /// AI auto-resolves safe conflicts based on autonomy level and security keywords.
    /// Risky conflicts (those matching security keywords) require human review.
    Hybrid,
}

impl Default for ConflictMode {
    fn default() -> Self {
        Self::Manual // Default to safest mode
    }
}

impl std::fmt::Display for ConflictMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Manual => write!(f, "manual"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl FromStr for ConflictMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(Error::InvalidConfig(format!(
                "Invalid conflict mode: {s}. Must be one of: auto, manual, hybrid"
            ))),
        }
    }
}
```

### 2.3 PartialConflictResolutionConfig

```rust
/// Partial configuration with Option<T> fields for explicit-key merge semantics
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartialConflictResolutionConfig {
    #[serde(default)]
    pub mode: Option<ConflictMode>,
    #[serde(default)]
    pub autonomy: Option<u8>,
    #[serde(default)]
    pub security_keywords: Option<Vec<String>>,
    #[serde(default)]
    pub log_resolutions: Option<bool>,
}
```

### 2.4 Integration with Main Config

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    // ... existing fields ...
    pub conflict_resolution: ConflictResolutionConfig,
}

impl ConflictResolutionConfig {
    /// Merge partial config, only updating fields that are Some(value)
    fn merge_partial(&mut self, partial: PartialConflictResolutionConfig) {
        if let Some(mode) = partial.mode {
            self.mode = mode;
        }
        if let Some(autonomy) = partial.autonomy {
            self.autonomy = autonomy;
        }
        if let Some(security_keywords) = partial.security_keywords {
            self.security_keywords = security_keywords;
        }
        if let Some(log_resolutions) = partial.log_resolutions {
            self.log_resolutions = log_resolutions;
        }
    }
}
```

---

## 3. Preconditions

### 3.1 Configuration Validity Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-CONF-001 | `autonomy` must be in range 0-100 | Validation in `validate()` |
| PRE-CONF-002 | `security_keywords` must not be empty | Validation in `validate()` |
| PRE-CONF-003 | `mode` must be valid (Auto/Manual/Hybrid) | Enum type ensures this |
| PRE-CONF-004 | `log_resolutions` must be boolean | Type system ensures this |

### 3.2 Runtime Preconditions

| Precondition ID | Description | Enforcement |
|-----------------|-------------|-------------|
| PRE-RUN-001 | Config must be loaded before conflict resolution | Load function ensures this |
| PRE-RUN-002 | Validation must pass before use | `validate()` called on load |
| PRE-RUN-003 | Environment variables must respect validation | Parse and validate on apply |

---

## 4. Postconditions

### 4.1 Configuration Load Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-LOAD-001 | Loaded config passes all validations | `validate()` returns Ok |
| POST-LOAD-002 | Default values are sensible | Defaults are Manual mode, autonomy 0, log_resolutions true |
| POST-LOAD-003 | Config hierarchy is respected | Project overrides global overrides defaults |
| POST-LOAD-004 | Unknown keys are rejected | TOML validation fails early |

### 4.2 Resolution Decision Postconditions

| Postcondition ID | Description | Verification |
|------------------|-------------|--------------|
| POST-RES-001 | `can_auto_resolve()` respects mode and autonomy | Logic verification |
| POST-RES-002 | `requires_human_review()` checks all keywords | Keyword matching verified |
| POST-RES-003 | Security keywords always trigger review | Overrides autonomy check |

---

## 5. Invariants

### 5.1 Data Integrity Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-DATA-001 | `0 <= autonomy <= 100` always holds |
| INV-DATA-002 | `security_keywords.len() > 0` always holds |
| INV-DATA-003 | `mode` is always one of: Auto, Manual, Hybrid |
| INV-DATA-004 | `log_resolutions` is always boolean |

### 5.2 Security Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-SEC-001 | Files matching security_keywords require human review regardless of autonomy |
| INV-SEC-002 | Manual mode never auto-resolves conflicts |
| INV-SEC-003 | Auto mode always allows auto-resolution (checked by caller) |
| INV-SEC-004 | Hybrid mode respects both autonomy and security keywords |

### 5.3 Configuration Hierarchy Invariants

| Invariant ID | Description |
|--------------|-------------|
| INV-CONF-001 | Higher-priority configs override lower-priority configs |
| INV-CONF-002 | Missing fields in partial config don't reset to defaults |
| INV-CONF-003 | Environment variables have highest priority |

---

## 6. Error Recovery

### 6.1 Recoverable Errors

| Error | Recovery Strategy |
|-------|-------------------|
| `ValidationError::AutonomyOutOfRange` | Use default autonomy (0) and warn user |
| `ValidationError::EmptyKeywords` | Use default keyword list and warn user |
| `ParseError::InvalidMode` | Use default mode (Manual) and warn user |

### 6.2 Non-Recoverable Errors

| Error | Action |
|-------|--------|
| `ValidationError::MultipleFailures` | Abort with list of all validation errors |
| `ParseError::MalformedTOML` | Abort with line number and error context |

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Unintended Auto-Resolution:** High autonomy in production causes data loss | Default to Manual mode; require explicit configuration for Auto/Hybrid |
| **Security Keyword Bypass:** Misconfigured keywords allow auto-resolution of sensitive files | Validation ensures keywords list is non-empty; default includes common sensitive patterns |
| **Audit Trail Loss:** log_resolutions=false prevents debugging | Default to true; document consequences of disabling |
| **Config Override:** Environment variables bypass validation | Parse and validate env vars just like file-based config |

### 7.2 Security Requirements

1. **SR-CONF-001:** Default configuration must be safest (Manual mode, autonomy 0)
2. **SR-CONF-002:** Security keywords must include at least: password, token, secret, key, credential
3. **SR-CONF-003:** Validation must fail fast on invalid config (no silent defaults)
4. **SR-CONF-004:** Environment variables must pass same validation as file config
5. **SR-CONF-005:** Auto mode should emit warning on first use

---

## 8. Configuration Examples

### 8.1 Minimal Config (Development)

```toml
[conflict_resolution]
mode = "manual"
autonomy = 0
security_keywords = ["password", "token", "secret", "key", "credential"]
log_resolutions = true
```

### 8.2 Balanced Config (CI with Human Review)

```toml
[conflict_resolution]
mode = "hybrid"
autonomy = 60
security_keywords = [
    "password",
    "token",
    "secret",
    "api_key",
    "private_key",
    "credential",
    "auth",
    "session",
]
log_resolutions = true
```

### 8.3 Aggressive Config (Trusted CI with Tests)

```toml
[conflict_resolution]
mode = "auto"
autonomy = 90
security_keywords = [
    "password",
    "token",
    "secret",
    "api_key",
    "private_key",
    "credential",
]
log_resolutions = true
```

### 8.4 Environment Variables

```bash
# Override mode for CI
export ZJJ_CONFLICT_RESOLUTION_MODE=auto

# Override autonomy for testing
export ZJJ_CONFLICT_RESOLUTION_AUTONOMY=80

# Disable logging in ephemeral environments
export ZJJ_CONFLICT_RESOLUTION_LOG_RESOLUTIONS=false
```

---

## 9. Default Values

```rust
impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            mode: ConflictMode::Manual,  // Safest default
            autonomy: 0,                  // Fully manual by default
            security_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "credential".to_string(),
            ],
            log_resolutions: true,        // Audit trail is important
        }
    }
}
```

---

## 10. Environment Variable Mapping

| Environment Variable | Field | Type | Validation |
|---------------------|-------|------|------------|
| `ZJJ_CONFLICT_RESOLUTION_MODE` | mode | ConflictMode | Must be "auto", "manual", or "hybrid" |
| `ZJJ_CONFLICT_RESOLUTION_AUTONOMY` | autonomy | u8 | Must be 0-100 |
| `ZJJ_CONFLICT_RESOLUTION_LOG_RESOLUTIONS` | log_resolutions | bool | Must be boolean |
| `ZJJ_CONFLICT_RESOLUTION_SECURITY_KEYWORDS` | security_keywords | Vec<String> | Must be comma-separated, non-empty |

---

## 11. Test Coverage Requirements

### 11.1 Contract Verification Tests

Every precondition, postcondition, and invariant must have at least one test:

- All PRE-CONF-* conditions must have validation tests
- All POST-LOAD-* conditions must have config loading tests
- All INV-* invariants must have property-based tests

### 11.2 Error Path Coverage

All validation errors must be:

1. Constructible in tests
2. Displayable with meaningful message
3. Serializable to JSON
4. Recoverable where applicable

### 11.3 Integration Tests

- Config hierarchy: defaults → global → project → env vars
- Partial config merge preserves unspecified fields
- Validation runs on all config sources
- Runtime decisions respect loaded config

---

## 12. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-02-18 | Initial contract specification |

---

## 13. References

- `/home/lewis/src/zjj/crates/zjj-core/src/config.rs` - Configuration system
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-016-config-conflict-resolution.cue` - Original bead spec
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-028-conflict-resolve-cmd.cue` - Conflict resolution command
- `/home/lewis/src/zjj/.beads/beads/zjj-20260217-014-db-conflict-resolutions-table.cue` - Audit trail table

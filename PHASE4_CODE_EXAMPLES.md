# Phase 4 Implementation: Key Code Changes

## 1. ActionVerb - From Unbounded to Validated Enum

### Before
```rust
/// A validated action verb
///
/// Action verbs are less constrained - they can be any string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionVerb(String);

impl ActionVerb {
    /// Create a new action verb (no validation required)
    #[must_use]
    pub fn new(verb: impl Into<String>) -> Self {
        Self(verb.into())
    }

    /// Get the action verb as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### After
```rust
/// Known action verbs in the system
///
/// These are predefined action verbs that represent operations.
/// Custom verbs can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionVerb {
    Run, Execute, Create, Delete, Update, Merge, Rebase,
    Sync, Fix, Check, Process, Focus, Attach, SwitchTab,
    Remove, Discover, WouldFix,
    Custom(String),
}

impl ActionVerb {
    /// Create an action verb from known verbs or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidActionVerb` if custom verb doesn't
    /// follow the pattern: lowercase alphanumeric with hyphens
    pub fn new(verb: impl Into<String>) -> Result<Self, OutputLineError> {
        let verb = verb.into();

        // Match against known verbs (case-insensitive)
        match verb.to_lowercase().as_str() {
            "run" => Ok(Self::Run),
            "create" => Ok(Self::Create),
            // ... other known verbs
            custom => {
                // Validate custom verb format
                if custom.trim().is_empty() {
                    return Err(OutputLineError::InvalidActionVerb(
                        "action verb cannot be empty".to_string(),
                    ));
                }

                // Must be lowercase alphanumeric with hyphens
                let lower = custom.to_lowercase();
                if lower != custom {
                    return Err(OutputLineError::InvalidActionVerb(
                        format!("action verb must be lowercase, got: {custom}"),
                    ));
                }

                if !lower.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                    return Err(OutputLineError::InvalidActionVerb(
                        format!("action verb must be lowercase alphanumeric with hyphens, got: {custom}"),
                    ));
                }

                // Must start with a letter
                if !lower.chars().next().map_or(false, |c| c.is_ascii_lowercase()) {
                    return Err(OutputLineError::InvalidActionVerb(
                        format!("action verb must start with a lowercase letter, got: {custom}"),
                    ));
                }

                Ok(Self::Custom(lower))
            }
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Run => "run",
            Self::Create => "create",
            // ... other known verbs
            Self::Custom(s) => s.as_str(),
        }
    }

    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}
```

## 2. ActionTarget - Adding Length Validation

### Before
```rust
/// A validated action target
///
/// Action targets are less constrained - they can be any string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTarget(String);

impl ActionTarget {
    /// Create a new action target (no validation required)
    #[must_use]
    pub fn new(target: impl Into<String>) -> Self {
        Self(target.into())
    }

    /// Get the action target as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### After
```rust
/// A validated action target
///
/// # Invariants
/// - Must be non-empty after trimming
/// - Maximum length of 1000 characters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTarget(String);

impl ActionTarget {
    pub const MAX_LENGTH: usize = 1000;

    /// Create a new action target, validating format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::EmptyMessage` if target is empty
    /// Returns `OutputLineError::InvalidActionTarget` if target exceeds max length
    pub fn new(target: impl Into<String>) -> Result<Self, OutputLineError> {
        let target = target.into();

        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(OutputLineError::InvalidActionTarget(format!(
                "action target exceeds maximum length of {} characters",
                Self::MAX_LENGTH
            )));
        }

        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

## 3. WarningCode - Converting to Enum

### Before
```rust
/// A validated warning code
///
/// Warning codes are less constrained - they can be empty
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WarningCode(String);

impl WarningCode {
    /// Create a new warning code (no validation required)
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// Get the warning code as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### After
```rust
/// Known warning codes in the system
///
/// These are predefined warning codes that have specific meanings.
/// Custom codes can be added via the `Custom` variant for extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    ConfigNotFound,
    ConfigInvalid,
    SessionLimitReached,
    WorkspaceNotFound,
    GitOperationFailed,
    MergeConflict,
    QueueEntryBlocked,
    AgentUnavailable,
    Custom(String),
}

impl WarningCode {
    /// Create a warning code from known codes or validate custom format
    ///
    /// # Errors
    ///
    /// Returns `OutputLineError::InvalidWarningCode` if custom code doesn't
    /// follow the pattern: letter followed by alphanumeric (e.g., "W001", "E123")
    pub fn new(code: impl Into<String>) -> Result<Self, OutputLineError> {
        let code = code.into();

        // Match against known codes
        match code.as_str() {
            "CONFIG_NOT_FOUND" => Ok(Self::ConfigNotFound),
            "MERGE_CONFLICT" => Ok(Self::MergeConflict),
            // ... other known codes
            custom => {
                // Validate custom code format
                if custom.is_empty() {
                    return Err(OutputLineError::InvalidWarningCode(
                        "warning code cannot be empty".to_string(),
                    ));
                }

                // Must start with a letter
                if !custom.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
                    return Err(OutputLineError::InvalidWarningCode(
                        format!("warning code must start with a letter, got: {custom}"),
                    ));
                }

                // All characters must be alphanumeric or underscore
                if !custom.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(OutputLineError::InvalidWarningCode(
                        format!("warning code must be alphanumeric or underscore, got: {custom}"),
                    ));
                }

                Ok(Self::Custom(custom))
            }
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::ConfigNotFound => "CONFIG_NOT_FOUND",
            Self::MergeConflict => "MERGE_CONFLICT",
            Self::Custom(s) => s.as_str(),
        }
    }

    #[must_use]
    pub const fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}
```

## 4. Error Handling

### Added to OutputLineError
```rust
#[derive(Debug, Clone, Error)]
pub enum OutputLineError {
    // ... existing variants

    #[error("invalid action verb: {0}")]
    InvalidActionVerb(String),

    #[error("invalid action target: {0}")]
    InvalidActionTarget(String),
}
```

## 5. Usage Pattern Changes

### Before (Unbounded)
```rust
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb),      // Returns Self directly
        ActionTarget::new(target),  // Returns Self directly
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}
```

### After (Validated)
```rust
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}
```

## 6. Test Examples

### Validation Tests
```rust
#[test]
fn test_action_validation_valid_verb() {
    let verb = ActionVerb::new("create");
    assert!(verb.is_ok());
    let verb = verb.expect("valid");
    assert_eq!(verb.as_str(), "create");
    assert!(!verb.is_custom());
}

#[test]
fn test_action_validation_custom_verb() {
    let verb = ActionVerb::new("custom-verb");
    assert!(verb.is_ok());
    let verb = verb.expect("valid");
    assert!(verb.is_custom());
    assert_eq!(verb.as_str(), "custom-verb");
}

#[test]
fn test_action_validation_invalid_verb_empty() {
    let verb = ActionVerb::new("");
    assert!(verb.is_err());
}

#[test]
fn test_action_validation_invalid_verb_uppercase() {
    let verb = ActionVerb::new("Create");
    assert!(verb.is_err());
}

#[test]
fn test_action_target_validation_too_long() {
    let long_target = "a".repeat(1001);
    let target = ActionTarget::new(long_target);
    assert!(target.is_err());
}

#[test]
fn test_warning_code_validation_known() {
    let code = WarningCode::new("CONFIG_NOT_FOUND");
    assert!(code.is_ok());
    let code = code.expect("valid");
    assert_eq!(code.as_str(), "CONFIG_NOT_FOUND");
    assert!(!code.is_custom());
}
```

## Key Design Decisions

1. **Enum vs Struct**: Chose enum for ActionVerb and WarningCode to make known values explicit
2. **Custom Variant**: Included `Custom(String)` variant for extensibility
3. **Validation at Construction**: All validation happens in `new()`, ensuring invalid states are unrepresentable
4. **Result Type**: Changed constructors to return `Result<Self, OutputLineError>` for proper error handling
5. **Trait Implems**: Maintained `Display`, `AsRef<str>`, and all serde derives for backward compatibility

# DDD Refactoring Analysis for zjj-core

## Overview
This document analyzes the current codebase against Scott Wlaschin's Domain-Driven Design principles and identifies systematic refactoring opportunities.

## Current State Assessment

### 1. Primitive Obsession (String/str used as identifiers)

**Found in `/crates/zjj-core/src/output/types.rs`:**
- `pub name: String` (line 134, 192, 267, 343, 376, 424, 489, 561, 570, 704, 705, 780, 1092)
- `pub id: String` (line 192, 703, 779)
- `pub code: String` (line 375)
- `pub verb: String` (line 343)
- `pub target: String` (line 344)
- `pub base_ref: String` (line 562)

**Issue:** These are primitives that should be semantic newtypes.

**Examples of problematic usage:**
```rust
// Line 192 - Issue struct
pub struct Issue {
    pub id: String,           // Should be IssueId newtype
    pub title: String,        // Should be IssueTitle newtype
    // ...
}

// Line 267 - Plan struct
pub struct Plan {
    pub title: String,        // Should be PlanTitle newtype
    pub description: String,  // Should be PlanDescription newtype
    // ...
}

// Line 704 - QueueEntry struct
pub struct QueueEntry {
    pub id: String,           // Should be QueueEntryId newtype
    pub session: String,      // Should be SessionName (already exists!)
    // ...
}
```

### 2. Boolean Flags for State Decisions

**Found in `/crates/zjj-core/src/output/types.rs`:**
- `pub recoverable: bool` (line 498)
- `pub automatic: bool` (line 516)
- `pub merge_safe: bool` (line 1095)
- `pub success: bool` (line 425)

**Issue:** Boolean flags encode state that should be explicit enums.

**Example:**
```rust
// Line 496 - Assessment struct
pub struct Assessment {
    pub severity: ErrorSeverity,
    pub recoverable: bool,        // Should be RecoveryStrategy enum
    pub recommended_action: String,
}

// Better approach:
pub enum RecoveryStrategy {
    Recoverable { action: RecommendedAction },
    NotRecoverable { reason: String },
}
```

### 3. Option Fields Encoding State Machines

**Found in `/crates/zjj-core/src/output/types.rs`:**
- `pub details: Option<String>` (line 92)
- `pub session: Option<String>` (line 197, 348, 708, 710)
- `pub suggestion: Option<String>` (line 199)
- `pub result: Option<String>` (line 347)
- `pub context: Option<Context>` (line 378)
- `pub command: Option<String>` (line 515, 941)
- `pub bead: Option<String>` (line 574, 708)

**Issue:** Option fields often represent state that should be explicit.

**Example:**
```rust
// Line 197 - Issue struct
pub struct Issue {
    // ...
    pub session: Option<String>,  // Should be: WithSession | Standalone
}

// Better approach:
pub enum IssueScope {
    Standalone,
    InSession { session: SessionName },
}
```

### 4. Repeated Validation Logic

**Found in `/crates/zjj-core/src/output/types.rs`:**
- Empty string checks repeated in constructors (lines 112, 160, 235, 299, 302, 397, 448, 466, 594, 736, 836)
- Path absoluteness check (line 166)
- Terminal status check (line 163)

**Issue:** Validation is scattered across constructors, not centralized.

### 5. Opaque Error Strings

**Found in `/crates/zjj-core/src/output/types.rs`:**
- All error messages are inline strings
- No structured error types for domain concepts

**Issue:** Error messages are not type-safe or composable.

## Refactoring Plan

### Phase 1: Semantic Newtypes for Identifiers

Create newtype wrappers for common primitives:

```rust
// New file: src/output/domain_types.rs
use serde::{Deserialize, Serialize};
use std::fmt;

/// Validated issue identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueId(String);

impl IssueId {
    pub fn new(id: impl Into<String>) -> Result<Self, OutputLineError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OutputLineError::EmptyIssueId);
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IssueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validated issue title
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTitle(String);

impl IssueTitle {
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated plan title
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTitle(String);

impl PlanTitle {
    pub fn new(title: impl Into<String>) -> Result<Self, OutputLineError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(OutputLineError::EmptyTitle);
        }
        Ok(Self(title))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated plan description
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDescription(String);

impl PlanDescription {
    pub fn new(desc: impl Into<String>) -> Result<Self, OutputLineError> {
        let desc = desc.into();
        if desc.trim().is_empty() {
            return Err(OutputLineError::EmptyDescription);
        }
        Ok(Self(desc))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated message content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message(String);

impl Message {
    pub fn new(msg: impl Into<String>) -> Result<Self, OutputLineError> {
        let msg = msg.into();
        if msg.trim().is_empty() {
            return Err(OutputLineError::EmptyMessage);
        }
        Ok(Self(msg))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated warning code
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WarningCode(String);

impl WarningCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated action verb
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionVerb(String);

impl ActionVerb {
    pub fn new(verb: impl Into<String>) -> Self {
        Self(verb.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated action target
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTarget(String);

impl ActionTarget {
    pub fn new(target: impl Into<String>) -> Self {
        Self(target.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Phase 2: Replace Boolean Flags with Enums

```rust
// Replace recoverable: bool with:
pub enum RecoveryCapability {
    Recoverable { recommended_action: RecommendedAction },
    NotRecoverable { reason: String },
}

// Replace automatic: bool with:
pub enum ExecutionMode {
    Automatic,
    Manual,
}

// Replace merge_safe: bool with:
pub enum MergeStatus {
    Safe,
    Unsafe { conflicts: Vec<ConflictDetail> },
}

// Replace success: bool with:
pub enum Outcome {
    Success,
    Failure,
}
```

### Phase 3: Replace Option Fields with Enums

```rust
// Replace session: Option<String> with:
pub enum IssueScope {
    Standalone,
    InSession { session: SessionName },
}

// Replace result: Option<String> with:
pub enum ActionResult {
    Pending,
    Completed { result: String },
}

// Replace command: Option<String> with:
pub enum RecoveryExecution {
    Automatic { command: Command },
    Manual,
}

pub struct Command(String);

impl Command {
    pub fn new(cmd: impl Into<String>) -> Self {
        Self(cmd.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Phase 4: Centralized Validation

Create a validation module:

```rust
// New file: src/output/validation.rs
use crate::{Error, Result};

/// Validate non-empty string
pub fn validate_non_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::ValidationError {
            message: format!("{field_name} cannot be empty"),
            field: Some(field_name.to_string()),
            value: Some(value.to_string()),
            constraints: vec!["non-empty".to_string()],
        });
    }
    Ok(())
}

/// Validate path is absolute
pub fn validate_absolute_path(path: &std::path::Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::ValidationError {
            message: "Path must be absolute".to_string(),
            field: Some("path".to_string()),
            value: Some(path.display().to_string()),
            constraints: vec!["absolute path".to_string()],
        });
    }
    Ok(())
}

/// Validate session is not in terminal state
pub fn validate_non_terminal_status(status: SessionStatus) -> Result<()> {
    if status.is_terminal() {
        return Err(Error::ValidationError {
            message: format!("Cannot create session output with terminal status {status:?}"),
            field: Some("status".to_string()),
            value: None,
            constraints: vec!["non-terminal status".to_string()],
        });
    }
    Ok(())
}
```

### Phase 5: Persistent Data Structures

Replace `Vec<T>` with `rpds::Vector<T>` for immutable collections:

```rust
use rpds::Vector;

pub struct Plan {
    pub title: PlanTitle,
    pub description: PlanDescription,
    pub steps: Vector<PlanStep>,  // Instead of Vec<PlanStep>
    pub created_at: DateTime<Utc>,
}

impl Plan {
    pub fn with_step(mut self, description: String, status: ActionStatus) -> Result<Self, OutputLineError> {
        let order = u32::try_from(self.steps.len()).map_err(|_| OutputLineError::PlanStepOverflow)?;
        let step = PlanStep { order, description, status };
        self.steps.push_back(step);
        Ok(self)
    }
}
```

## Implementation Order

1. **Create domain_types.rs** with newtype wrappers
2. **Update output/types.rs** to use newtypes
3. **Replace boolean flags** with enums
4. **Replace Option fields** with explicit enums
5. **Create validation.rs** with centralized validation
6. **Update constructors** to use validation helpers
7. **Replace Vec with rpds::Vector** for persistent collections
8. **Run cargo fmt and cargo clippy** after each change
9. **Update tests** to use new constructors

## Files to Modify

1. `/crates/zjj-core/src/output/types.rs` - Main refactoring target
2. `/crates/zjj-core/src/output/mod.rs` - Export new types
3. `/crates/zjj-core/src/lib.rs` - Re-export if needed
4. `/crates/zjj-core/Cargo.toml` - Already has rpds dependency

## Testing Strategy

1. Ensure all existing tests pass
2. Add property-based tests for newtype validation
3. Add tests for enum state transitions
4. Verify serialization/deserialization works
5. Test error messages are still helpful

## Benefits

1. **Type Safety:** Compiler catches invalid states
2. **Self-Documenting:** Types express domain concepts
3. **Refactoring Safety:** Changes propagate through type system
4. **No Invalid States:** Enums make illegal states unrepresentable
5. **Better Error Messages:** Structured error types
6. **Immutable by Default:** Persistent data structures

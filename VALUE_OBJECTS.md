# Value Objects Reference

> A comprehensive guide to value objects in the zjj codebase, following Domain-Driven Design principles.

## What are Value Objects?

Value objects are DDD's fundamental building blocks for ensuring validity at creation time. Unlike entities, they are defined by their attributes rather than identity. When you have a value object, **it is always valid** - no need to check again.

### Key Principles

1. **Parse at boundaries, validate once** - Validate when data enters the system
2. **Make illegal states unrepresentable** - Use enums instead of boolean flags
3. **Self-documenting code** - `SessionName` instead of `String`
4. **Immutable by default** - Cannot be modified after creation

---

## Text Value Objects

### Message

A validated message content.

**Purpose**: Ensure user-facing messages are never empty.

**Invariants**:
- Non-empty after trimming whitespace

**Validation Rules**:
```rust
// Empty messages fail
Message::new("")        // => Err(OutputLineError::EmptyMessage)
Message::new("   ")     // => Err(OutputLineError::EmptyMessage)

// Valid messages succeed
Message::new("Hello")   // => Ok(Message("Hello"))
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::Message;

let msg = Message::new("Operation completed successfully")?;
println!("{}", msg.as_str());
```

**Error Types**:
- `OutputLineError::EmptyMessage` - Message is empty or whitespace-only

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### PlanTitle

A validated plan title.

**Purpose**: Ensure plans always have descriptive titles.

**Invariants**:
- Non-empty after trimming whitespace

**Validation Rules**:
```rust
PlanTitle::new("")           // => Err(OutputLineError::EmptyTitle)
PlanTitle::new("Migration")  // => Ok(PlanTitle("Migration"))
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::PlanTitle;

let title = PlanTitle::new("Database Migration Plan")?;
```

**Error Types**:
- `OutputLineError::EmptyTitle` - Title is empty or whitespace-only

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### PlanDescription

A validated plan description.

**Purpose**: Ensure plans have meaningful descriptions.

**Invariants**:
- Non-empty after trimming whitespace

**Validation Rules**:
```rust
PlanDescription::new("")                              // => Err(OutputLineError::EmptyDescription)
PlanDescription::new("Step-by-step migration guide")  // => Ok(...)
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::PlanDescription;

let desc = PlanDescription::new("Migrate user data to new schema")?;
```

**Error Types**:
- `OutputLineError::EmptyDescription` - Description is empty or whitespace-only

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### IssueTitle

A validated issue title.

**Purpose**: Ensure issues always have meaningful titles.

**Invariants**:
- Non-empty after trimming whitespace

**Validation Rules**:
```rust
IssueTitle::new("")                           // => Err(OutputLineError::EmptyTitle)
IssueTitle::new("Fix authentication bug")     // => Ok(...)
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::IssueTitle;

let title = IssueTitle::new("Session limit exceeded")?;
```

**Error Types**:
- `OutputLineError::EmptyTitle` - Title is empty or whitespace-only

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### Command

A validated command string.

**Purpose**: Represent shell commands (can be empty for manual steps).

**Invariants**:
- No validation enforced (can be any string including empty)

**Validation Rules**:
```rust
Command::new("git status")      // => Ok(Command("git status"))
Command::new("")                 // => Ok(Command(""))  // Manual step
Command::new("any string")       // => Always succeeds
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::Command;

let cmd = Command::new("jj resolve");
if !cmd.is_empty() {
    println!("Execute: {}", cmd.as_str());
}
```

**Error Types**:
- None (construction never fails)

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

## Action Value Objects

### ActionVerb

A validated action verb representing operations.

**Purpose**: Prevent typos in action verbs, enable autocomplete.

**Invariants**:
- Known verb or custom lowercase alphanumeric with hyphens
- Must start with a lowercase letter

**Known Verbs**:
```rust
// All predefined verbs
ActionVerb::Run        // "run"
ActionVerb::Execute    // "execute"
ActionVerb::Create     // "create"
ActionVerb::Delete     // "delete"
ActionVerb::Update     // "update"
ActionVerb::Merge      // "merge"
ActionVerb::Rebase     // "rebase"
ActionVerb::Sync       // "sync"
ActionVerb::Fix        // "fix"
ActionVerb::Check      // "check"
ActionVerb::Process    // "process"
ActionVerb::Focus      // "focus"
ActionVerb::Attach     // "attach"
ActionVerb::SwitchTab  // "switch-tab"
ActionVerb::Remove     // "remove"
ActionVerb::Discover   // "discovered"
ActionVerb::WouldFix   // "would_fix"
```

**Validation Rules**:
```rust
// Known verbs (case-insensitive)
ActionVerb::new("run")        // => Ok(ActionVerb::Run)
ActionVerb::new("RUN")        // => Ok(ActionVerb::Run)  // Case-insensitive

// Custom verbs (must be lowercase alphanumeric with hyphens)
ActionVerb::new("deploy")     // => Ok(ActionVerb::Custom("deploy"))
ActionVerb::new("switch-tab") // => Ok(ActionVerb::Custom("switch-tab"))

// Invalid custom verbs
ActionVerb::new("")           // => Err(OutputLineError::InvalidActionVerb)
ActionVerb::new("Run")        // => Err(OutputLineError::InvalidActionVerb)  // Must be lowercase
ActionVerb::new("1task")      // => Err(OutputLineError::InvalidActionVerb)  // Must start with letter
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::ActionVerb;

let verb = ActionVerb::new("merge")?;
if verb.is_custom() {
    println!("Custom action: {}", verb.as_str());
}
```

**Error Types**:
- `OutputLineError::InvalidActionVerb` - Invalid verb format

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### ActionTarget

A validated action target.

**Purpose**: Prevent empty targets and unreasonably long targets.

**Invariants**:
- Non-empty after trimming
- Maximum 1000 characters

**Validation Rules**:
```rust
ActionTarget::new("")  // => Err(OutputLineError::EmptyMessage)
ActionTarget::new(" ")  // => Err(OutputLineError::EmptyMessage)

ActionTarget::new("workspace/name")  // => Ok(...)
ActionTarget::new("a".repeat(1001))  // => Err(OutputLineError::InvalidActionTarget)
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::ActionTarget;

let target = ActionTarget::new("session-auth-fix")?;
println!("Target: {}", target.as_str());
```

**Error Types**:
- `OutputLineError::EmptyMessage` - Target is empty
- `OutputLineError::InvalidActionTarget` - Target exceeds max length

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

## Warning Value Objects

### WarningCode

A validated warning code.

**Purpose**: Standardize warning codes, prevent typos.

**Invariants**:
- Known code or custom alphanumeric/underscore starting with letter

**Known Codes**:
```rust
WarningCode::ConfigNotFound       // "CONFIG_NOT_FOUND"
WarningCode::ConfigInvalid        // "CONFIG_INVALID"
WarningCode::SessionLimitReached  // "SESSION_LIMIT_REACHED"
WarningCode::WorkspaceNotFound    // "WORKSPACE_NOT_FOUND"
WarningCode::GitOperationFailed   // "GIT_OPERATION_FAILED"
WarningCode::MergeConflict        // "MERGE_CONFLICT"
WarningCode::QueueEntryBlocked    // "QUEUE_ENTRY_BLOCKED"
WarningCode::AgentUnavailable     // "AGENT_UNAVAILABLE"
```

**Validation Rules**:
```rust
// Known codes
WarningCode::new("CONFIG_NOT_FOUND")     // => Ok(WarningCode::ConfigNotFound)

// Custom codes (must start with letter, alphanumeric/underscore)
WarningCode::new("W001")                 // => Ok(WarningCode::Custom("W001"))
WarningCode::new("CUSTOM_ERROR")         // => Ok(WarningCode::Custom("CUSTOM_ERROR"))

// Invalid custom codes
WarningCode::new("")                     // => Err(OutputLineError::InvalidWarningCode)
WarningCode::new("123")                  // => Err(OutputLineError::InvalidWarningCode)  // No letter start
WarningCode::new("INVALID-CODE")         // => Err(OutputLineError::InvalidWarningCode)  // Hyphen not allowed
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::WarningCode;

let code = WarningCode::new("CONFIG_NOT_FOUND")?;
println!("Warning: {}", code.as_str());
```

**Error Types**:
- `OutputLineError::InvalidWarningCode` - Invalid warning code format

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

## State Value Objects (Enums Instead of Booleans)

### Outcome

Replaces `success: bool` with an explicit enum.

**Purpose**: Make operation results explicit and self-documenting.

**Variants**:
```rust
Outcome::Success   // Operation succeeded
Outcome::Failure   // Operation failed
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::Outcome;

// Direct construction
let result = Outcome::Success;

// From bool (for backward compatibility)
let result = Outcome::from_bool(true);   // => Outcome::Success

// To bool (for backward compatibility)
let success = result.to_bool();           // => true
```

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### RecoveryCapability

Replaces `recoverable: bool` with context.

**Purpose**: Provide recovery context (recommended action or reason).

**Variants**:
```rust
RecoveryCapability::Recoverable {
    recommended_action: String,  // What to do to recover
}

RecoveryCapability::NotRecoverable {
    reason: String,  // Why it cannot be recovered
}
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::RecoveryCapability;

let recoverable = RecoveryCapability::Recoverable {
    recommended_action: "Run jj resolve".to_string(),
};

let not_recoverable = RecoveryCapability::NotRecoverable {
    reason: "Manual merge required".to_string(),
};
```

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### ExecutionMode

Replaces `automatic: bool` with an explicit enum.

**Purpose**: Make step execution mode explicit.

**Variants**:
```rust
ExecutionMode::Automatic  // Step executes automatically
ExecutionMode::Manual     // Step requires manual execution
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::ExecutionMode;

let mode = ExecutionMode::Automatic;
```

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

### ActionResult

Replaces `result: Option<String>` with explicit state.

**Purpose**: Make action completion state explicit.

**Variants**:
```rust
ActionResult::Pending           // Action is still pending
ActionResult::Completed {
    result: String,  // Action completed with this result
}
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::ActionResult;

// Pending action
let pending = ActionResult::Pending;
assert!(pending.result().is_none());

// Completed action
let completed = ActionResult::completed("Success");
assert_eq!(completed.result(), Some("Success"));
```

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

## Metadata Value Objects

### ValidatedMetadata

Validated JSON metadata for extensibility.

**Purpose**: Ensure metadata is valid JSON, provide type-level distinction.

**Invariants**:
- Always valid JSON (wraps `serde_json::Value`)

**Validation Rules**:
```rust
// Always succeeds - any serde_json::Value is valid
ValidatedMetadata::new(serde_json::json!({"key": "value"}))  // => Ok(...)

// Empty/null metadata
ValidatedMetadata::empty()  // => ValidatedMetadata(Value::Null)
```

**Usage Example**:
```rust
use zjj_core::output::domain_types::ValidatedMetadata;

// From JSON value
let metadata = ValidatedMetadata::new(serde_json::json!({
    "attempt": 1,
    "timestamp": "2024-01-01"
}));

// Check if empty
if !metadata.is_empty() {
    // Get a field
    if let Some(value) = metadata.get("attempt") {
        println!("Attempt: {}", value);
    }
}

// Convert to serde_json::Value
let json_value: serde_json::Value = metadata.into_value();
```

**Location**: `crates/zjj-core/src/output/domain_types.rs`

---

## Priority Value Objects

### Priority (CLI Contracts)

Queue priority value (lower = higher priority).

**Purpose**: Type-safe priority values without magic numbers.

**Invariants**:
- i32 value with ordering

**Validation Rules**:
```rust
Priority::new(5)  // => Priority(5)
```

**Usage Example**:
```rust
use zjj_core::cli_contracts::domain_types::Priority;

let p = Priority::try_from(5)?;
println!("Priority: {}", p.value());

// Compare priorities (lower is higher)
if p1 < p2 {
    println!("p1 has higher priority");
}
```

**Error Types**:
- `ContractError::InvalidInput` - Priority exceeds 1000

**Location**: `crates/zjj-core/src/cli_contracts/domain_types.rs`

---

### Priority (Coordination)

Queue priority value for coordination layer.

**Purpose**: Priority with convenience constructors.

**Invariants**:
- i32 value with ordering (lower = higher)

**Constants**:
```rust
Priority::default()  // => Priority(5)
Priority::high()     // => Priority(1)
Priority::low()      // => Priority(10)
```

**Usage Example**:
```rust
use zjj_core::coordination::domain_types::Priority;

let p = Priority::high();
assert_eq!(p.value(), 1);

// Ordering works
assert!(Priority::high() < Priority::default());
assert!(Priority::default() < Priority::low());
```

**Location**: `crates/zjj-core/src/coordination/domain_types.rs`

---

## Deduplication Value Objects

### DedupeKey

Deduplication key for queue entries.

**Purpose**: Prevent duplicate work by rejecting entries with duplicate keys.

**Invariants**:
- Non-empty string

**Validation Rules**:
```rust
DedupeKey::new("workspace-123".to_string())  // => Ok(...)
DedupeKey::new(String::new())                // => Err(DomainError::Empty { .. })
```

**Usage Example**:
```rust
use zjj_core::coordination::domain_types::DedupeKey;

let key = DedupeKey::new_from_str("workspace-auth-fix")?;
println!("Dedupe key: {}", key.as_str());

// Convert to inner string
let inner = key.into_inner();
```

**Error Types**:
- `DomainError::Empty { field: "dedupe_key" }` - Key is empty

**Location**: `crates/zjj-core/src/coordination/domain_types.rs`

---

## Related Documentation

- [DOMAIN_TYPES_GUIDE.md](DOMAIN_TYPES_GUIDE.md) - Full domain type reference
- [DDD_QUICK_START.md](DDD_QUICK_START.md) - DDD patterns in zjj
- [IDENTIFIERS.md](IDENTIFIERS.md) - Entity identifiers (SessionName, AgentId, etc.)

---

## Value Object vs Entity

**Value Objects** (this document):
- Defined by their attributes
- No identity
- Immutable
- `Message`, `PlanTitle`, `Priority`, `DedupeKey`

**Entities** (see IDENTIFIERS.md):
- Defined by their identity
- Mutable attributes
- `Bead`, `Session`, `Workspace`

---

## Creating New Value Objects

When adding a new value object, follow this pattern:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValueObjectError {
    #[error("value cannot be empty")]
    Empty,
    #[error("value too long: {actual} (max {max})")]
    TooLong { max: usize, actual: usize },
}

/// A validated value object.
///
/// # Invariants
/// - Must be non-empty
/// - Must not exceed 100 characters
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyValueObject(String);

impl MyValueObject {
    /// Maximum allowed length
    pub const MAX_LENGTH: usize = 100;

    /// Create and validate a new value object.
    ///
    /// # Errors
    ///
    /// Returns `ValueObjectError` if validation fails.
    pub fn new(value: String) -> Result<Self, ValueObjectError> {
        if value.is_empty() {
            return Err(ValueObjectError::Empty);
        }
        if value.len() > Self::MAX_LENGTH {
            return Err(ValueObjectError::TooLong {
                max: Self::MAX_LENGTH,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Get the underlying value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## Best Practices

1. **Always validate at construction** - Never have an invalid value object
2. **Use `#[must_use]`** - Prevent accidentally ignoring results
3. **Implement `Display`** - Make output easy
4. **Document invariants** - Clear comments on what's guaranteed
5. **Prefer enums over booleans** - Make states explicit
6. **Use serde transparently** - `#[serde(transparent)]` for wrapper types

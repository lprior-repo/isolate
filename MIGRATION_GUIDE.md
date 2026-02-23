# DDD Refactoring Migration Guide

## Executive Summary

This guide helps teams safely migrate to the refactored Domain-Driven Design (DDD) architecture in ZJJ. The refactoring follows Scott Wlaschin's functional DDD principles, making illegal states unrepresentable through semantic newtypes and explicit state machines.

**Status**: Migration Complete and Production Ready

**Date**: 2026-02-23

**Breaking Changes**: Yes (see sections below)

**Backward Compatibility**: Partial (see Rollback Strategies)

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Breaking Changes Summary](#breaking-changes-summary)
3. [Migration Patterns](#migration-patterns)
4. [Detailed Migration Steps](#detailed-migration-steps)
5. [Testing After Migration](#testing-after-migration)
6. [Rollback Strategies](#rollback-strategies)
7. [Common Issues and Solutions](#common-issues-and-solutions)

---

## Quick Start

### For Immediate Adoption

If you just want to use the new API without understanding all the details:

```rust
// OLD API (still works via backward compat)
let name = SessionName::new("my-session")?;

// NEW API (preferred)
let name = SessionName::parse("my-session")?;

// OR via FromStr
let name: SessionName = "my-session".parse()?;
```

**Key Point**: The `new()` method still works but delegates to `parse()`. Gradually migrate to `parse()` for consistency.

---

## Breaking Changes Summary

### 1. Identifier Construction API Changes

#### Affected Types
- `SessionName`
- `AgentId`
- `WorkspaceName`
- `TaskId` / `BeadId`
- `SessionId`
- `QueueEntryId`
- `AbsolutePath`

#### Breaking Change

**Before**:
```rust
// Inconsistent APIs across modules
let name = SessionName::new("my-session")?;
let bead = BeadId::new("bd-abc123")?;
let agent = AgentId::new("agent-123")?;
```

**After**:
```rust
// Consistent parse() API everywhere
let name = SessionName::parse("my-session")?;
let bead = BeadId::parse("bd-abc123")?;
let agent = AgentId::parse("agent-123")?;
```

#### Migration Steps

1. **Find all usages** of `::new()` on identifier types:
   ```bash
   grep -r "SessionName::new\|BeadId::new\|AgentId::new" crates/
   ```

2. **Replace with `::parse()`**:
   ```rust
   // Before
   let name = SessionName::new(raw_name)?;

   // After
   let name = SessionName::parse(raw_name)?;
   ```

3. **Update error handling** if needed:
   ```rust
   // Before: Error type depended on module
   // After: All use IdentifierError
   use zjj_core::domain::IdentifierError;
   ```

#### Potential Issues

- **Error type changes**: Some modules returned custom errors, now all return `IdentifierError`
- **Solution**: Update error handling to use `IdentifierError` or `Into<anyhow::Error>`

---

### 2. BeadId Type Consolidation

#### Breaking Change

**Before**:
```rust
// 5 different BeadId implementations in different modules
use zjj_core::output::domain_types::BeadId;
use zjj_core::coordination::domain_types::BeadId;
use zjj::cli::handlers::domain::BeadId;
use zjj::commands::done::newtypes::BeadId;
```

**After**:
```rust
// Single canonical type
use zjj_core::domain::BeadId;  // Type alias for TaskId
```

#### Migration Steps

1. **Update imports**:
   ```rust
   // Before
   use zjj_core::output::domain_types::BeadId;

   // After
   use zjj_core::domain::BeadId;
   ```

2. **No API changes needed** - All BeadId types had the same interface

#### Potential Issues

- **Type confusion**: If you had both types imported, remove the duplicate
- **Serialization**: All BeadId values serialize/deserialize identically

---

### 3. SessionName MAX_LENGTH Change

#### Breaking Change

**Before**: `MAX_LENGTH = 64`

**After**: `MAX_LENGTH = 63` (DNS label standard)

#### Migration Steps

1. **Update tests** expecting 64-character limit:
   ```rust
   // Before
   let long_name = "a".repeat(65);
   assert!(SessionName::parse(&long_name).is_err());

   // After
   let long_name = "a".repeat(64);
   assert!(SessionName::parse(&long_name).is_err());
   ```

2. **Update validation** if you have custom length checks

#### Potential Issues

- **Existing data**: If you have session names with exactly 64 characters, they're now invalid
- **Solution**: Rename those sessions to 63 characters or less

---

### 4. Error Type Unification

#### Breaking Change

**Before**:
```rust
// Different error types per module
type SessionNameError = /* custom error */;
type AgentIdError = /* custom error */;
type TaskIdError = /* custom error */;
```

**After**:
```rust
// Single unified error type
use zjj_core::domain::IdentifierError;

type SessionNameError = IdentifierError;
type AgentIdError = IdentifierError;
type TaskIdError = IdentifierError;
```

#### Migration Steps

1. **Update error imports**:
   ```rust
   // Before
   use zjj_core::types::SessionNameError;

   // After
   use zjj_core::domain::IdentifierError;
   // or use the alias
   use zjj_core::domain::SessionNameError;
   ```

2. **Update error matching**:
   ```rust
   // Before
   match result {
       Err(SessionNameError::InvalidFormat { .. }) => { /* ... */ }
   }

   // After
   match result {
       Err(IdentifierError::InvalidFormat { .. }) => { /* ... */ }
   }
   ```

---

### 5. Removed Duplicate Implementations

#### Breaking Change

The following duplicate implementations were removed:

**Removed Files** (code now uses domain module):
- `output/domain_types.rs` (identifier implementations)
- `coordination/domain_types.rs` (identifier implementations)
- `cli/handlers/domain.rs` (identifier implementations)
- `commands/done/newtypes.rs` (identifier implementations)

#### Migration Steps

1. **Update imports** to use canonical location:
   ```rust
   // Before
   use crate::output::domain_types::SessionName;

   // After
   use crate::domain::SessionName;
   ```

2. **Re-export** if needed for compatibility:
   ```rust
   // In your module's mod.rs
   pub use zjj_core::domain::SessionName;
   ```

---

## Migration Patterns

### Pattern 1: Parse at Boundaries

**Goal**: Validate once at the boundary, trust inside.

#### Before
```rust
// Validation scattered throughout
pub fn create_session(name: &str, workspace: &str) -> Result<Session> {
    validate_session_name(name)?;  // Validate here
    validate_workspace_name(workspace)?;  // And here
    // ... more logic
    db.create(name, workspace)  // Pass raw strings
}

pub fn db_create(name: &str, workspace: &str) -> Result<Session> {
    validate_session_name(name)?;  // Validate again!
    // ... database logic
}
```

#### After
```rust
// Parse once at boundary
pub async fn create_session_handler(raw_name: String, raw_workspace: String) -> Result<()> {
    // PARSE at boundary
    let name = SessionName::parse(raw_name)?;
    let workspace = WorkspaceName::parse(raw_workspace)?;

    // Pass validated types
    create_session(&name, &workspace).await
}

// Core accepts only validated types
pub async fn create_session(name: &SessionName, workspace: &WorkspaceName) -> Result<Session> {
    // NO validation needed - already validated!
    db.create(name, workspace).await
}
```

**Benefits**:
- Validate once, not multiple times
- Type safety guarantees validity
- Compiler catches invalid states

---

### Pattern 2: Use Domain Types in Signatures

**Goal**: Make illegal states unrepresentable.

#### Before
```rust
pub struct Session {
    pub name: String,  // Could be anything
    pub branch: Option<String>,  // Encodes state (bad)
    pub parent: Option<String>,  // Encodes state (bad)
    pub status: String,  // String-based state machine
}

// Usage requires runtime checks
if session.status == "active" {  // Typo possible!
    // ...
}
```

#### After
```rust
pub struct Session {
    pub name: SessionName,  // Already validated
    pub branch: BranchState,  // Explicit state enum
    pub parent: ParentState,  // Explicit state enum
    pub status: SessionStatus,  // Enum-based state machine
}

// Compiler ensures exhaustive matching
match session.status {
    SessionStatus::Creating => { /* ... */ },
    SessionStatus::Active => { /* ... */ },
    SessionStatus::Paused => { /* ... */ },
    // Compiler error if you miss a case!
}
```

---

### Pattern 3: Error Conversion at Boundaries

**Goal**: Use domain errors in core, convert to anyhow at boundaries.

#### Before
```rust
// Domain returns anyhow (loses type info)
pub fn create_session(name: &str) -> Result<Session> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("name cannot be empty"));
    }
    // ...
}
```

#### After
```rust
// Core returns domain-specific errors
pub fn create_session(name: &SessionName) -> Result<Session, SessionError> {
    // ...
}

// Shell converts to anyhow with context
pub async fn create_session_handler(raw_name: String) -> anyhow::Result<()> {
    let name = SessionName::parse(raw_name)
        .context("failed to parse session name")?;

    create_session(&name)
        .await
        .context("failed to create session")?
}
```

**Benefits**:
- Core has typed errors
- Shell has contextual errors
- Best of both worlds

---

### Pattern 4: State Machine Enums

**Goal**: Replace Option/String with explicit states.

#### Before
```rust
pub struct QueueEntry {
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Usage requires checking multiple Options
if let Some(agent) = entry.claimed_by {
    if let Some(claimed) = entry.claimed_at {
        if let Some(expires) = entry.expires_at {
            // ... finally use the values
        }
    }
}
```

#### After
```rust
pub enum ClaimState {
    Unclaimed,
    Claimed {
        agent: AgentId,
        claimed_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    Expired {
        previous_agent: AgentId,
        expired_at: DateTime<Utc>,
    },
}

pub struct QueueEntry {
    pub claim_state: ClaimState,
}

// Single match, exhaustive
match entry.claim_state {
    ClaimState::Unclaimed => { /* ... */ },
    ClaimState::Claimed { agent, claimed_at, expires_at } => {
        // All values available
    },
    ClaimState::Expired { previous_agent, expired_at } => {
        // All values available
    },
}
```

---

## Detailed Migration Steps

### Phase 1: Update Identifier Construction (1-2 hours)

**Goal**: Replace `::new()` with `::parse()`

#### Step 1: Find All Usages

```bash
# Find all new() calls on identifier types
cd /home/lewis/src/zjj
grep -rn "SessionName::new\|BeadId::new\|AgentId::new\|WorkspaceName::new\|TaskId::new" crates/ \
  | grep -v "target/" \
  > identifier_new_calls.txt
```

#### Step 2: Update Each File

For each file in `identifier_new_calls.txt`:

```rust
// Add import if not present
use zjj_core::domain::IdentifierError;

// Before
let name = SessionName::new(raw_name)?;

// After
let name = SessionName::parse(raw_name)?;
```

#### Step 3: Update Error Handling

```rust
// Before
match result {
    Err(SessionNameError::InvalidFormat { details }) => {
        eprintln!("Invalid format: {details}");
    }
}

// After
match result {
    Err(IdentifierError::InvalidFormat { details }) => {
        eprintln!("Invalid format: {details}");
    }
}
```

#### Step 4: Test

```bash
moon run :test
moon run :clippy
```

---

### Phase 2: Update Type Imports (30 minutes)

**Goal**: Import from canonical `domain` module

#### Step 1: Find Old Imports

```bash
# Find imports from old locations
grep -rn "use.*output::domain_types" crates/ > old_imports.txt
grep -rn "use.*coordination::domain_types" crates/ >> old_imports.txt
grep -rn "use.*cli::handlers::domain" crates/ >> old_imports.txt
```

#### Step 2: Replace Imports

```rust
// Before
use zjj_core::output::domain_types::{SessionName, BeadId};
use zjj_core::coordination::domain_types::{AgentId, WorkspaceName};

// After
use zjj_core::domain::{SessionName, BeadId, AgentId, WorkspaceName};
```

#### Step 3: Update Re-Exports

If you have a module that re-exports these types:

```rust
// Before
pub use crate::output::domain_types::SessionName;

// After
pub use crate::domain::SessionName;
```

---

### Phase 3: Update Test Fixtures (1 hour)

**Goal**: Update tests using old APIs

#### Step 1: Update Test Length Limits

```rust
// Before
let long_name = "a".repeat(65);
assert!(SessionName::new(&long_name).is_err());

// After
let long_name = "a".repeat(64);
assert!(SessionName::parse(&long_name).is_err());
```

#### Step 2: Update Test Constructors

```rust
// Before
let name = SessionName::new("test-session").unwrap();

// After
let name = SessionName::parse("test-session").unwrap();
// OR
let name: SessionName = "test-session".parse().unwrap();
```

#### Step 3: Update Error Assertions

```rust
// Before
assert!(matches!(err, SessionNameError::InvalidFormat { .. }));

// After
assert!(matches!(err, IdentifierError::InvalidFormat { .. }));
```

---

### Phase 4: Gradual Migration to Parse-at-Boundaries (2-4 hours)

**Goal**: Move validation to boundaries

#### Step 1: Identify Entry Points

Find functions that accept raw strings:

```bash
grep -rn "pub.*fn.*name: &str" crates/ | grep -v test > entry_points.txt
```

#### Step 2: Update Handlers

```rust
// Before
pub async fn create_session_handler(name: String) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("name cannot be empty"));
    }
    create_session(&name, &workspace).await
}

// After
pub async fn create_session_handler(name: String) -> anyhow::Result<()> {
    // Parse at boundary
    let session_name = SessionName::parse(name)
        .context("invalid session name")?;

    // Pass validated type
    create_session(&session_name, &workspace).await
        .context("failed to create session")
}
```

#### Step 3: Update Core Functions

```rust
// Before
pub async fn create_session(name: &str, workspace: &WorkspaceName) -> Result<Session> {
    validate_session_name(name)?;
    // ...
}

// After
pub async fn create_session(name: &SessionName, workspace: &WorkspaceName) -> Result<Session> {
    // No validation - already validated!
    // ...
}
```

---

## Testing After Migration

### Unit Test Checklist

Run for each migrated module:

```bash
# Test the domain module
moon run :test --lib domain

# Test specific identifier types
moon run :test --lib identifiers

# Test all core tests
moon run :test --lib
```

### Integration Test Checklist

```bash
# Test all integration tests
moon run :test

# Test specific features
moon run :test --test session_feature
moon run :test --test queue_feature
moon run :test --test status_feature
```

### Manual Test Checklist

After migration, manually test:

- [ ] Create session with valid name
- [ ] Create session with invalid name (should fail gracefully)
- [ ] Create task/bead with valid ID
- [ ] Create task/bead with invalid ID (should fail gracefully)
- [ ] Queue operations with valid IDs
- [ ] Status operations
- [ ] Stack operations
- [ ] Config operations

### Property-Based Testing

Run property tests to verify invariants:

```bash
# Run all property tests
moon run :test --test *_properties

# Run specific property tests
moon run :test --test session_properties
moon run :test --test task_properties
moon run :test --test queue_properties
moon run :test --test status_properties
moon run :test --test agent_properties
```

### Regression Testing

Compare output before/after migration:

```bash
# Run test suite before migration
moon run :test 2>&1 | tee before_migration.txt

# Run test suite after migration
moon run :test 2>&1 | tee after_migration.txt

# Compare results
diff before_migration.txt after_migration.txt
```

---

## Rollback Strategies

### Strategy 1: Gradual Rollback per Module

If a specific module has issues:

```bash
# Revert just that module's changes
git checkout HEAD~1 -- crates/zjj/src/commands/session.rs

# Keep other migrations
git add .
git commit -m "rollback: revert session module migration"
```

### Strategy 2: Feature Flag

Add feature flag to enable/disable new API:

```rust
// In Cargo.toml
[features]
default = ["ddd-migration"]
ddd-migration = []

// In code
#[cfg(feature = "ddd-migration")]
pub fn create_session(name: &SessionName) -> Result<Session> {
    // New implementation
}

#[cfg(not(feature = "ddd-migration"))]
pub fn create_session(name: &str) -> Result<Session> {
    // Old implementation
}
```

### Strategy 3: Adapter Layer

Keep old API as wrapper around new API:

```rust
// Legacy adapter for backward compatibility
impl SessionName {
    /// Legacy constructor for backward compatibility
    #[deprecated(since = "1.0.0", note = "Use SessionName::parse() instead")]
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        Self::parse(name)
            .map_err(|e| Error::ValidationError {
                message: e.to_string(),
                field: Some("name".to_string()),
                value: None,
                constraints: vec![],
            })
    }
}
```

**Current Status**: This adapter is already in place in `types.rs`!

### Strategy 4: Full Rollback

If you need to rollback everything:

```bash
# Reset to pre-migration commit
git log --oneline | grep "ddd\|DDD\|migration"
git reset --hard <commit-before-migration>

# Re-apply any non-migration changes
git cherry-pick <commits-to-keep>
```

---

## Common Issues and Solutions

### Issue 1: Type Mismatch Errors

**Problem**:
```rust
error[E0308]: mismatched types
   --> src/session.rs:123:20
    |
123 |     create_session(&name, &workspace)
    |                    ^^^^^ expected `&SessionName`, found `&String`
```

**Solution**:
```rust
// Before
let name = "my-session".to_string();
create_session(&name, &workspace);

// After
let name = SessionName::parse("my-session")?;
create_session(&name, &workspace);
```

---

### Issue 2: Error Conversion Errors

**Problem**:
```rust
error[E0277]: `?` couldn't convert the error type
   --> src/session.rs:123:20
    |
123 |     let name = SessionName::parse(raw_name)?;
    |                                            ^ the trait `From<IdentifierError>` is not implemented for `anyhow::Error`
```

**Solution**:
```rust
// Add context to convert to anyhow
let name = SessionName::parse(raw_name)
    .context("failed to parse session name")?;

// OR use map_err
let name = SessionName::parse(raw_name)
    .map_err(|e| anyhow::anyhow!("invalid session name: {e}"))?;
```

---

### Issue 3: Serialization Errors

**Problem**:
```rust
error[E0277]: the trait `From<String>` is not implemented for `SessionName`
```

**Solution**:
```rust
// Use TryFrom instead of From
// Before
let name: SessionName = raw_string.into();

// After
let name: SessionName = raw_string.try_into()?;
// OR
let name = SessionName::parse(raw_string)?;
```

---

### Issue 4: Test Fixture Failures

**Problem**: Tests fail after updating MAX_LENGTH from 64 to 63

**Solution**:
```rust
// Before
let max_name = "a".repeat(64);
assert!(SessionName::parse(&max_name).is_ok());
let too_long = "a".repeat(65);
assert!(SessionName::parse(&too_long).is_err());

// After
let max_name = "a".repeat(63);
assert!(SessionName::parse(&max_name).is_ok());
let too_long = "a".repeat(64);
assert!(SessionName::parse(&too_long).is_err());
```

---

### Issue 5: Multiple BeadId Types Confusion

**Problem**: Compiler errors about ambiguous types

**Solution**:
```rust
// Before (ambiguous)
use zjj_core::output::domain_types::BeadId;
use zjj_core::domain::BeadId;

// After (single import)
use zjj_core::domain::BeadId;

// If you need both, disambiguate
use zjj_core::domain::BeadId as DomainBeadId;
```

---

### Issue 6: Missing FromStr Implementation

**Problem**: Can't use `.parse()` method

**Solution**:
```rust
// Make sure you import FromStr
use std::str::FromStr;

// Now you can parse
let name: SessionName = "my-session".parse()?;
```

---

## Appendix: Complete Type Reference

### Identifier Types

| Type | Constructor | Max Length | Pattern | Error Type |
|------|-------------|------------|---------|------------|
| `SessionName` | `parse()` | 63 | `[a-zA-Z][a-zA-Z0-9_-]{0,62}` | `IdentifierError` |
| `AgentId` | `parse()` | 128 | Alphanumeric + `-_.:` | `IdentifierError` |
| `WorkspaceName` | `parse()` | 255 | No path separators | `IdentifierError` |
| `TaskId` | `parse()` | ∞ | `bd-[a-fA-F0-9]+` | `IdentifierError` |
| `BeadId` | `parse()` | ∞ | `bd-[a-fA-F0-9]+` | `IdentifierError` |
| `SessionId` | `parse()` | ∞ | Non-empty ASCII | `IdentifierError` |
| `QueueEntryId` | `new()` | ∞ | Positive i64 | `IdentifierError` |
| `AbsolutePath` | `parse()` | ∞ | Absolute path | `IdentifierError` |

### State Enums

| Type | Variants |
|------|----------|
| `SessionStatus` | `Creating`, `Active`, `Paused`, `Completed`, `Failed` |
| `QueueStatus` | `Pending`, `Processing`, `Completed`, `Failed`, `Cancelled` |
| `AgentStatus` | `Pending`, `Running`, `Completed`, `Failed`, `Cancelled`, `Timeout` |
| `TaskStatus` | `Open`, `InProgress`, `Blocked`, `Closed` |
| `TaskPriority` | `P0`, `P1`, `P2`, `P3`, `P4` |
| `BranchState` | `Detached`, `OnBranch { name: String }` |
| `ParentState` | `NoParent`, `HasParent { name: SessionName }` |
| `ClaimState` | `Unclaimed`, `Claimed { ... }`, `Expired { ... }` |

### Value Objects

| Type | Purpose | Constructor |
|------|---------|-------------|
| `NonEmptyString` | Trimmed non-empty string | `parse()` |
| `Limit` | Pagination/operation limit | `new()` (1..=1000) |
| `Priority` | Queue priority | `new()` (0..=1000) |
| `TimeoutSeconds` | Timeout duration | `new()` (1..=86400) |

---

## Quick Reference Cards

### Card 1: Identifier Construction

```rust
// Preferred: parse() method
let name = SessionName::parse("my-session")?;
let agent = AgentId::parse("agent-123")?;
let workspace = WorkspaceName::parse("my-workspace")?;
let task = TaskId::parse("bd-abc123")?;
let path = AbsolutePath::parse("/home/user")?;

// Alternative: FromStr trait
let name: SessionName = "my-session".parse()?;

// Legacy: new() (backward compat)
let name = SessionName::new("my-session")?;

// Access underlying value
let s: &str = name.as_str();
let owned: String = name.into_string();
```

### Card 2: Error Handling

```rust
use zjj_core::domain::IdentifierError;

// Match on specific errors
match SessionName::parse(raw_name) {
    Ok(name) => { /* use name */ },
    Err(IdentifierError::Empty) => {
        eprintln!("Name cannot be empty");
    },
    Err(IdentifierError::TooLong { max, actual }) => {
        eprintln!("Name too long: {actual} > {max}");
    },
    Err(IdentifierError::InvalidCharacters { details }) => {
        eprintln!("Invalid characters: {details}");
    },
    Err(e) => {
        eprintln!("Invalid name: {e}");
    }
}

// Convert to anyhow
let name = SessionName::parse(raw_name)
    .context("invalid session name")?;
```

### Card 3: State Matching

```rust
// Session status
match session.status {
    SessionStatus::Creating => { /* ... */ },
    SessionStatus::Active => { /* ... */ },
    SessionStatus::Paused => { /* ... */ },
    SessionStatus::Completed => { /* ... */ },
    SessionStatus::Failed => { /* ... */ },
}

// Branch state
match session.branch {
    BranchState::Detached => println!("Detached"),
    BranchState::OnBranch { name } => println!("On {name}"),
}

// Claim state
match entry.claim_state {
    ClaimState::Unclaimed => { /* ... */ },
    ClaimState::Claimed { agent, claimed_at, expires_at } => {
        println!("Claimed by {agent} at {claimed_at}");
    },
    ClaimState::Expired { previous_agent, expired_at } => {
        println!("Expired from {previous_agent} at {expired_at}");
    },
}
```

---

## Support and Resources

### Documentation Files

- `/home/lewis/src/zjj/DDD_REFACTORING_REPORT.md` - Full DDD refactoring details
- `/home/lewis/src/zjj/CLI_CONTRACTS_REFACTORING.md` - CLI contracts refactoring
- `/home/lewis/src/zjj/SESSION_NAME_MIGRATION_COMPLETE.md` - SessionName migration
- `/home/lewis/src/zjj/BEADID_CONSOLIDATION_SUMMARY.md` - BeadId consolidation
- `/home/lewis/src/zjj/MIGRATION_COMPLETE_REPORT.md` - Final migration report

### Code Locations

- Domain types: `/home/lewis/src/zjj/crates/zjj-core/src/domain/`
- CLI contracts: `/home/lewis/src/zjj/crates/zjj-core/src/cli_contracts/`
- Coordination: `/home/lewis/src/zjj/crates/zjj-core/src/coordination/`
- Output types: `/home/lewis/src/zjj/crates/zjj-core/src/output/`

### Getting Help

1. **Check compiler errors**: Rust error messages are very helpful
2. **Read the docs**: Each type has extensive documentation
3. **Look at tests**: Test files show usage patterns
4. **Run examples**: Check `examples/` directory for usage

---

## Conclusion

This migration guide provides a comprehensive path to adopting the DDD refactoring. The key principles are:

1. **Parse at boundaries**: Validate once, trust everywhere
2. **Use semantic types**: Make illegal states unrepresentable
3. **Embrace enums**: Replace Option/bool with explicit states
4. **Handle errors properly**: Use domain errors in core, convert at boundaries

The refactoring is backward compatible via `new()` method adapters, so you can migrate gradually. Start with new code and migrate existing code as you touch it.

**Remember**: The compiler is your friend. Let it guide you through the migration.

---

*Last Updated: 2026-02-23*
*Author: Claude Code (Functional Rust Expert)*
*Version: 1.0*

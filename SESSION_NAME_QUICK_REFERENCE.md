# SessionName Consolidation - Quick Reference

## The Problem

```rust
// ❌ THREE different implementations with INCONSISTENT rules

// Implementation 1: domain/identifiers.rs
SessionName::parse("name")  // → IdError, MAX=63

// Implementation 2: types.rs
SessionName::new("name")     // → Error, MAX=64

// Implementation 3: cli_contracts/domain_types.rs
SessionName::try_from("name")  // → ContractError, MAX=64
SessionName::new_unchecked("⚠️")  // → NO VALIDATION! (SECURITY ISSUE)
```

## The Solution

```rust
// ✅ ONE implementation with CONSISTENT validation

// domain/identifiers.rs (canonical source)
SessionName::parse("  name  ")  // → IdError, trims whitespace, MAX=63

// All other modules re-export this implementation
```

## Key Changes

### 1. Enhanced Validation (Trim-Then-Validate)

```rust
// BEFORE: No trimming
SessionName::parse(" my-session ")?
// Error: contains spaces

// AFTER: Automatic trimming
SessionName::parse(" my-session ")?
// Result: "my-session" (whitespace removed)
```

### 2. Removed Security Bypass

```rust
// BEFORE: Unsafe bypass existed
let name = SessionName::new_unchecked("invalid@#$%");
// ❌ NO VALIDATION - SECURITY RISK!

// AFTER: Bypass removed
// This method no longer exists - must always validate
```

### 3. Consistent MAX_LENGTH

```rust
// BEFORE: Inconsistent
domain::SessionName       → MAX 63
types::SessionName        → MAX 64
cli_contracts::SessionName → MAX 64

// AFTER: Consistent
All SessionName → MAX 63
```

## Migration Checklist

### For Library Users

- [ ] Update `SessionName::new()` to `SessionName::parse()`
- [ ] Update error handling from `Error` to `IdError`
- [ ] Adjust length validation (64 → 63)
- [ ] Test with whitespace inputs

### For Contributors

- [ ] Remove local `SessionName` implementations
- [ ] Re-export from `domain::identifiers`
- [ ] Remove `new_unchecked()` methods
- [ ] Update tests to use `parse()`
- [ ] Update documentation

## Validation Rules

```rust
// ✅ Valid
"my-session"
"Feature_Auth"
"session-123"
"a"
"A"
"test_session_123"

// ❌ Invalid
""                           // Empty
"   "                        // Whitespace only
"123-session"                // Starts with number
"-session"                   // Starts with dash
"_session"                   // Starts with underscore
"session name"               // Contains space
"session@name"               // Special char
"session.name"               // Contains dot
&"a".repeat(64)              // Too long (max 63)
```

## Code Examples

### CLI Command (Parsing User Input)

```rust
use zjj_core::domain::SessionName;

// Parse and validate user input (with trimming)
let name = SessionName::parse(user_input)
    .map_err(|e| anyhow!("Invalid session name: {e}"))?;

// Use the validated name
println!("Session: {}", name);
```

### Domain Logic (Using Validated Name)

```rust
fn create_session(name: SessionName) -> Result<Session> {
    // No validation needed - already validated!
    Ok(Session {
        name: name.clone(),
        status: SessionStatus::Active,
        // ...
    })
}
```

### Testing

```rust
#[test]
fn test_trim_whitespace() {
    let name = SessionName::parse("  my-session  ").unwrap();
    assert_eq!(name.as_str(), "my-session");
}

#[test]
fn test_reject_invalid() {
    assert!(SessionName::parse("").is_err());
    assert!(SessionName::parse("123-invalid").is_err());
    assert!(SessionName::parse("invalid@name").is_err());
}
```

## File Locations

### Source of Truth
- `crates/zjj-core/src/domain/identifiers.rs` - Canonical implementation

### Re-Exports (to be updated)
- `crates/zjj-core/src/types.rs` - Re-export domain version
- `crates/zjj-core/src/cli_contracts/domain_types.rs` - Re-export domain version
- `crates/zjj-core/src/output/domain_types.rs` - Re-export domain version

## Error Handling

```rust
use zjj_core::domain::{SessionName, IdError};

fn parse_name(input: &str) -> anyhow::Result<SessionName> {
    SessionName::parse(input)
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))
}

// Or use ? directly
fn parse_name_direct(input: &str) -> Result<SessionName, IdError> {
    SessionName::parse(input)
}
```

## Benefits

1. **Security** - No validation bypasses
2. **Consistency** - Single source of truth
3. **Correctness** - Trim-then-validate prevents edge cases
4. **Maintainability** - One implementation to update
5. **DDD Compliance** - Domain owns validation logic

## Questions?

See detailed documentation:
- `SESSION_NAME_PHASE1_REPORT.md` - Full analysis and plan
- `SESSION_NAME_CONSOLIDATION.md` - Migration guide

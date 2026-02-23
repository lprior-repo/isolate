# Migration Guide - Quick Reference

## TL;DR - What Changed

The ZJJ codebase was refactored using Domain-Driven Design (DDD) principles. The main changes:

1. **Single identifier type** instead of 5+ duplicate implementations
2. **Consistent `parse()` API** instead of mixed `new()` methods
3. **Unified error types** (`IdentifierError`) instead of module-specific errors
4. **MAX_LENGTH changed**: SessionName 64 → 63 characters

## Quick Migration

### Step 1: Update Constructor Calls (5 minutes)

```bash
# Find all usages
grep -r "SessionName::new\|BeadId::new" crates/

# Replace with parse()
```

**Before:**
```rust
let name = SessionName::new("my-session")?;
let bead = BeadId::new("bd-abc123")?;
```

**After:**
```rust
let name = SessionName::parse("my-session")?;
let bead = BeadId::parse("bd-abc123")?;
```

### Step 2: Update Imports (2 minutes)

```bash
# Find old imports
grep -r "use.*output::domain_types\|use.*coordination::domain_types" crates/
```

**Before:**
```rust
use zjj_core::output::domain_types::SessionName;
use zjj_core::coordination::domain_types::BeadId;
```

**After:**
```rust
use zjj_core::domain::{SessionName, BeadId};
```

### Step 3: Update Error Handling (5 minutes)

**Before:**
```rust
use zjj_core::types::SessionNameError;

match result {
    Err(SessionNameError::InvalidFormat { .. }) => { /* ... */ }
}
```

**After:**
```rust
use zjj_core::domain::IdentifierError;

match result {
    Err(IdentifierError::InvalidFormat { .. }) => { /* ... */ }
}
```

### Step 4: Update Tests (5 minutes)

**Before:**
```rust
let long_name = "a".repeat(65);  // MAX_LENGTH was 64
assert!(SessionName::new(&long_name).is_err());
```

**After:**
```rust
let long_name = "a".repeat(64);  // MAX_LENGTH is now 63
assert!(SessionName::parse(&long_name).is_err());
```

### Step 5: Test Everything

```bash
moon run :test
moon run :clippy
```

## Common Issues

### Issue: Type mismatch

**Error:**
```
expected `&SessionName`, found `&String`
```

**Fix:**
```rust
// Before
let name = "my-session".to_string();
create_session(&name);

// After
let name = SessionName::parse("my-session")?;
create_session(&name);
```

### Issue: Error conversion

**Error:**
```
the trait `From<IdentifierError>` is not implemented for `anyhow::Error`
```

**Fix:**
```rust
// Add context
let name = SessionName::parse(raw_name)
    .context("invalid session name")?;
```

## Backward Compatibility

**Good news**: The `new()` method still works via a compatibility shim!

```rust
// This still works
let name = SessionName::new("my-session")?;

// But please migrate to this
let name = SessionName::parse("my-session")?;
```

## Validation Rules

| Type | Max Length | Pattern |
|------|------------|---------|
| `SessionName` | 63 | `[a-zA-Z][a-zA-Z0-9_-]{0,62}` |
| `AgentId` | 128 | Alphanumeric + `-_.:` |
| `WorkspaceName` | 255 | No path separators |
| `TaskId` / `BeadId` | ∞ | `bd-[a-fA-F0-9]+` |

## Getting Help

- Full guide: `/home/lewis/src/zjj/MIGRATION_GUIDE.md`
- Domain types: `/home/lewis/src/zjj/crates/zjj-core/src/domain/`
- Examples: Check test files for usage patterns

## Checklist

- [ ] Replace `::new()` with `::parse()`
- [ ] Update imports from old locations
- [ ] Update error type imports
- [ ] Update test length expectations (64→63)
- [ ] Run `moon run :test`
- [ ] Run `moon run :clippy`
- [ ] Manually test CLI commands

---

**Estimated Migration Time**: 30-60 minutes for a typical module

**Rollback**: If issues occur, the `new()` method still works, so you can revert gradually.

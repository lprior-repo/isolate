# SessionName - Quick Reference

## Single Source of Truth

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/identifiers.rs`

```rust
pub struct SessionName(String);
```

## Construction Methods

### 1. `parse()` - Canonical Method

```rust
use zjj_core::domain::SessionName;

let name = SessionName::parse("my-session")?;
```

**Returns**: `Result<SessionName, IdError>`

**Use for**: New code, consistency with other domain types

---

### 2. `new()` - Compatibility Shim

```rust
use zjj_core::types::SessionName; // or use crate::types::SessionName

let name = SessionName::new("my-session")?;
```

**Returns**: `Result<SessionName, Error>`

**Use for**: Existing code, backward compatibility

---

### 3. `from_str()` - FromStr Trait

```rust
use std::str::FromStr;
use zjj_core::domain::SessionName;

let name: SessionName = "my-session".parse()?;
```

**Returns**: `Result<SessionName, IdError>`

**Use for**: String parsing, generic contexts

---

## Validation Rules

| Rule | Value |
|------|-------|
| **MIN_LENGTH** | 1 character |
| **MAX_LENGTH** | 63 characters |
| **First Char** | Letter (a-z, A-Z) |
| **Allowed Chars** | Alphanumeric, `-`, `_` |
| **Pattern** | `^[a-zA-Z][a-zA-Z0-9_-]{0,62}$` |

---

## Common Operations

```rust
use zjj_core::domain::SessionName;

// Create
let name = SessionName::parse("my-session")?;

// Access as string
let s: &str = name.as_str();

// Convert to String
let owned: String = name.into_string();

// Display
println!("{}", name);  // Implements Display

// Clone
let name2 = name.clone();

// Compare
assert_eq!(name, name2);

// Hash (can be used as HashMap key)
use std::collections::HashMap;
let mut map = HashMap::new();
map.insert(name, "value");
```

---

## Examples

### Valid Names

```rust
✅ "my-session"
✅ "my_session"
✅ "MyFeature123"
✅ "a"
✅ "feature-auth"
```

### Invalid Names

```rust
❌ ""                    // Empty
❌ "123-start"           // Starts with number
❌ "my session"          // Contains space
❌ "my.session"          // Contains dot
❌ "a".repeat(64)        // Too long (max 63)
```

---

## Migration Guide

### Old Code (Still Works)

```rust
use zjj_core::types::SessionName;

let name = SessionName::new("my-session")?;
```

### New Code (Preferred)

```rust
use zjj_core::domain::SessionName;

let name = SessionName::parse("my-session")?;
```

**Both create the same type** - the `types` module re-exports from `domain`.

---

## Error Handling

```rust
use zjj_core::domain::{SessionName, IdError};

match SessionName::parse("invalid name") {
    Ok(name) => println!("Valid: {}", name),
    Err(IdError::Empty) => eprintln!("Name cannot be empty"),
    Err(IdError::InvalidCharacters(msg)) => eprintln!("{}", msg),
    Err(IdError::TooLong(name, len)) => eprintln!("{} too long (max 63)", name),
    Err(e) => eprintln!("Other error: {}", e),
}
```

---

## Type Compatibility

All three methods create the **SAME TYPE**:

```rust
use zjj_core::domain::SessionName;
use std::str::FromStr;

let name1 = SessionName::parse("test")?;
let name2: SessionName = "test".parse()?;
let name3 = zjj_core::types::SessionName::new("test")?;

assert_eq!(name1, name2);  // ✅ Same type
assert_eq!(name2, name3);  // ✅ Same type
```

---

## Module Re-exports

```rust
// Canonical location
use zjj_core::domain::SessionName;

// Re-exported in types (backward compat)
use zjj_core::types::SessionName;

// Both are the SAME type
```

---

## Constants

```rust
use zjj_core::domain::SessionName;

assert_eq!(SessionName::MAX_LENGTH, 63);
```

---

## Serde Support

```rust
use serde::{Serialize, Deserialize};
use zjj_core::domain::SessionName;

#[derive(Serialize, Deserialize)]
struct MyStruct {
    #[serde(rename = "session")]
    name: SessionName,
}

// Automatically validates on deserialization
```

---

## Quick Test

```bash
# Run SessionName tests
cargo test --lib SessionName

# Run types tests
cargo test --lib types_tests

# Run domain tests
cargo test --lib domain::identifiers::tests
```

---

**Remember**: Single source of truth is `domain::SessionName`. Everything else is a convenience re-export.

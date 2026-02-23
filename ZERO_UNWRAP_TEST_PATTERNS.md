# Zero-Unwrap Test Patterns - Reference Guide

## Overview
This document provides reference patterns for writing tests without `unwrap()`, `expect()`, or `panic!()` while maintaining ergonomics and test clarity.

## Core Principles

1. **Explicit Error Handling**: All `Result` types must be explicitly handled
2. **Contextual Error Messages**: Panic messages must explain what operation failed
3. **No Hidden Panics**: Even test setup must use proper error handling
4. **Ergonomic Macros**: Use helper macros to reduce boilerplate

## Pattern Catalog

### Pattern 1: Simple Validation Test
**Use when**: Testing that a valid input produces an expected output

```rust
// ❌ BEFORE - Uses unwrap()
#[test]
fn test_session_name_display() {
    let name = SessionName::parse("test-session").unwrap();
    assert_eq!(name.to_string(), "test-session");
}

// ✅ AFTER - Proper error handling
#[test]
fn test_session_name_display() {
    match SessionName::parse("test-session") {
        Ok(name) => {
            assert_eq!(name.to_string(), "test-session");
            assert_eq!(name.as_str(), "test-session");
        }
        Err(e) => panic!("Failed to parse valid session name: {e}"),
    }
}
```

### Pattern 2: Helper Macro for Repeated Operations
**Use when**: Multiple similar Result unwraps in a test

```rust
// Define macro at top of test module
macro_rules! unwrap_ok {
    ($expr:expr, $msg:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

// Use in tests
#[test]
fn test_add_entry() {
    let queue = PureQueue::new();
    let result = queue.add("ws-test", 5, None);
    assert!(result.is_ok());
    let queue = unwrap_ok!(result, "Failed to add entry");
    assert_eq!(queue.len(), 1);
}

// For chained operations
#[test]
fn test_queue_operations() {
    let queue = unwrap_ok!(
        PureQueue::new().add("ws-a", 5, None),
        "Failed to add ws-a"
    );
    let queue = unwrap_ok!(
        queue.add("ws-b", 3, None),
        "Failed to add ws-b"
    );
    assert_eq!(queue.len(), 2);
}
```

### Pattern 3: Enum Parsing Tests
**Use when**: Testing `FromStr` implementations for enums

```rust
// ❌ BEFORE
#[test]
fn test_session_status_from_str() {
    assert_eq!(SessionStatus::from_str("active").unwrap(), SessionStatus::Active);
    assert_eq!(SessionStatus::from_str("paused").unwrap(), SessionStatus::Paused);
}

// ✅ AFTER - Each parse is independent and explicit
#[test]
fn test_session_status_from_str() {
    match SessionStatus::from_str("active") {
        Ok(status) => assert_eq!(status, SessionStatus::Active),
        Err(e) => panic!("Failed to parse 'active': {e}"),
    }
    match SessionStatus::from_str("paused") {
        Ok(status) => assert_eq!(status, SessionStatus::Paused),
        Err(e) => panic!("Failed to parse 'paused': {e}"),
    }
}
```

### Pattern 4: Tuple Matching
**Use when**: Multiple Results must all succeed

```rust
// ❌ BEFORE
#[test]
fn test_bead_id_is_task_id() {
    let bead: BeadId = BeadId::parse("bd-test123").unwrap();
    let task: TaskId = TaskId::parse("bd-test123").unwrap();
    assert_eq!(bead.as_str(), task.as_str());
}

// ✅ AFTER - Match both results together
#[test]
fn test_bead_id_is_task_id() {
    match (BeadId::parse("bd-test123"), TaskId::parse("bd-test123")) {
        (Ok(bead), Ok(task)) => {
            assert_eq!(bead.as_str(), task.as_str());
        }
        (Err(e), _) => panic!("Failed to parse bead ID: {e}"),
        (_, Err(e)) => panic!("Failed to parse task ID: {e}"),
    }
}
```

### Pattern 5: Contract Test Setup
**Use when**: Building complex test input structures

```rust
// ❌ BEFORE
#[test]
fn test_create_session_contract_preconditions() {
    let input = CreateSessionInput {
        name: SessionName::try_from("valid-name").unwrap(),
        parent: None,
        branch: None,
        dedupe_key: None,
    };
    assert!(SessionContracts::preconditions(&input).is_ok());
}

// ✅ AFTER - Use helper macro
#[test]
fn test_create_session_contract_preconditions() {
    macro_rules! unwrap_ok {
        ($expr:expr, $msg:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => panic!("{}: {:?}", $msg, e),
            }
        };
    }

    let input = CreateSessionInput {
        name: unwrap_ok!(SessionName::try_from("valid-name"), "Failed to create SessionName"),
        parent: None,
        branch: None,
        dedupe_key: None,
    };
    assert!(SessionContracts::preconditions(&input).is_ok());
}
```

### Pattern 6: Nested Value Access
**Use when**: Need to access Option inside Result

```rust
// ❌ BEFORE
#[test]
fn test_queue_claim() {
    let result = queue.claim_next("agent1").unwrap();
    let (queue, workspace) = result;
    assert!(queue.get("ws-high").unwrap().is_claimed());
}

// ✅ AFTER - Nested match with context
#[test]
fn test_queue_claim() {
    match queue.claim_next("agent1") {
        Ok((queue, workspace)) => {
            assert_eq!(workspace, "ws-high");
            match queue.get("ws-high") {
                Some(entry) => assert!(entry.is_claimed()),
                None => panic!("ws-high entry not found"),
            }
        }
        Err(e) => panic!("Failed to claim: {e}"),
    }
}
```

### Pattern 7: Collection Operations
**Use when**: Building test data with Results

```rust
// ❌ BEFORE
#[test]
fn test_list_sessions() {
    let result = SessionListResult {
        sessions: vec![
            SessionResult {
                id: "1".to_string(),
                name: SessionName::try_from("s1").unwrap(),
                status: SessionStatus::Active,
                workspace_path: PathBuf::from("/tmp/1"),
            },
            SessionResult {
                id: "2".to_string(),
                name: SessionName::try_from("s2").unwrap(),
                status: SessionStatus::Active,
                workspace_path: PathBuf::from("/tmp/2"),
            },
        ],
        current: Some(SessionName::try_from("s1").unwrap()),
    };
    assert!(SessionContracts::postconditions(&input, &result).is_ok());
}

// ✅ AFTER - Use macro for each field
#[test]
fn test_list_sessions() {
    macro_rules! unwrap_ok {
        ($expr:expr, $msg:expr) => {
            match $expr {
                Ok(v) => v,
                Err(e) => panic!("{}: {:?}", $msg, e),
            }
        };
    }

    let result = SessionListResult {
        sessions: vec![
            SessionResult {
                id: "1".to_string(),
                name: unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName"),
                status: SessionStatus::Active,
                workspace_path: PathBuf::from("/tmp/1"),
            },
            SessionResult {
                id: "2".to_string(),
                name: unwrap_ok!(SessionName::try_from("s2"), "Failed to create SessionName"),
                status: SessionStatus::Active,
                workspace_path: PathBuf::from("/tmp/2"),
            },
        ],
        current: Some(unwrap_ok!(SessionName::try_from("s1"), "Failed to create SessionName")),
    };
    assert!(SessionContracts::postconditions(&input, &result).is_ok());
}
```

## Best Practices

### DO ✅
- Always provide context in panic messages
- Use helper macros for repeated operations
- Test both success and error paths explicitly
- Keep test assertions focused and readable
- Use `match` for complex Result handling

### DON'T ❌
- Use `unwrap()` or `expect()` in test code
- Use `panic!()` directly without context
- Hide error handling in helper functions without good reason
- Create overly complex macros that obscure test intent
- Use `.ok().unwrap()` as a workaround

## Testing Error Paths

Zero-unwrap patterns make testing error paths more natural:

```rust
#[test]
fn test_invalid_input() {
    let result = SessionName::parse("");
    assert!(result.is_err());
    assert!(matches!(result, Err(IdError::Empty)));
}

#[test]
fn test_error_message_quality() {
    match SessionName::parse("invalid@name") {
        Ok(_) => panic!("Should have failed to parse invalid name"),
        Err(IdError::InvalidCharacters(msg)) => {
            assert!(msg.contains("invalid characters"));
        }
        Err(e) => panic!("Wrong error type: {e}"),
    }
}
```

## Property-Based Testing with Proptest

Zero-unwrap patterns work well with proptest:

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_session_name_never_empty(s in "[a-zA-Z][a-zA-Z0-9_-]{0,62}") {
            match SessionName::parse(&s) {
                Ok(name) => assert_eq!(name.as_str(), s),
                Err(e) => panic!("Valid name failed to parse: {s} - {e}"),
            }
        }
    }
}
```

## Performance Considerations

- Match statements are zero-cost at runtime
- Compiler optimizes away the Err branches in release builds
- Test ergonomics >> micro-optimizations in test code
- Error messages only constructed on test failure

## Migration Checklist

When migrating tests to zero-unwrap:

- [ ] Identify all `unwrap()` calls in test code
- [ ] Choose appropriate pattern from this catalog
- [ ] Add helper macros if needed
- [ ] Ensure error messages provide context
- [ ] Test both success and error paths
- [ ] Verify lints pass: `#![deny(clippy::unwrap_used)]`
- [ ] Run tests to ensure behavior is unchanged

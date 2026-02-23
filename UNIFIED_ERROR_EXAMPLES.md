# Unified Error Types - Usage Examples

## Overview

This document provides practical examples of using the unified `IdentifierError` type across the codebase.

## Basic Usage

### Parsing Identifiers

```rust
use zjj_core::domain::{SessionName, IdentifierError};

fn parse_session_name(input: &str) -> Result<SessionName, IdentifierError> {
    SessionName::parse(input)
}

// Example usage
match parse_session_name("my-session") {
    Ok(name) => println!("Valid session: {}", name),
    Err(IdentifierError::Empty) => eprintln!("Session name cannot be empty"),
    Err(IdentifierError::TooLong { max, actual }) => {
        eprintln!("Session name too long: {} characters (max {})", actual, max)
    }
    Err(IdentifierError::InvalidStart { .. }) => {
        eprintln!("Session name must start with a letter")
    }
    Err(IdentifierError::InvalidCharacters { details }) => {
        eprintln!("Invalid characters: {}", details)
    }
    Err(e) => eprintln!("Invalid session name: {}", e),
}
```

### Creating Errors Programmatically

```rust
use zjj_core::domain::identifiers::IdentifierError;

fn validate_custom_identifier(id: &str) -> Result<(), IdentifierError> {
    if id.is_empty() {
        return Err(IdentifierError::empty());
    }

    if id.len() > 100 {
        return Err(IdentifierError::too_long(100, id.len()));
    }

    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(IdentifierError::invalid_characters(
            "identifier must contain only letters, numbers, and hyphens"
        ));
    }

    if !id.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
        return Err(IdentifierError::invalid_start('a'));
    }

    Ok(())
}
```

## Error Handling Patterns

### Pattern 1: Match on Specific Variants

```rust
use zjj_core::domain::{TaskId, IdentifierError};

fn handle_task_id(input: &str) -> Result<String, IdentifierError> {
    match TaskId::parse(input) {
        Ok(task_id) => Ok(format!("Valid task: {}", task_id)),
        Err(IdentifierError::Empty) => {
            Err(IdentifierError::invalid_format("task ID cannot be empty"))
        }
        Err(IdentifierError::InvalidPrefix { prefix, .. }) => {
            Err(IdentifierError::invalid_format(
                format!("task ID must start with '{}'", prefix)
            ))
        }
        Err(IdentifierError::InvalidHex { .. }) => {
            Err(IdentifierError::invalid_format(
                "task ID must be in format: bd-{hex}"
            ))
        }
        Err(e) => Err(e),
    }
}
```

### Pattern 2: Map Errors to Domain-Specific Context

```rust
use zjj_core::domain::{SessionName, WorkspaceName, IdentifierError};

fn create_session(name: &str, workspace: &str) -> Result<(SessionName, WorkspaceName), String> {
    let session_name = SessionName::parse(name)
        .map_err(|e| format!("Invalid session name: {}", e))?;

    let workspace_name = WorkspaceName::parse(workspace)
        .map_err(|e| format!("Invalid workspace name: {}", e))?;

    Ok((session_name, workspace_name))
}
```

### Pattern 3: Collect Multiple Validation Errors

```rust
use zjj_core::domain::{SessionName, IdentifierError};
use std::collections::HashMap;

fn validate_multiple_names(names: &[&str]) -> HashMap<&str, IdentifierError> {
    names
        .iter()
        .filter_map(|&name| {
            match SessionName::parse(name) {
                Ok(_) => None,
                Err(e) => Some((name, e)),
            }
        })
        .collect()
}

// Example:
let names = vec!["valid", "", "123invalid", "a"];
let errors = validate_multiple_names(&names);
// errors contains: {"": Empty, "123invalid": InvalidStart}
```

## Module-Specific Error Aliases

### Using Type Aliases for Clarity

```rust
use zjj_core::domain::{
    SessionName, SessionNameError,
    AgentId, AgentIdError,
    TaskId, TaskIdError,
};

// Each function has a clear error type
fn parse_session(input: &str) -> Result<SessionName, SessionNameError> {
    SessionName::parse(input)
}

fn parse_agent(input: &str) -> Result<AgentId, AgentIdError> {
    AgentId::parse(input)
}

fn parse_task(input: &str) -> Result<TaskId, TaskIdError> {
    TaskId::parse(input)
}

// All these error types are actually IdentifierError underneath
fn handle_any_identifier_error<E: Into<IdentifierError>>(error: E) {
    let id_error: IdentifierError = error.into();
    println!("Error: {}", id_error);
}
```

## Backward Compatibility

### Legacy Code Using IdError

```rust
use zjj_core::domain::identifiers::IdError; // Still works!

fn old_api(input: &str) -> Result<SessionName, IdError> {
    SessionName::parse(input)
}

// New code can still use the old API
fn new_code() {
    match old_api("my-session") {
        Ok(name) => println!("{}", name),
        Err(IdError::Empty) => println!("Empty!"),
        Err(e) => println!("Error: {}", e),
    }
}
```

## Advanced Patterns

### Custom Error Types Wrapping IdentifierError

```rust
use thiserror::Error;
use zjj_core::domain::{SessionName, IdentifierError};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("invalid session name: {0}")]
    InvalidName(IdentifierError),

    #[error("session not found: {0}")]
    NotFound(String),

    #[error("session already exists: {0}")]
    AlreadyExists(String),
}

fn create_session(name: &str) -> Result<SessionName, SessionError> {
    SessionName::parse(name)
        .map_err(SessionError::InvalidName)
}

// Usage
match create_session("my-session") {
    Ok(name) => println!("Created: {}", name),
    Err(SessionError::InvalidName(IdentifierError::Empty)) => {
        println!("Name cannot be empty")
    }
    Err(SessionError::AlreadyExists(name)) => {
        println!("Session '{}' already exists", name)
    }
    Err(e) => println!("Error: {}", e),
}
```

### Validation with Context

```rust
use zjj_core::domain::{SessionName, IdentifierError};

struct ValidationError {
    field: String,
    error: IdentifierError,
}

fn validate_with_context(input: &str) -> Result<SessionName, ValidationError> {
    SessionName::parse(input).map_err(|e| ValidationError {
        field: "session_name".to_string(),
        error: e,
    })
}

// Usage
match validate_with_context("123invalid") {
    Ok(name) => println!("Valid: {}", name),
    Err(ValidationError { field, error }) => {
        eprintln!("Validation failed for field '{}': {}", field, error);
    }
}
```

### Combinator Chains

```rust
use zjj_core::domain::{SessionName, WorkspaceName, AgentId, IdentifierError};

fn validate_all(
    session: &str,
    workspace: &str,
    agent: &str,
) -> Result<(SessionName, WorkspaceName, AgentId), IdentifierError> {
    let session_name = SessionName::parse(session)?;
    let workspace_name = WorkspaceName::parse(workspace)?;
    let agent_id = AgentId::parse(agent)?;

    Ok((session_name, workspace_name, agent_id))
}

// Or with more detailed error context
fn validate_all_detailed(
    session: &str,
    workspace: &str,
    agent: &str,
) -> Result<(), String> {
    SessionName::parse(session)
        .map_err(|e| format!("Invalid session name: {}", e))?;

    WorkspaceName::parse(workspace)
        .map_err(|e| format!("Invalid workspace name: {}", e))?;

    AgentId::parse(agent)
        .map_err(|e| format!("Invalid agent ID: {}", e))?;

    Ok(())
}
```

## Testing with IdentifierError

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zjj_core::domain::{SessionName, IdentifierError};

    #[test]
    fn test_empty_name() {
        let result = SessionName::parse("");
        assert!(matches!(result, Err(IdentifierError::Empty)));
    }

    #[test]
    fn test_too_long() {
        let long_name = "a".repeat(100);
        let result = SessionName::parse(&long_name);
        assert!(matches!(result, Err(IdentifierError::TooLong { .. })));
    }

    #[test]
    fn test_invalid_start() {
        let result = SessionName::parse("123session");
        assert!(matches!(result, Err(IdentifierError::InvalidStart { .. })));
    }

    #[test]
    fn test_valid_name() {
        let result = SessionName::parse("valid-session-123");
        assert!(result.is_ok());
    }
}
```

### Property-Based Testing

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    use zjj_core::domain::{SessionName, IdentifierError};

    proptest! {
        #[test]
        fn prop_empty_always_fails(s in "") {
            let result = SessionName::parse(&s);
            assert!(matches!(result, Err(IdentifierError::Empty)));
        }

        #[test]
        fn prop_too_long_fails(max_len in 64usize..1000) {
            let long_name = "a".repeat(max_len);
            let result = SessionName::parse(&long_name);
            assert!(matches!(result, Err(IdentifierError::TooLong { .. })));
        }

        #[test]
        fn prop_valid_alphanumeric_passes(
            prefix in "[a-zA-Z]",
            rest in "[a-zA-Z0-9_-]{0,62}"
        ) {
            let name = format!("{}{}", prefix, rest);
            let result = SessionName::parse(&name);
            prop_assert!(result.is_ok());
        }
    }
}
```

## Error Display and User Messages

### Formatting Errors for Users

```rust
use zjj_core::domain::IdentifierError;

fn user_friendly_error(error: IdentifierError) -> String {
    match error {
        IdentifierError::Empty => {
            "The name cannot be empty. Please provide a valid name.".to_string()
        }
        IdentifierError::TooLong { max, actual } => {
            format!(
                "The name is too long ({} characters). Maximum allowed is {} characters.",
                actual, max
            )
        }
        IdentifierError::InvalidStart { .. } => {
            "The name must start with a letter (a-z or A-Z).".to_string()
        }
        IdentifierError::InvalidCharacters { details } => {
            format!("Invalid characters: {}", details)
        }
        IdentifierError::InvalidPrefix { prefix, .. } => {
            format!("The identifier must start with '{}'", prefix)
        }
        IdentifierError::InvalidFormat { details } => {
            format!("Invalid format: {}", details)
        }
        IdentifierError::NotAbsolutePath { .. } => {
            "The path must be absolute (starting with / on Unix or C:\\ on Windows)".to_string()
        }
        IdentifierError::NullBytesInPath => {
            "The path cannot contain null bytes".to_string()
        }
        IdentifierError::NotAscii { .. } => {
            "The identifier must contain only ASCII characters".to_string()
        }
        IdentifierError::ContainsPathSeparators => {
            "The identifier cannot contain path separators (/ or \\)".to_string()
        }
        IdentifierError::InvalidHex { .. } => {
            "The identifier must be in hexadecimal format (0-9, a-f)".to_string()
        }
    }
}

// Usage
match SessionName::parse("123invalid") {
    Ok(_) => println!("Valid!"),
    Err(e) => println!("{}", user_friendly_error(e)),
}
// Output: "The name must start with a letter (a-z or A-Z)."
```

## Summary

The unified `IdentifierError` type provides:

1. **Type Safety**: Compile-time guarantees about error handling
2. **Clarity**: Specific error variants make intent clear
3. **Flexibility**: Helper methods and combinators for easy error creation
4. **Compatibility**: Backward compatible with legacy `IdError`
5. **Testability**: Easy to test and mock errors

Use the pattern that best fits your code's needs:
- Simple cases: Direct `IdentifierError`
- Semantic clarity: Module-specific aliases like `SessionNameError`
- Backward compatibility: Continue using `IdError`

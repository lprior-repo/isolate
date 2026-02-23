# Handler Integration Examples

This document shows practical examples of how to integrate the new domain types into CLI handlers.

## Pattern: Parse at Boundary

The key principle is to parse user input into domain types at the handler boundary, then use the validated types throughout the core logic.

### Before (String-Based)

```rust
// In handler
pub async fn create_session_handler(name: String) -> Result<()> {
    let input = CreateSessionInput {
        name,  // Unvalidated string!
        parent: None,
        branch: None,
        dedupe_key: None,
    };

    // Validation happens in contract
    SessionContracts::preconditions(&input)?;

    // ... rest of logic
}
```

### After (Domain Types)

```rust
// In handler (BOUNDARY - parse once, validate once)
use zjj_core::cli_contracts::{SessionName, CreateSessionInput, ContractError};
use anyhow::{Context, Result};

pub async fn create_session_handler(name: String) -> Result<()> {
    // Parse and validate at boundary
    let session_name = SessionName::try_from(name.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))
        .context("Session name must start with a letter and contain only alphanumeric, dash, or underscore")?;

    let input = CreateSessionInput {
        name: session_name,  // Already validated!
        parent: None,
        branch: None,
        dedupe_key: None,
    };

    // No validation needed in contract - already validated!
    SessionContracts::preconditions(&input)
        .context("Session contract preconditions failed")?;

    // ... rest of logic uses validated types
}
```

## Real-World Examples

### Example 1: Session Creation with Parent

```rust
use zjj_core::cli_contracts::{SessionName, NonEmptyString, CreateSessionInput};
use anyhow::Result;

pub async fn create_stacked_session_handler(
    name: String,
    parent: String,
    branch: Option<String>,
) -> Result<()> {
    // Parse all inputs at boundary
    let session_name = SessionName::try_from(name.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid session name '{name}': {e}"))?;

    let parent_name = SessionName::try_from(parent.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid parent name '{parent}': {e}"))?;

    let branch_name = branch
        .map(|b| NonEmptyString::try_from(b.as_str()))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid branch name: {e}"))?;

    let input = CreateSessionInput {
        name: session_name,
        parent: Some(parent_name),
        branch: branch_name,
        dedupe_key: None,
    };

    // Core logic works with validated types
    SessionContracts::preconditions(&input)?;

    // ... database operations, etc.

    Ok(())
}
```

### Example 2: Task Creation with Priority

```rust
use zjj_core::cli_contracts::{TaskPriority, NonEmptyString};
use std::str::FromStr;

pub async fn create_task_handler(
    title: String,
    priority: Option<String>,
) -> Result<()> {
    let task_title = NonEmptyString::try_from(title.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid task title: {e}"))?;

    let task_priority = priority
        .map(|p| TaskPriority::from_str(p.as_str()))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid priority: {e}"))?;

    let input = CreateTaskInput {
        title: task_title,
        priority: task_priority,
        task_type: None,
        labels: vec![],
        description: None,
    };

    TaskContracts::preconditions(&input)?;

    Ok(())
}
```

### Example 3: Queue Operations

```rust
use zjj_core::cli_contracts::{SessionName, Priority, EnqueueInput};

pub async fn enqueue_handler(
    session: String,
    priority: Option<u32>,
) -> Result<()> {
    let session_name = SessionName::try_from(session.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

    let queue_priority = priority
        .map(|p| Priority::try_from(p))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid priority (0-1000): {e}"))?;

    let input = EnqueueInput {
        session: session_name,
        priority: queue_priority,
    };

    QueueContracts::preconditions(&input)?;

    Ok(())
}
```

### Example 4: Status Filtering

```rust
use zjj_core::cli_contracts::{SessionStatus, OutputFormat, ListSessionsInput};
use std::str::FromStr;

pub async fn list_sessions_handler(
    status: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let session_status = status
        .map(|s| SessionStatus::from_str(s.as_str()))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid status (must be: creating, active, paused, completed, failed): {e}"))?;

    let output_format = format
        .map(|f| OutputFormat::from_str(f.as_str()))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid format (must be: text, json, yaml): {e}"))?;

    let input = ListSessionsInput {
        status: session_status,
        include_stacked: false,
    };

    SessionContracts::preconditions(&input)?;

    // ... format output based on output_format

    Ok(())
}
```

### Example 5: Agent Operations

```rust
use zjj_core::cli_contracts::{
    SessionName, AgentType, TimeoutSeconds, SpawnAgentInput
};
use std::str::FromStr;

pub async fn spawn_agent_handler(
    session: String,
    agent_type: String,
    task: String,
    timeout: Option<u64>,
) -> Result<()> {
    let session_name = SessionName::try_from(session.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

    let agent = AgentType::from_str(agent_type.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid agent type (must be: claude, cursor, aider, copilot): {e}"))?;

    let task_description = NonEmptyString::try_from(task.as_str())
        .map_err(|e| anyhow::anyhow!("Task description cannot be empty"))?;

    let agent_timeout = timeout
        .map(|t| TimeoutSeconds::try_from(t))
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid timeout (1-86400 seconds): {e}"))?;

    let input = SpawnAgentInput {
        session: session_name,
        agent_type: agent,
        task: task_description,
        timeout: agent_timeout,
    };

    AgentContracts::preconditions(&input)?;

    Ok(())
}
```

### Example 6: Batch Operations

```rust
use zjj_core::cli_contracts::SessionName;

pub async fn batch_remove_handler(names: Vec<String>) -> Result<()> {
    // Parse all names, collect errors
    let session_names: Vec<SessionName> = names
        .into_iter()
        .map(|n| SessionName::try_from(n.as_str()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|errors| {
            anyhow::anyhow!(
                "Invalid session names: {}",
                errors
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // Now work with validated types
    for session_name in session_names {
        let input = RemoveSessionInput {
            session: session_name.clone(),
            force: ForceMode::NoForce,
        };

        SessionContracts::preconditions(&input)?;

        // ... remove session
    }

    Ok(())
}
```

## Error Handling Pattern

### Helper Function for Consistent Error Messages

```rust
use zjj_core::cli_contracts::ContractError;

/// Convert ContractError to user-friendly message
fn contract_error_to_message(error: ContractError) -> String {
    match error {
        ContractError::InvalidInput { field, reason } => {
            format!("Invalid {field}: {reason}")
        }
        ContractError::PreconditionFailed { name, description } => {
            format!("Precondition '{name}' failed: {description}")
        }
        ContractError::PostconditionFailed { name, description } => {
            format!("Postcondition '{name}' failed: {description}")
        }
        _ => error.to_string(),
    }
}

// Usage in handler
pub async fn create_session_handler(name: String) -> Result<()> {
    let session_name = SessionName::try_from(name.as_str())
        .map_err(|e| {
            anyhow::anyhow!("{}", contract_error_to_message(e))
        })?;

    // ... rest of logic
}
```

## CLI Integration

### Example with clap Arguments

```rust
use clap::Parser;

#[derive(Parser, Debug)]
struct CreateSessionArgs {
    /// Session name (must start with letter, contain only alphanumeric/-/_)
    #[arg(short, long)]
    name: String,

    /// Parent session name
    #[arg(short = 'p', long)]
    parent: Option<String>,

    /// Branch name
    #[arg(short, long)]
    branch: Option<String>,
}

impl From<CreateSessionArgs> for Result<CreateSessionInput> {
    fn from(args: CreateSessionArgs) -> Self {
        let name = SessionName::try_from(args.name.as_str())
            .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))?;

        let parent = args
            .parent
            .map(|p| SessionName::try_from(p.as_str()))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid parent name: {e}"))?;

        let branch = args
            .branch
            .map(|b| NonEmptyString::try_from(b.as_str()))
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid branch name: {e}"))?;

        Ok(CreateSessionInput {
            name,
            parent,
            branch,
            dedupe_key: None,
        })
    }
}

// Usage
pub async fn handle_create_session(args: CreateSessionArgs) -> Result<()> {
    let input: CreateSessionInput = args.into()?;  // Parses and validates

    SessionContracts::preconditions(&input)?;

    Ok(())
}
```

## Testing Handlers

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zjj_core::cli_contracts::SessionName;

    #[test]
    fn test_create_session_handler_valid_input() {
        let name = "valid-session".to_string();
        let result = create_session_handler(name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_session_handler_invalid_name() {
        let name = "123-invalid".to_string();  // Starts with number
        let result = create_session_handler(name);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid session name"));
    }

    #[test]
    fn test_create_session_handler_empty_name() {
        let name = "".to_string();
        let result = create_session_handler(name);
        assert!(result.is_err());
    }
}
```

## Benefits Demonstrated

### 1. Single Validation Point
```rust
// Validate once at boundary
let session_name = SessionName::try_from(name)?;
// Use everywhere without re-validating
db.get_session(&session_name)?;
queue.enqueue(&session_name)?;
```

### 2. Compile-Time Safety
```rust
// Compiler prevents invalid status
let status = SessionStatus::from_str("invalid")?;  // Compile error handled
let status: SessionStatus = SessionStatus::Active;  // Always valid
```

### 3. Self-Documenting Code
```rust
// Clear intent from types
fn process_task(task: TaskId, priority: TaskPriority) { ... }
// vs
fn process_task(task: String, priority: String) { ... }
```

### 4. Better Error Messages
```rust
// Domain types provide context
SessionName::try_from("1invalid")?;
// Error: "Invalid name: must start with a letter (a-z, A-Z)"
```

## Summary

The pattern is simple:
1. **Parse at boundary** - Convert raw strings to domain types immediately
2. **Validate once** - Domain types validate in constructors
3. **Use validated types** - Core logic works with safe types
4. **Handle errors gracefully** - Convert ContractError to user messages
5. **Never escape types** - Keep validation at the boundary

This approach makes handlers simpler, core logic safer, and error messages better.

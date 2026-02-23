# Bounded Types Quick Reference

## ActionVerb

Validates action verbs against known operations with extensibility for custom values.

### Known Verbs
```rust
// Core operations
ActionVerb::new("run")        // Run a command
ActionVerb::new("execute")    // Execute a task
ActionVerb::new("create")     // Create a resource
ActionVerb::new("delete")     // Delete a resource
ActionVerb::new("update")     // Update a resource

// Git operations
ActionVerb::new("merge")      // Merge resources
ActionVerb::new("rebase")     // Rebase changes
ActionVerb::new("sync")       // Sync with remote

// Session operations
ActionVerb::new("fix")        // Fix an issue
ActionVerb::new("check")      // Check status
ActionVerb::new("focus")      // Focus on a target
ActionVerb::new("attach")     // Attach to a session
ActionVerb::new("switch-tab") // Switch zellij tabs
ActionVerb::new("remove")     // Remove a resource

// Queue operations
ActionVerb::new("process")    // Process a queue entry

// Discovery
ActionVerb::new("discover")   // Discover resources
ActionVerb::new("would_fix")  // Would fix (dry run)
```

### Custom Verbs
```rust
// Must be lowercase alphanumeric with hyphens
ActionVerb::new("custom-verb")?;          // ✓ Valid
ActionVerb::new("deploy-to-prod")?;       // ✓ Valid
ActionVerb::new("custom")?;               // ✓ Valid

// Invalid examples
ActionVerb::new("Run")?;                  // ✗ Uppercase
ActionVerb::new("run@verb")?;             // ✗ Special chars
ActionVerb::new("123verb")?;              // ✗ Starts with number
ActionVerb::new("")?;                     // ✗ Empty
```

## ActionTarget

Validates action targets with length constraints.

### Rules
- Must be non-empty after trimming whitespace
- Maximum 1000 characters
- Value is trimmed before storage

### Examples
```rust
ActionTarget::new("session-1")?;          // ✓ Valid
ActionTarget::new("/path/to/workspace")?; // ✓ Valid
ActionTarget::new("  target  ")?;         // ✓ Trimmed to "target"
ActionTarget::new("")?;                   // ✗ Empty
ActionTarget::new("   ")?;                // ✗ Whitespace only

// Too long (1001 characters)
let too_long = "a".repeat(1001);
ActionTarget::new(&too_long)?;            // ✗ Exceeds limit
```

## WarningCode

Validates warning codes against known codes with extensibility.

### Known Codes
```rust
ActionVerb::new("CONFIG_NOT_FOUND")?;      // Configuration file not found
ActionVerb::new("CONFIG_INVALID")?;        // Invalid configuration value
ActionVerb::new("SESSION_LIMIT_REACHED")?; // Session limit reached
ActionVerb::new("WORKSPACE_NOT_FOUND")?;   // Workspace path not found
ActionVerb::new("GIT_OPERATION_FAILED")?;  // Git operation failed
ActionVerb::new("MERGE_CONFLICT")?;        // Merge conflict detected
ActionVerb::new("QUEUE_ENTRY_BLOCKED")?;   // Queue entry blocked
ActionVerb::new("AGENT_UNAVAILABLE")?;     // Agent not available
```

### Custom Codes
```rust
// Must be alphanumeric with underscores, start with letter
WarningCode::new("W001")?;           // ✓ Valid
WarningCode::new("E123")?;           // ✓ Valid
WarningCode::new("CUSTOM_CODE")?;    // ✓ Valid

// Invalid examples
WarningCode::new("123")?;            // ✗ Starts with number
WarningCode::new("")?;               // ✗ Empty
WarningCode::new("INVALID-CODE!")?;  // ✗ Special chars
```

## Usage Patterns

### Creating Actions
```rust
use zjj_core::output::{Action, ActionStatus, ActionVerb, ActionTarget};

// In a command handler
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action))?;
    Ok(())
}
```

### Creating Warnings
```rust
use zjj_core::output::{Warning, WarningCode, Message};

// Known code
let warning = Warning::new(
    WarningCode::new("MERGE_CONFLICT")?,
    Message::new("Merge conflict detected")?,
)?;

// Custom code
let warning = Warning::new(
    WarningCode::new("W001")?,
    Message::new("Custom warning")?;
```

### Handling Validation Errors
```rust
// Convert to anyhow::Error
ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?;

// Or provide context
ActionVerb::new(verb)
    .map_err(|e| anyhow::anyhow!("Failed to create action verb '{verb}': {e}"))?;

// Or use the ? operator directly in functions returning Result
fn create_verb(verb: &str) -> Result<ActionVerb, OutputLineError> {
    ActionVerb::new(verb)
}
```

## Testing

### Property-based testing example
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_action_target_never_empty(input in "\\PC*") {
        let result = ActionTarget::new(input);
        if result.is_ok() {
            assert!(!result.unwrap().as_str().is_empty());
        }
    }

    #[test]
    fn test_action_verb_always_lowercase(input in "[a-z0-9-]+") {
        if let Ok(verb) = ActionVerb::new(&input) {
            assert_eq!(verb.as_str(), input.to_lowercase());
        }
    }
}
```

## Migration Checklist

When updating code to use bounded types:

- [ ] Change `ActionVerb::new(...)` to `ActionVerb::new(...)?`
- [ ] Change `ActionTarget::new(...)` to `ActionTarget::new(...)?`
- [ ] Change `WarningCode::new(...)` to `WarningCode::new(...)?`
- [ ] Add error context: `.map_err(|e| anyhow::anyhow!("...: {e}"))?`
- [ ] Update tests to use `.expect("valid")` instead of direct calls
- [ ] Verify serialization still works for JSONL output
- [ ] Check for any hardcoded string literals that should be known variants

## Common Patterns

### Emitting actions with validation
```rust
// Helper function
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

// Usage
emit_action("create", "session-1", ActionStatus::Completed)?;
```

### Testing with known values
```rust
#[test]
fn test_action_creation() {
    let action = Action::new(
        ActionVerb::new("create").expect("valid verb"),
        ActionTarget::new("session-1").expect("valid target"),
        ActionStatus::Completed,
    );
    assert!(matches!(action.status, ActionStatus::Completed));
}
```

### Adding new known verbs
```rust
// 1. Add variant to enum
pub enum ActionVerb {
    // ... existing variants
    Deploy,  // New variant
    Custom(String),
}

// 2. Add match arm in new()
match verb.to_lowercase().as_str() {
    // ... existing matches
    "deploy" => Ok(Self::Deploy),
    custom => { /* ... */ }
}

// 3. Add match arm in as_str()
match self {
    // ... existing matches
    Self::Deploy => "deploy",
    Self::Custom(s) => s.as_str(),
}
```

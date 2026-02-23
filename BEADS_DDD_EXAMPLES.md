# Beads DDD Refactoring - Concrete Examples

This document shows concrete before/after comparisons for common operations.

## Example 1: Creating an Issue

### Before (Legacy)
```rust
use beads::BeadIssue;

// No validation - can create invalid issues
let issue = BeadIssue {
    id: "".to_string(),  // Empty ID - invalid!
    title: "".to_string(),  // Empty title - invalid!
    status: IssueStatus::Closed,
    closed_at: None,  // Closed without timestamp - invalid!
    // ... many more fields
    created_at: Utc::now(),
    updated_at: Utc::now(),
};
// Compiles but represents an invalid state!
```

### After (DDD)
```rust
use beads::{Issue, IssueId, Title};

// Validation at construction - cannot create invalid issues
let issue = Issue::new("", "")?;
// ^ Returns Err(DomainError::EmptyId)

let issue = Issue::new("issue-123", "Fix the bug")?;
// ^ Returns Ok(Issue) - guaranteed to be valid

// Timestamps are automatic
assert_eq!(issue.created_at, issue.updated_at);
```

## Example 2: Closing an Issue

### Before (Legacy)
```rust
// Easy to make mistakes
issue.status = IssueStatus::Closed;
// Forgot: issue.closed_at = Some(Utc::now());
// Now we have an invalid state!

// Requires runtime validation everywhere
fn validate_issue(issue: &BeadIssue) -> Result<(), BeadsError> {
    if issue.status == IssueStatus::Closed && issue.closed_at.is_none() {
        return Err(BeadsError::ValidationFailed(
            "closed_at must be set".into()
        ));
    }
    Ok(())
}
```

### After (DDD)
```rust
// Type system prevents mistakes
issue.close();
// ^ Automatically sets closed_at to Utc::now()

// Invalid state is unrepresentable
issue.state = IssueState::Closed { closed_at: Utc::now() };
// ^ Timestamp is REQUIRED - code won't compile without it!

// No runtime validation needed - impossible to have invalid state
```

## Example 3: Checking if Blocked

### Before (Legacy)
```rust
// Scattered logic, repeated everywhere
fn is_blocked(issue: &BeadIssue) -> bool {
    issue.status == IssueStatus::Blocked
        || issue.blocked_by
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}

// Must remember to use this function everywhere
if is_blocked(&issue) {
    // ...
}
```

### After (DDD)
```rust
// Logic encapsulated in aggregate
if issue.is_blocked() {
    // ...
}

// Clear, self-documenting, consistent
```

## Example 4: Updating Fields

### Before (Legacy)
```rust
// No validation on update
issue.title = "".to_string();  // Empty title - invalid!
issue.id = "invalid id!".to_string();  // Invalid pattern - invalid!

// Validation happens elsewhere (if at all)
```

### After (DDD)
```rust
// Validation on every update
issue.update_title("")?;
// ^ Returns Err(DomainError::EmptyTitle)

issue.update_title("New title")?;
// ^ Returns Ok(())

issue.update_title("  Trimmed  ")?;
// ^ Automatically trimmed to "Trimmed"
```

## Example 5: Working with Labels

### Before (Legacy)
```rust
// Raw vector, no validation
issue.labels = Some(vec![]);
// Can add as many as we want
issue.labels.as_mut().unwrap().push("label".repeat(100));
// Can exceed limits
issue.labels.as_mut().unwrap().push("label");
issue.labels.as_mut().unwrap().push("label");
// ... 100 times

// Validation happens later (if at all)
```

### After (DDD)
```rust
// Validated collection type
issue.set_labels(vec!["label1".to_string(), "label2".to_string()])?;

// Adding labels validates limits
issue.add_label("new-label".to_string())?;
// ^ Returns Err(DomainError::InvalidFilter) if exceeds MAX_COUNT

// Cannot add invalid labels
issue.add_label("a".repeat(100).to_string())?;
// ^ Returns Err(DomainError::InvalidFilter) if exceeds MAX_LABEL_LENGTH
```

## Example 6: Error Handling

### Before (Legacy)
```rust
// Opaque string errors
match result {
    Err(BeadsError::ValidationFailed(msg)) => {
        // What failed? Have to parse the string
        if msg.contains("ID") {
            // Handle ID error
        } else if msg.contains("Title") {
            // Handle title error
        }
        // Fragile - breaks if messages change
    }
    // ...
}
```

### After (DDD)
```rust
// Structured errors - match on variants
match result {
    Err(DomainError::EmptyId) => {
        // Handle empty ID
    }
    Err(DomainError::InvalidIdPattern(msg)) => {
        // Handle invalid pattern with details
    }
    Err(DomainError::TitleTooLong { max, got }) => {
        // Handle with specific values
        eprintln!("Title too long: max {max}, got {got}");
    }
    // Testable, robust, self-documenting
}
```

## Example 7: State Transitions

### Before (Legacy)
```rust
// Can set any state, even invalid transitions
issue.status = IssueStatus::Closed;
issue.closed_at = None;  // Oops!

// Requires validation logic everywhere
fn validate_transition(
    from: IssueStatus,
    to: IssueStatus,
) -> Result<(), BeadsError> {
    match (from, to) {
        // ... complex validation
    }
}
```

### After (DDD)
```rust
// State transitions are explicit and validated
issue.transition_to(IssueState::Closed {
    closed_at: Utc::now()
})?;
// ^ Validates transition, returns Result

// Common transitions have convenience methods
issue.close();  // Automatically sets timestamp
issue.reopen()?;  // Validates that issue was closed
```

## Example 8: Type Safety

### Before (Legacy)
```rust
// Can accidentally pass wrong values
let issue = BeadIssue {
    id: "not-a-title",
    title: "not-an-id",  // Swapped!
    // ...
};

// Can't detect at compile time
```

### After (DDD)
```rust
// Types prevent misuse
let issue = Issue::new(
    "not-a-title",  // This is an ID
    "not-an-id",    // This is a title
)?;
// Both validated according to their specific rules

// Can't swap - different types!
fn set_id(id: IssueId) { /* ... */ }
fn set_title(title: Title) { /* ... */ }

set_id(issue.title);  // Won't compile!
```

## Example 9: Collections

### Before (Legacy)
```rust
// Raw vectors, no validation
pub depends_on: Option<Vec<String>>,

// Can add invalid IDs
issue.depends_on = Some(vec![
    "invalid id!".to_string(),  // Invalid pattern!
    "".to_string(),             // Empty!
]);

// Can exceed limits
issue.depends_on = Some((0..1000).map(|i| i.to_string()).collect());
```

### After (DDD)
```rust
// Validated collection types
pub depends_on: DependsOn,  // Always valid

// Construction validates
issue.set_depends_on(vec![
    "dep-1".to_string(),
    "dep-2".to_string(),
])?;
// ^ Returns Err(DomainError) if any ID is invalid

// Cannot add invalid elements after construction
// (immutable by default, or validation on add)
```

## Example 10: Query Methods

### Before (Legacy)
```rust
// Inconsistent query methods
fn is_open(issue: &BeadIssue) -> bool {
    issue.status == IssueStatus::Open
        || issue.status == IssueStatus::InProgress
}

fn is_closed(issue: &BeadIssue) -> bool {
    issue.status == IssueStatus::Closed
}

// Logic scattered, inconsistent
```

### After (DDD)
```rust
// Consistent query methods on aggregate
issue.is_open();    // Encapsulates logic
issue.is_active();  // Open or InProgress
issue.is_closed();  // Checks state enum
issue.is_blocked(); // Combines state + blocked_by

// Single source of truth, consistent
```

## Benefits Summary

| Aspect | Before | After |
|--------|--------|-------|
| **Compile-time safety** | Runtime errors | Compile errors |
| **Validation** | Scattered, optional | Centralized, required |
| **State consistency** | Easy to create invalid states | Impossible to create invalid states |
| **Error handling** | String parsing | Structured matching |
| **Code clarity** | What does this String hold? | Type tells you |
| **Refactoring** | Compiler can't help | Compiler guides you |
| **Testing** | Need many integration tests | Unit tests per type |

## Conclusion

The DDD refactoring transforms the beads module from a **data-oriented** design with **runtime validation** to a **type-oriented** design with **compile-time guarantees**. This leads to:

1. **Fewer bugs** - Invalid states are unrepresentable
2. **Better error messages** - Structured errors with context
3. **Easier refactoring** - Compiler catches misuse
4. **Self-documenting code** - Types encode domain rules
5. **Confidence** - If it compiles, it's valid

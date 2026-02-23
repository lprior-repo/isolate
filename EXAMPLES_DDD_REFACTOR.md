# DDD Refactoring - Usage Examples

This document shows practical examples of using the refactored domain types in `/crates/zjj-core/src/output/`.

## Table of Contents
- [Creating Output Lines](#creating-output-lines)
- [Working with Issues](#working-with-issues)
- [Creating Plans](#creating-plans)
- [Tracking Actions](#tracking-actions)
- [Queue Management](#queue-management)
- [Error Handling](#error-handling)
- [Migration Examples](#migration-examples)

## Creating Output Lines

### Summary Output

```rust
use zjj_core::output::{Summary, SummaryType, Message};

// Create a basic summary
let summary = Summary::new(
    SummaryType::Info,
    Message::new("Operation completed successfully")?,
)?;

// Add details
let summary = summary.with_details("Processed 15 sessions".to_string());

// Convert to output line
let output_line = zjj_core::output::OutputLine::Summary(summary);
```

### Warning Output

```rust
use zjj_core::output::{Warning, WarningCode, Message};
use std::path::PathBuf;

// Create a warning
let warning = Warning::new(
    WarningCode::new("DEPRECATED_SESSION"),
    Message::new("Session format is deprecated")?,
)?;

// Add context
let warning = warning.with_context(
    "old-session".to_string(),
    PathBuf::from("/workspace"),
);

// Convert to output line
let output_line = zjj_core::output::OutputLine::Warning(warning);
```

## Working with Issues

### Creating Issues

```rust
use zjj_core::output::{
    Issue, IssueId, IssueTitle, IssueKind, IssueSeverity,
};

// Create a standalone issue
let issue = Issue::new(
    IssueId::new("ISSUE-123")?,
    IssueTitle::new("Authentication token expired")?,
    IssueKind::Validation,
    IssueSeverity::Error,
)?;

// Add session context
use zjj_core::types::SessionName;
let session_name = SessionName::new("auth-fix")?;
let issue = issue.with_session(session_name);

// Add suggestion
let issue = issue.with_suggestion("Run 'zjj refresh-tokens' to renew".to_string());

// Convert to output line
let output_line = zjj_core::output::OutputLine::Issue(issue);
```

### Issue Scope

```rust
use zjj_core::output::IssueScope;
use zjj_core::types::SessionName;

// Standalone issue
let scope = IssueScope::Standalone;
assert!(scope.session().is_none());

// Issue in session
let session_name = SessionName::new("my-session")?;
let scope = IssueScope::InSession {
    session: session_name.clone(),
};
assert_eq!(scope.session(), Some(&session_name));
```

## Creating Plans

### Creating a Plan

```rust
use zjj_core::output::{
    Plan, PlanTitle, PlanDescription, ActionStatus,
};

// Create a new plan
let plan = Plan::new(
    PlanTitle::new("Migration Plan")?,
    PlanDescription::new("Migrate sessions to new format")?,
)?;

// Add steps
let plan = plan.with_step(
    "Backup existing sessions".to_string(),
    ActionStatus::Pending,
)?;

let plan = plan.with_step(
    "Run migration script".to_string(),
    ActionStatus::Completed,
)?;

let plan = plan.with_step(
    "Verify migration".to_string(),
    ActionStatus::Pending,
)?;

// Convert to output line
let output_line = zjj_core::output::OutputLine::Plan(plan);
```

## Tracking Actions

### Creating Actions

```rust
use zjj_core::output::{
    Action, ActionVerb, ActionTarget, ActionStatus, ActionResult,
};

// Create a pending action
let action = Action::new(
    ActionVerb::new("Created"),
    ActionTarget::new("session 'auth-fix'"),
    ActionStatus::Pending,
);

// Mark as completed with result
let action = action.with_result("Session created successfully".to_string());

// Convert to output line
let output_line = zjj_core::output::OutputLine::Action(action);
```

### Action Results

```rust
use zjj_core::output::ActionResult;

// Pending action
let result = ActionResult::Pending;
assert!(result.result().is_none());

// Completed action
let result = ActionResult::Completed {
    result: "Success".to_string(),
};
assert_eq!(result.result(), Some("Success"));
```

## Queue Management

### Queue Entries

```rust
use zjj_core::output::{
    QueueEntry, QueueEntryId, QueueEntryStatus,
    BeadAttachment, AgentAssignment,
};
use zjj_core::types::SessionName;

// Create a queue entry
let entry = QueueEntry::new(
    QueueEntryId::new("QUEUE-456")?,
    SessionName::new("feature-auth")?,
    1, // priority
)?;

// Attach a bead
use zjj_core::output::BeadId;
let entry = entry.with_bead(BeadId::new("bead-abc")?);

// Assign an agent
let entry = entry.with_agent("agent-1".to_string());

// Update status
let entry = entry.with_status(QueueEntryStatus::InProgress);

// Convert to output line
let output_line = zjj_core::output::OutputLine::QueueEntry(entry);
```

### Queue Entry State

```rust
use zjj_core::output::{BeadAttachment, BeadId, AgentAssignment};

// Check bead attachment
let attachment = BeadAttachment::Attached {
    bead_id: BeadId::new("bead-123")?,
};
assert!(attachment.bead_id().is_some());
assert!(!attachment.is_none());

// Check agent assignment
let assignment = AgentAssignment::Assigned {
    agent_id: "agent-1".to_string(),
};
assert!(assignment.agent_id().is_some());
assert!(!assignment.is_unassigned());
```

## Recovery Actions

### Creating Recovery

```rust
use zjj_core::output::{
    Recovery, IssueId, Assessment, RecoveryCapability,
    ErrorSeverity,
};

// Create a recoverable assessment
let assessment = Assessment {
    severity: ErrorSeverity::Medium,
    capability: RecoveryCapability::Recoverable {
        recommended_action: "Run 'jj resolve'".to_string(),
    },
};

let recovery = Recovery::new(
    IssueId::new("ISSUE-789")?,
    assessment,
)?;

// Add automatic recovery action
let recovery = recovery.with_action(
    "Resolve conflicts".to_string(),
    Some("jj resolve".to_string()),  // command
    true,  // automatic
)?;

// Add manual recovery action
let recovery = recovery.with_action(
    "Manual merge".to_string(),
    None,  // no command
    false,  // manual
)?;

// Convert to output line
let output_line = zjj_core::output::OutputLine::Recovery(recovery);
```

### Recovery Execution

```rust
use zjj_core::output::RecoveryExecution;

// Automatic execution
let execution = RecoveryExecution::automatic("jj resolve");
assert!(execution.is_automatic());
assert!(execution.command().is_some());

// Manual execution
let execution = RecoveryExecution::manual();
assert!(!execution.is_automatic());
assert!(execution.command().is_none());
```

## Error Handling

### Creating Error Results

```rust
use zjj_core::output::{
    ResultOutput, ResultKind, Outcome, Message,
};

// Success result
let result = ResultOutput::success(
    ResultKind::Command,
    Message::new("Command completed")?,
)?;

// Failure result
let result = ResultOutput::failure(
    ResultKind::Operation,
    Message::new("Operation failed")?,
)?;

// Add data
use serde_json::json;
let result = result.with_data(json!({
    "exit_code": 1,
    "stderr": "File not found"
}));

// Convert to output line
let output_line = zjj_core::output::OutputLine::Result(result);
```

### Outcome Conversion

```rust
use zjj_core::output::Outcome;

// From bool (for backward compatibility)
let outcome = Outcome::from_bool(true);
assert!(matches!(outcome, Outcome::Success));

let outcome = Outcome::from_bool(false);
assert!(matches!(outcome, Outcome::Failure));

// To bool (for backward compatibility)
let success = Outcome::Success.to_bool();
assert!(success);

let failure = Outcome::Failure.to_bool();
assert!(!failure);
```

## Migration Examples

### Before (Primitives)

```rust
// Old code - could have empty strings
let issue = Issue::new(
    "ISSUE-123".to_string(),
    "Fix auth bug".to_string(),
    IssueKind::Validation,
    IssueSeverity::Error,
)?;

// Old code - ambiguous Option
let issue = issue.with_session(Some("my-session".to_string()));

// Old code - boolean flag
let assessment = Assessment {
    severity: ErrorSeverity::Medium,
    recoverable: true,  // What does true mean?
    recommended_action: "Run fix".to_string(),
};
```

### After (Semantic Newtypes)

```rust
// New code - validated at construction
let issue = Issue::new(
    IssueId::new("ISSUE-123")?,  // Validates non-empty
    IssueTitle::new("Fix auth bug")?,  // Validates non-empty
    IssueKind::Validation,
    IssueSeverity::Error,
)?;

// New code - explicit state
use zjj_core::types::SessionName;
let session_name = SessionName::new("my-session")?;
let issue = issue.with_session(session_name);

// New code - explicit capability
let assessment = Assessment {
    severity: ErrorSeverity::Medium,
    capability: RecoveryCapability::Recoverable {
        recommended_action: "Run fix".to_string(),
    },
};
```

## Helper Methods

### IssueScope Helpers

```rust
use zjj_core::output::IssueScope;
use zjj_core::types::SessionName;

// Constructor helpers
let standalone = IssueScope::standalone();
let in_session = IssueScope::in_session(SessionName::new("test")?);

// Accessor helpers
match standalone {
    IssueScope::Standalone => println!("No session"),
    IssueScope::InSession { session } => println!("Session: {}", session),
}
```

### ActionResult Helpers

```rust
use zjj_core::output::ActionResult;

// Constructor helpers
let pending = ActionResult::pending();
let completed = ActionResult::completed("Success".to_string());

// Accessor helpers
match completed {
    ActionResult::Pending => println!("Still pending"),
    ActionResult::Completed { result } => println!("Result: {}", result),
}
```

### BeadAttachment Helpers

```rust
use zjj_core::output::{BeadAttachment, BeadId};

// Constructor helpers
let none = BeadAttachment::none();
let attached = BeadAttachment::attached(BeadId::new("bead-123")?);

// Accessor helpers
match attached {
    BeadAttachment::None => println!("No bead"),
    BeadAttachment::Attached { bead_id } => println!("Bead: {}", bead_id),
}
```

### AgentAssignment Helpers

```rust
use zjj_core::output::AgentAssignment;

// Constructor helpers
let unassigned = AgentAssignment::unassigned();
let assigned = AgentAssignment::assigned("agent-1".to_string());

// Accessor helpers
match assigned {
    AgentAssignment::Unassigned => println!("Unassigned"),
    AgentAssignment::Assigned { agent_id } => println!("Agent: {}", agent_id),
}
```

## Complete Example

```rust
use zjj_core::output::{
    Issue, IssueId, IssueTitle, IssueKind, IssueSeverity,
    IssueScope, OutputLine,
};
use zjj_core::types::SessionName;

fn create_issue_report() -> Result<OutputLine, Box<dyn std::error::Error>> {
    // Create validated IDs and titles
    let issue_id = IssueId::new("ISSUE-123")?;
    let issue_title = IssueTitle::new("Authentication token expired")?;

    // Create the issue
    let issue = Issue::new(
        issue_id,
        issue_title,
        IssueKind::Validation,
        IssueSeverity::Error,
    )?;

    // Add session context if applicable
    let session_name = SessionName::new("auth-fix")?;
    let issue = issue.with_session(session_name);

    // Add suggestion
    let issue = issue.with_suggestion(
        "Run 'zjj refresh-tokens' to renew authentication".to_string(),
    );

    // Convert to output line
    Ok(OutputLine::Issue(issue))
}

// Usage
let output_line = create_issue_report()?;
println!("{}", serde_json::to_string(&output_line)?);
```

## Testing Examples

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_id_rejects_empty() {
        assert!(IssueId::new("").is_err());
        assert!(IssueId::new("   ").is_err());
    }

    #[test]
    fn test_issue_id_accepts_valid() {
        assert!(IssueId::new("ISSUE-123").is_ok());
    }

    #[test]
    fn test_issue_scope_standalone() {
        let scope = IssueScope::Standalone;
        assert!(scope.session().is_none());
    }

    #[test]
    fn test_issue_scope_in_session() {
        let session_name = SessionName::new("test").unwrap();
        let scope = IssueScope::InSession {
            session: session_name.clone(),
        };
        assert_eq!(scope.session(), Some(&session_name));
    }
}
```

### Property-Based Tests

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_issue_id_never_empty(id in "[a-zA-Z0-9-]{1,100}") {
            let issue_id = IssueId::new(&id).unwrap();
            assert!(!issue_id.as_str().is_empty());
        }

        #[test]
        fn test_message_never_empty(msg in "[a-zA-Z0-9 ]{1,1000}") {
            let message = Message::new(&msg).unwrap();
            assert!(!message.as_str().is_empty());
        }
    }
}
```

## Summary

The refactored domain types provide:
- ✅ **Type Safety**: Compiler catches invalid data
- ✅ **Validation Once**: Validate at boundaries, trust everywhere
- ✅ **Explicit States**: No ambiguous bool/Option
- ✅ **Self-Documenting**: Types express domain concepts
- ✅ **Zero Panic**: No unwrap(), no expect()
- ✅ **Better Errors**: Structured error types

All constructors follow railway-oriented programming with `Result<T, E>` and enforce zero-panic principles.

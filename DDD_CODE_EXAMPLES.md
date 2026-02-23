# DDD Refactoring Code Examples

## Semantic Newtype Pattern

### Creating a Validated Identifier

```rust
use zjj_core::domain::{SessionName, IdError};

// Parse and validate at boundary
let name = SessionName::parse("my-session")?;
// Returns Result<SessionName, IdError>

// Once created, guaranteed valid
println!("Session: {}", name.as_str());

// Can also convert from String (fallible)
let name: SessionName = "my-session".try_into()?;
```

### Identifier Validation Rules

```rust
// SessionName
// - 1-63 characters
// - Must start with a letter
// - Can contain: alphanumeric, hyphen, underscore
SessionName::parse("valid-session").unwrap();   // OK
SessionName::parse("123-invalid").unwrap_err(); // Error: must start with letter

// AgentId
// - 1-128 characters
// - Can contain: alphanumeric, hyphen, underscore, dot, colon
AgentId::parse("agent-123").unwrap();           // OK
AgentId::parse("agent:123").unwrap();           // OK
AgentId::parse("agent/123").unwrap_err();       // Error: invalid characters

// TaskId/BeadId
// - Must start with "bd-"
// - Followed by hexadecimal characters
TaskId::parse("bd-abc123").unwrap();            // OK
TaskId::parse("abc123").unwrap_err();           // Error: missing prefix
TaskId::parse("bd-xyz").unwrap_err();           // Error: not hex
```

## State Enums Replacing Option Fields

### BranchState (replaces Option<String> for branch)

```rust
use zjj_core::domain::session::BranchState;

// Instead of:
// pub branch: Option<String>

// Use:
pub branch: BranchState;

// Create
let detached = BranchState::Detached;
let on_main = BranchState::OnBranch { name: "main".to_string() };

// Use
match branch {
    BranchState::Detached => println!("Detached HEAD"),
    BranchState::OnBranch { name } => println!("On branch: {name}"),
}

// Helper methods
let is_detached = branch.is_detached();
let branch_name = branch.branch_name(); // Option<&str>
```

### ParentState (replaces Option<String> for parent_session)

```rust
use zjj_core::domain::session::ParentState;
use zjj_core::domain::SessionName;

// Instead of:
// pub parent_session: Option<String>

// Use:
pub parent_session: ParentState;

// Create
let root = ParentState::Root;
let parent_name = SessionName::parse("parent-session").unwrap();
let child = ParentState::ChildOf { parent: parent_name };

// Use
match parent_session {
    ParentState::Root => println!("Root session"),
    ParentState::ChildOf { parent } => println!("Child of: {parent}"),
}

// Helper methods
let is_root = parent_session.is_root();
let parent_name = parent_session.parent_name(); // Option<&SessionName>
```

### ClaimState (replaces claimed_by/claimed_at options)

```rust
use zjj_core::domain::queue::ClaimState;
use zjj_core::domain::AgentId;
use chrono::Utc;

// Instead of:
// pub claimed_by: Option<String>
// pub claimed_at: Option<DateTime<Utc>>
// pub claim_expires_at: Option<DateTime<Utc>>

// Use:
pub claim_state: ClaimState;

// Create
let unclaimed = ClaimState::Unclaimed;

let agent = AgentId::parse("agent-1").unwrap();
let now = Utc::now();
let expires = now + chrono::Duration::seconds(300);
let claimed = ClaimState::Claimed {
    agent: agent.clone(),
    claimed_at: now,
    expires_at: expires,
};

// Use
match claim_state {
    ClaimState::Unclaimed => println!("Not claimed"),
    ClaimState::Claimed { agent, expires_at, .. } => {
        println!("Claimed by {agent} until {expires_at}")
    }
    ClaimState::Expired { previous_agent, .. } => {
        println!("Expired from {previous_agent}")
    }
}

// Helper methods
let is_claimed = claim_state.is_claimed();
let holder = claim_state.holder(); // Option<&AgentId>
```

## Command Enums Replacing Boolean Flags

### QueueCommand (replaces QueueOptions with 10+ booleans)

```rust
use zjj_core::domain::queue::QueueCommand;
use zjj_core::domain::{WorkspaceName, AgentId};

// Instead of:
// pub struct QueueOptions {
//     pub list: bool,
//     pub process: bool,
//     pub next: bool,
//     pub stats: bool,
//     pub add: Option<String>,
//     pub remove: Option<String>,
//     // ... more fields
// }

// Use:
pub enum QueueCommand {
    List,
    Process,
    Next,
    Stats,
    ShowStatus { workspace: WorkspaceName },
    Add {
        workspace: WorkspaceName,
        bead: Option<String>,
        priority: i32,
        agent: Option<AgentId>,
    },
    Remove { workspace: WorkspaceName },
    Retry { entry_id: i64 },
    Cancel { entry_id: i64 },
    ReclaimStale { threshold_secs: i64 },
    ShowById { entry_id: i64 },
}

// Usage with exhaustive pattern matching
match command {
    QueueCommand::List => handle_list(),
    QueueCommand::Process => handle_process(),
    QueueCommand::Add { workspace, bead, priority, agent } => {
        handle_add(workspace, bead, priority, agent).await
    }
    // Compiler ensures all cases handled
}
```

## Parse-Once Pattern

### Shell Layer (Parse and Validate)

```rust
use anyhow::Result;
use zjj_core::domain::SessionName;

// At CLI boundary - parse once
fn parse_session_name_arg(raw: &str) -> Result<SessionName> {
    SessionName::parse(raw)
        .map_err(|e| anyhow::anyhow!("Invalid session name: {e}"))
}

// In CLI handler
let name = parse_session_name_arg(raw_input)?;
// From here on, `name` is guaranteed valid
```

### Core Layer (Trust and Use)

```rust
use zjj_core::domain::{SessionName, WorkspacePath};
use zjj_core::domain::session::ParentState;

// Core business logic accepts only validated types
pub async fn create_session(
    &self,
    name: &SessionName,          // Already validated!
    workspace: &WorkspacePath,   // Already validated!
    parent: Option<&SessionName>,// Already validated!
) -> Result<Session, SessionError> {
    // No validation needed - done at boundary
    // Pure business logic only

    // Check for duplicate
    if self.db.get(name).await?.is_some() {
        return Err(SessionError::AlreadyExists(name.clone()));
    }

    // Create session
    let session = self.db.create(name, workspace).await?;

    // Set parent state
    let parent_state = match parent {
        None => ParentState::Root,
        Some(p) => ParentState::ChildOf { parent: p.clone() },
    };

    Ok(session.with_parent(parent_state))
}
```

## Error Handling with Domain Types

```rust
use thiserror::Error;
use zjj_core::domain::SessionName;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session '{0}' not found")]
    NotFound(SessionName),

    #[error("session '{0}' already exists")]
    AlreadyExists(SessionName),

    #[error("parent session '{0}' not found")]
    ParentNotFound(SessionName),

    #[error("session '{name}' is in '{status}' state, cannot '{action}'")]
    InvalidState {
        name: SessionName,
        status: SessionStatus,
        action: String,
    },

    #[error("database error: {0}")]
    Database(#[from] DbError),
}

// Usage
return Err(SessionError::NotFound(name.clone()));
```

## Integration with Existing Code

### Gradual Migration Strategy

```rust
// Step 1: Add new field alongside old field
pub struct Session {
    pub branch: Option<String>,           // OLD
    pub branch_state: Option<BranchState>, // NEW
}

// Step 2: Add conversion methods
impl Session {
    fn get_branch_state(&self) -> BranchState {
        self.branch_state.clone().unwrap_or_else(|| {
            match &self.branch {
                None => BranchState::Detached,
                Some(name) => BranchState::OnBranch { name: name.clone() },
            }
        })
    }
}

// Step 3: Update all call sites
let state = session.get_branch_state();

// Step 4: Remove old field
pub struct Session {
    pub branch_state: BranchState,
}
```

## Testing Domain Types

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zjj_core::domain::SessionName;

    #[test]
    fn test_valid_session_name() {
        assert!(SessionName::parse("my-session").is_ok());
    }

    #[test]
    fn test_invalid_session_name() {
        assert!(SessionName::parse("123-invalid").is_err());
        assert!(SessionName::parse("").is_err());
    }

    #[test]
    fn test_session_name_display() {
        let name = SessionName::parse("test").unwrap();
        assert_eq!(name.to_string(), "test");
        assert_eq!(name.as_str(), "test");
    }

    #[test]
    fn test_serialization() {
        let name = SessionName::parse("my-session").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        assert!(json.contains("my-session"));

        let decoded: SessionName = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, name);
    }
}
```

## Key Benefits

1. **Type Safety**: Invalid states cannot be represented
   ```rust
   let name = SessionName::parse("...")?;  // Must validate
   create_session(&name, ...);              // Guaranteed valid
   ```

2. **Parse Once**: Validation at boundaries only
   ```rust
   // Shell: parse once
   let name = SessionName::parse(raw)?;
   
   // Core: trust the type
   use_validated_type(&name);
   ```

3. **Self-Documenting**: Types convey intent
   ```rust
   fn create_session(name: &SessionName)  // Clear intent
   fn create_session(name: &str)           // Unclear validation
   ```

4. **Exhaustive Matching**: Compiler guides changes
   ```rust
   match command {
       QueueCommand::List => ...,
       QueueCommand::Process => ...,
       // Compiler error if you forget a case
   }
   ```

5. **Better Errors**: Domain-specific with context
   ```rust
   Err(SessionError::NotFound(name.clone()))  // Clear
   Err(anyhow!("not found"))                   // Vague
   ```

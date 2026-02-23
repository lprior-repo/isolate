# State Machines in ZJJ Domain

This document catalogs all state machines used in the ZJJ codebase. State machines are a core DDD pattern that make illegal states unrepresentable through the type system.

## Table of Contents

1. [SessionStatus](#sessionstatus)
2. [WorkspaceState](#workspacestate)
3. [AgentState](#agentstate)
4. [ClaimState](#claimstate)
5. [BranchState](#branchstate)
6. [ParentState](#parentstate)
7. [IssueState/BeadState](#issuestatebeadstate)

---

## SessionStatus

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/cli_contracts/domain_types.rs`

Session lifecycle management - tracks the creation, active use, and termination of sessions.

### States

- **Creating** - Session is being initialized
- **Active** - Session is ready and in use
- **Paused** - Session exists but is temporarily inactive
- **Completed** - Session finished successfully
- **Failed** - Session terminated with errors

### State Diagram

```
                    ┌─────────────┐
                    │  Creating   │
                    └──────┬──────┘
                           │
                           │ session initialized
                           ▼
                    ┌─────────────┐
         ┌──────────│   Active    │──────────┐
         │          └──────┬──────┘          │
         │                 │                 │
         │                 │ pause           │ complete
         │                 ▼                 │
    ┌────┴────┐       ┌─────────┐       ┌────▼────┐
    │ Paused  │       │ Failed  │       │Completed│
    └────┬────┘       └─────────┘       └─────────┘
         │
         │ resume
         │
         └──────────────►
```

### Valid Transitions

| From      | To        | Description          |
|-----------|-----------|----------------------|
| Creating  | Active    | Session initialized  |
| Creating  | Failed    | Creation failed      |
| Active    | Paused    | User paused session  |
| Active    | Completed | Session finished     |
| Paused    | Active    | Session resumed      |
| Paused    | Completed | Session ended        |

### Invalid Transitions

- Creating → Paused (cannot pause before creation)
- Completed → *any* (terminal state)
- Failed → *any* (terminal state)
- Active → Creating (cannot go back)
- Paused → Creating (cannot go back)

### Code Reference

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionStatus {
    Creating,
    Active,
    Paused,
    Completed,
    Failed,
}

impl SessionStatus {
    pub const fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (Self::Creating, Self::Active | Self::Failed)
                | (Self::Active, Self::Paused | Self::Completed)
                | (Self::Paused, Self::Active | Self::Completed)
        )
    }
}
```

---

## WorkspaceState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/workspace.rs`

Workspace lifecycle management - tracks workspace creation, usage, and cleanup.

### States

- **Creating** - Workspace is being initialized
- **Ready** - Workspace exists and is ready for use
- **Active** - Workspace is currently in use
- **Cleaning** - Workspace is being cleaned up
- **Removed** - Workspace has been deleted

### State Diagram

```
                    ┌─────────────┐
                    │  Creating   │
                    └──────┬──────┘
                           │
                           │ initialized
                           ▼
                    ┌─────────────┐
              ┌─────│    Ready    │─────┐
              │     └──────┬──────┘     │
              │            │             │
              │            │ in use      │ skip use
              │            ▼             │ direct cleanup
              │     ┌─────────────┐      │
              │     │   Active    │      │
              │     └──────┬──────┘      │
              │            │             │
              │            │ cleanup     │
              │            ▼             ▼
              │     ┌─────────────┐      │
              └─────│  Cleaning   │◄─────┘
                    └──────┬──────┘
                           │
                           │ cleaned
                           ▼
                    ┌─────────────┐
                    │   Removed   │
                    └─────────────┘
                    [TERMINAL]
```

### Valid Transitions

| From      | To        | Description                  |
|-----------|-----------|------------------------------|
| Creating  | Ready     | Workspace created            |
| Creating  | Removed   | Cleanup during creation      |
| Ready     | Active    | Workspace started           |
| Ready     | Cleaning  | Direct cleanup              |
| Ready     | Removed   | Direct removal              |
| Active    | Cleaning  | Cleanup after use           |
| Active    | Removed   | Direct removal              |
| Cleaning  | Removed   | Cleanup complete            |

### Invalid Transitions

- Creating → Active (must go through Ready first)
- Creating → Cleaning (cannot clean before creation)
- Active → Ready (cannot go back from active)
- Active → Creating (cannot go back)
- Removed → *any* (terminal state, no self-loops)
- Cleaning → Ready/Active (cannot reverse cleanup)
- Ready → Creating (cannot go back)

### Code Reference

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceState {
    Creating,
    Ready,
    Active,
    Cleaning,
    Removed,
}

impl WorkspaceState {
    pub const fn can_transition_to(self, target: &Self) -> bool {
        match (self, target) {
            (Self::Creating, Self::Ready | Self::Removed) => true,
            (Self::Ready, Self::Active | Self::Cleaning | Self::Removed) => true,
            (Self::Active, Self::Cleaning | Self::Removed) => true,
            (Self::Cleaning, Self::Removed) => true,
            _ => false,
        }
    }

    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Removed)
    }
}
```

---

## AgentState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/agent.rs`

Agent lifecycle management - tracks agent availability and processing status.

### States

- **Idle** - Agent is available and waiting
- **Active** - Agent is processing work
- **Offline** - Agent is disconnected
- **Error** - Agent encountered an error

### State Diagram

```
         ┌──────────────────────────────────┐
         │                                  │
         ▼                                  │
    ┌─────────┐      process       ┌─────────┐
    │  Idle   │◄───────────────────│  Active │
    └────┬────┘                   └────┬────┘
         │                             │
         │ offline/error               │ error
         ▼                             ▼
    ┌─────────┐                   ┌─────────┐
    │ Offline │                   │  Error  │
    └─────────┘                   └─────────┘
         ▲                             │
         │                             │
         │ recover                      │ offline
         └─────────────────────────────┘
```

### Valid Transitions

| From     | To      | Description              |
|----------|---------|--------------------------|
| Idle     | Active  | Agent starts processing  |
| Active   | Idle    | Agent completes work     |
| Active   | Offline | Agent disconnects        |
| Active   | Error   | Agent encounters error   |
| Idle     | Offline | Agent disconnects        |
| Idle     | Error   | Agent enters error state |
| Error    | Offline | Agent taken offline      |
| Offline  | Idle    | Agent comes online       |

### Invalid Transitions

- Idle → Idle (self-loops not allowed)
- Active → Active (self-loops not allowed)
- Offline → Offline (self-loops not allowed)
- Error → Error (self-loops not allowed)
- Offline → Active (must go through Idle first)
- Offline → Error (must go through Idle first)
- Error → Active (must go through Idle first)

### Code Reference

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Active,
    Idle,
    Offline,
    Error,
}

impl AgentState {
    pub const fn can_transition_to(self, target: &Self) -> bool {
        match (self, target) {
            (Self::Idle, Self::Active) | (Self::Active, Self::Idle) => true,
            (Self::Idle | Self::Active | Self::Error, Self::Offline) => true,
            (Self::Idle | Self::Active | Self::Offline, Self::Error) => true,
            (Self::Offline, Self::Idle) => true,
            _ => false,
        }
    }

    pub fn valid_transitions(&self) -> Vec<Self> {
        Self::all()
            .iter()
            .filter(|&target| self.can_transition_to(target))
            .copied()
            .collect()
    }
}
```

---

## ClaimState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/queue.rs`

Queue entry claim lifecycle - tracks ownership and expiration of work items.

### States

- **Unclaimed** - Entry is available for claiming
- **Claimed** - Entry is owned by an agent with expiration
- **Expired** - Previous claim has expired

### State Diagram

```
         ┌─────────────┐
         │  Unclaimed  │◄────────────┐
         └──────┬──────┘             │
                │                    │
                │ claim              │ reclaim
                │                    │ (after expiry)
                ▼                    │
         ┌─────────────┐             │
         │   Claimed   │─────────────┤
         │  {agent,    │             │
         │   expires}  │             │
         └──────┬──────┘             │
                │                    │
                │ expires            │
                │ or release         │
                ▼                    │
         ┌─────────────┐             │
         │   Expired   │─────────────┘
         │{previous    │
         │ agent}      │
         └─────────────┘
```

### Valid Transitions

| From      | To        | Description                  |
|-----------|-----------|------------------------------|
| Unclaimed | Claimed   | Agent claims entry           |
| Claimed   | Expired   | Claim expired                |
| Claimed   | Unclaimed | Agent releases claim         |
| Expired   | Unclaimed | Entry reclaimed (retry)      |

### Invalid Transitions

- Unclaimed → Unclaimed (self-loop not a transition)
- Unclaimed → Expired (cannot expire without claim)
- Claimed → Claimed (self-loop not a transition, must update timestamp)
- Expired → Expired (self-loop not a transition)
- Expired → Claimed (must go through Unclaimed to reclaim)

### Code Reference

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimState {
    Unclaimed,
    Claimed {
        agent: AgentId,
        claimed_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },
    Expired {
        previous_agent: AgentId,
        expired_at: DateTime<Utc>,
    },
}

impl ClaimState {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Unclaimed, Self::Claimed { .. }) => true,
            (Self::Claimed { .. }, Self::Expired { .. } | Self::Unclaimed) => true,
            (Self::Expired { .. }, Self::Unclaimed) => true,
            _ => false,
        }
    }

    pub fn valid_transition_types(&self) -> Vec<&'static str> {
        match self {
            Self::Unclaimed => vec!["Claimed"],
            Self::Claimed { .. } => vec!["Expired", "Unclaimed"],
            Self::Expired { .. } => vec!["Unclaimed"],
        }
    }
}
```

---

## BranchState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`

Session branch association - tracks whether a session is on a branch or detached.

### States

- **Detached** - Session is not associated with a branch
- **OnBranch { name }** - Session is on a specific named branch

### State Diagram

```
    ┌─────────────┐
    │  Detached   │◄─────────────────┐
    └──────┬──────┘                  │
           │                         │
           │ checkout branch         │ detach
           │                         │
           ▼                         │
    ┌─────────────┐                  │
    │ OnBranch    │──────────────────┘
    │ {name: ...} │
    └──────┬──────┘
           │
           │ switch branch
           │
           ▼
    ┌─────────────┐
    │ OnBranch    │ (different name)
    │ {name: ...} │
    └─────────────┘
```

### Valid Transitions

| From         | To           | Description                  |
|--------------|--------------|------------------------------|
| Detached     | OnBranch     | Checkout to a branch        |
| OnBranch     | Detached     | Detach from branch          |
| OnBranch     | OnBranch     | Switch to different branch  |

### Invalid Transitions

- Detached → Detached (not a transition, staying in same state)

### Code Reference

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    Detached,
    OnBranch { name: String },
}

impl BranchState {
    pub const fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Detached, Self::OnBranch { .. })
            | (Self::OnBranch { .. }, Self::Detached)
            | (Self::OnBranch { .. }, Self::OnBranch { .. }) => true,
            (Self::Detached, Self::Detached) => false,
        }
    }
}
```

---

## ParentState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/session.rs`

Session hierarchy - tracks whether a session is a root or has a parent.

### States

- **Root** - Session has no parent (top-level session)
- **ChildOf { parent }** - Session has a named parent session

### State Diagram

```
    ┌─────────┐
    │  Root   │
    └─────────┘
    [INITIAL ONLY - NO TRANSITIONS OUT]


    ┌──────────────────┐
    │  ChildOf         │
    │  {parent: ...}   │
    └────────┬─────────┘
             │
             │ reassign parent
             │ (adoption/restructuring)
             ▼
    ┌──────────────────┐
    │  ChildOf         │ (different parent)
    │  {parent: ...}   │
    └──────────────────┘
```

### Valid Transitions

| From      | To        | Description                       |
|-----------|-----------|-----------------------------------|
| ChildOf   | ChildOf   | Reassign to different parent     |

### Invalid Transitions

- Root → Root (not a transition)
- Root → ChildOf (root cannot become a child)
- ChildOf → Root (child cannot become root)

**Important**: `ParentState::Root` is an **initial-only state**. Once a session is created as a child, it cannot become a root, and a root cannot transition to be a child. This prevents session hierarchy corruption.

### Code Reference

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentState {
    Root,
    ChildOf { parent: SessionName },
}

impl ParentState {
    pub const fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Root, Self::ChildOf { .. } | Self::Root) => false,
            (Self::ChildOf { .. }, Self::Root) => false,
            (Self::ChildOf { .. }, Self::ChildOf { .. }) => true,
        }
    }
}
```

---

## IssueState / BeadState

**Location**: `/home/lewis/src/zjj/crates/zjj-core/src/beads/domain.rs`

Issue/bead lifecycle for the beads issue tracker - tracks workflow states with type-safe timestamp enforcement.

### States

- **Open** - Issue is newly created and open
- **InProgress** - Issue is being actively worked on
- **Blocked** - Issue is blocked by dependencies
- **Deferred** - Issue is postponed to later
- **Closed { closed_at }** - Issue is resolved with timestamp

### State Diagram

```
                    ┌─────────────┐
                    │    Open     │
                    └──────┬──────┘
                           │
                           │ start work
                           ▼
                    ┌─────────────┐
         ┌──────────│ InProgress  │
         │          └──────┬──────┘
         │                 │
         │                 │ blocked by dependency
         │                 ▼
         │          ┌─────────────┐
         │          │  Blocked    │
         │          └──────┬──────┘
         │                 │
         │                 │ unblock/defer
         │                 ▼
         │    ┌─────────────┐
         └────│  Deferred   │
              └──────┬──────┘
                     │
                     │ reactivate
                     │
                     └──────────────┐
                                    │
                                    │ close (with timestamp)
                                    ▼
                             ┌─────────────┐
                             │   Closed    │
                             │{closed_at}  │
                             └─────────────┘
                             [TERMINAL]
```

### Key DDD Feature: Type-Safe Timestamp

The `Closed` variant **must** include a `closed_at` timestamp, making it impossible to have a closed issue without tracking when it was closed:

```rust
pub enum IssueState {
    Open,           // No timestamp
    InProgress,     // No timestamp
    Blocked,        // No timestamp
    Deferred,       // No timestamp
    Closed {
        closed_at: DateTime<Utc>,  // REQUIRED by type system
    },
}
```

### Valid Transitions

| From         | To          | Description                      |
|--------------|-------------|----------------------------------|
| Open         | InProgress  | Work started                     |
| Open         | Blocked     | Blocked before start            |
| Open         | Deferred    | Postponed                        |
| Open         | Closed      | Closed without work             |
| InProgress   | Open        | Work stopped, back to open      |
| InProgress   | Blocked     | Blocked during work             |
| InProgress   | Deferred    | Work postponed                  |
| InProgress   | Closed      | Work completed                  |
| Blocked      | Open        | Unblocked                       |
| Blocked      | InProgress  | Unblocked, ready to work        |
| Blocked      | Deferred    | Blocked and postponed           |
| Blocked      | Closed      | Closed as blocked               |
| Deferred     | Open        | Reopened                        |
| Deferred     | InProgress  | Reactivated and started         |
| Deferred     | Blocked     | Still blocked                   |
| Deferred     | Closed      | Closed without reopening       |

**Note**: The current implementation allows flexible workflow transitions - any state can transition to any other state. This enables team-specific workflows while maintaining the closed timestamp invariant.

### Invalid Transitions

- **Closed → *any*** (terminal state, no transitions out)
- Self-transitions are technically allowed but represent no change

### Code Reference

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed {
        closed_at: DateTime<Utc>,
    },
}

impl IssueState {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    pub const fn is_blocked(self) -> bool {
        matches!(self, Self::Blocked)
    }

    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Closed { .. })
    }

    pub const fn closed_at(self) -> Option<DateTime<Utc>> {
        match self {
            Self::Closed { closed_at } => Some(closed_at),
            _ => None,
        }
    }

    pub const fn transition_to(self, new_state: Self) -> Result<Self, DomainError> {
        // Flexible workflow - any state can transition to any state
        // But Closed MUST have a timestamp (enforced by type)
        Ok(new_state)
    }
}
```

---

## Summary Table

| State Machine  | States | Terminal States | Key Invariant                          |
|----------------|--------|-----------------|----------------------------------------|
| SessionStatus  | 5      | Completed, Failed| Cannot return from terminal states     |
| WorkspaceState | 5      | Removed         | Removed has no outgoing transitions    |
| AgentState     | 4      | None*           | Active must go through Idle to Offline |
| ClaimState     | 3      | None            | Expired requires prior Claimed         |
| BranchState    | 2      | None            | Detached → Detached not allowed        |
| ParentState    | 2      | Root**          | Root cannot transition to ChildOf      |
| IssueState     | 5      | Closed          | Closed must have timestamp             |

\* AgentState has no true terminal states but Error and Offline are sink-like states.
\*\* ParentState::Root is effectively terminal - no transitions out allowed.

---

## Design Principles

All state machines follow these DDD principles:

1. **Make Illegal States Unrepresentable**
   - Use enum variants instead of optional fields
   - Embed required data (e.g., `closed_at` timestamp) in variants

2. **Explicit Transition Validation**
   - Each state implements `can_transition_to()`
   - Compile-time type safety prevents invalid states

3. **Terminal State Semantics**
   - Terminal states have no outgoing transitions
   - Clear lifecycle completion

4. **Self-Documenting**
   - State names clearly indicate meaning
   - Transition logic is explicit and testable

---

## Testing State Machines

All state machines should have tests covering:

1. **Valid transitions** - All allowed transitions work
2. **Invalid transitions** - All disallowed transitions fail
3. **Self-transitions** - Either allowed or explicitly blocked
4. **Terminal states** - Cannot exit once entered
5. **State queries** - Helper methods (`is_active()`, `is_terminal()`, etc.)

Example test pattern:

```rust
#[test]
fn test_workspace_state_transitions() {
    // Valid transitions
    assert!(WorkspaceState::Creating.can_transition_to(&WorkspaceState::Ready));
    assert!(WorkspaceState::Ready.can_transition_to(&WorkspaceState::Active));
    assert!(WorkspaceState::Active.can_transition_to(&WorkspaceState::Cleaning));
    assert!(WorkspaceState::Cleaning.can_transition_to(&WorkspaceState::Removed));

    // Invalid transitions
    assert!(!WorkspaceState::Creating.can_transition_to(&WorkspaceState::Active));
    assert!(!WorkspaceState::Removed.can_transition_to(&WorkspaceState::Ready));
    assert!(!WorkspaceState::Active.can_transition_to(&WorkspaceState::Creating));
}
```

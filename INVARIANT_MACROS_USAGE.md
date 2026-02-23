# Invariant Macros - Usage Guide

## Quick Start

The invariant macros are now available throughout the `zjj-core` crate for enforcing domain invariants with zero-panic, zero-unwrap guarantees.

## Import the Macros

```rust
// At the top of your file
use zjj_core::invariant;        // Runtime checks (always enabled)
use zjj_core::assert_invariant; // Test-only checks
use zjj_core::debug_invariant;  // Debug-only checks
```

## Basic Usage

### Example 1: Timestamp Ordering in Bead

```rust
use chrono::{DateTime, Utc};
use zjj_core::domain::{BeadError, invariant};

fn reconstruct_bead(
    id: BeadId,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<Bead, BeadError> {
    // Enforce monotonic timestamps
    invariant!(
        updated_at >= created_at,
        BeadError::NonMonotonicTimestamps {
            created_at,
            updated_at,
        }
    );

    // ... rest of construction
    Ok(Bead { id, title, created_at, updated_at })
}
```

### Example 2: State Validation in Session

```rust
use zjj_core::domain::{SessionError, invariant};

impl Session {
    pub fn activate(&self) -> Result<Self, SessionError> {
        // Cannot activate an already active session
        invariant!(
            !self.is_active(),
            SessionError::CannotActivate
        );

        // ... transition logic
        Ok(self.clone())
    }
}
```

### Example 3: Chained Invariants

```rust
fn validate_workspace_creation(
    name: &str,
    path: &Path,
) -> Result<(), WorkspaceError> {
    invariant!(
        !name.is_empty(),
        WorkspaceError::InvalidName("name cannot be empty".into())
    );

    invariant!(
        path.exists(),
        WorkspaceError::PathNotFound(path.to_path_buf())
    );

    Ok(())
}
```

## Advanced Usage

### Debug-Only Validation

Use `debug_invariant!` for expensive checks during development:

```rust
impl Bead {
    pub fn update_title(&self, new_title: String) -> Result<Self, BeadError> {
        let updated = Self {
            title: new_title,
            updated_at: Utc::now(),
            ..self.clone()
        };

        // Only validates in debug builds - zero cost in production
        debug_invariant!(
            updated.validate().is_ok(),
            BeadError::InvalidStateTransition {
                from: self.state,
                to: updated.state,
            }
        );

        Ok(updated)
    }
}
```

### Test-Only Validation

Use `assert_invariant!` for expensive validation only in tests:

```rust
impl QueueEntry {
    pub fn claim(&self, agent: AgentId) -> Result<Self, QueueEntryError> {
        // Runtime check - always enforced
        invariant!(
            self.is_claimable(),
            QueueEntryError::AlreadyClaimed(agent.clone())
        );

        // Expensive validation - only in tests
        assert_invariant!(
            self.validate_consistency().is_ok(),
            QueueEntryError::InvalidExpiration
        );

        // ... claim logic
        Ok(self.clone())
    }
}
```

## Migration Pattern

To migrate existing invariant checks:

### Before
```rust
impl Bead {
    fn validate(&self) -> Result<(), BeadError> {
        if self.updated_at < self.created_at {
            return Err(BeadError::NonMonotonicTimestamps {
                created_at: self.created_at,
                updated_at: self.updated_at,
            });
        }

        if self.title.is_empty() {
            return Err(BeadError::TitleRequired);
        }

        Ok(())
    }
}
```

### After
```rust
impl Bead {
    fn validate(&self) -> Result<(), BeadError> {
        invariant!(
            self.updated_at >= self.created_at,
            BeadError::NonMonotonicTimestamps {
                created_at: self.created_at,
                updated_at: self.updated_at,
            }
        );

        invariant!(
            !self.title.is_empty(),
            BeadError::TitleRequired
        );

        Ok(())
    }
}
```

## Benefits of Using Invariant Macros

1. **Consistency**: Standardized pattern across all domain code
2. **Safety**: Zero panic, zero unwrap guarantees
3. **Performance**: Conditional compilation for test/debug checks
4. **Readability**: Clear intent when reading code
5. **Composability**: Easy to chain multiple invariants

## Files

- **Main module**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/macros.rs`
- **Examples**: `/home/lewis/src/zjj/crates/zjj-core/src/domain/macros_examples.rs`
- **Report**: `/home/lewis/src/zjj/INVARIANT_MACROS_REPORT.md`

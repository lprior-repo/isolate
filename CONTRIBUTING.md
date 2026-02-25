# Contributing to ZJJ

Thank you for your interest in contributing to ZJJ! This guide will help you get started and set clear expectations for contributing to the project.

## Table of Contents

- [Quick Start](#quick-start)
- [Development Environment Setup](#development-environment-setup)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Common Tasks](#common-tasks)
- [Useful Commands](#useful-commands)
- [Getting Help](#getting-help)

---

## Quick Start

### Automated Setup (Recommended)

```bash
# Clone the repository
git clone https://github.com/your-username/zjj.git
cd zjj

# Run the automated setup script
./scripts/dev-setup.sh
```

The setup script will:
- Check and install prerequisites (Rust, Moon, JJ)
- Set up the development database
- Run initial build and tests
- Provide clear next steps

For non-interactive mode (CI/CD):
```bash
./scripts/dev-setup.sh --yes
```

To check prerequisites only:
```bash
./scripts/dev-setup.sh --check
```

### Manual Setup

If you prefer manual setup or the script fails:

```bash
# 1. Install dependencies
# - Moon (build tooling): https://moonrepo.dev/docs/install
# - JJ (Jujutsu): https://github.com/martinvonz/jj#installation
# - Rust 1.80+: https://rustup.rs/

# 2. Install Moon
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# 3. Verify your setup
moon run :check

# 4. Run the test suite
moon run :test
```

---

## Development Environment Setup

### Prerequisites

1. **Moon** - Required build tool (DO NOT use `cargo` directly)
   - Install from https://moonrepo.dev/docs/install
   - Provides hyper-fast caching with bazel-remote

2. **Rust 1.80+** - Programming language
   - Install via rustup: https://rustup.rs/

3. **JJ (Jujutsu)** - Version control system
   - Install from https://github.com/martinvonz/jj#installation

4. **bazel-remote** - Local build cache (optional but recommended)
   - Improves build performance by ~98.5%

### Workspace Structure

```
zjj/
├── crates/
│   ├── zjj-core/       # Core library (domain logic, types)
│   └── zjj/            # CLI application
├── tests/              # Integration tests
├── docs/               # Documentation source
├── book/               # mdBook documentation
└── .moon/              # Moon configuration
```

### Build System

**IMPORTANT**: All commands must be run through Moon. Direct `cargo` commands are banned.

```bash
# Available Moon tasks
moon run :fmt        # Check formatting
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check (no build artifacts)
moon run :test       # Run all tests
moon run :build      # Release build
moon run :ci         # Full CI pipeline
moon run :quick      # fmt + check (fastest dev loop)
```

---

## Code Style Guidelines

### Core Principles (Non-Negotiable)

ZJJ follows **Functional Rust** with strict zero-unwrap laws and **Domain-Driven Design (DDD)** principles.

#### Zero Unwrap Law

**NEVER use** these banned patterns:
- `unwrap()`
- `expect()`
- `unwrap_or()`
- `unwrap_or_else()`
- `unwrap_or_default()`
- `panic!()`
- `todo!()`
- `unimplemented!()`
- `unsafe` code

**INSTEAD use**:
- `Result<T, E>` for all fallible operations
- `?` operator for error propagation
- `match` or `if let` for branching
- `map()`, `and_then()`, `or_else()` for transformation
- `ok_or_else()`, `map_or()`, `map_or_else()` for defaults

These are enforced at the **compiler level** via workspace lints:

```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
```

#### Functional Core, Imperative Shell

**Core (Domain Logic)** - Pure functions:
- No I/O, no global state, no side effects
- Deterministic: same input = same output
- Sync only (async belongs in shell)
- Uses `thiserror` for domain errors

**Shell (Imperative)** - I/O and external APIs:
- Handles I/O, async, external systems
- Uses `anyhow` for boundary errors with context
- Delegates to pure core for business logic

Example:

```rust
// CORE: Pure function in zjj-core/src/domain/
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid session name: {0}")]
    InvalidName(String),
}

pub fn validate_session_name(name: &str) -> Result<&str, DomainError> {
    match name.is_empty() {
        true => Err(DomainError::InvalidName("empty".into())),
        false => Ok(name),
    }
}

// SHELL: Async I/O in zjj/src/commands/
use anyhow::{Result, Context};

pub async fn create_session(name: &str) -> Result<Session> {
    validate_session_name(name)  // Call pure core
        .context("session validation failed")?;  // Add context with anyhow
    // ... async I/O operations
}
```

#### Domain-Driven Design (DDD)

**Bounded Contexts**:
- `domain/` - Core aggregates, value objects, domain events
- `beads/` - Issue tracking domain
- `coordination/` - Queue and coordination domain
- `cli_contracts/` - CLI boundary contracts

**Aggregates**:
- Cluster entities and value objects
- Enforce invariants at aggregate root
- Example: `Session`, `Workspace`, `Bead`, `QueueEntry`

**Value Objects**:
- Immutable types for domain concepts
- Equality by value, not identity
- Example: `SessionName`, `BeadId`, `WorkspaceState`

**Repository Pattern**:
- Abstract persistence behind trait
- Domain doesn't know about storage details

```rust
// Example: Domain aggregate with builder
use crate::domain::{SessionName, WorkspaceName};

#[derive(Debug, Clone)]
pub struct Session {
    pub name: SessionName,
    pub workspace: WorkspaceName,
    pub state: SessionState,
}

impl Session {
    /// Pure state transition (no mut)
    pub fn transition_to(&self, new_state: SessionState) -> Result<Self, DomainError> {
        self.state.transition_to(new_state)?;
        Ok(Session {
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            state: new_state,
        })
    }
}
```

#### The Core 6 Libraries

When implementing Rust code, prioritize these libraries (in order):

1. **itertools** (0.14) - Iterator pipelines, loop-free transforms
2. **tap** (1.0) - Suffix pipelines (pipe/tap/conv ergonomics)
3. **rpds** (1.2) - Persistent state with structural sharing
4. **thiserror** (2.0) - Domain errors in core
5. **anyhow** (1.0) - Boundary errors with context in shell
6. **futures-util** (0.3) - Async combinators (StreamExt/TryStreamExt)

Example patterns:

```rust
use itertools::Itertools;

// Iterator pipelines (preferred over loops)
fn process(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unique()
        .sorted()
        .collect_vec()
}

// Persistent state with rpds
use rpds::Vector;

#[derive(Clone)]
struct State {
    events: Vector<String>,
}

// Pure state transition (no mut)
fn append_event(state: State, event: String) -> State {
    State {
        events: state.events.push_back(event),
    }
}
```

#### File Header (Required)

Every Rust file must include:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

### Code Organization

#### Domain Types (`crates/zjj-core/src/domain/`)

```rust
//! Semantic newtypes for domain concepts
//!
//! # Parse-at-Boundaries Pattern
//!
//! - Validates input on construction
//! - Cannot represent invalid states
//! - Provides safe access to underlying value

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// A validated session name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionName(String);

impl SessionName {
    pub const MAX_LENGTH: usize = 64;

    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.is_empty() {
            return Err(DomainError::InvalidInput("empty".into()));
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidInput("too long".into()));
        }
        Ok(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

#### Error Handling

**Use `thiserror` in core**:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("session not found: {0}")]
    NotFound(String),

    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: State, to: State },
}
```

**Use `anyhow` in shell**:

```rust
use anyhow::{Result, Context};

pub async fn execute() -> Result<()> {
    do_something()
        .await
        .context("failed to do something")?;
    Ok(())
}
```

#### Async Patterns

All database operations use **async/await** with Tokio:

```rust
use sqlx::SqlitePool;

pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<Session>> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions")
        .fetch_all(pool)
        .await
        .context("failed to fetch sessions")
}
```

---

## Testing Requirements

### Three Types of Tests

1. **Unit Tests** - Test pure functions in isolation
2. **Property Tests** - Use proptest for invariant testing
3. **Integration Tests** - Test CLI commands end-to-end

### Unit Tests

Place unit tests in the same module as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_name_valid() {
        assert!(SessionName::new("valid-name").is_ok());
    }

    #[test]
    fn test_session_name_empty() {
        assert!(matches!(
            SessionName::new(""),
            Err(DomainError::InvalidInput(_))
        ));
    }
}
```

### Property-Based Tests

Use `proptest` for invariant testing (see `tests/status_properties.rs`):

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_roundtrip(name in "[a-zA-Z0-9_-]{1,64}") {
        let parsed = SessionName::new(&name);
        prop_assert!(parsed.is_ok());
        prop_assert_eq!(parsed.unwrap().as_str(), name);
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/session_feature.rs
use assert_cmd::Command;

#[test]
fn test_create_session() {
    Command::cargo_bin("zjj")
        .unwrap()
        .args(["add", "test-session"])
        .assert()
        .success();
}
```

### Test Requirements

- All PRs must pass `moon run :ci`
- New features require property tests for invariants
- Bug fixes require regression tests
- Aim for high test coverage (codecov reports available)

---

## Pull Request Process

### Before Submitting

1. **Run quality gates**:
   ```bash
   moon run :ci
   ```

2. **Format your code**:
   ```bash
   moon run :fmt-fix
   ```

3. **Test manually**:
   ```bash
   # Build and test locally
   moon run :build
   ./target/release/zjj --help

   # Test your changes manually
   ./target/release/zjj add test-session
   ```

4. **Update documentation**:
   - If adding commands, update `docs/`
   - If changing behavior, update relevant `.md` files
   - Run `moon run :check` to verify docs build

### Submitting a PR

1. **Descriptive title**: Use conventional commit format
   - Good: `feat(queue): add retry mechanism for failed entries`
   - Bad: `fixed stuff` or `update`

2. **Description template**:
   ```markdown
   ## Summary
   - What changed
   - Why it's needed

   ## Testing
   - [ ] Unit tests pass
   - [ ] Property tests pass
   - [ ] Integration tests pass
   - [ ] Manual testing completed

   ## Breaking Changes
   - List any breaking changes

   ## Documentation
   - [ ] Updated docs/
   - [ ] Added examples
   ```

3. **Link issues**: Reference related issues with `fixes #123`

### Review Process

- At least one maintainer approval required
- All CI checks must pass
- Code review focuses on:
  - Functional Rust principles (no unwrap)
  - DDD patterns (clear domain boundaries)
  - Test coverage
  - Documentation completeness

### Landing Your PR

After approval:

```bash
# If you have commit access
git push

# If you don't, maintainers will land it for you
```

---

## Common Tasks

### Adding a New Domain Type

```rust
// 1. Create in crates/zjj-core/src/domain/

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyTypeError {
    #[error("invalid value: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MyType(String);

impl MyType {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(value: impl Into<String>) -> Result<Self, MyTypeError> {
        let value = value.into();
        // Validation logic
        if value.len() > Self::MAX_LENGTH {
            return Err(MyTypeError::Invalid("too long".into()));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// 2. Add tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_type_valid() {
        assert!(MyType::new("valid").is_ok());
    }

    #[test]
    fn test_my_type_invalid() {
        assert!(MyType::new("").is_err());
    }
}
```

### Adding a State Machine

```rust
use strum::{Display, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
    EnumString,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MyState {
    Initial,
    InProgress,
    Completed,
    Failed,
}

impl MyState {
    /// Transition to new state with validation
    pub fn transition_to(self, new_state: Self) -> Result<Self, DomainError> {
        match (self, new_state) {
            (Self::Initial, Self::InProgress) => Ok(Self::InProgress),
            (Self::InProgress, Self::Completed) => Ok(Self::Completed),
            (Self::InProgress, Self::Failed) => Ok(Self::Failed),
            (from, to) => Err(DomainError::InvalidTransition { from, to }),
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}
```

### Adding a CLI Command

```rust
// 1. Define handler in crates/zjj/src/commands/
use anyhow::Result;
use zjj_core::Context;

pub async fn run(args: Args, ctx: &Context) -> Result<()> {
    // Your command logic here
    ctx.output("Success!")
}

// 2. Register in crates/zjj/src/cli/commands.rs
// 3. Add tests in tests/
```

### Adding Property Tests

```rust
// tests/my_feature_properties.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property(input in ".*") {
        // Test invariant
        prop_assert!(condition_holds(input));
    }
}
```

---

## Useful Commands

### Development Loop

```bash
# Fastest: format + type check (6-7ms with cache)
moon run :quick

# Full test suite
moon run :test

# Single test
moon run :test -- session_feature

# With output
moon run :test -- --nocapture session_feature
```

### Build Commands

```bash
# Release build
moon run :build

# Run built binary
./target/release/zjj --help

# Dev build (faster)
cargo build  # Only use this for debugging!
```

### Database/Cache

```bash
# View cache stats
curl http://localhost:9090/status | jq

# Restart cache service
systemctl --user restart bazel-remote

# View cache logs
journalctl --user -u bazel-remote -f
```

### Git Workflow

```bash
# See what changed
git status

# View diff
git diff

# Stage files
git add path/to/file.rs

# Commit (use conventional commits)
git commit -m "feat: add new feature"

# Push
git push
```

---

## Getting Help

### Documentation

- **[Full Documentation](https://lprior-repo.github.io/zjj/)** - Complete user and dev guide
- **[AGENTS.md](AGENTS.md)** - Agent workflow and mandatory rules
- **[README.md](README.md)** - Project overview
- **Source docs** - Run `moon run :test -- --doc` to generate

### Code Examples

- `crates/zjj-core/src/domain/` - Domain type examples
- `crates/zjj-core/src/beads/domain.rs` - DDD patterns
- `tests/` - Integration test examples

### Communication

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Questions and ideas
- **Pull Requests** - Code contributions

### Learning Resources

**Functional Rust**:
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [itertools docs](https://docs.rs/itertools/)

**Domain-Driven Design**:
- [DDD Reference](https://www.domainlanguage.com/ddd/reference/)
- [Vue.js DDD](https://www.youtube.com/watch?v=8N1kp9o4G5c) (concepts apply)

**Property-Based Testing**:
- [proptest book](https://altsysrq.github.io/proptest-book/)

---

## Code Review Checklist

When reviewing or submitting PRs, check:

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- [ ] No `unsafe` code
- [ ] Async only in shell, sync in core
- [ ] `thiserror` in core, `anyhow` in shell
- [ ] Domain types use semantic newtypes
- [ ] State transitions are validated
- [ ] Tests cover happy path + error cases
- [ ] Property tests for invariants
- [ ] Documentation updated
- [ ] Manual testing completed

---

## Recognition

Contributors are recognized in:
- Release notes
- Contributors section of README
- Git commit history (with proper attribution)

Thank you for contributing to ZJJ! Every contribution helps make parallel development safer and more productive.

---

## License

By contributing, you agree that your contributions will be licensed under the **MIT License**.

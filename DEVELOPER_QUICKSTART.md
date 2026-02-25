# Developer Quickstart

Get productive in 15 minutes. This guide covers the essentials for contributing to ZJJ.

---

## 5-Minute Setup

### Automated Setup (Recommended)

```bash
# Clone and setup
git clone https://github.com/lprior-repo/zjj.git
cd zjj
./scripts/dev-setup.sh
```

The script:
- Checks prerequisites (Rust 1.80+, Moon, JJ)
- Installs missing dependencies
- Runs initial build and tests
- Prints next steps

**Non-interactive mode**: `./scripts/dev-setup.sh --yes`

**Check prerequisites only**: `./scripts/dev-setup.sh --check`

### Manual Setup

If automation fails, install manually:

```bash
# 1. Install dependencies
# - Moon: https://moonrepo.dev/docs/install
# - JJ: https://github.com/martinvonz/jj#installation
# - Rust 1.80+: https://rustup.rs/

# 2. Verify setup
moon run :check
```

---

## Understanding the Codebase (10 Minutes)

### Architecture: Functional Core, Imperative Shell

```
┌─────────────────────────────────────────┐
│  SHELL (crates/zjj/)                    │  ← I/O, async, external APIs
│  - CLI parsing (clap)                   │
│  - Database (sqlx)                      │
│  - Command execution                    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│  CORE (crates/zjj-core/)                │  ← Pure domain logic
│  - Domain types (semantic newtypes)     │
│  - State transitions                    │
│  - Validation                           │
│  - Coordination algorithms              │
└─────────────────────────────────────────┘
```

### Key Directories

| Directory | Purpose |
|-----------|---------|
| `crates/zjj-core/src/domain/` | Domain primitives, aggregates, value objects |
| `crates/zjj-core/src/beads/` | Issue tracking domain |
| `crates/zjj-core/src/coordination/` | Queue and coordination logic |
| `crates/zjj-core/src/cli_contracts/` | CLI boundary contracts |
| `crates/zjj/src/commands/` | CLI command handlers |
| `crates/zjj/src/cli/` | CLI parsing and routing |
| `tests/` | Integration tests |

### Core Concepts

**Domain-Driven Design (DDD)**:
- **Aggregates**: `Session`, `Workspace`, `Bead`, `QueueEntry`
- **Value Objects**: `SessionName`, `BeadId`, `WorkspaceState`
- **Repository Pattern**: Abstract persistence, domain doesn't know storage

**Functional Rust**:
- **Zero Unwrap Law**: No `unwrap()`, `expect()`, `panic!()` (compiler-enforced)
- **Parse at Boundaries**: Validate once, use semantic types throughout
- **Pure Core**: Domain logic is deterministic, no I/O

**The Core 6 Libraries** (use in this order):
1. `itertools` - Iterator pipelines
2. `tap` - Pipeline observation
3. `rpds` - Persistent state
4. `thiserror` - Domain errors (core)
5. `anyhow` - Boundary errors (shell)
6. `futures-util` - Async streams

---

## Making Your First Code Change

### 1. Pick a Task

**Good first issues**:
- Add a new domain type
- Implement a state machine
- Add CLI output formatting
- Write property tests

### 2. Development Loop

```bash
# Fastest: format + type check (6-7ms with cache)
moon run :quick

# Run tests
moon run :test

# Auto-fix formatting
moon run :fmt-fix

# Full CI pipeline
moon run :ci
```

### 3. Example: Add a Domain Type

```rust
// crates/zjj-core/src/domain/my_type.rs

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyTypeError {
    #[error("invalid value: {0}")]
    Invalid(String),
}

/// A validated semantic type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MyType(String);

impl MyType {
    pub const MAX_LENGTH: usize = 100;

    pub fn new(value: impl Into<String>) -> Result<Self, MyTypeError> {
        let value = value.into();
        if value.is_empty() || value.len() > Self::MAX_LENGTH {
            return Err(MyTypeError::Invalid("validation failed".into()));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid() {
        assert!(MyType::new("valid").is_ok());
    }

    #[test]
    fn test_empty() {
        assert!(MyType::new("").is_err());
    }
}
```

### 4. Manual Testing

```bash
# Build release binary
moon run :build

# Test your changes
./target/release/zjj add test-session
./target/release/zjj list
./target/release/zjj status test-session
```

---

## Running Tests

### Three Types of Tests

**1. Unit Tests** (in same file as code)
```bash
moon run :test -- my_module
```

**2. Property Tests** (invariant testing with proptest)
```bash
moon run :test -- status_properties
moon run :test -- session_properties
```

**3. Integration Tests** (CLI end-to-end)
```bash
moon run :test -- session_feature
moon run :test -- queue_feature
```

### Test Requirements

- All PRs must pass `moon run :ci`
- New features require property tests for invariants
- Bug fixes require regression tests

---

## Submitting a PR

### Before Submitting

```bash
# 1. Run quality gates
moon run :ci

# 2. Format code
moon run :fmt-fix

# 3. Manual testing
moon run :build
./target/release/zjj <your-command>

# 4. Update docs if needed
```

### PR Checklist

- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- [ ] No `unsafe` code
- [ ] Async only in shell, sync in core
- [ ] `thiserror` in core, `anyhow` in shell
- [ ] Domain types use semantic newtypes
- [ ] State transitions are validated
- [ ] Tests cover happy path + error cases
- [ ] Property tests for invariants
- [ ] Manual testing completed

### PR Template

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
- [ ] Updated relevant docs
```

---

## Essential Commands (Cheat Sheet)

```bash
# Development
moon run :quick              # fmt + check (6-7ms cached)
moon run :test               # Run all tests
moon run :test -- <name>     # Run specific test
moon run :fmt-fix            # Auto-fix formatting
moon run :build              # Release build
moon run :ci                 # Full CI pipeline

# Git
git status                   # See changes
git diff                     # View diff
git add <file>               # Stage file
git commit -m "feat: msg"    # Commit (use conventional commits)
git push                     # Push changes

# Binary
./target/release/zjj --help  # Show help
./target/release/zjj <cmd>   # Run command
```

---

## Where to Get Help

### Documentation

- **[README.md](README.md)** - Project overview
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Full contribution guide
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design
- **[DOMAIN_TYPES_GUIDE.md](DOMAIN_TYPES_GUIDE.md)** - Domain types reference
- **[ERROR_HANDLING_GUIDE.md](ERROR_HANDLING_GUIDE.md)** - Error patterns
- **[Full Docs](https://lprior-repo.github.io/zjj/)** - Complete documentation

### Code Examples

- `crates/zjj-core/src/domain/` - Domain type examples
- `crates/zjj-core/src/beads/domain.rs` - DDD patterns
- `tests/` - Integration test examples

### Community

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Questions and ideas
- **AGENTS.md** - Agent development guidelines

---

## Common First Tasks

1. **Fix a bug with a failing test** - Great way to learn the codebase
2. **Add property tests** - Improve coverage with proptest
3. **Add a CLI command** - Practice shell layer patterns
4. **Create a domain type** - Learn semantic validation
5. **Improve error messages** - Enhance user experience

---

## Key Principles Recap

**Non-Negotiable**:
1. **Zero Unwrap** - Use `Result<T, E>`, `?`, `match`, `map`, `and_then`
2. **Parse at Boundaries** - Validate once, use semantic types
3. **Pure Core** - No I/O in domain logic
4. **DDD Patterns** - Model domain explicitly

**File Header (Required)**:
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

**Important**:
- **ALWAYS use `moon run`** instead of `cargo` commands
- Core uses `thiserror`, shell uses `anyhow`
- Write tests for everything
- Manual test before submitting

---

## Next Steps

1. Run `./scripts/dev-setup.sh` if you haven't already
2. Read [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines
3. Explore [ARCHITECTURE.md](ARCHITECTURE.md) to understand the system
4. Pick a "good first issue" and make your first contribution!

**Welcome to ZJJ! We're glad you're here.**

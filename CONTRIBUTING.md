# Contributing to ZJJ

Thank you for your interest in contributing to ZJJ! This guide will help you get started with development, understand our workflows, and submit high-quality contributions.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Setup](#development-setup)
3. [Code Standards](#code-standards)
4. [Build System](#build-system)
5. [Testing](#testing)
6. [Development Workflow](#development-workflow)
7. [Pull Request Process](#pull-request-process)
8. [Common Tasks](#common-tasks)
9. [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

#### Required

- **Rust Nightly** (managed via `rust-toolchain.toml`)
  ```bash
  rustup toolchain install nightly
  rustc --version  # Verify installation
  ```

- **Moon Build System** (required for all builds)
  ```bash
  brew install moonrepo/tools/moon  # macOS
  # or download from https://moonrepo.dev
  ```

- **JJ (Jujutsu) 0.8.0+**
  ```bash
  jj --version  # Must support workspace commands
  ```

- **Zellij 0.35.1+**
  ```bash
  zellij --version  # Must support KDL layouts and go-to-tab-name
  ```

#### Optional but Recommended

- **Beads** - Issue tracking integration
  ```bash
  # Install from https://github.com/beadorg/beads
  bd --version
  ```

- **mise** - Development environment management
  ```bash
  curl https://mise.run | sh
  mise install  # Installs all project tools
  ```

- **sccache** - Compiler cache for faster builds
  ```bash
  cargo install sccache
  ```

### Initial Setup

1. **Fork and Clone**
   ```bash
   # Fork the repository on GitHub first
   git clone https://github.com/YOUR_USERNAME/zjj.git
   cd zjj
   ```

2. **Initialize JJ Repository**
   ```bash
   # Convert Git repo to JJ
   jj init --git

   # Or if already initialized
   jj git fetch --all-remotes
   ```

3. **Install Dependencies**
   ```bash
   # Moon will handle Rust dependencies
   moon setup
   ```

4. **Verify Installation**
   ```bash
   # Run quick validation
   moon run :quick

   # Run full test suite
   moon run :test

   # Build release binary
   moon run :build
   ```

## Development Setup

### Project Structure

```
zjj/
├── crates/
│   ├── zjj-core/       # Core library
│   │   ├── src/
│   │   │   ├── error.rs       # Error types
│   │   │   ├── jujutsu.rs     # JJ integration
│   │   │   ├── zellij.rs      # Zellij integration
│   │   │   ├── beads.rs       # Beads integration
│   │   │   └── lib.rs         # Public API
│   │   └── Cargo.toml
│   └── zjj/            # CLI binary
│       ├── src/
│       │   ├── commands/      # Command implementations
│       │   ├── db/            # Database layer
│       │   ├── config.rs      # Configuration
│       │   └── main.rs        # Entry point
│       └── Cargo.toml
├── docs/               # Documentation
├── .github/            # CI/CD workflows
├── .clippy.toml        # Clippy configuration (DO NOT MODIFY)
├── moon.yml            # Moon task definitions
└── rust-toolchain.toml # Rust version specification
```

### Environment Configuration

Create a local development configuration:

```bash
# Set environment variables (optional)
export RUST_LOG=debug          # Enable debug logging
export ZJJ_DB_PATH=".zjj/sessions.db"  # Custom database path
export RUST_BACKTRACE=1        # Full backtraces
```

## Code Standards

ZJJ follows strict Rust standards to ensure safety and reliability.

### The Zero Unwrap Law

**ABSOLUTE RULE: No panics, unwraps, or unsafe code allowed.**

These will cause compilation errors:

```rust
❌ .unwrap()        // Forbidden
❌ .expect()        // Forbidden
❌ panic!()         // Forbidden
❌ unsafe { }       // Forbidden
❌ unimplemented!() // Forbidden
❌ todo!()          // Forbidden
```

### Required Patterns

#### 1. Always Return `Result<T, Error>`

```rust
// ✅ Correct
fn operation(input: &str) -> Result<Output> {
    validate(input)?;
    Ok(transform(input))
}

// ❌ Wrong - doesn't handle errors
fn operation(input: &str) -> Output {
    validate(input).unwrap();  // COMPILE ERROR
    transform(input)
}
```

#### 2. Use the `?` Operator for Error Propagation

```rust
fn process_data(path: &str) -> Result<Data> {
    let content = std::fs::read_to_string(path)?;
    let parsed = parse_json(&content)?;
    validate(&parsed)?;
    Ok(parsed)
}
```

#### 3. Handle Options Safely

```rust
// ✅ Correct
if let Some(value) = maybe_value {
    use_value(value);
}

// ✅ Also correct
maybe_value.map(use_value).unwrap_or_default()

// ❌ Wrong
let value = maybe_value.unwrap();  // COMPILE ERROR
```

#### 4. Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MyError>;
```

### Documentation Requirements

All public items MUST be documented:

```rust
/// Brief description of the function.
///
/// Longer description with details about behavior.
///
/// # Arguments
///
/// * `input` - Description of the input parameter
///
/// # Errors
///
/// Returns an error if:
/// - Input is empty
/// - Validation fails
///
/// # Examples
///
/// ```ignore
/// let result = my_function("valid_input")?;
/// assert!(result.is_ok());
/// ```
pub fn my_function(input: &str) -> Result<Output> {
    // implementation
}
```

### Clippy Configuration

The project has strict clippy rules configured in `.clippy.toml`:

**CRITICAL: NEVER modify `.clippy.toml` or lint configurations.**

If clippy reports warnings, fix the **code**, not the lint rules.

## Build System

### Moon Commands

**ALWAYS use Moon. NEVER use raw cargo commands.**

```bash
# ✅ Correct
moon run :quick      # Fast lint check
moon run :test       # Run tests
moon run :build      # Release build
moon run :ci         # Full CI pipeline

# ❌ Wrong - DO NOT USE
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
```

### Available Tasks

| Command | Description | Duration |
|---------|-------------|----------|
| `moon run :quick` | Format + lint check | ~10-15s |
| `moon run :test` | Run all tests | ~30-45s |
| `moon run :build` | Release build | ~45-90s |
| `moon run :ci` | Full CI pipeline | ~60-120s |
| `moon run :fmt-fix` | Auto-fix formatting | ~5s |
| `moon run :check` | Fast type check | ~10s |

### What Each Command Does

#### `:quick` - Fast Validation
```
1. Format check (cargo fmt --check)
2. Clippy lint (-D warnings)
```
Use before committing.

#### `:test` - Test Suite
```
1. Run all unit tests
2. Run integration tests
3. Run doc tests
```
Use after making logic changes.

#### `:build` - Release Build
```
1. Build with optimizations (--release)
2. Copy binaries to bin/
3. Verify binary exists
```
Use to create production binaries.

#### `:ci` - Full Pipeline
```
1. Format check
2. Clippy lint
3. Run tests
4. Build release
5. Validate YAML schemas
```
Use before pushing or creating PRs.

## Testing

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_success() {
        let result = operation("valid_input");
        assert!(result.is_ok());
    }

    #[test]
    fn test_operation_failure() {
        let result = operation("invalid_input");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_type() {
        match operation("invalid") {
            Err(Error::Validation(_)) => {}, // ✓ Expected
            other => panic!("unexpected: {:?}", other),
        }
    }
}
```

### Testing Guidelines

1. **Test both success AND failure paths**
   ```rust
   #[test]
   fn test_success() {
       assert!(operation("valid").is_ok());
   }

   #[test]
   fn test_failure() {
       assert!(operation("invalid").is_err());
   }
   ```

2. **Use pattern matching for specific errors**
   ```rust
   #[test]
   fn test_specific_error() {
       match operation("") {
           Err(Error::ValidationError(msg)) => {
               assert_eq!(msg, "input cannot be empty");
           }
           other => panic!("unexpected result: {:?}", other),
       }
   }
   ```

3. **Organize tests logically**
   ```rust
   #[cfg(test)]
   mod tests {
       mod success_cases { /* ... */ }
       mod error_cases { /* ... */ }
       mod edge_cases { /* ... */ }
   }
   ```

### Running Tests

```bash
# Run all tests
moon run :test

# Run specific test
cargo test --lib test_name

# Run with output
cargo test -- --nocapture

# Run tests for specific crate
cargo test -p zjj-core
```

## Development Workflow

### Daily Workflow with Beads + JJ + Moon

#### 1. Start Work

```bash
# View available issues
bd list

# Claim an issue
bd claim zjj-123

# Fetch latest changes
jj git fetch --all-remotes
```

#### 2. Make Changes

```bash
# Create a new change
jj new

# Edit files (automatically tracked by JJ)
vim crates/zjj-core/src/lib.rs

# Check status
jj status
jj diff

# Test locally
moon run :test
```

#### 3. Commit Changes

```bash
# Describe change using conventional commits
jj describe -m "feat: add new feature

- Implementation detail 1
- Implementation detail 2

Closes zjj-123"

# Start next change
jj new
```

#### 4. Pre-Push Validation

```bash
# Run full CI pipeline
moon run :ci

# If all pass, push
jj git push

# Verify
jj log -r @
```

#### 5. Close Issue

```bash
# Mark complete
bd complete zjj-123 --commit-hash <hash>
```

### Conventional Commits

Use these prefixes for commit messages:

```
feat:     New feature
fix:      Bug fix
refactor: Code refactoring (no behavior change)
chore:    Build, dependencies, tooling
docs:     Documentation changes
test:     Test additions/modifications
perf:     Performance improvements
```

Example:
```bash
jj describe -m "feat: add session validation

- Implement validation builder
- Add comprehensive error types
- Add unit tests for validation logic

Closes zjj-456"
```

## Pull Request Process

### Before Creating a PR

1. **Run Full CI Pipeline**
   ```bash
   moon run :ci
   ```
   All checks must pass before submitting.

2. **Update Documentation**
   - Add/update inline code documentation
   - Update relevant docs/ files if needed
   - Add examples for new features

3. **Add Tests**
   - Unit tests for new functions
   - Integration tests for new features
   - Test both success and failure cases

4. **Check for Breaking Changes**
   - Document any breaking changes
   - Update version following semantic versioning
   - Add migration guide if needed

### Creating a Pull Request

1. **Push Your Changes**
   ```bash
   # Ensure changes are committed
   jj log -r origin/main..@

   # Push to your fork
   jj git push
   ```

2. **Open PR on GitHub**
   - Use a descriptive title
   - Reference related issues (e.g., "Closes zjj-123")
   - Provide clear description of changes
   - Include test plan
   - List any breaking changes

3. **PR Title Format**
   ```
   feat: Add session validation
   fix: Resolve database race condition
   docs: Update contributing guidelines
   ```

### PR Template

```markdown
## Summary

Brief description of what this PR does.

## Changes

- Change 1
- Change 2
- Change 3

## Test Plan

- [ ] Added unit tests
- [ ] Added integration tests
- [ ] Tested manually with: ...
- [ ] `moon run :ci` passes

## Breaking Changes

List any breaking changes and migration steps.

## Related Issues

Closes zjj-123
```

### Review Process

1. **Automated Checks**
   - CI pipeline must pass
   - All tests must succeed
   - No clippy warnings
   - Code coverage maintained

2. **Code Review**
   - At least one approval required
   - Address all review comments
   - Ensure Zero Unwrap Law compliance

3. **Merge**
   - Squash commits if requested
   - Update changelog
   - Delete branch after merge

## Common Tasks

### Adding a New Command

1. **Create command module**
   ```bash
   vim crates/zjj/src/commands/mycommand.rs
   ```

2. **Implement command**
   ```rust
   use zjj_core::Result;

   pub fn execute(args: Args) -> Result<()> {
       // Implementation
       Ok(())
   }
   ```

3. **Add to CLI**
   ```rust
   // In main.rs or commands/mod.rs
   mod mycommand;

   match cli.command {
       Commands::MyCommand(args) => mycommand::execute(args)?,
       // ...
   }
   ```

4. **Add tests**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_mycommand_success() {
           // Test implementation
       }
   }
   ```

5. **Update documentation**
   ```bash
   # Update README.md with new command
   # Add command to docs/
   ```

### Adding a New Error Type

```rust
// In zjj-core/src/error.rs
#[derive(Error, Debug)]
pub enum Error {
    // ... existing errors ...

    #[error("my new error: {0}")]
    MyNewError(String),
}
```

### Updating Dependencies

```bash
# Update Cargo.toml
vim Cargo.toml

# Check for issues
cargo tree

# Run tests
moon run :test

# Run security audit
cargo audit
```

### Running Specific Tests

```bash
# Single test
cargo test test_name

# Tests in a module
cargo test commands::add

# Tests for a crate
cargo test -p zjj-core

# Integration tests only
cargo test --test integration_test
```

## Troubleshooting

### Build Issues

#### "moon not found"
```bash
# Install Moon
brew install moonrepo/tools/moon

# Verify installation
moon --version
```

#### "Rust nightly not found"
```bash
# Install nightly toolchain
rustup toolchain install nightly

# Verify
rustc --version
```

#### "Clippy errors"
```bash
# Fix the code, not the lint rules
# Common fixes:
cargo fmt              # Auto-format
moon run :quick        # Check lints
```

### Test Failures

#### "Test panicked"
```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run single test with output
cargo test test_name -- --nocapture
```

#### "Database locked"
```bash
# Clean up test databases
rm -rf /tmp/test_*.db
cargo test
```

### JJ Issues

#### "Not a JJ repository"
```bash
# Initialize JJ
jj init --git

# Or clone with JJ
jj clone https://github.com/...
```

#### "Can't push"
```bash
# Fetch first
jj git fetch --all-remotes

# Then push
jj git push
```

### Moon Issues

#### "Task failed"
```bash
# Run with debug logging
moon run :ci --log debug

# Check task definition
moon dump :test
```

#### "Cache not working"
```bash
# Clear cache
moon clean

# Rebuild
moon run :build
```

## Getting Help

- **Documentation**: See [docs/INDEX.md](/home/lewis/src/zjj/docs/INDEX.md) for all guides
- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions for questions
- **Code of Conduct**: Be respectful and constructive

### Key Documentation

| Document | Purpose |
|----------|---------|
| [00_START_HERE.md](/home/lewis/src/zjj/docs/00_START_HERE.md) | Quick onboarding |
| [11_ARCHITECTURE.md](/home/lewis/src/zjj/docs/11_ARCHITECTURE.md) | System architecture |
| [05_RUST_STANDARDS.md](/home/lewis/src/zjj/docs/05_RUST_STANDARDS.md) | Coding standards |
| [02_MOON_BUILD.md](/home/lewis/src/zjj/docs/02_MOON_BUILD.md) | Build system details |
| [03_WORKFLOW.md](/home/lewis/src/zjj/docs/03_WORKFLOW.md) | Daily workflow |
| [07_TESTING.md](/home/lewis/src/zjj/docs/07_TESTING.md) | Testing guide |

## Code Review Checklist

Before submitting a PR, ensure:

- [ ] No `unwrap()` calls (enforced by compiler)
- [ ] No `expect()` calls (enforced by compiler)
- [ ] No `panic!()` calls (enforced by compiler)
- [ ] No `unsafe { }` (enforced by compiler)
- [ ] All public items documented
- [ ] All errors return `Result<T, Error>`
- [ ] Tests cover success and failure paths
- [ ] `moon run :ci` passes
- [ ] No clippy warnings
- [ ] Conventional commit format used
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)

## License

By contributing to ZJJ, you agree that your contributions will be licensed under the MIT License.

---

**Welcome to the ZJJ community!** We're excited to have you contribute.

If you have questions, please open an issue or start a discussion on GitHub.

# Coding Conventions

**Analysis Date:** 2026-01-16

## Naming Patterns

**Files:**
- Snake_case for Rust source files: `error_codes.rs`, `json_schema.rs`, `session_lifecycle.rs`
- Test files use `test_` prefix: `test_error_display.rs`, `test_session_lifecycle.rs`
- Integration tests in `tests/` directory, unit tests in `src/` modules with `#[cfg(test)]`
- Common test utilities: `tests/common/mod.rs`
- Benchmarks use descriptive names: `config_operations.rs`, `validation.rs`

**Functions:**
- Snake_case for all functions: `validate_session_name()`, `create_config_files()`, `load_config()`
- Builder methods use `with_` prefix: `with_name()`, `with_env()`
- Async functions marked with `async` keyword
- Command handlers: `run()`, `run_with_options()`, `run_with_flags()`

**Variables:**
- Snake_case: `workspace_dir`, `session_name`, `config_path`
- Descriptive names preferred over abbreviations
- Boolean flags use `is_` prefix: `is_tty()`, `is_inside_zellij()`
- Mutable variables explicitly marked with `mut`

**Types:**
- PascalCase for structs, enums, traits: `SessionStatus`, `AddOptions`, `TestHarness`
- Type aliases use PascalCase: `FallibleTransform`, `Validator`
- Enum variants use PascalCase: `SessionStatus::Active`, `Error::InvalidConfig`
- Const generics and associated types follow PascalCase

## Code Style

**Formatting:**
- Tool: `rustfmt` with custom configuration in `rustfmt.toml`
- Max line width: 100 characters
- 4 spaces for indentation (no tabs)
- Unix line endings (`\n`)
- Trailing comma for vertical layouts
- Use field init shorthand: `Config { name }` not `Config { name: name }`
- Use try shorthand: `?` operator preferred

**Linting:**
- Tool: `clippy` with strict configuration in `.clippy.toml` and `Cargo.toml`
- All warnings treated as errors in CI
- Zero-tolerance rules enforced at compile time:
  - `unwrap_used = "forbid"`
  - `expect_used = "forbid"`
  - `panic = "forbid"`
  - `todo = "forbid"`
  - `unimplemented = "forbid"`
  - `unsafe_code = "forbid"`
- Complexity thresholds:
  - Cognitive complexity: 20
  - Max function arguments: 5
  - Type complexity: 250
- All lints from `pedantic`, `nursery`, `correctness`, `suspicious` enabled

## Import Organization

**Order:**
1. Standard library imports: `use std::fs;`, `use std::path::PathBuf;`
2. External crate imports: `use anyhow::{Context, Result};`, `use serde::{Deserialize, Serialize};`
3. Internal crate imports: `use crate::{Error, Result};`, `use zjj_core::config::Config;`

**Pattern:**
```rust
use std::{fs, path::PathBuf, process};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use zjj_core::jj;

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij},
    commands::{check_prerequisites, get_session_db},
    session::{validate_session_name, SessionStatus},
};
```

**Path Aliases:**
- No path aliases configured
- Relative imports from `crate::` for same-crate modules
- Full path imports for cross-crate dependencies

## Error Handling

**Patterns:**
- All fallible operations return `Result<T, Error>`
- Custom error type in `crates/zjj-core/src/error.rs`
- Error variants are descriptive enums: `Error::InvalidConfig(String)`, `Error::DatabaseError(String)`
- Use `?` operator for error propagation
- Use `.context()` from `anyhow` for adding context to errors
- Use `.ok_or_else()` for `Option` to `Result` conversion
- Use `.and_then()`, `.map()` for Result chaining
- Use `.unwrap_or_default()`, `.unwrap_or_else()` for safe unwrapping with fallbacks
- Never use `.unwrap()` or `.expect()` - enforced by clippy

**Example:**
```rust
pub fn build(self) -> Result<Config> {
    self.name
        .ok_or_else(|| Error::InvalidConfig("name is required".into()))
        .and_then(|name| {
            if name.is_empty() {
                Err(Error::InvalidConfig("name cannot be empty".into()))
            } else {
                Ok(Config { name })
            }
        })
}
```

**From Trait Implementations:**
- Implement `From<T>` for common error conversions: `std::io::Error`, `serde_json::Error`, `toml::de::Error`, `sqlx::Error`
- Custom error display via `fmt::Display` trait
- Implement `std::error::Error` trait for all error types

## Logging

**Framework:** `tracing` crate with `tracing-subscriber`

**Patterns:**
- Logging initialized in `main.rs`:
```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    )
    .with_writer(std::io::stderr)
    .init();
```
- Log to stderr, not stdout (stdout reserved for structured output)
- Use `eprintln!` for error messages in non-JSON mode
- JSON mode outputs structured errors instead of logs

## Comments

**When to Comment:**
- Module-level documentation (crate/module purpose): `//! # ZJJ Core`
- Public API documentation: `/// Create a new config builder.`
- Complex algorithms or non-obvious logic
- Tests include descriptive comments explaining what they verify
- Avoid comments that restate the code

**JSDoc/TSDoc:**
- Use Rust doc comments (`///`) for public items
- Use inner doc comments (`//!`) for module/crate level docs
- Doc comments support Markdown formatting
- Include examples in doc comments when helpful
- Valid identifiers configured in `.clippy.toml`: JSON, API, Jujutsu, Zellij, etc.

## Function Design

**Size:** Keep functions focused and under cognitive complexity threshold of 20

**Parameters:**
- Maximum 5 parameters (enforced by clippy)
- Use option structs for commands with many flags: `AddOptions`, `RemoveOptions`, `SyncOptions`
- Use builder pattern for complex construction: `ConfigBuilder::new().with_name("test").build()`

**Return Values:**
- Return `Result<T, Error>` for fallible operations
- Use `#[must_use]` attribute for functions where ignoring return value is likely a bug
- Use `Option<T>` for values that may not exist
- Return owned values or references, not mixed approaches

**Attributes:**
- `#[must_use]` for builder methods and functions with important return values
- `#[allow(dead_code)]` sparingly, only in test utilities
- `#[allow(clippy::struct_excessive_bools)]` for option structs with many flags
- `#[deny(clippy::unwrap_used)]` at module level to enforce zero-unwrap rule

## Module Design

**Exports:**
- Re-export commonly used types at crate root: `pub use error::Error;`, `pub use result::Result;`
- Keep internal implementation details private
- Public API surfaces in `lib.rs`

**Barrel Files:**
- Commands module uses `mod.rs` to expose all command functions
- Test common utilities in `tests/common/mod.rs`

**Module Structure:**
```
crates/
  zjj-core/        # Core library
    src/
      lib.rs       # Public API surface
      error.rs     # Error types
      result.rs    # Result extensions
      functional.rs # Functional utilities
  zjj/             # CLI binary
    src/
      main.rs      # Entry point
      cli.rs       # CLI utilities
      commands/    # Command implementations
        mod.rs     # Re-exports
        add.rs     # Individual commands
```

## Functional Rust Patterns

**Philosophy:** Strictly functional approach with zero panics

**Patterns:**
- Use combinators over imperative loops: `.map()`, `.filter()`, `.fold()`
- Use `try_fold` for fallible accumulation
- Use `and_then` for chaining Results
- Custom functional utilities in `crates/zjj-core/src/functional.rs`:
  - `validate_all()` - run multiple validators
  - `compose_result()` - compose Result-returning functions
  - `apply_transforms()` - apply sequence of transformations
  - `fold_result()` - fold with fallible function
  - `map_result()` - map with fallible function
  - `filter_result()` - filter with fallible predicate

**Example:**
```rust
pub fn validate_all<T, F>(item: &T, validators: &[F]) -> Result<()>
where
    F: Fn(&T) -> Result<()>,
{
    validators
        .iter()
        .try_fold((), |(), validator| validator(item))
}
```

## Special Considerations

**Moon Build System:**
- NEVER use raw `cargo` commands
- Always use `moon run :task` for build operations
- Available tasks: `:quick`, `:test`, `:build`, `:ci`, `:fmt-fix`, `:check`
- Tasks defined in `.moon/tasks.yml`

**Async Runtime:**
- Use `tokio` runtime
- Async functions throughout CLI commands
- Runtime created in `main()` with proper error handling

**JSON Output Mode:**
- Many commands support `--json` flag
- JSON errors use structured format via `json_output` module
- Regular output goes to stdout, errors to stderr
- In JSON mode, errors printed as JSON to stdout

---

*Convention analysis: 2026-01-16*

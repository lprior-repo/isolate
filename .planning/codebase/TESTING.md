# Testing Patterns

**Analysis Date:** 2026-01-16

## Test Framework

**Runner:**
- `cargo test` (Rust built-in test framework)
- Config: Tests use standard Rust test attributes

**Assertion Library:**
- Standard Rust `assert!`, `assert_eq!`, `assert!` macros
- Custom assertion methods in test harness

**Run Commands:**
```bash
moon run :test              # Run all tests via Moon
moon run :test-doc          # Run documentation tests
cargo test --workspace      # Direct cargo (discouraged - use Moon)
```

## Test File Organization

**Location:**
- Unit tests: Co-located in `#[cfg(test)] mod tests` within source files
- Integration tests: Separate `tests/` directory at crate root
- Pattern: `crates/zjj/tests/*.rs` for integration tests
- Common utilities: `crates/zjj/tests/common/mod.rs`

**Naming:**
- Integration test files: `test_*.rs` prefix
  - `test_session_lifecycle.rs`
  - `test_error_display.rs`
  - `test_cli_parsing.rs`
  - `test_init.rs`
  - `test_tty_detection.rs`
- End-to-end tests: `e2e_*.rs` prefix
  - `e2e_mvp_commands.rs`
- Error tests: `error_*.rs` prefix
  - `error_recovery.rs`
- Edge case tests: `command_edge_cases.rs`

**Structure:**
```
crates/zjj/
├── src/
│   └── *.rs           # Unit tests in #[cfg(test)] modules
├── tests/
│   ├── common/
│   │   └── mod.rs     # Shared test utilities
│   ├── test_*.rs      # Integration tests
│   └── e2e_*.rs       # End-to-end tests
└── benches/
    └── *.rs           # Performance benchmarks
```

## Test Structure

**Suite Organization:**
```rust
//! Integration tests for session lifecycle
//!
//! Tests the complete workflow: init → add → list → status → remove

mod common;

use common::TestHarness;

// ============================================================================
// Session Creation (add command)
// ============================================================================

#[test]
fn test_add_creates_session() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test-session", "--no-open"]);
    harness.assert_workspace_exists("test-session");
}
```

**Patterns:**
- Test functions named `test_*` with descriptive suffixes
- Use doc comments to explain test purpose
- Section tests with comments for clarity
- Conditional test execution when external dependencies unavailable
- Use `TestHarness::try_new()` pattern to skip tests gracefully

**Unit Test Pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_success() {
        let result = ConfigBuilder::new().with_name("test").build();
        assert!(result.is_ok());

        let name_matches = result.map(|c| c.name == "test").unwrap_or_else(|_| false);
        assert!(name_matches);
    }

    #[test]
    fn test_config_builder_missing_name() {
        let result = ConfigBuilder::new().build();
        assert!(result.is_err());
    }
}
```

## Mocking

**Framework:** No dedicated mocking framework - use test harness and real commands

**Patterns:**
- Integration tests use real JJ repository via `TestHarness`
- Tests create temporary directories with `TempDir`
- Environment variables set via `Command::env()` or `jjz_with_env()`
- No mocking of internal functions - tests exercise real code paths
- Skip tests that require unavailable external dependencies

**Test Isolation:**
```rust
pub struct TestHarness {
    _temp_dir: TempDir,              // Auto-cleanup on drop
    pub repo_path: PathBuf,
    jjz_bin: PathBuf,
}

impl TestHarness {
    pub fn try_new() -> Option<Self> {
        Self::new().ok()
    }
}
```

**What NOT to Mock:**
- Internal functions - test them directly
- File system operations - use real temp directories
- Database operations - use real SQLite with temp files
- JJ commands - use real JJ installation (skip test if unavailable)

## Fixtures and Factories

**Test Data:**
```rust
/// Create a temp directory with config files
fn create_config_files() -> TempDir {
    let dir = TempDir::new().unwrap_or_else(|e| {
        eprintln!("Failed to create temp dir: {e}");
        std::process::exit(1);
    });

    let config_content = r#"
workspace_dir = "../{repo}__workspaces"
main_branch = "main"
[watch]
enabled = true
"#;

    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, config_content).map_or_else(
        |e| {
            eprintln!("Failed to write config: {e}");
            std::process::exit(1);
        },
        |()| (),
    );

    dir
}
```

**Location:**
- Fixture functions defined inline in test files
- Reusable test utilities in `tests/common/mod.rs`
- Benchmark fixtures in benchmark files (`benches/`)

**TestHarness Utilities:**
```rust
// Create files
harness.create_file("README.md", "# Test")?;

// Write config
harness.write_config("workspace_dir = '/tmp'")?;

// Run commands
let result = harness.jjz(&["add", "test"]);
let result = harness.jj(&["workspace", "list"]);

// Assertions
harness.assert_success(&["init"]);
harness.assert_failure(&["add", "-invalid"], "Invalid session name");
harness.assert_workspace_exists("test-session");
```

## Coverage

**Requirements:** No explicit coverage target enforced

**View Coverage:**
```bash
# Coverage not currently configured in Moon tasks
# Would typically use cargo-tarpaulin or cargo-llvm-cov
```

**Coverage Gaps:**
- Interactive prompts (tested manually)
- TTY detection (has dedicated test)
- Zellij integration (requires real Zellij session)

## Test Types

**Unit Tests:**
- Scope: Individual functions and methods
- Location: Co-located with source in `#[cfg(test)] mod tests`
- Examples:
  - `test_config_builder_success()` - validates ConfigBuilder
  - `test_validate_name_empty()` - validates name validation
  - `test_error_display_invalid_config()` - validates error formatting
- Run with: `moon run :test`

**Integration Tests:**
- Scope: Command execution end-to-end
- Location: `crates/zjj/tests/test_*.rs`
- Examples:
  - `test_add_creates_session()` - full add command workflow
  - `test_list_shows_multiple_sessions()` - list command output
  - `test_remove_deletes_session()` - remove command cleanup
- Use `TestHarness` for setup and execution
- Run with: `moon run :test`

**E2E Tests:**
- Scope: Complete user workflows across multiple commands
- Location: `crates/zjj/tests/e2e_*.rs`
- Examples:
  - `test_complete_session_lifecycle()` - init → add → list → status → remove
  - `test_multiple_sessions_lifecycle()` - concurrent session management
- Use `#[serial]` attribute for tests that can't run in parallel
- Run with: `moon run :test`

**Benchmarks:**
- Framework: `criterion` crate
- Location: `crates/zjj/benches/`
- Examples:
  - `config_operations.rs` - config parsing/merging performance
  - `validation.rs` - session name validation performance
- Run with: `cargo bench --bench <name>`

## Common Patterns

**Async Testing:**

Due to conflict between `#[tokio::test]` macro and `#![deny(clippy::expect_used)]`, async unit tests use a helper pattern:

```rust
// Helper function to run async tests
fn run_async<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        panic!("Failed to create tokio runtime for test: {e}");
    });
    runtime.block_on(f());
}

// Use in tests
#[test]
fn test_async_function() {
    run_async(|| async {
        let result = some_async_function().await;
        assert!(result.is_ok());
    });
}
```

**Rationale:** The `#[tokio::test]` macro generates code with `#[allow(clippy::expect_used)]`, which conflicts with the workspace-level `#![deny(clippy::expect_used)]`. The helper function maintains zero-unwrap policy (uses `unwrap_or_else` with explicit panic) while providing clean async test syntax.

Most async code is tested through integration tests which use `TestHarness` and synchronous `Command::output()`.

**Error Testing:**
```rust
#[test]
fn test_add_invalid_session_name_with_spaces() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);

    harness.assert_failure(&["add", "has spaces", "--no-open"], "Invalid session name");
}

#[test]
fn test_error_no_stack_trace() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.jjz(&["list"]);
    assert!(!result.success, "Command should fail without init");

    // Error output should NOT contain stack trace indicators
    assert!(
        !result.stderr.contains("Stack backtrace:"),
        "Error should not contain stack trace"
    );
}
```

**JSON Output Testing:**
```rust
#[test]
fn test_list_json_format() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "test", "--no-open"]);

    let result = harness.jjz(&["list", "--json"]);
    assert!(result.success);

    // Verify it's valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}
```

**Conditional Test Execution:**
```rust
#[test]
fn test_example() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };
    // Test body only runs if JJ is installed
}
```

**Serial Test Execution:**
```rust
use serial_test::serial;

#[test]
#[serial]
fn test_that_cannot_run_parallel() {
    // Tests marked with #[serial] run one at a time
}
```

## Test Utilities (TestHarness)

**Core Methods:**
- `TestHarness::try_new()` - Create test environment, returns `None` if JJ unavailable
- `harness.jjz(&[args])` - Execute jjz command, returns `CommandResult`
- `harness.jj(&[args])` - Execute jj command directly
- `harness.assert_success(&[args])` - Assert command succeeds
- `harness.assert_failure(&[args], "error text")` - Assert command fails with error

**File Operations:**
- `harness.repo_path` - Path to test JJ repository
- `harness.jjz_dir()` - Path to `.jjz` directory
- `harness.workspace_path("session")` - Path to session workspace
- `harness.state_db_path()` - Path to state database
- `harness.create_file("path", "content")` - Create file in repo
- `harness.write_config("toml")` - Write config file
- `harness.read_config()` - Read config file

**Assertions:**
- `harness.assert_workspace_exists("session")` - Assert workspace directory exists
- `harness.assert_workspace_not_exists("session")` - Assert workspace deleted
- `harness.assert_jjz_dir_exists()` - Assert `.jjz` initialized
- `harness.assert_file_exists(path)` - Assert file exists
- `harness.assert_file_not_exists(path)` - Assert file deleted

**CommandResult Methods:**
- `result.assert_stdout_contains("text")` - Assert stdout contains text
- `result.assert_stderr_contains("text")` - Assert stderr contains text
- `result.assert_output_contains("text")` - Assert either output contains text
- `result.success` - Boolean indicating exit success
- `result.exit_code` - Optional exit code
- `result.stdout` - Command stdout as String
- `result.stderr` - Command stderr as String

## Zero-Unwrap Testing

**Rule:** Tests also enforce zero-unwrap policy

**Patterns:**
```rust
// WRONG - unwrap in test
let config = ConfigBuilder::new().with_name("test").build().unwrap();

// RIGHT - assert on Result
let result = ConfigBuilder::new().with_name("test").build();
assert!(result.is_ok());

// RIGHT - use combinators for extraction
let name_matches = result.map(|c| c.name == "test").unwrap_or_else(|_| false);
assert!(name_matches);
```

**Exception:** TestHarness uses `.unwrap_or_else()` internally with explicit error handling:
```rust
let output = Command::new("jj")
    .output()
    .context("Failed to run jj git init")?;

if !output.status.success() {
    anyhow::bail!(
        "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
```

## Special Test Attributes

**Standard:**
- `#[test]` - Mark function as test
- `#[cfg(test)]` - Conditional compilation for test module
- `#[ignore]` - Skip test by default (not commonly used)

**Custom:**
- `#[serial]` - From `serial_test` crate, run test serially
- Used for tests that modify global state or can't run concurrently

**Clippy Allows:**
- `#[allow(dead_code)]` - In `tests/common/mod.rs` for utilities
- `#[allow(clippy::unused_self)]` - For test helper methods
- Zero-unwrap rule still enforced in tests via crate-level `#![deny(clippy::unwrap_used)]`

---

*Testing analysis: 2026-01-16*

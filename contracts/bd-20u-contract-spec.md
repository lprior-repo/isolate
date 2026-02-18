# Contract Specification: bd-20u - Remove colored, console, and similar crates from dependencies

**Bead ID:** bd-20u
**Title:** Remove colored, console, and similar crates from dependencies
**Status:** Design Contract
**Version:** 1.0

## Overview

This contract defines the removal of unused UI-related dependencies from the ZJJ Rust project. The primary focus is on removing the `colored` crate dependency from the standalone audit tool (`tools/audit/`), while ensuring that no similar unused dependencies exist in the main workspace crates.

### Scope of Changes

**Affected Components:**
- `tools/audit/Cargo.toml` - Remove `colored = "2.1"` dependency
- `tools/audit/src/main.rs` - Replace colored output with plain text

**NOT Affected:**
- `crates/zjj/Cargo.toml` - No colored/console dependencies (verified)
- `crates/zjj-core/Cargo.toml` - No colored/console dependencies (verified)
- Root workspace `Cargo.toml` - No colored/console dependencies (verified)

### Key Behavioral Change

**Before:** `zjj-audit` tool uses colored terminal output (bold, cyan, green, red, yellow, dimmed)
**After:** `zjj-audit` tool uses plain text output (no colors or formatting)

The audit tool's **functionality remains identical** - only visual formatting is removed.

## Preconditions

### Global Preconditions (MUST hold before execution)

1. **Audit Tool is Standalone**
   - `tools/audit/` is NOT in the workspace members
   - Verified by checking root `Cargo.toml` - only `members = ["crates/*"]`
   - **Violation:** Changes required to workspace structure

2. **No Colored Dependencies in Workspace**
   - `crates/zjj/Cargo.toml` contains no `colored`, `console`, `termcolor`, `ansi_term`, or similar
   - `crates/zjj-core/Cargo.toml` contains no colored/console dependencies
   - **Status:** VERIFIED - No removal needed from workspace crates

3. **Audit Tool Uses Colored Crate**
   - `tools/audit/Cargo.toml` contains `colored = "2.1"`
   - `tools/audit/src/main.rs` contains `use colored::Colorize`
   - **Status:** VERIFIED - Removal required

4. **No Alternative Terminal Styling Dependencies**
   - No `console`, `termcolor`, `ansi_term`, `colored_json` in workspace
   - **Status:** VERIFIED - Only `colored` needs removal

### Mode-Specific Preconditions

5. **Audit Tool is Not a Core Dependency**
   - `zjj` and `zjj-core` do NOT depend on `zjj-audit`
   - Audit tool is a standalone development utility
   - **Violation:** Changes required to dependency graph

## Postconditions

### Success Postconditions (MUST hold after successful execution)

1. **Colored Crate Removed from Audit Tool**
   - `tools/audit/Cargo.toml` no longer contains `colored` dependency
   - Line 15 (`colored = "2.1"`) is deleted
   - Cargo successfully resolves dependencies without colored

2. **Colored Import Removed from Source**
   - `tools/audit/src/main.rs` no longer contains `use colored::Colorize`
   - Line 10 is deleted
   - Code compiles without colored import

3. **All Colored Method Calls Replaced**
   - `.bold()`, `.cyan()`, `.green()`, `.red()`, `.yellow()`, `.dimmed()` calls removed
   - Output is plain text (no terminal styling)
   - All functional output preserved (status, counts, file paths, line numbers)

4. **Audit Tool Functionality Preserved**
   - All detection logic works identically
   - Exit codes unchanged (0 for clean, 1 for violations)
   - JSON output format preserved (if applicable)
   - Command-line interface unchanged

5. **Workspace Compiles Successfully**
   - `cargo check --workspace` succeeds
   - `cargo build --workspace` succeeds
   - `cargo test --workspace` succeeds (excluding audit tool tests)

### Special Case Postconditions

6. **Audit Tool Builds and Runs**
   - `cargo build --manifest-path tools/audit/Cargo.toml` succeeds
   - `cargo run --manifest-path tools/audit/Cargo.toml` executes
   - Audit output displays correctly (without colors)

7. **No Dependency Conflicts**
   - Cargo.lock updates cleanly
   - No conflicting dependency versions introduced
   - Dependency tree remains acyclic

8. **Emoji Output Preserved**
   - Emoji characters (🔍, ✅, 🔴, ⚠️, 🎯, 📊, 🎓, ❌, 🧪) are preserved
   - These are Unicode, not colored styling, so they remain

## Invariants

### Always True (during and after execution)

1. **Audit Detection Logic Unchanged**
   - Forbidden pattern detection (unwrap, expect, panic, todo, unimplemented, unsafe) unchanged
   - File scanning logic unchanged
   - Violation counting unchanged
   - Test/production code separation unchanged

2. **Output Information Content Preserved**
   - All status messages present
   - All violation details present (file, line, pattern, context)
   - All summary statistics present
   - All grade calculations present

3. **Exit Codes Unchanged**
   - Exit code 0 when production code is clean
   - Exit code 1 when production violations found
   - Exit code 1 when crates directory not found

4. **Command-Line Interface Unchanged**
   - No arguments added or removed
   - Help text unchanged (except color examples if any)
   - Usage pattern unchanged

5. **Error Handling Unchanged**
   - Directory not found error unchanged
   - File read error handling unchanged
   - Regex compilation error handling unchanged

## Error Taxonomy

### Exhaustive Error Variants

```rust
// No new error types introduced
// Existing errors from std::io::Error and regex::Error preserved

pub enum AuditToolError {
    // Compile-time errors (should not occur):
    DependencyResolveError,    // If colored removal breaks dependency tree
    CompilationError,          // If colored syntax not fully removed

    // Runtime errors (unchanged):
    DirectoryNotFoundError,    // crates/ directory missing
    RegexCompilationError,     // Invalid regex pattern
    FileReadError,            // Cannot read .rs file
}
```

### Error Propagation Mapping

```rust
// No changes to error handling
// All existing error paths preserved:
impl From<std::io::Error> for Box<dyn std::error::Error> {
    fn from(err: std::io::Error) -> Self {
        // Unchanged
    }
}

impl From<regex::Error> for Box<dyn std::error::Error> {
    fn from(err: regex::Error) -> Self {
        // Unchanged
    }
}
```

## Function Signatures

### Audit Tool Functions (All Unchanged)

```rust
/// Main entry point - unchanged signature
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Implementation unchanged (except for colored output)
}

/// Audit codebase for forbidden patterns - unchanged
fn audit_codebase(root: &Path) -> Result<AuditReport, Box<dyn std::error::Error>> {
    // Implementation unchanged
}

/// Print audit report - CHANGED to remove colored styling
fn print_report(report: &AuditReport) {
    // BEFORE: println!("{}", "=== REPORT ===".bold().cyan());
    // AFTER:  println!("=== REPORT ===");

    // All colored method calls removed
    // All functional output preserved
}

/// Check if line is a comment - unchanged
fn is_comment_line(line: &str) -> bool {
    // Implementation unchanged
}
```

### Data Structures (All Unchanged)

```rust
#[derive(Debug, Clone)]
struct Violation {
    file: String,
    line: usize,
    pattern: String,
    context: String,
}

#[derive(Debug)]
struct AuditReport {
    production_violations: Vec<Violation>,
    test_violations: Vec<Violation>,
}

impl AuditReport {
    fn total_violations(&self) -> usize {
        // Unchanged
    }

    fn is_clean(&self) -> bool {
        // Unchanged
    }
}
```

## Behavioral Change Summary

### Removed Behavior

1. **Colored Terminal Output**
   - NO bold text formatting (`.bold()`)
   - NO cyan color highlighting (`.cyan()`)
   - NO green color for success (`.green()`)
   - NO red color for errors (`.red()`)
   - NO yellow color for warnings (`.yellow()`)
   - NO dimmed text for context (`.dimmed()`)
   - Output is plain text, suitable for any terminal or redirection

2. **Colored Dependency**
   - `colored` crate removed from `tools/audit/Cargo.toml`
   - `colored` crate no longer in dependency tree
   - No alternative styling crate added

### Retained Behavior

1. **All Functional Output**
   - All status messages present
   - All violation details present
   - All statistics present
   - All emoji preserved (Unicode, not styling)

2. **All Detection Logic**
   - Pattern matching unchanged
   - File scanning unchanged
   - Violation classification unchanged
   - Test/production separation unchanged

3. **All Exit Codes**
   - Exit code 0 for clean production code
   - Exit code 1 for production violations
   - Exit code 1 for missing directory

4. **Error Handling**
   - All error paths unchanged
   - Error messages unchanged (except styling)

### Code Changes Required

#### File: `/home/lewis/src/zjj/tools/audit/Cargo.toml`

**Delete line 15:**
```toml
# DELETED:
# colored = "2.1"
```

**Final dependencies section:**
```toml
[dependencies]
regex = "1.10"
walkdir = "2.4"
```

#### File: `/home/lewis/src/zjj/tools/audit/src/main.rs`

**Delete line 10:**
```rust
// DELETED:
// use colored::Colorize;
```

**Replace all colored method calls in `print_report` function (lines 107-255):**

```rust
// BEFORE (example from line 108):
println!("\n{}", "=== ZJJ CODEBASE AUDIT REPORT ===".bold().cyan());

// AFTER:
println!("\n=== ZJJ CODEBASE AUDIT REPORT ===");

// BEFORE (example from line 116):
"Production Code: CLEAN (0 violations)".green().bold()

// AFTER:
"Production Code: CLEAN (0 violations)"

// BEFORE (example from line 133):
v.file.yellow()

// AFTER:
&v.file

// BEFORE (example from line 136):
v.context.dimmed()

// AFTER:
&v.context
```

**Complete list of method calls to remove:**
- `.bold()` - 18 occurrences
- `.cyan()` - 3 occurrences
- `.green()` - 9 occurrences
- `.red()` - 8 occurrences
- `.yellow()` - 8 occurrences
- `.dimmed()` - 6 occurrences
- Total: ~52 method call removals

## Migration Notes

### Breaking Changes

1. **Visual Output Change**
   - Users accustomed to colored output will see plain text
   - No functional changes, only visual

2. **Scripts Parsing Output**
   - Scripts should be unaffected (text content unchanged)
   - If scripts rely on ANSI escape codes, they may need updates

### Backwards Compatibility

1. **Functionality Preserved**
   - All detection logic unchanged
   - All exit codes unchanged
   - All output text content unchanged (except styling)

2. **No API Changes**
   - No function signatures changed
   - No data structures changed
   - No new arguments added

### Testing Strategy

See `/home/lewis/src/zjj/contracts/bd-20u-martin-fowler-tests.md` for comprehensive test plan covering:

- Dependency removal verification
- Compilation success
- Audit tool functionality preserved
- Output content verification (without styling)
- Exit code verification
- Error handling unchanged

## Verification Steps

### Manual Verification

1. **Remove colored dependency:**
   ```bash
   # Edit tools/audit/Cargo.toml
   # Remove line: colored = "2.1"
   ```

2. **Remove colored import and usage:**
   ```bash
   # Edit tools/audit/src/main.rs
   # Remove: use colored::Colorize
   # Remove all .bold(), .cyan(), .green(), .red(), .yellow(), .dimmed() calls
   ```

3. **Verify build:**
   ```bash
   cargo build --manifest-path tools/audit/Cargo.toml
   ```

4. **Verify functionality:**
   ```bash
   cargo run --manifest-path tools/audit/Cargo.toml
   ```

5. **Verify workspace:**
   ```bash
   cargo check --workspace
   cargo build --workspace
   ```

### Automated Verification

Run tests from `/home/lewis/src/zjj/contracts/bd-20u-martin-fowler-tests.md`

## Impact Assessment

### Dependencies Removed

1. **Direct Dependency:**
   - `colored = "2.1"` from `tools/audit/Cargo.toml`
   - Reduces audit tool dependencies by 1

2. **Transitive Dependencies:**
   - `colored` has no dependencies itself
   - No transitive dependency reduction

### Code Changes

- **Files Modified:** 2
  - `tools/audit/Cargo.toml` (1 line deleted)
  - `tools/audit/src/main.rs` (~53 lines modified)

- **Lines Changed:** ~54
  - 1 deletion (import)
  - 1 deletion (dependency)
  - ~52 method call removals

### Risk Assessment

- **Risk Level:** LOW
  - No functional changes
  - No API changes
  - Only visual formatting removed
  - Audit tool is standalone (not in workspace)

- **Rollback:** Simple (re-add colored dependency and method calls)

---

**Contract Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation

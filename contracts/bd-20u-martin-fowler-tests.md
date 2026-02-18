# Martin Fowler Test Plan: bd-20u - Remove colored, console, and similar crates from dependencies

**Bead ID:** bd-20u
**Title:** Remove colored, console, and similar crates from dependencies
**Test Framework:** Given-When-Then (BDD style)
**Coverage Target:** 100% of contract specification

## Test Suite Organization

```
bd-20u-tests/
├── happy_path/          # HP-001 to HP-010
├── error_path/          # EP-001 to EP-010
├── edge_cases/          # EC-001 to EC-010
└── contract_verification/ # CV-001 to CV-015
```

---

## Happy Path Tests (HP)

### HP-001: Audit tool builds without colored dependency

**GIVEN** the audit tool Cargo.toml has colored dependency removed
**AND** the source code has all colored method calls removed
**WHEN** developer runs `cargo build --manifest-path tools/audit/Cargo.toml`
**THEN** build succeeds without errors
**AND** no compilation errors related to missing `colored` crate
**AND** no compilation errors related to missing `Colorize` trait
**AND** binary is produced at `tools/audit/target/debug/zjj-audit`

```rust
#[test]
fn test_hp001_audit_tool_builds_without_colored() {
    // Manual verification test
    // 1. Remove colored from Cargo.toml
    // 2. Remove colored imports and usage from main.rs
    // 3. Run: cargo build --manifest-path tools/audit/Cargo.toml
    // 4. Assert: Exit code 0
}
```

---

### HP-002: Audit tool runs successfully with plain output

**GIVEN** the audit tool is built without colored dependency
**AND** crates directory exists with Rust code
**WHEN** developer runs `cargo run --manifest-path tools/audit/Cargo.toml`
**THEN** audit tool executes successfully
**AND** exit code is 0 (clean production code) or 1 (violations found)
**AND** output is plain text (no ANSI color codes)
**AND** output contains expected sections (header, violations, summary)

```rust
#[test]
fn test_hp002_audit_tool_runs_with_plain_output() {
    // Manual verification test
    // 1. Build audit tool
    // 2. Run: cargo run --manifest-path tools/audit/Cargo.toml
    // 3. Assert: Exit code 0 or 1
    // 4. Assert: Output contains "=== ZJJ CODEBASE AUDIT REPORT ==="
    // 5. Assert: No ANSI escape codes in output
}
```

---

### HP-003: Audit tool detects violations correctly (no changes to detection)

**GIVEN** the audit tool is built without colored dependency
**AND** a test file contains forbidden patterns (unwrap, expect, panic)
**WHEN** developer runs audit tool on the test directory
**THEN** violations are detected correctly
**AND** file paths are reported correctly
**AND** line numbers are reported correctly
**AND** pattern names are reported correctly
**AND** context snippets are reported correctly

```rust
#[test]
fn test_hp003_violation_detection_unchanged() {
    // Implementation: Create test file with unwrap()
    // Run audit tool
    // Assert: Violation detected with correct details
    // Compare output before and after colored removal
}
```

---

### HP-004: Audit tool separates test and production violations

**GIVEN** the audit tool is built without colored dependency
**AND** code has production violations
**AND** code has test violations (in tests/ directory)
**WHEN** developer runs audit tool
**THEN** production violations are listed in "Production Code" section
**AND** test violations are listed in "Test Code" section
**AND** exit code is 1 (production violations present)
**AND** separation logic is unchanged

```rust
#[test]
fn test_hp004_test_production_separation_unchanged() {
    // Implementation: Create files in src/ and tests/
    // Run audit tool
    // Assert: Proper separation in output
}
```

---

### HP-005: Audit tool grades code correctly

**GIVEN** the audit tool is built without colored dependency
**AND** code has varying numbers of violations
**WHEN** developer runs audit tool
**THEN** grade is calculated correctly
**AND** "A+" grade when production clean and tests clean
**AND** "A" grade when production clean and tests have violations
**AND** "B" grade when production has < 5 violations
**AND** "C" grade when production has >= 5 violations
**AND** grading logic is unchanged

```rust
#[test]
fn test_hp005_grading_logic_unchanged() {
    // Implementation: Test each grade scenario
    // Assert: Correct grade in output
}
```

---

### HP-006: Audit tool handles clean codebase

**GIVEN** the audit tool is built without colored dependency
**AND** code has no production violations
**WHEN** developer runs audit tool
**THEN** exit code is 0
**AND** output shows "Production Code: CLEAN (0 violations)"
**AND** output shows "Production code passes all checks!"
**AND** grade is "A+" or "A"

```rust
#[test]
fn test_hp006_clean_codebase_success() {
    // Implementation: Run on clean code
    // Assert: Exit code 0, appropriate messages
}
```

---

### HP-007: Audit tool handles violations correctly

**GIVEN** the audit tool is built without colored dependency
**AND** code has 10 production violations
**WHEN** developer runs audit tool
**THEN** exit code is 1
**AND** output shows "Production Code: 10 VIOLATIONS"
**AND** all 10 violations are listed with details
**AND** output shows "Production code has 10 critical violations"

```rust
#[test]
fn test_hp007_violations_reported_correctly() {
    // Implementation: Create 10 violations
    // Run audit tool
    // Assert: All violations listed, exit code 1
}
```

---

### HP-008: Workspace builds successfully after colored removal

**GIVEN** the audit tool has colored dependency removed
**WHEN** developer runs `cargo check --workspace`
**THEN** workspace check succeeds
**AND** no errors related to missing colored crate
**AND** no dependency resolution errors
**AND** exit code is 0

```rust
#[test]
fn test_hp008_workspace_builds_successfully() {
    // Manual verification test
    // Run: cargo check --workspace
    // Assert: Exit code 0
}
```

---

### HP-009: Cargo.lock updates cleanly

**GIVEN** the audit tool has colored dependency removed
**WHEN** developer runs `cargo build --manifest-path tools/audit/Cargo.toml`
**THEN** Cargo.lock is updated
**AND** colored crate is removed from Cargo.lock
**AND** no new dependencies are added
**AND** dependency tree remains valid

```rust
#[test]
fn test_hp009_cargo_lock_updates_cleanly() {
    // Manual verification test
    // 1. Check Cargo.lock contains colored before
    // 2. Remove colored from Cargo.toml
    // 3. Run cargo build
    // 4. Check Cargo.lock does NOT contain colored after
}
```

---

### HP-010: Audit tool output is readable without colors

**GIVEN** the audit tool is built without colored dependency
**AND** code has violations
**WHEN** developer runs audit tool and captures output
**THEN** output is readable plain text
**AND** sections are separated by blank lines
**AND** emoji characters are preserved (🔍, ✅, 🔴, ⚠️, 🎯, 📊, 🎓, ❌)
**AND** violation details are clear and parseable

```rust
#[test]
fn test_hp010_output_readable_without_colors() {
    // Implementation: Run audit tool, capture output
    // Assert: Output is readable, no ANSI codes
    // Assert: Emoji present
}
```

---

## Error Path Tests (EP)

### EP-001: Audit tool fails to build if colored import not removed

**GIVEN** the audit tool Cargo.toml has colored dependency removed
**AND** the source code still has `use colored::Colorize`
**WHEN** developer runs `cargo build --manifest-path tools/audit/Cargo.toml`
**THEN** build fails with compilation error
**AND** error message indicates "cannot find crate `colored`"
**AND** exit code is non-zero

```rust
#[test]
fn test_ep001_build_fails_if_import_not_removed() {
    // Manual verification test
    // 1. Remove colored from Cargo.toml
    // 2. Keep: use colored::Colorize
    // 3. Run cargo build
    // 4. Assert: Build fails with "cannot find crate"
}
```

---

### EP-002: Audit tool fails to build if colored method calls not removed

**GIVEN** the audit tool Cargo.toml has colored dependency removed
**AND** the source code has no colored import
**AND** the source code still has `.bold()`, `.cyan()`, etc. calls
**WHEN** developer runs `cargo build --manifest-path tools/audit/Cargo.toml`
**THEN** build fails with compilation error
**AND** error message indicates "no method named `bold` found"
**AND** exit code is non-zero

```rust
#[test]
fn test_ep002_build_fails_if_method_calls_not_removed() {
    // Manual verification test
    // 1. Remove colored from Cargo.toml
    // 2. Remove import
    // 3. Keep .bold() calls
    // 4. Run cargo build
    // 5. Assert: Build fails with "no method named"
}
```

---

### EP-003: Audit tool fails if crates directory missing

**GIVEN** the audit tool is built without colored dependency
**AND** crates directory does not exist
**WHEN** developer runs audit tool
**THEN** tool exits with error
**AND** error message contains "Crates directory not found"
**AND** exit code is 1
**AND** error handling is unchanged

```rust
#[test]
fn test_ep003_missing_crates_directory_error() {
    // Implementation: Run from wrong directory
    // Assert: Appropriate error message
}
```

---

### EP-004: Audit tool handles file read errors

**GIVEN** the audit tool is built without colored dependency
**AND** a .rs file has permission denied
**WHEN** developer runs audit tool
**THEN** tool skips unreadable file
**AND** continues scanning other files
**AND** error handling is unchanged
**AND** exit code reflects remaining violations

```rust
#[test]
fn test_ep004_file_read_error_handling() {
    // Implementation: Create unreadable .rs file
    // Assert: Tool continues without crashing
}
```

---

### EP-005: Audit tool handles regex compilation errors

**GIVEN** the audit tool is built without colored dependency
**AND** a forbidden pattern regex is invalid
**WHEN** developer runs audit tool
**THEN** tool exits with error
**AND** error message indicates regex compilation failure
**AND** error handling is unchanged
**AND** exit code is non-zero

```rust
#[test]
fn test_ep005_regex_compilation_error() {
    // Implementation: Modify patterns to include invalid regex
    // Assert: Appropriate error message
}
```

---

### EP-006: Audit tool handles empty crates directory

**GIVEN** the audit tool is built without colored dependency
**AND** crates directory exists but is empty
**WHEN** developer runs audit tool
**THEN** tool succeeds
**AND** output shows "No violations found"
**AND** exit code is 0
**AND** behavior is unchanged

```rust
#[test]
fn test_ep006_empty_crates_directory() {
    // Implementation: Run on empty directory
    // Assert: Clean success
}
```

---

### EP-007: Audit tool handles no .rs files

**GIVEN** the audit tool is built without colored dependency
**AND** crates directory exists but has no .rs files
**WHEN** developer runs audit tool
**THEN** tool succeeds
**AND** output shows "No violations found"
**AND** exit code is 0
**AND** behavior is unchanged

```rust
#[test]
fn test_ep007_no_rust_files() {
    // Implementation: Create directory with non-.rs files
    // Assert: Clean success
}
```

---

### EP-008: Audit tool handles very long file paths

**GIVEN** the audit tool is built without colored dependency
**AND** a .rs file has a very long path (200+ characters)
**AND** the file contains violations
**WHEN** developer runs audit tool
**THEN** tool scans the file successfully
**AND** violations are reported with full path
**AND** output is formatted correctly (no line wrapping issues)
**AND** behavior is unchanged

```rust
#[test]
fn test_ep008_very_long_file_paths() {
    // Implementation: Create deeply nested file
    // Assert: Path displayed correctly
}
```

---

### EP-009: Audit tool handles Unicode in file paths

**GIVEN** the audit tool is built without colored dependency
**AND** a .rs file has Unicode characters in path
**AND** the file contains violations
**WHEN** developer runs audit tool
**THEN** tool scans the file successfully
**AND** violations are reported with correct Unicode path
**AND** no encoding issues occur
**AND** behavior is unchanged

```rust
#[test]
fn test_ep009_unicode_in_file_paths() {
    // Implementation: Create file with Unicode name
    // Assert: Path displayed correctly
}
```

---

### EP-010: Audit tool handles special characters in context

**GIVEN** the audit tool is built without colored dependency
**AND** a violation line contains special characters (quotes, backslashes)
**WHEN** developer runs audit tool
**THEN** context is displayed correctly
**AND** no escaping issues occur
**AND** output is parseable
**AND** behavior is unchanged

```rust
#[test]
fn test_ep010_special_characters_in_context() {
    // Implementation: Create file with special chars
    // Assert: Context displayed correctly
}
```

---

## Edge Case Tests (EC)

### EC-001: Output piped to file retains information

**GIVEN** the audit tool is built without colored dependency
**AND** code has violations
**WHEN** developer runs audit tool and redirects output to file
**THEN** file contains all violation information
**AND** file contains no ANSI escape codes (already plain text)
**AND** file is readable with any text editor
**AND** all emoji are preserved

```rust
#[test]
fn test_ec001_output_piped_to_file() {
    // Implementation: Run tool > output.txt
    // Assert: File contains all info, no ANSI codes
}
```

---

### EC-002: Output in non-UTF8 locale

**GIVEN** the audit tool is built without colored dependency
**AND** system locale is not UTF-8
**WHEN** developer runs audit tool
**THEN** tool handles encoding gracefully
**AND** emoji may not display but don't cause crashes
**AND** tool continues to function
**AND** behavior is unchanged from before

```rust
#[test]
fn test_ec002_non_utf8_locale() {
    // Implementation: Run with ASCII locale
    // Assert: Tool doesn't crash
}
```

---

### EC-003: Very large number of violations (performance)

**GIVEN** the audit tool is built without colored dependency
**AND** code has 500+ violations
**WHEN** developer runs audit tool
**THEN** tool completes in reasonable time
**AND** all violations are reported
**AND** output is formatted correctly
**AND** no memory issues occur

```rust
#[test]
fn test_ec003_very_many_violations() {
    // Implementation: Create 500 violations
    // Assert: All reported, reasonable time
}
```

---

### EC-004: Violation context with very long lines

**GIVEN** the audit tool is built without colored dependency
**AND** a violation line is 500+ characters long
**WHEN** developer runs audit tool
**THEN** context is displayed (possibly truncated)
**AND** output formatting is not broken
**AND** other violations are still readable
**AND** behavior is unchanged

```rust
#[test]
fn test_ec004_very_long_context_lines() {
    // Implementation: Create 500-char line with unwrap()
    // Assert: Displayed reasonably
}
```

---

### EC-005: Multiple violations in same file

**GIVEN** the audit tool is built without colored dependency
**AND** a single file has 10 violations
**WHEN** developer runs audit tool
**THEN** all 10 violations are reported
**AND** each has correct line number
**AND** each has correct pattern name
**AND** each has correct context
**AND** behavior is unchanged

```rust
#[test]
fn test_ec005_multiple_violations_same_file() {
    // Implementation: Create file with 10 unwrap() calls
    // Assert: All reported correctly
}
```

---

### EC-006: Violation on first line of file

**GIVEN** the audit tool is built without colored dependency
**AND** a file has violation on line 1
**WHEN** developer runs audit tool
**THEN** violation is reported with line number 1
**AND** context is displayed correctly
**AND** no off-by-one errors occur
**AND** behavior is unchanged

```rust
#[test]
fn test_ec006_violation_on_first_line() {
    // Implementation: Create file with unwrap() on line 1
    // Assert: Line number is 1
}
```

---

### EC-007: Violation in comment-only file

**GIVEN** the audit tool is built without colored dependency
**AND** a file contains only comments
**WHEN** developer runs audit tool
**THEN** file is scanned
**AND** no violations are reported (comment-only lines skipped)
**AND** behavior is unchanged

```rust
#[test]
fn test_ec007_violation_in_comment_only_file() {
    // Implementation: Create file with // unwrap() comment
    // Assert: No violation reported
}
```

---

### EC-008: Violation immediately after comment

**GIVEN** the audit tool is built without colored dependency
**AND** a file has comment on line 1
**AND** same file has violation on line 2
**WHEN** developer runs audit tool
**THEN** violation on line 2 is reported
**AND** comment on line 1 is not reported
**AND** line numbers are correct
**AND** behavior is unchanged

```rust
#[test]
fn test_ec008_violation_after_comment() {
    // Implementation: Line 1: // comment, Line 2: unwrap()
    // Assert: Only line 2 reported
}
```

---

### EC-009: Test violations with test_ prefix in filename

**GIVEN** the audit tool is built without colored dependency
**AND** a file named `test_utils.rs` has violations
**WHEN** developer runs audit tool
**THEN** violations are classified as test violations
**AND** appear in "Test Code" section
**AND** don't affect exit code
**AND** behavior is unchanged

```rust
#[test]
fn test_ec009_test_file_prefix_classification() {
    // Implementation: Create test_utils.rs with unwrap()
    // Assert: Classified as test violation
}
```

---

### EC-010: Violation in tests/ subdirectory

**GIVEN** the audit tool is built without colored dependency
**AND** a file in `tests/` subdirectory has violations
**WHEN** developer runs audit tool
**THEN** violations are classified as test violations
**AND** appear in "Test Code" section
**AND** don't affect exit code
**AND** behavior is unchanged

```rust
#[test]
fn test_ec010_tests_subdirectory_classification() {
    // Implementation: Create tests/test.rs with unwrap()
    // Assert: Classified as test violation
}
```

---

## Contract Verification Tests (CV)

### CV-001: Verify precondition - no colored in workspace crates

**GIVEN** the ZJJ workspace
**WHEN** checking `crates/zjj/Cargo.toml`
**AND** checking `crates/zjj-core/Cargo.toml`
**THEN** neither file contains `colored` dependency
**AND** neither file contains `console`, `termcolor`, `ansi_term` dependencies
**AND** precondition is verified

```rust
#[test]
fn test_cv001_no_colored_in_workspace_crates() {
    // Verification test: Check Cargo.toml files
    // Assert: No colored dependencies
}
```

---

### CV-002: Verify precondition - audit tool uses colored

**GIVEN** the audit tool before changes
**WHEN** checking `tools/audit/Cargo.toml`
**AND** checking `tools/audit/src/main.rs`
**THEN** Cargo.toml contains `colored = "2.1"`
**AND** main.rs contains `use colored::Colorize`
**AND** main.rs contains colored method calls
**AND** precondition is verified

```rust
#[test]
fn test_cv002_audit_tool_uses_colored_before_changes() {
    // Verification test: Check before state
    // Assert: Colored is present
}
```

---

### CV-003: Verify postcondition - colored removed from Cargo.toml

**GIVEN** the audit tool after changes
**WHEN** checking `tools/audit/Cargo.toml`
**THEN** file does NOT contain `colored = "2.1"`
**AND** dependencies section only contains `regex` and `walkdir`
**AND** postcondition is verified

```rust
#[test]
fn test_cv003_colored_removed_from_cargo_toml() {
    // Verification test: Check after state
    // Assert: Colored is removed
}
```

---

### CV-004: Verify postcondition - colored import removed from source

**GIVEN** the audit tool after changes
**WHEN** checking `tools/audit/src/main.rs`
**THEN** file does NOT contain `use colored::Colorize`
**AND** file does NOT contain `use colored`
**AND** postcondition is verified

```rust
#[test]
fn test_cv004_colored_import_removed() {
    // Verification test: Check source file
    // Assert: No colored import
}
```

---

### CV-005: Verify postcondition - no colored method calls

**GIVEN** the audit tool after changes
**WHEN** searching `tools/audit/src/main.rs`
**THEN** no `.bold()` method calls
**AND** no `.cyan()` method calls
**AND** no `.green()` method calls
**AND** no `.red()` method calls
**AND** no `.yellow()` method calls
**AND** no `.dimmed()` method calls
**AND** postcondition is verified

```rust
#[test]
fn test_cv005_no_colored_method_calls() {
    // Verification test: Search for method calls
    // Assert: None found
}
```

---

### CV-006: Verify invariant - detection logic unchanged

**GIVEN** the audit tool after changes
**WHEN** running on test code with violations
**THEN** detection patterns are identical
**AND** `unwrap()` is detected
**AND** `expect(` is detected
**AND** `panic!(` is detected
**AND** `todo!(` is detected
**AND** `unimplemented!(` is detected
**AND** `unsafe` is detected
**AND** invariant is verified

```rust
#[test]
fn test_cv006_detection_logic_unchanged() {
    // Implementation: Test each pattern
    // Assert: All detected
}
```

---

### CV-007: Verify invariant - exit codes unchanged

**GIVEN** the audit tool after changes
**WHEN** running on clean production code
**THEN** exit code is 0
**WHEN** running on production code with violations
**THEN** exit code is 1
**AND** invariant is verified

```rust
#[test]
fn test_cv007_exit_codes_unchanged() {
    // Implementation: Test both scenarios
    // Assert: Exit codes match
}
```

---

### CV-008: Verify invariant - output content preserved

**GIVEN** the audit tool after changes
**WHEN** running on code with violations
**THEN** output contains "=== ZJJ CODEBASE AUDIT REPORT ==="
**AND** output contains "Production Code:" section
**AND** output contains "Test Code:" section
**AND** output contains violation details (file, line, pattern, context)
**AND** output contains "=== SUMMARY ===" section
**AND** output contains grade
**AND** invariant is verified

```rust
#[test]
fn test_cv008_output_content_preserved() {
    // Implementation: Run and capture output
    // Assert: All sections present
}
```

---

### CV-009: Verify invariant - emoji preserved

**GIVEN** the audit tool after changes
**WHEN** running on code with violations
**THEN** output contains 🔍 emoji
**AND** output contains ✅ emoji (if clean)
**AND** output contains 🔴 or ⚠️ emoji (if violations)
**AND** output contains 🎯, 📊, 🎓 emoji in summary
**AND** output contains ❌ emoji for violations
**AND** invariant is verified

```rust
#[test]
fn test_cv009_emoji_preserved() {
    // Implementation: Run and capture output
    // Assert: Emoji present
}
```

---

### CV-010: Verify postcondition - workspace builds

**GIVEN** the audit tool after changes
**WHEN** running `cargo check --workspace`
**THEN** check succeeds
**AND** exit code is 0
**AND** no dependency resolution errors
**AND** postcondition is verified

```rust
#[test]
fn test_cv010_workspace_builds_successfully() {
    // Verification test: Run cargo check
    // Assert: Success
}
```

---

### CV-011: Verify postcondition - audit tool builds

**GIVEN** the audit tool after changes
**WHEN** running `cargo build --manifest-path tools/audit/Cargo.toml`
**THEN** build succeeds
**AND** exit code is 0
**AND** binary is produced
**AND** postcondition is verified

```rust
#[test]
fn test_cv011_audit_tool_builds() {
    // Verification test: Run cargo build
    // Assert: Success
}
```

---

### CV-012: Verify invariant - error handling unchanged

**GIVEN** the audit tool after changes
**WHEN** crates directory is missing
**THEN** error message says "Crates directory not found"
**AND** exit code is 1
**WHEN** a file cannot be read
**THEN** tool continues scanning
**AND** invariant is verified

```rust
#[test]
fn test_cv012_error_handling_unchanged() {
    // Implementation: Test error scenarios
    // Assert: Behavior unchanged
}
```

---

### CV-013: Verify invariant - comment filtering unchanged

**GIVEN** the audit tool after changes
**AND** a file has `// unwrap()` comment
**AND** same file has real `unwrap()` call
**WHEN** running audit tool
**THEN** comment is not reported
**AND** real call is reported
**AND** invariant is verified

```rust
#[test]
fn test_cv013_comment_filtering_unchanged() {
    // Implementation: Test comment vs real code
    // Assert: Correct filtering
}
```

---

### CV-014: Verify invariant - test/production separation unchanged

**GIVEN** the audit tool after changes
**AND** code has violations in src/
**AND** code has violations in tests/
**WHEN** running audit tool
**THEN** src/ violations appear in "Production Code" section
**AND** tests/ violations appear in "Test Code" section
**AND** exit code reflects production violations only
**AND** invariant is verified

```rust
#[test]
fn test_cv014_test_production_separation_unchanged() {
    // Implementation: Test both directories
    // Assert: Correct separation
}
```

---

### CV-015: Verify postcondition - no ANSI color codes in output

**GIVEN** the audit tool after changes
**WHEN** running and capturing output
**THEN** output contains NO ANSI escape sequences
**AND** specifically no `\x1b[` sequences
**AND** output is plain text
**AND** postcondition is verified

```rust
#[test]
fn test_cv015_no_ansi_color_codes() {
    // Implementation: Run and capture output
    // Assert: No ANSI codes
}
```

---

## Test Execution Order

### Phase 1: Contract Verification (CV-001 to CV-015)
- Run first to verify contract fundamentals
- Verify preconditions, postconditions, invariants
- All MUST pass before proceeding

### Phase 2: Happy Path (HP-001 to HP-010)
- Basic functionality tests
- Verify tool builds and runs
- Verify output is correct
- All MUST pass

### Phase 3: Error Path (EP-001 to EP-010)
- Error handling tests
- Verify graceful failures
- All MUST pass

### Phase 4: Edge Cases (EC-001 to EC-010)
- Boundary condition tests
- Verify special cases handled correctly
- All SHOULD pass

## Success Criteria

- **P0 (Critical):** All CV tests pass (contract verification)
- **P0 (Critical):** All HP tests pass (happy path)
- **P1 (High):** All EP tests pass (error handling)
- **P1 (High):** All EC tests pass (edge cases)
- **Coverage:** 100% of contract specification tested

## Test Metrics

- Total tests: 45
- Critical (P0): 25 tests
- High priority (P1): 20 tests
- Estimated execution time: 10-15 minutes (includes manual verification)

## Implementation Location

Tests should be implemented in:
```
/home/lewis/src/zjj/tools/audit/tests/
├── test_build.rs         # HP-001, HP-008, HP-009, EP-001, EP-002
├── test_functionality.rs # HP-002 to HP-007, HP-010
├── test_detection.rs     # HP-003, CV-006, CV-013, CV-014
├── test_output.rs        # HP-010, EC-001, CV-008, CV-009, CV-015
├── test_errors.rs        # EP-003 to EP-010
└── test_edge_cases.rs    # EC-001 to EC-010
```

Contract verification tests (CV series) are mostly manual verification steps.

## Manual Verification Checklist

### Before Changes
- [ ] CV-001: Verify no colored in workspace crates
- [ ] CV-002: Verify audit tool uses colored

### After Changes
- [ ] CV-003: Verify colored removed from Cargo.toml
- [ ] CV-004: Verify colored import removed
- [ ] CV-005: Verify no colored method calls
- [ ] CV-010: Verify workspace builds
- [ ] CV-011: Verify audit tool builds
- [ ] CV-015: Verify no ANSI codes in output

### Build Verification
- [ ] HP-001: Audit tool builds without colored
- [ ] HP-008: Workspace builds successfully
- [ ] HP-009: Cargo.lock updates cleanly

### Functional Verification
- [ ] HP-002: Audit tool runs with plain output
- [ ] HP-003: Violation detection unchanged
- [ ] CV-006: Detection logic unchanged
- [ ] CV-007: Exit codes unchanged

---

**Test Plan Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation

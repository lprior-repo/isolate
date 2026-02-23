# Test Quality Gate Report

**Generated:** 2026-02-23
**Repository:** zjj
**Commit:** main branch

## Executive Summary

**Overall Status:** FAILED

The codebase fails to compile tests due to API signature mismatches. Clippy reports 475 violations primarily related to `unwrap()` usage (364), `expect()` usage (250), and `panic` violations (118). Code formatting has minor issues with 5936 lines of diff output.

---

## 1. Test Results

### Compilation Status: FAILED

Tests could not be executed due to compilation errors in the test suite.

**Error Location:** `crates/zjj-core/tests/domain_event_serialization.rs`

**Error Type:** `error[E0061]` - Function argument count mismatch (4 locations)

**Root Cause:** The `DomainEvent::queue_entry_claimed()` function signature has changed but test code was not updated. The function now expects `ClaimTimestamps` struct but tests are passing separate `DateTime<Utc>` parameters.

**Affected Lines:**
- Line 191
- Line 446
- Line 524
- Line 690

**Compilation Error Message:**
```
error[E0061]: this function takes 4 arguments but 6 arguments were supplied
   --> crates/zjj-core/tests/domain_event_serialization.rs:191:17
    |
191 |     let event = DomainEvent::queue_entry_claimed(
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
196 |         expires_at,
    |         ---------- unexpected argument #5 of type `DateTime<Utc>`
197 |         timestamp,
    |         --------- unexpected argument #6 of type `DateTime<Utc>`
    |
note: expected `ClaimTimestamps`, found `DateTime<Utc>`
```

**Additional Compilation Warnings:** 16 unused doc comment warnings in `cli_properties.rs`

### Test Pass/Fail Counts

**Tests Run:** 0 (compilation failed before execution)
**Tests Passed:** N/A
**Tests Failed:** N/A
**Test Execution Rate:** 0%

---

## 2. Clippy Analysis

### Summary: FAILED

**Total Violations:** 475 clippy errors (treated as errors with `-D warnings`)

### Violation Breakdown

| Category | Count | Percentage |
|----------|-------|------------|
| `unwrap_used` | 364 | 76.6% |
| `expect_used` | 250 | 52.6% |
| `panic` (various) | 118 | 24.8% |
| Other | 21 | 4.4% |

**Note:** Some files have multiple violation types, so percentages sum >100%.

### Top Violating Files (based on error count)

| File | Estimated Violations |
|------|---------------------|
| `crates/zjj-core/src/beads/issue.rs` | High (test functions with unwrap) |
| `crates/zjj-core/src/beads/invariant_tests.rs` | High (test code) |
| Multiple test files | Widespread |

### Violation Examples

1. **unwrap_used in test code:**
```rust
// crates/zjj-core/src/beads/issue.rs:552
let issue = Issue::new("test-1", "Test Issue").unwrap();
```

2. **doc_markdown violations:**
```rust
// crates/zjj-core/src/beads/invariant_tests.rs:13
//! - Closed beads cannot be reopened (they must use reopen() method)
// Should be: `reopen()`
```

3. **manual_string_new:**
```rust
// crates/zjj-core/src/beads/invariant_tests.rs:58
Just("".to_string()),
// Should be: String::new()
```

4. **uninlined_format_args:**
```rust
// crates/zjj-core/src/beads/invariant_tests.rs:191
let with_spaces = format!("  {}  ", title);
// Should be: format!("  {title}  ")
```

---

## 3. Code Formatting Status

### Status: MINOR ISSUES

**Format Check:** FAILED with minor differences

**Lines of Diff:** ~5936 lines in output

**Issue Type:** Whitespace and line-breaking differences only (no semantic changes)

**Sample Differences:**

1. **Comment alignment:**
```diff
-        assert!(BeadId::parse("bd-123-456").is_err());  // Hyphens not allowed
+        assert!(BeadId::parse("bd-123-456").is_err()); // Hyphens not allowed
```

2. **Line length formatting:**
```diff
-        let mut issue = Issue::new(...).map_err(...)?;
+        let mut issue =
+            Issue::new(...).map_err(...)?;
```

3. **Function argument formatting:**
```diff
-        assert_eq!(stack.entries[0].session, SessionName::parse("root").expect("valid"));
+        assert_eq!(
+            stack.entries[0].session,
+            SessionName::parse("root").expect("valid")
+        );
```

**Configuration Warnings:** The `rustfmt.toml` uses nightly-only features:
- `wrap_comments`
- `format_code_in_doc_comments`
- `imports_granularity = "Crate"`
- `group_imports = "StdExternalCrate"`
- And several others

These features are ignored on stable Rust but don't cause check failures.

---

## 4. Detailed Issue Analysis

### Critical Issues (Must Fix)

1. **Test Compilation Failure (E0061)**
   - **Impact:** Tests cannot run
   - **Fix Required:** Update 4 test calls to `DomainEvent::queue_entry_claimed()` to use `ClaimTimestamps` struct instead of separate DateTime parameters
   - **Priority:** P0 - Blocks all testing

### High Priority Issues (Should Fix)

2. **unwrap/expect/panic Violations (732 total)**
   - **Impact:** Violates functional Rust principles; potential panic points in production
   - **Note:** Many are in test code where unwrap is more acceptable
   - **Fix Required:**
     - For production code: Replace with proper error handling using `?`, `match`, or combinators
     - For test code: Consider `#[allow(clippy::unwrap_used)]` with justification or use test helpers that don't unwrap
   - **Priority:** P1 - Code quality and safety

3. **Unused Doc Comments (16)**
   - **Impact:** Documentation not being generated
   - **Fix Required:** Move doc comments inside proptest macros or use `#[doc]` attribute
   - **Priority:** P2 - Documentation completeness

### Low Priority Issues (Nice to Fix)

4. **Code Formatting**
   - **Impact:** Minor consistency issues
   - **Fix Required:** Run `cargo fmt` to apply formatting
   - **Priority:** P3 - Style consistency

5. **Other Clippy Warnings**
   - `doc_markdown`: Add backticks to code terms in docs
   - `manual_string_new`: Use `String::new()` instead of `"".to_string()`
   - `uninlined_format_args`: Use inline format args like `"{var}"`
   - **Priority:** P3 - Code quality

---

## 5. Root Cause Analysis

### Test Compilation Failure

The `queue_entry_claimed` function signature was refactored to use a `ClaimTimestamps` struct for better type safety, but the corresponding test code was not updated. This is a classic refactoring completeness issue.

### High Unwrap/Expect Count

The codebase has enforced functional Rust principles with `#![deny(clippy::unwrap_used)]` and similar lints at the file level. However:

1. **Test Code:** Many tests use unwrap for simplicity
2. **Production Code:** Some code paths haven't been fully migrated to error-handling patterns
3. **Issue Tracking:** The `Issue` type in `beads/issue.rs` has extensive test code using unwrap

### Formatting Issues

The formatting differences appear to be from:
1. Different rustfmt versions or configurations between developers
2. Manual formatting that doesn't match the automated rules
3. Long lines being reformatted over time

---

## 6. Recommendations

### Immediate Actions (Required to unblock testing)

1. **Fix test compilation:**
   ```rust
   // OLD (failing):
   DomainEvent::queue_entry_claimed(
       id,
       agent_id,
       claimed_at,
       expires_at,
       timestamp,  // <-- remove
       timestamp,  // <-- remove
   )

   // NEW (fixed):
   DomainEvent::queue_entry_claimed(
       id,
       agent_id,
       ClaimTimestamps { claimed_at, expires_at },
       timestamp,
   )
   ```

2. **Address unwrap violations strategically:**
   - For production code paths: Implement proper error handling
   - For test code: Add `#[allow(clippy::unwrap_used)]` with comments explaining why unwrap is safe in tests
   - Create test helper functions that encapsulate unwrap usage

### Medium-term Actions

1. **Run cargo fmt** to resolve formatting differences
2. **Update rustfmt.toml** to either:
   - Remove nightly-only features (for stable compatibility)
   - Document that nightly is required for full formatting
3. **Fix doc comment warnings** by moving them inside macro invocations

### Long-term Actions

1. **Establish pre-commit hooks** for clippy and fmt
2. **Create a testing guide** documenting how to write tests without unwrap
3. **Audit all unwrap usage** and categorize as:
   - Safe to unwrap (infallible operations)
   - Needs error handling (fallible operations)
4. **Consider test helper macros** that reduce unwrap need

---

## 7. Next Steps

### To Run Tests Successfully

1. Fix the 4 compilation errors in `domain_event_serialization.rs`
2. Re-run: `cargo test --all`
3. Address any remaining test failures

### To Pass Clippy

1. Fix test compilation first
2. Address unwrap violations in production code (critical)
3. Add `#[allow(...)]` for test code with justification
4. Fix minor warnings (doc_markdown, manual_string_new, etc.)
5. Re-run: `cargo clippy --all-targets -- -D warnings`

### To Pass Formatting

1. Run: `cargo fmt`
2. Verify: `cargo fmt --check` passes
3. Consider updating CI to run fmt check

---

## 8. Health Assessment

| Metric | Status | Score |
|--------|--------|-------|
| Test Compilation | FAILED | 0/10 |
| Test Execution | BLOCKED | N/A |
| Clippy Compliance | FAILED | 3/10 |
| Code Formatting | MINOR ISSUES | 8/10 |
| **Overall** | **FAILED** | **3.7/10** |

### Blocking Issues

1. Test compilation failure (P0)
2. 475 clippy violations treated as errors (P1)

### Non-Blocking Issues

1. Code formatting differences (P3)
2. 16 unused doc comment warnings (P2)

---

## Appendix: Quick Reference

### Run Commands

```bash
# Re-run tests after fixes
cargo test --all 2>&1 | tee test_results.txt

# Re-run clippy after fixes
cargo clippy --all-targets -- -D warnings 2>&1 | tee clippy_results.txt

# Fix formatting
cargo fmt

# Verify formatting
cargo fmt --check
```

### Key Files Mentioned

- `/home/lewis/src/zjj/crates/zjj-core/tests/domain_event_serialization.rs` - Test compilation failures
- `/home/lewis/src/zjj/crates/zjj-core/src/beads/issue.rs` - Unwrap violations
- `/home/lewis/src/zjj/crates/zjj-core/src/beads/invariant_tests.rs` - Test code with unwrap
- `/home/lewis/src/zjj/crates/zjj-core/tests/cli_properties.rs` - Unused doc comments
- `/home/lewis/src/zjj/rustfmt.toml` - Formatting configuration

---

**End of Report**

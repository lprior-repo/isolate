# Module Split Guide

This guide provides a standardized, repeatable process for splitting large Rust files into modular structures while maintaining functional programming principles and test coverage.

## Overview

**Goal:** Break down files >250 lines into focused modules of <250 lines each, maintaining:
- Zero clippy warnings
- 100% test pass rate
- Public API compatibility
- Functional Core / Imperative Shell separation
- Immutable data structures (im::Vector, im::HashMap)

**Process Duration:** ~2-4 hours per file depending on size

---

## Phase 1: Planning (15-30 min)

### 1.1 Analyze Current Structure

```bash
# Count lines in target file
wc -l crates/zjj-core/src/TARGET.rs

# Identify logical boundaries
grep -n "^pub fn\|^pub struct\|^pub enum" crates/zjj-core/src/TARGET.rs
```

### 1.2 Identify Module Boundaries

Look for natural groupings:
- **Types module** (`types.rs`): Enums, structs, type aliases
- **Query module** (`query.rs`): Read operations, database queries
- **Operations module** (`operations.rs`): Write operations, mutations
- **Validation module** (`validation.rs`): Input validation, business rules
- **Filter/Transform** (`filter.rs`): Pure functions operating on data
- **Analysis** (`analysis.rs`): Derived data, statistics

### 1.3 Plan Public API Surface

**Checklist:**
- [ ] List all `pub fn`, `pub struct`, `pub enum` that external code uses
- [ ] Decide which modules own which public items
- [ ] Plan `mod.rs` re-exports to maintain compatibility
- [ ] Document any breaking changes (require major version bump)

**Example Plan:**
```rust
// Before: beads.rs (2,130 lines)
pub struct BeadIssue { ... }
pub fn query_beads() -> Result<Vec<BeadIssue>> { ... }
pub fn filter_issues() -> Vec<BeadIssue> { ... }

// After: beads/
//   mod.rs      - Re-exports, module organization
//   types.rs    - BeadIssue, IssueStatus, Priority
//   query.rs    - query_beads, query_labels
//   filter.rs   - filter_issues, sort_issues
//   analysis.rs - find_blockers, calculate_critical_path
```

---

## Phase 2: Setup Module Structure (10-15 min)

### 2.1 Create Module Directory

```bash
cd /home/lewis/src/zjj

# For zjj-core modules
mkdir -p crates/zjj-core/src/TARGET_NAME

# For zjj CLI modules
mkdir -p crates/zjj/src/commands/TARGET_NAME
```

### 2.2 Create Initial mod.rs

```rust
// crates/zjj-core/src/TARGET_NAME/mod.rs

// Public modules (will contain re-exports)
pub mod types;
pub mod query;
pub mod filter;
pub mod analysis;

// Re-export public API for backward compatibility
pub use types::{BeadIssue, IssueStatus, Priority};
pub use query::{query_beads, query_labels};
pub use filter::{filter_issues, sort_issues};
pub use analysis::{find_blockers, calculate_critical_path};
```

### 2.3 Update Parent Module

```rust
// In crates/zjj-core/src/lib.rs or parent module
- pub mod beads;
+ pub mod beads;  // Now points to beads/ directory instead of beads.rs
```

---

## Phase 3: Extraction (1-2 hours)

### 3.1 Extract Types First (Fewest Dependencies)

**Order:** Enums → Simple structs → Complex structs with methods

```bash
# Create types.rs
touch crates/zjj-core/src/TARGET_NAME/types.rs
```

```rust
// crates/zjj-core/src/TARGET_NAME/types.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Issue status in the beads workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
}

/// Main issue representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    pub labels: Option<im::Vector<String>>,  // Use im::Vector, not Vec
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BeadIssue {
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.status == IssueStatus::Blocked
    }
}
```

**Verification:**
```bash
# Compile types module in isolation
moon run :check --filter crates/zjj-core
```

### 3.2 Extract Pure Functions (Query/Filter)

**Pure functions:** No side effects, same input → same output

```rust
// crates/zjj-core/src/TARGET_NAME/filter.rs

use super::types::{BeadIssue, IssueStatus};

/// Filter issues by status
#[must_use]
pub fn filter_by_status(
    issues: &[BeadIssue],
    status: IssueStatus
) -> im::Vector<BeadIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == status)
        .cloned()
        .collect()
}
```

### 3.3 Extract Imperative Shell (I/O, Async)

**Imperative Shell:** Database queries, file I/O, network calls

```rust
// crates/zjj-core/src/TARGET_NAME/query.rs

use super::types::BeadIssue;
use crate::{Result, Error};
use sqlx::SqlitePool;

/// Query all beads from database
pub async fn query_beads(pool: &SqlitePool) -> Result<im::Vector<BeadIssue>> {
    let rows = sqlx::query("SELECT * FROM issues")
        .fetch_all(pool)
        .await?;

    rows.into_iter()
        .map(|row| parse_row(&row))
        .collect()
}
```

### 3.4 Handle Cross-Module Dependencies

**Problem:** Module A needs private function from Module B

**Solutions:**

1. **Make function public:** If it's genuinely useful
```rust
pub fn helper_function() { ... }  // Was: fn helper_function()
```

2. **Use `pub(crate)`:** Visible within crate only
```rust
pub(crate) fn internal_helper() { ... }
```

3. **Move function:** Place in most appropriate module

4. **Duplicate if trivial:** For 1-2 line helpers, duplication OK

---

## Phase 4: Test Migration (30-60 min)

### 4.1 Identify Test Structure

```bash
# Find tests in original file
grep -n "#\[test\]" crates/zjj-core/src/TARGET.rs
```

### 4.2 Migrate Tests to Modules

**Option A: Inline Tests (preferred)**
```rust
// crates/zjj-core/src/TARGET_NAME/types.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_blocked() {
        let issue = BeadIssue {
            status: IssueStatus::Blocked,
            ..Default::default()
        };
        assert!(issue.is_blocked());
    }
}
```

**Option B: Separate Test Files (for large test suites)**
```bash
mkdir -p crates/zjj-core/src/TARGET_NAME/tests
touch crates/zjj-core/src/TARGET_NAME/tests/types_test.rs
```

```rust
// crates/zjj-core/src/TARGET_NAME/tests/types_test.rs

use crate::TARGET_NAME::types::*;

#[test]
fn test_issue_status_serialization() {
    // Test JSON round-trip
}
```

### 4.3 Run Tests Incrementally

```bash
# Test specific module
moon run :test -- TARGET_NAME::types

# Test entire crate
moon run :test -- zjj-core

# Full test suite
moon run :ci
```

**Checklist:**
- [ ] All tests pass
- [ ] No tests were lost in migration
- [ ] Test names remain descriptive
- [ ] Test coverage maintained (check baseline)

---

## Phase 5: Verification & Cleanup (15-30 min)

### 5.1 Run Full Verification Suite

```bash
# From /home/lewis/src/zjj

# 1. Format check
moon run :fmt

# 2. Clippy (zero warnings)
moon run :check

# 3. Full test suite
moon run :test

# 4. Build release
moon run :build
```

### 5.2 Verify Against Baseline

```bash
# Compare file sizes
find crates -name '*.rs' -type f -exec wc -l {} + | sort -rn | head -20 > .file-sizes-after.txt
diff .file-sizes-before.txt .file-sizes-after.txt

# Count tests
grep -r "#\[test\]" crates | wc -l  # Should be >= 723

# Count clippy warnings
moon run :check 2>&1 | grep -i warning | wc -l  # Should be 0
```

### 5.3 Checklist Before Committing

**Pre-Commit Verification:**
- [ ] All module files <250 lines
- [ ] All tests pass (`moon run :ci`)
- [ ] Zero new clippy warnings
- [ ] Public API unchanged (no breaking changes)
- [ ] Test count >= baseline (723)
- [ ] Documentation updated (if public API changed)
- [ ] `mod.rs` has all necessary re-exports
- [ ] No `pub(crate)` leaking into public API

### 5.4 Commit Strategy

```bash
git add crates/zjj-core/src/TARGET_NAME/
git add crates/zjj-core/src/lib.rs  # If parent module updated
git rm crates/zjj-core/src/TARGET.rs  # Remove old file

git commit -m "refactor: split TARGET.rs into modular structure

Split 2,130-line TARGET.rs into focused modules:
- types.rs (180 lines): Core types and enums
- query.rs (220 lines): Database query operations
- filter.rs (190 lines): Pure filtering functions
- analysis.rs (240 lines): Derived data and statistics

Benefits:
- All modules <250 lines (maintainability goal)
- Improved separation of concerns
- Easier to locate and modify specific functionality
- Test organization mirrors code structure

Verification:
✓ All 723 tests pass
✓ Zero clippy warnings
✓ Public API unchanged
✓ FC/IS separation maintained

Refs: zjj-uxqs.3"
```

---

## Phase 6: Rollback Procedure (If Tests Fail)

### 6.1 Immediate Rollback

```bash
# Discard all changes
git restore crates/zjj-core/src/TARGET_NAME/
git restore crates/zjj-core/src/TARGET.rs
git restore crates/zjj-core/src/lib.rs

# Or: Revert committed changes
git revert HEAD
```

### 6.2 Partial Rollback (Keep Some Modules)

```bash
# Keep working modules, restore broken ones
git restore crates/zjj-core/src/TARGET_NAME/broken_module.rs
```

### 6.3 Debug Failing Tests

```bash
# Run specific failing test with output
moon run :test -- TARGET_NAME::module::test_name --nocapture

# Check for:
# - Missing imports
# - Incorrect visibility (pub vs pub(crate))
# - Moved functions not re-exported
# - Type mismatches (Vec vs im::Vector)
```

---

## Common Pitfalls & Solutions

### Pitfall 1: Circular Dependencies

**Problem:** Module A depends on Module B, which depends on Module A

**Solutions:**
1. **Extract common types to separate module:**
```rust
// types.rs
pub struct SharedType;

// module_a.rs
use super::types::SharedType;

// module_b.rs
use super::types::SharedType;
```

2. **Use trait abstraction:**
```rust
// traits.rs
pub trait Operation {
    fn execute(&self) -> Result<()>;
}

// module_a.rs
impl Operation for TypeA { ... }

// module_b.rs
fn process(op: &dyn Operation) { ... }
```

### Pitfall 2: Re-export Collision

**Problem:** Multiple modules export same name

**Solution:**
```rust
// mod.rs
pub use types::{BeadIssue, IssueStatus};
pub use query::{BeadIssue as BeadIssueQuery};  // Rename on re-export
```

### Pitfall 3: Test Access to Private Functions

**Problem:** Tests need to call private helper functions

**Solutions:**
1. **Inline tests in same file** (preferred):
```rust
// module.rs
fn private_helper() { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_helper() {
        assert_eq!(private_helper(), expected);
    }
}
```

2. **Make `pub(crate)` for testing:**
```rust
#[cfg_attr(test, visibility::make(pub))]
fn helper() { ... }
```

### Pitfall 4: Lost Re-exports

**Problem:** External code breaks because `pub use` missing

**Solution:** Always update `mod.rs` with ALL public items:
```rust
// mod.rs
pub mod types;
pub mod query;

// Re-export EVERYTHING that was public before
pub use types::*;
pub use query::*;
```

### Pitfall 5: Vec vs im::Vector Mismatch

**Problem:** Forgot to replace Vec with im::Vector during extraction

**Solution:**
```bash
# Find all Vec usage
grep -rn "Vec<" crates/zjj-core/src/TARGET_NAME/

# Replace with im::Vector
# Use im::vector! macro instead of vec!
# Use .push_back() instead of .push()
# Use im::Vector::new() instead of Vec::new()
```

---

## Module Naming Conventions

### Core Modules (Always Needed)

- `types.rs` - Enums, structs, type aliases, trait definitions
- `error.rs` - Error types specific to this module
- `mod.rs` - Re-exports, module organization

### Common Module Patterns

- `query.rs` / `read.rs` - Read operations, database queries
- `operations.rs` / `write.rs` - Write operations, mutations
- `validation.rs` - Input validation, business rules
- `filter.rs` - Pure filtering functions
- `transform.rs` - Data transformation, mapping
- `analysis.rs` - Derived data, statistics, complex queries
- `formatting.rs` / `display.rs` - String formatting, Display impls
- `parsing.rs` - Parse strings into types
- `serialization.rs` - Custom serde implementations

### Specialized Modules

- `security.rs` - Security validations, sanitization
- `async_ops.rs` - Async I/O operations
- `builder.rs` - Builder pattern implementations
- `tests/` - Large test suites (separate directory)

---

## File Size Guidelines

### Target Sizes

- **<100 lines:** Ideal for focused modules
- **100-250 lines:** Acceptable, well-focused
- **250-500 lines:** Needs further splitting
- **>500 lines:** Immediate refactoring required

### Line Count by Type

```bash
# Count by file type
find crates/zjj-core/src/TARGET_NAME -name '*.rs' -type f -exec sh -c 'echo "$(wc -l < "$1") $1"' _ {} \; | sort -rn
```

---

## Verification Commands

### Quick Checks

```bash
# All checks pass
moon run :quick

# Tests pass
moon run :test

# Build succeeds
moon run :build
```

### Comprehensive Checks

```bash
# Compare against baseline
diff <(wc -l .file-sizes-before.txt) <(find crates -name '*.rs' -type f -exec wc -l {} + | sort -rn)

# Test count maintained
test $(grep -r "#\[test\]" crates | wc -l) -ge 723 && echo "Test count OK"

# Clippy clean
test $(moon run :check 2>&1 | grep -i warning | wc -l) -eq 0 && echo "Clippy clean"
```

---

## Example: Splitting beads.rs (2,130 lines)

### Before
```
crates/zjj-core/src/
  beads.rs  (2,130 lines)
```

### After
```
crates/zjj-core/src/beads/
  mod.rs         (50 lines)   - Re-exports, module organization
  types.rs       (180 lines)  - BeadIssue, IssueStatus, Priority, BeadFilter
  query.rs       (220 lines)  - query_beads, query_labels, query_dependencies
  filter.rs      (190 lines)  - filter_issues, sort_issues, paginate
  analysis.rs    (240 lines)  - find_blockers, calculate_critical_path, find_stale
  summary.rs     (140 lines)  - BeadsSummary, aggregation functions
  tests/         - Large test suites if needed
```

### Commands Used

```bash
mkdir -p crates/zjj-core/src/beads
touch crates/zjj-core/src/beads/{mod,types,query,filter,analysis,summary}.rs

# Extract types (copy-paste from beads.rs lines 1-200)
# Extract query functions (copy-paste from beads.rs lines 395-626)
# ...etc

git rm crates/zjj-core/src/beads.rs
git add crates/zjj-core/src/beads/
git commit -m "refactor: split beads.rs into modular structure (refs: zjj-uxqs.3)"
```

---

## Maintenance

### After Refactoring

1. **Update CHANGELOG.md** if public API changed
2. **Update documentation** in `/docs` if architectural changes
3. **Update baseline metrics:**
```bash
find crates -name '*.rs' -type f -exec wc -l {} + | sort -rn > .file-sizes-after.txt
```

### Future Splits

As modules grow beyond 250 lines, repeat this process:
```bash
# Check for growing modules
find crates -name '*.rs' -type f -exec sh -c 'wc -l < "$1" | xargs -I {} sh -c "test {} -gt 250 && echo {} $1"' _ {} \;
```

---

## Appendix: Quick Reference Checklist

**Pre-Extraction:**
- [ ] Baseline metrics captured
- [ ] All tests passing
- [ ] File >250 lines
- [ ] Logical boundaries identified

**During Extraction:**
- [ ] Module directory created
- [ ] mod.rs with re-exports created
- [ ] Types extracted first
- [ ] Pure functions extracted
- [ ] Imperative shell extracted
- [ ] Tests migrated
- [ ] Incremental testing done

**Post-Extraction:**
- [ ] All modules <250 lines
- [ ] moon run :ci passes
- [ ] Zero clippy warnings
- [ ] Test count >= baseline
- [ ] Public API unchanged
- [ ] Committed with refs: to bead ID

**Rollback Triggers:**
- Tests fail after extraction
- Clippy warnings introduced
- Compilation errors not resolved in 15 min
- Public API accidentally broken

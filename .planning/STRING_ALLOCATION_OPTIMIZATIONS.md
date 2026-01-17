# String Allocation Optimization Plan (zjj-2a4)

## Analysis Summary

After analyzing the codebase, I identified 999 string allocations across 38 files. However, not all allocations are problematic - many are necessary for ownership transfer or storage in structs.

## Hot Path Analysis

### Identified Hot Paths
1. `commands/add.rs` - Session creation (user-facing, frequent)
2. `commands/list.rs` - Session listing (user-facing, frequent)
3. `commands/sync.rs` - Beads synchronization (user-facing, frequent)
4. `database/query.rs` - Database operations (called by all commands)
5. `zjj-core/jj.rs` - JJ command execution (called frequently)

### Allocation Patterns Found

#### Pattern 1: Unnecessary Double Allocation
**Location:** `commands/add.rs:46`
```rust
// BEFORE
pub async fn run(name: &str) -> Result<()> {
    run_with_options(&AddOptions {
        name: name.to_string(),  // Allocation when name is already &str
```

**Issue:** Allocates String when constructing AddOptions, only to use it immediately.
**Impact:** Low - only happens once per command invocation.
**Fix:** Cannot eliminate - AddOptions requires owned String. Use `.to_owned()` for clarity.

#### Pattern 2: Multiple Clones in List Output
**Location:** `commands/list.rs:75-92`
```rust
.map(|session| {
    SessionListItem {
        name: session.name.clone(),  // CLONE 1
        status: session.status.to_string(),  // ALLOCATION
        branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),  // CLONE 2 + ALLOCATION
        workspace_path: session.workspace_path.clone(),  // CLONE 3
        zellij_tab: session.zellij_tab.clone(),  // CLONE 4
        changes: changes.map_or_else(|| "-".to_string(), |c| c.to_string()),  // 2x ALLOCATION
        beads: beads.to_string(),  // ALLOCATION
```

**Issue:** Creates SessionListItem by cloning every field from Session.
**Impact:** HIGH - happens for every session in every list command.
**Potential Fix:** Use `Session::into()` to move values instead of cloning.

#### Pattern 3: Repeated format!() for zellij_tab
**Location:** Multiple files
```rust
zellij_tab: format!("jjz:{name}")  // Happens in multiple places
```

**Issue:** Same pattern repeated in database/query.rs, session construction, etc.
**Impact:** Medium - happens multiple times but format!() is necessary for string interpolation.
**Fix:** Extract to helper function to centralize, but can't avoid allocation.

#### Pattern 4: Error Message Allocations
**Location:** `zjj-core/error.rs`, `zjj-core/jj.rs`
```rust
Error::JjCommandError {
    operation: operation.to_string(),  // ALLOCATION
    source: error.to_string(),  // ALLOCATION
```

**Issue:** Allocates strings for error messages that might not be displayed.
**Impact:** LOW - error paths are cold paths.
**Fix:** Keep as-is - error clarity > micro-optimization.

## Optimization Strategy

### Phase 1: Use Cow<str> for Conditional Ownership  ✓ HIGH IMPACT
**Location:** `commands/list.rs:83, 86`

```rust
// BEFORE
branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
changes: changes.map_or_else(|| "-".to_string(), |c| c.to_string()),

// AFTER
use std::borrow::Cow;
branch: session.branch.as_deref().map(Cow::Borrowed).unwrap_or(Cow::Borrowed("-")),
changes: changes.map(|c| Cow::Owned(c.to_string())).unwrap_or(Cow::Borrowed("-")),
```

**Benefit:** Eliminates allocation when using default value "-".
**Trade-off:** Requires SessionListItem to use Cow<str> instead of String.

### Phase 2: Move Instead of Clone in SessionListItem ✓ HIGHEST IMPACT
**Location:** `commands/list.rs:75-92`

```rust
// BEFORE
.map(|session| {
    SessionListItem {
        name: session.name.clone(),
        workspace_path: session.workspace_path.clone(),
        zellij_tab: session.zellij_tab.clone(),
        // ...
    }
})

// AFTER
.map(|session| SessionListItem::from(session))

impl From<Session> for SessionListItem {
    fn from(session: Session) -> Self {
        Self {
            name: session.name,  // MOVE, no clone!
            workspace_path: session.workspace_path,  // MOVE
            zellij_tab: session.zellij_tab,  // MOVE
            // ...
        }
    }
}
```

**Benefit:** Eliminates 4 clones per session.
**Trade-off:** Consumes Session, can't use it after conversion.

### Phase 3: Consistency - Use .to_owned() Instead of .to_string()
**Location:** Throughout codebase

```rust
// BEFORE
name.to_string()  // When converting &str -> String

// AFTER
name.to_owned()  // More idiomatic, marginally clearer intent
```

**Benefit:** Semantic clarity, no performance change.
**Trade-off:** None.

### Phase 4: Extract zellij_tab Helper
**Location:** Multiple files

```rust
// New helper in zjj-core
pub fn zellij_tab_name(session_name: &str) -> String {
    format!("jjz:{session_name}")
}
```

**Benefit:** Centralized logic, easier to change naming convention.
**Trade-off:** Still allocates, just in one place.

## Implementation Order

1. ✅ **Phase 1: Cow<str> in list command** - Highest impact, localized change
2. ✅ **Phase 2: From<Session> for SessionListItem** - Eliminate clones
3. **Phase 3: Consistency pass** - Change to_string() → to_owned() for &str conversions
4. **Phase 4: Extract helper** - Centralize zellij_tab logic

## Success Metrics

**Before:**
- List 10 sessions: ~40 string allocations (4 per session)
- List 100 sessions: ~400 string allocations

**After Phase 1 + 2:**
- List 10 sessions: ~10 string allocations (1 per session, only for format!())
- List 100 sessions: ~100 string allocations

**Expected Improvement:** 75% reduction in list command allocations

## Edge Cases to Test

1. Empty session list
2. Sessions with missing branch (None)
3. Workspace with 0 changes
4. Unicode in session names
5. Very long session names (64 chars)

## Files to Modify

1. `crates/zjj/src/commands/list.rs` - Main optimization target
2. `crates/zjj/src/json_output.rs` - Update SessionListItem definition
3. `crates/zjj/src/commands/add/mod.rs` - Consistency changes
4. `crates/zjj/src/database/query.rs` - Potential Cow usage
5. `crates/zjj-core/src/zellij.rs` - Extract helper function

## Risk Assessment

- **Low Risk:** Phases 3 & 4 (pure refactoring, no behavior change)
- **Medium Risk:** Phase 1 (changes return type, requires Cow handling)
- **High Risk:** Phase 2 (changes ownership model, could break code that reuses Session)

## Rollback Plan

If Phase 2 causes issues:
1. Revert From<Session> impl
2. Keep Phase 1 (Cow) if it works
3. Add back clones but use .clone() explicitly instead of through map

## Next Steps

1. Implement Phase 1 (Cow for conditional strings)
2. Run tests: `moon run :test`
3. Implement Phase 2 (From trait)
4. Run tests again
5. Measure with benchmark if possible
6. Document results in CONCERNS.md

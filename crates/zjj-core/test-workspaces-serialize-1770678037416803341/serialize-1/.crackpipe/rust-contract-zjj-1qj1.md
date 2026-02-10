# Rust Contract Specification: Fix Bookmark Move (zjj-1qj1)

**Generated**: 2026-02-08 06:00:00 UTC
**Bead**: zjj-1qj1
**Title**: bookmark: Fix bookmark move fails
**Issue Type**: Bug fix (CRITICAL-007 + CRITICAL-016)

---

## Problem Statement

The `zjj bookmark move` command has two critical issues:

**CRITICAL-007**: Parser dependency issue
- Bookmark parsing may fail incorrectly
- Depends on fragile parsing logic

**CRITICAL-016**: Validation failure
- Moving to non-existent revision creates the bookmark (BUG!)
- Should reject with error instead

**Current Broken Behavior**:
```bash
zjj bookmark move my-bookmark --to nonexistent-revision
# Creates bookmark pointing to non-existent revision (WRONG!)
```

**Expected Correct Behavior**:
```bash
zjj bookmark move my-bookmark --to nonexistent-revision
# Error: Revision 'nonexistent-revision' not found
# Exit code: 3
# No bookmark created
```

---

## Module Structure

**Files Involved**:
- `crates/zjj/src/commands/bookmark.rs` - Main bookmark command
- `crates/zjj-core/src/bookmark/mod.rs` - Bookmark operations
- `crates/zjj-core/src/jj/mod.rs` - JJ backend integration

**Changes Required**:
1. Fix CRITICAL-007: Refactor bookmark parser (dependency issue)
2. Fix CRITICAL-016: Add revision validation before moving
3. Ensure error messages are clear
4. Add tests for both fixes

---

## Public API (No Changes)

The bookmark move API remains unchanged - we're fixing bugs, not changing the interface.

```rust
pub async fn move_bookmark(
    name: &str,
    to_revision: &str,
) -> Result<()>
```

---

## Error Taxonomy

### Error::BookmarkNotFound
- When: Source bookmark does not exist
- User message: "Bookmark '{name}' not found"
- Exit code: 3 (invalid argument)
- Recovery: List available bookmarks with `zjj bookmark list`

### Error::RevisionNotFound (NEW - CRITICAL-016 fix)
- When: Target revision does not exist in repository
- User message: "Revision '{revision}' not found"
- Exit code: 3 (invalid argument)
- Recovery: Check available revisions with `jj log`

### Error::InvalidBookmarkName
- When: Bookmark name contains invalid characters
- User message: "Invalid bookmark name: {reason}"
- Exit code: 3 (invalid argument)
- Recovery: Use alphanumeric names with hyphens

### Error::BookmarkMoveFailed
- When: Underlying `jj` move operation fails
- User message: "Failed to move bookmark: {reason}"
- Exit code: 5 (I/O error)
- Recovery: Check JJ repository health

---

## Preconditions

### For Move Operation
- Source bookmark must exist
- Target revision must exist in repository (NEW - CRITICAL-016)
- Repository must be in valid state
- User must have write permissions

### For Parser (CRITICAL-007 fix)
- Bookmark name must be parseable
- Revision string must be valid format
- No ambiguous references

---

## Postconditions

### Success Case
- Bookmark points to target revision
- Bookmark list reflects updated target
- Return value: `Ok(())`
- Exit code: 0

### Failure Cases
- No bookmark created if target revision doesn't exist (CRITICAL-016)
- Error message is specific and actionable
- Exit code: 3 for invalid input, 5 for system errors
- Repository state unchanged (atomic operation)

---

## Invariants

### Bookmark State
- Bookmark always points to valid revision (never dangling)
- Bookmark move through zjj equals jj bookmark move result
- No partial updates (all-or-nothing)

### Parser Behavior (CRITICAL-007 fix)
- Parser accepts all valid JJ bookmark names
- Parser rejects invalid names with clear error
- No ambiguous parsing results

### Validation (CRITICAL-016 fix)
- Moving to non-existent revision ALWAYS fails
- Error message mentions specific revision that wasn't found
- No bookmarks created on validation failure

---

## Contract Signatures

```rust
/// Move bookmark to different revision
///
/// # Errors
///
/// - Returns `Error::BookmarkNotFound` if source bookmark doesn't exist
/// - Returns `Error::RevisionNotFound` if target revision doesn't exist (NEW!)
/// - Returns `Error::InvalidBookmarkName` if bookmark name is invalid
/// - Returns `Error::BookmarkMoveFailed` if JJ operation fails
pub async fn move_bookmark(
    name: &str,
    to_revision: &str,
) -> Result<()>

/// Validate revision exists (NEW - CRITICAL-016 fix)
///
/// # Errors
///
/// - Returns `Error::RevisionNotFound` if revision doesn't exist
async fn validate_revision_exists(
    revision: &str,
) -> Result<()>

/// Parse bookmark name (CRITICAL-007 fix)
///
/// # Errors
///
/// - Returns `Error::InvalidBookmarkName` if name is invalid
fn parse_bookmark_name(
    name: &str,
) -> Result<String>
```

---

## Implementation Strategy

### Phase 0: Research
- Read existing bookmark implementation
- Review CRITICAL-007 parser issues
- Review CRITICAL-016 validation gaps
- Document current broken behavior

### Phase 1: Add Tests (TDD)
1. Test moving to valid revision succeeds
2. Test moving to non-existent revision fails (CRITICAL-016)
3. Test parser accepts valid names (CRITICAL-007)
4. Test parser rejects invalid names (CRITICAL-007)
5. Test error messages are clear

### Phase 2: Implementation
1. Implement `validate_revision_exists()` function
2. Call validation before move operation
3. Fix parser dependency issues (CRITICAL-007)
4. Update error handling to use new errors

### Phase 3: Verification
- All tests pass
- Manual testing with real JJ repo
- Error messages verified
- Integration tests pass

---

## Non-goals
- No changes to bookmark creation
- No changes to bookmark deletion
- No changes to bookmark listing
- No changes to other bookmark commands

---

## Testing Requirements

### Unit Tests Required

1. **CRITICAL-016: Reject non-existent revision**
   ```rust
   #[tokio::test]
   async fn move_to_nonexistent_revision_fails() {
       // Setup: Create bookmark
       // Execute: Move to non-existent revision
       // Verify: Returns Error::RevisionNotFound
       // Verify: No bookmark created
       // Verify: Error message mentions revision
   }
   ```

2. **CRITICAL-007: Parser accepts valid names**
   ```rust
   #[test]
   fn parse_bookmark_accepts_valid_names() {
       // Test various valid bookmark names
       // Should all parse successfully
   }
   ```

3. **CRITICAL-007: Parser rejects invalid names**
   ```rust
   #[test]
   fn parse_bookmark_rejects_invalid_names() {
       // Test invalid bookmark names
       // Should return Error::InvalidBookmarkName
   }
   ```

4. **Happy path: Move succeeds**
   ```rust
   #[tokio::test]
   async fn move_to_valid_revision_succeeds() {
       // Setup: Create bookmark and revision
       // Execute: Move bookmark
       // Verify: Bookmark points to new revision
       // Verify: Returns Ok(())
   }
   ```

### Integration Tests Required

1. **End-to-end workflow**
   ```bash
   zjj bookmark create my-bookmark
   zjj bookmark move my-bookmark --to main
   # Should succeed
   ```

2. **Error workflow**
   ```bash
   zjj bookmark move my-bookmark --to nonexistent
   # Should fail with clear error
   ```

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: Follow Rule 4 of CLAUDE.md

```rust
// ❌ FORBIDDEN
let revision = revisions.get(name).unwrap();
let count = bookmarks.len().expect("non-empty");

// ✅ REQUIRED
let revision = revisions.get(name)
    .ok_or_else(|| Error::RevisionNotFound(name.to_string()))?;
```

**All validation functions** must return `Result<T, Error>` with semantic errors.

---

## Success Criteria

1. ✅ CRITICAL-007 fixed: Parser no longer has dependency issues
2. ✅ CRITICAL-016 fixed: Moving to non-existent revision fails
3. ✅ Error messages are clear and actionable
4. ✅ All tests pass (unit + integration)
5. ✅ No unwrap/expect/panic in new code
6. ✅ Manual testing confirms fix

---

## Performance Constraints

- Bookmark move must complete in <500ms
- Revision validation must complete in <100ms
- No performance regression for valid moves

---

## Migration Guide

No migration needed - this is a bug fix only.

**For users**:
- If you were relying on broken behavior (creating bookmarks to non-existent revisions), you'll now get an error
- This is correct behavior - the previous behavior was a bug

---

## Documentation Updates

- Update `docs/bookmarks.md` to clarify validation behavior
- Add error code reference
- Update examples with error handling

---

**Contract Status**: ✅ Ready for Builder

**Estimated Implementation Time**: 1 hour (2 critical bugs)

**Risk Level**: Medium (fixing validation, may affect users relying on broken behavior)

**Confidence**: High (clear bugs, clear fixes)

# Martin Fowler Test Plan: Fix Bookmark Move (zjj-1qj1)

**Generated**: 2026-02-08 06:00:30 UTC
**Bead**: zjj-1qj1
**Contract**: `/tmp/rust-contract-zjj-1qj1.md`
**Issue Type**: Bug fix (CRITICAL-007 + CRITICAL-016)

---

## Test Strategy

This is a **dual bug fix**:
1. **CRITICAL-007**: Parser dependency issue
2. **CRITICAL-016**: Validation failure (creates bookmarks to non-existent revisions)

Our test strategy focuses on:
1. **Regression Prevention**: Ensure valid moves still work
2. **Bug Validation**: Prove both bugs are fixed
3. **Error Quality**: Verify error messages are clear
4. **Edge Cases**: Parser handles various inputs

**Martin Fowler Principles Applied**:
- **Classical Testing**: Verify state (bookmark points to correct revision)
- **Minimal Mocking**: Test against real JJ repository (temp)
- **Clear Intent**: Test names describe behavior, not implementation

---

## Test Categories

### 1. Happy Path Tests (Regression Prevention)

**Purpose**: Ensure we don't break existing functionality.

```rust
#[tokio::test]
async fn move_bookmark_to_valid_revision_succeeds() {
    // Setup: Create temp JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    
    // Create bookmark pointing to revision A
    repo.create_bookmark("test-bookmark", "revision-a").await.unwrap();
    
    // Create revision B
    repo.create_commit("revision-b").await.unwrap();
    
    // Execute: Move bookmark from A to B
    let result = move_bookmark("test-bookmark", "revision-b").await;
    
    // Verify: Success
    assert!(result.is_ok(), "Move to valid revision should succeed");
    
    // Verify: Bookmark now points to B
    let bookmark = repo.get_bookmark("test-bookmark").await.unwrap();
    assert_eq!(bookmark.target, "revision-b", 
               "Bookmark should point to new revision");
}

#[tokio::test]
async fn move_bookmark_preserves_other_bookmarks() {
    // Setup: Multiple bookmarks
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    repo.create_bookmark("bookmark-1", "rev-1").await.unwrap();
    repo.create_bookmark("bookmark-2", "rev-2").await.unwrap();
    repo.create_bookmark("bookmark-3", "rev-3").await.unwrap();
    
    // Execute: Move only bookmark-2
    move_bookmark("bookmark-2", "rev-new").await.unwrap();
    
    // Verify: Other bookmarks unchanged
    assert_eq!(repo.get_bookmark("bookmark-1").await.unwrap().target, "rev-1");
    assert_eq!(repo.get_bookmark("bookmark-2").await.unwrap().target, "rev-new");
    assert_eq!(repo.get_bookmark("bookmark-3").await.unwrap().target, "rev-3");
}
```

**Fowler's Classification**: **State Verification** (Classical)
- Checking final state (bookmark targets)
- Real repository operations
- No mocks

---

### 2. CRITICAL-016: Reject Non-Existent Revision

**Purpose**: Fix the critical bug where moving to non-existent revision creates bookmark.

```rust
#[tokio::test]
async fn move_to_nonexistent_revision_fails_with_error() {
    // Setup: Create bookmark
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    repo.create_bookmark("test-bookmark", "revision-a").await.unwrap();
    
    // Execute: Try to move to non-existent revision
    let result = move_bookmark("test-bookmark", "nonexistent-revision").await;
    
    // Verify: Fails with correct error
    assert!(result.is_err(), "Move to non-existent revision should fail");
    
    let error = result.unwrap_err();
    assert!(matches!(error, Error::RevisionNotFound(_)), 
            "Should return RevisionNotFound error");
    
    // Verify: Error message mentions the revision
    let error_msg = error.to_string();
    assert!(error_msg.contains("nonexistent-revision"), 
            "Error should mention the revision that wasn't found");
}

#[tokio::test]
async fn move_to_nonexistent_revision_does_not_create_bookmark() {
    // Setup: No bookmark exists yet
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    
    // Execute: Try to move non-existent bookmark to non-existent revision
    let result = move_bookmark("new-bookmark", "nonexistent-revision").await;
    
    // Verify: Fails
    assert!(result.is_err());
    
    // Verify: No bookmark created (CRITICAL-016 bug fix!)
    let bookmark = repo.get_bookmark("new-bookmark").await;
    assert!(bookmark.is_err(), 
            "Bookmark should NOT be created for non-existent revision");
}

#[tokio::test]
async fn move_to_nonexistent_revision_leaves_original_bookmark_unchanged() {
    // Setup: Bookmark pointing to revision A
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    repo.create_bookmark("test-bookmark", "revision-a").await.unwrap();
    
    // Execute: Try to move to non-existent revision
    let result = move_bookmark("test-bookmark", "nonexistent").await;
    
    // Verify: Fails
    assert!(result.is_err());
    
    // Verify: Original bookmark unchanged (atomic operation)
    let bookmark = repo.get_bookmark("test-bookmark").await.unwrap();
    assert_eq!(bookmark.target, "revision-a", 
               "Bookmark should remain pointing to original revision");
}
```

**Fowler's Classification**: **State Verification** (Classical)
- Verifying no state change on error
- Checking atomic behavior (all-or-nothing)
- Real repository operations

**Test Smell Avoided**:
- ❌ Only checking return value (doesn't prove no bookmark created)
- ✅ Checking both return value AND repository state

---

### 3. CRITICAL-007: Parser Behavior

**Purpose**: Fix parser dependency issues and ensure robust parsing.

```rust
#[test]
fn parse_bookmark_accepts_valid_names() {
    let valid_names = vec![
        "simple",
        "with-hyphen",
        "with_underscore",
        "CamelCase",
        "snake_case",
        "kebab-case",
        "with123numbers",
        "a", // single character
        "very-long-name-with-many-parts",
    ];
    
    for name in valid_names {
        let result = parse_bookmark_name(name);
        assert!(result.is_ok(), 
                "Should accept valid bookmark name: '{}'", name);
        assert_eq!(result.unwrap(), name,
                   "Parsed name should match input");
    }
}

#[test]
fn parse_bookmark_rejects_invalid_names() {
    let invalid_names = vec![
        "", // empty
        "with space", // space
        "with/slash", // slash
        "with\\backslash", // backslash
        "with:colon", // colon
        "with;semicolon", // semicolon
        "with.period", // period (may conflict with JJ)
        "with@at", // at sign
        "with#hash", // hash
    ];
    
    for name in invalid_names {
        let result = parse_bookmark_name(name);
        assert!(result.is_err(), 
                "Should reject invalid bookmark name: '{}'", name);
        
        let error = result.unwrap_err();
        assert!(matches!(error, Error::InvalidBookmarkName { .. }),
                "Should return InvalidBookmarkName error");
    }
}

#[test]
fn parse_bookmark_provides_clear_error_messages() {
    let result = parse_bookmark_name("invalid name");
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    
    assert!(error_msg.contains("Invalid bookmark name"), 
            "Error should mention it's about bookmark name");
    assert!(error_msg.contains("space") || error_msg.contains("character"), 
            "Error should mention the specific problem");
}
```

**Fowler's Classification**: **Behavior Verification** (Mockist/Classical hybrid)
- Testing parser behavior (isolated function)
- Table-driven tests (many inputs)
- Checking error message quality

---

### 4. Error Message Quality Tests

**Purpose**: Ensure users get helpful error messages.

```rust
#[tokio::test]
async fn error_messages_are_specific_and_actionable() {
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    
    // Test 1: Non-existent bookmark
    let result = move_bookmark("nonexistent-bookmark", "revision-a").await;
    let error = result.unwrap_err();
    let msg = error.to_string();
    
    assert!(msg.contains("Bookmark") || msg.contains("bookmark"), 
            "Error should mention 'bookmark'");
    assert!(msg.contains("not found") || msg.contains("doesn't exist"), 
            "Error should say bookmark doesn't exist");
    
    // Test 2: Non-existent revision
    repo.create_bookmark("test", "rev-a").await.unwrap();
    let result = move_bookmark("test", "nonexistent-rev").await;
    let error = result.unwrap_err();
    let msg = error.to_string();
    
    assert!(msg.contains("Revision") || msg.contains("revision"), 
            "Error should mention 'revision'");
    assert!(msg.contains("nonexistent-rev"), 
            "Error should mention the specific revision");
}

#[test]
fn error_exit_codes_are_correct() {
    // This would be an integration test checking CLI exit codes
    // Exit code 3: Invalid argument (not found, invalid name)
    // Exit code 5: I/O error (JJ operation failed)
}
```

**Fowler's Classification**: **Documentation Testing**
- Verifying error messages meet usability standards
- Testing user-facing strings
- Ensuring actionability

---

### 5. Integration Tests (End-to-End)

**Purpose**: Verify full workflow works with real CLI.

```bash
#!/bin/bash
# test/integration/bookmark_move.sh

set -euo pipefail

# Setup: Create test environment
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
jj init --config-toml=""
jj bookmark create main

# Create test commits
echo "test" > file.txt
jj new
COMMIT_1=$(jj log -T commit_id -r @ -R.)

echo "test2" > file2.txt
jj new
COMMIT_2=$(jj log -T commit_id -r @ -R.)

# Test 1: Create and move bookmark (happy path)
zjj bookmark create test-bookmark --to "$COMMIT_1"
zjj bookmark move test-bookmark --to "$COMMIT_2"

# Verify: Bookmark moved
TARGET=$(zjj bookmark show test-bookmark --json | jq -r '.target')
if [ "$TARGET" = "$COMMIT_2" ]; then
    echo "✓ Happy path: Bookmark moved successfully"
else
    echo "✗ Happy path: Bookmark not moved to correct revision"
    exit 1
fi

# Test 2: Move to non-existent revision (CRITICAL-016)
if zjj bookmark move test-bookmark --to "nonexistent-revision" 2>&1 | grep -q "not found"; then
    echo "✓ CRITICAL-016: Non-existent revision rejected"
else
    echo "✗ CRITICAL-016: Should reject non-existent revision"
    exit 1
fi

# Verify: Original bookmark unchanged
TARGET=$(zjj bookmark show test-bookmark --json | jq -r '.target')
if [ "$TARGET" = "$COMMIT_2" ]; then
    echo "✓ CRITICAL-016: Bookmark unchanged after failed move"
else
    echo "✗ CRITICAL-016: Bookmark should remain unchanged"
    exit 1
fi

# Test 3: Invalid bookmark name (CRITICAL-007)
if zjj bookmark move "invalid name" --to "$COMMIT_1" 2>&1 | grep -q "Invalid"; then
    echo "✓ CRITICAL-007: Invalid bookmark name rejected"
else
    echo "✗ CRITICAL-007: Should reject invalid bookmark name"
    exit 1
fi

# Cleanup
cd -
rm -rf "$TEST_DIR"

echo "✓ All integration tests passed"
```

**Fowler's Classification**: **State Verification** (Classical)
- Real CLI invocation
- Real JJ repository
- Real filesystem
- No test doubles

---

## Test Doubles Strategy

### What We DON'T Mock:

1. **JJ Repository**: Use real temp repo with `jj init`
2. **Filesystem**: Use `tempfile` crate for real temp directories
3. **Parser**: Test real parser function
4. **CLI Invocation**: Integration tests run real binary

### What We MIGHT Mock (If Needed):

1. **JJ Command Execution**: For unit tests (not integration)
   ```rust
   // If we need to test error handling without real repo
   let mock_jj = MockJJRunner::new();
   when!(mock_jj.run).thenReturn(Err("revision not found"));
   ```
   
   **But**: Prefer integration tests with real repo.

2. **Current Time**: For deterministic timestamps
   ```rust
   // Use chrono::Utc::now() (fast enough, no need to mock)
   ```

**Fowler's Advice**: Fakes over mocks. For this bug fix, integration tests with real repo are best.

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Line Coverage** | 95% | Bug fix needs high coverage |
| **Branch Coverage** | 90% | Many error paths to test |
| | | |
| **Specific Coverage** | | |
| `move_bookmark()` | 100% | Core function being fixed |
| `validate_revision_exists()` | 100% | New function (CRITICAL-016) |
| `parse_bookmark_name()` | 100% | Fixed function (CRITICAL-007) |
| Error paths | 100% | All error variants tested |

---

## Regression Test Checklist

Before merging, verify:

- [ ] All existing bookmark tests pass
- [ ] New tests for CRITICAL-007 pass
- [ ] New tests for CRITICAL-016 pass
- [ ] Integration tests pass
- [ ] Error messages are clear (manual review)
- [ ] No new unwrap/expect/panic patterns
- [ ] `moon run :ci` passes
- [ ] Manual testing with real JJ repo

---

## Property-Based Tests (Optional)

Using `proptest` to verify parser invariants:

```rust
proptest! {
    #[test]
    fn parse_bookmark_roundtrip(name in "[a-zA-Z0-9_-]{1,50}") {
        // Property: Valid names parse successfully
        let result = parse_bookmark_name(&name);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), name);
    }
    
    #[test]
    fn parse_rejects_invalid_chars(name in "[^a-zA-Z0-9_-]") {
        // Property: Names with invalid chars fail
        let invalid_name = format!("invalid{}", name);
        let result = parse_bookmark_name(&invalid_name);
        prop_assert!(result.is_err());
    }
}
```

**Recommendation**: Add property tests for parser. Critical for validation logic.

---

## Test Smells to Avoid

### 1. **Testing Implementation Details**

❌ **Bad**: Testing internal parser state
```rust
assert!(parser.state == ParserState::Valid);
```

✅ **Good**: Testing parser behavior
```rust
assert!(parse_bookmark_name("valid").is_ok());
```

### 2. **Fragile Error Message Tests**

❌ **Bad**: Exact string matching
```rust
assert_eq!(error_msg, "Revision 'xyz' not found");
```

✅ **Good**: Key content matching
```rust
assert!(error_msg.contains("not found"));
assert!(error_msg.contains("xyz"));
```

### 3. **Missing State Verification**

❌ **Bad**: Only checking return value
```rust
assert!(move_bookmark(...).is_err());
```

✅ **Good**: Checking return value AND state
```rust
assert!(move_bookmark(...).is_err());
assert!(repo.get_bookmark(...).is_err()); // No bookmark created!
```

---

## Continuous Integration (CI) Configuration

Add to `.github/workflows/test.yml`:

```yaml
name: Bookmark Move Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1
      - run: moon run :test bookmark
      - run: |
          # Run integration tests
          ./test/integration/bookmark_move.sh
```

---

## Performance Tests

Verify moves are still fast after adding validation:

```rust
#[tokio::test]
async fn move_bookmark_completes_under_500ms() {
    let repo = create_test_repo(temp_dir.path()).await.unwrap();
    repo.create_bookmark("test", "rev-a").await.unwrap();
    
    let start = std::time::Instant::now();
    let result = move_bookmark("test", "rev-b").await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    assert!(elapsed.as_millis() < 500, 
            "Bookmark move should complete in <500ms, took {}", elapsed.as_millis());
}
```

---

## Manual Testing Checklist

Before closing bead:

- [ ] Create real JJ repo with bookmarks
- [ ] Move bookmark to valid revision (succeeds)
- [ ] Move bookmark to non-existent revision (fails with clear error)
- [ ] Try invalid bookmark names (rejected)
- [ ] Check error messages are helpful
- [ ] Verify no bookmarks created on validation failure
- [ ] Test with various bookmark names (hyphens, underscores, etc.)

---

## Post-Deployment Monitoring

After merging, watch for:

1. **User reports**: "Bookmark move stopped working"
   - Action: Check if they were relying on broken behavior
   - Investigate: Regression in happy path

2. **Error reports**: "Can't move bookmarks anymore"
   - Action: Check if revision validation is too strict
   - Investigate: Valid rejections being treated as bugs

3. **Parser issues**: "My bookmark name is rejected"
   - Action: Review if parser is too restrictive
   - Consider: Relaxing validation if needed

---

## Summary

**Test Approach**: Classical (state verification) + Error path testing

**Test Count**: ~10 tests
- 2 happy path tests (regression)
- 3 CRITICAL-016 tests (validation)
- 3 CRITICAL-007 tests (parser)
- 2 error quality tests

**Execution Time**: <10 seconds (fast feedback)

**Risk Coverage**: High (both critical bugs have tests)

**Fowler Compliance**: ✅
- ✅ Minimal mocking (real repo preferred)
- ✅ Clear intent (test names describe behavior)
- ✅ State verification (check actual bookmark state)
- ✅ No test smells (no brittle assertions)

---

**Test Plan Status**: ✅ Ready for Builder

**Estimated Test Implementation Time**: 45 minutes

**Confidence Level**: High (clear bugs, clear tests)

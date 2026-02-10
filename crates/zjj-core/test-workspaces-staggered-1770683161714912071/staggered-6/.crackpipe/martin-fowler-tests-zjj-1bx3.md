# Martin Fowler Test Plan: Ghost Sessions After Corruption (zjj-1bx3)

**Generated**: 2026-02-08 06:44:30 UTC
**Bead**: zjj-1bx3
**Contract**: `.crackpipe/rust-contract-zjj-1bx3.md`
**Issue Type**: Bug fix (database validation)

---

## Test Strategy

Since this is a **database validation bug fix**, our test strategy focuses on:

1. **Validation Logic**: Ensure corrupt data is detected
2. **Filtering Behavior**: Verify corrupt entries are excluded
3. **Error Handling**: System doesn't crash on corruption
4. **Performance**: Validation doesn't slow down normal operations

**Martin Fowler Principles Applied**:
- **Classical Testing**: Verify final state (filtered list)
- **Realistic Data**: Use actual corrupt database entries
- **No Mocking**: Test against real SQLite database
- **Clear Intent**: Test names describe behavior, not implementation

---

## Test Categories

### 1. Validation Unit Tests

**Purpose**: Verify validation logic detects all corruption scenarios.

```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    fn make_session(name: &str, path: &str, metadata: Option<&str>) -> SessionRow {
        SessionRow {
            id: 1,
            name: name.to_string(),
            workspace_path: path.to_string(),
            metadata: metadata.map(|s| s.to_string()),
            created_at: Utc::now(),
            is_active: false,
        }
    }

    #[test]
    fn valid_session_passes_validation() {
        let session = make_session("test-session", "/tmp/workspace", None);
        assert!(validate_session(&session).is_ok());
    }

    #[test]
    fn empty_name_is_rejected() {
        let session = make_session("", "/tmp/workspace", None);
        assert!(matches!(
            validate_session(&session),
            Err(SessionError::InvalidName(_))
        ));
    }

    #[test]
    fn empty_path_is_rejected() {
        let session = make_session("test", "", None);
        assert!(matches!(
            validate_session(&session),
            Err(SessionError::InvalidPath(_))
        ));
    }

    #[test]
    fn invalid_metadata_json_is_rejected() {
        let session = make_session("test", "/tmp/workspace", Some("not json"));
        assert!(matches!(
            validate_session(&session),
            Err(SessionError::InvalidMetadata)
        ));
    }

    #[test]
    fn valid_metadata_passes() {
        let metadata = r#"{"created":"2024-01-01","tags":["test"]}"#;
        let session = make_session("test", "/tmp/workspace", Some(metadata));
        assert!(validate_session(&session).is_ok());
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let session = make_session("   ", "/tmp/workspace", None);
        assert!(matches!(
            validate_session(&session),
            Err(SessionError::InvalidName(_))
        ));
    }

    #[test]
    fn whitespace_only_path_is_rejected() {
        let session = make_session("test", "   ", None);
        assert!(matches!(
            validate_session(&session),
            Err(SessionError::InvalidPath(_))
        ));
    }
}
```

**Fowler's Classification**: **State Verification** (Classical)
- Testing validation logic directly
- Checking error types match corruption scenarios
- No mocking, pure functions

**Test Smell Avoided**:
- ❌ Testing internal implementation details
- ✅ Testing public validation contract

---

### 2. Filtering Integration Tests

**Purpose**: Verify corrupt entries are filtered from listings.

```rust
#[tokio::test]
async fn list_sessions_filters_corrupt_entries() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Insert valid session
    db.create("valid-session", "/tmp/workspace").await.unwrap();

    // Insert corrupt entries directly (bypassing validation)
    sqlx::query("INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp')")
        .execute(&db.pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO sessions (name, workspace_path, metadata) VALUES ('test', '', 'invalid')")
        .execute(&db.pool)
        .await
        .unwrap();

    // List should only return valid session
    let sessions = db.list_sessions().await.unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "valid-session");
}

#[tokio::test]
async fn list_sessions_with_multiple_corrupt_entries() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Insert valid sessions
    db.create("session-1", "/tmp/workspace1").await.unwrap();
    db.create("session-2", "/tmp/workspace2").await.unwrap();

    // Insert multiple corrupt entries
    for i in 1..=10 {
        sqlx::query("INSERT INTO sessions (name, workspace_path) VALUES ('', ?)")
            .bind(format!("/tmp/corrupt{}", i))
            .execute(&db.pool)
            .await
            .unwrap();
    }

    // Should only return 2 valid sessions
    let sessions = db.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].name, "session-1");
    assert_eq!(sessions[1].name, "session-2");
}

#[tokio::test]
async fn list_sessions_returns_empty_when_all_corrupt() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Insert only corrupt entries
    sqlx::query("INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp')")
        .execute(&db.pool)
        .await
        .unwrap();

    // Should return empty list, not error
    let sessions = db.list_sessions().await.unwrap();
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn list_sessions_succeeds_when_database_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Empty database should not error
    let sessions = db.list_sessions().await.unwrap();
    assert!(sessions.is_empty());
}
```

**Fowler's Classification**: **State Verification** (Classical)
- Testing database state after operations
- Real SQLite database (in-memory or temp file)
- No mocks

**Test Smell Avoided**:
- ❌ Mocking database queries
- ✅ Using real database (realistic)

---

### 3. CLI Integration Tests

**Purpose**: Verify CLI handles corrupt databases gracefully.

```bash
#!/bin/bash
# test/integration/corruption_cli_test.sh

set -euo pipefail

echo "Testing CLI corruption handling..."

# Setup: Create test database
TEST_DIR=$(mktemp -d)
TEST_DB="$TEST_DIR/test.db"

# Create valid session
zjj --db "$TEST_DB" add valid-session --no-open

# Corrupt database
sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('', '/tmp')"
sqlite3 "$TEST_DB" "INSERT INTO sessions (name, workspace_path) VALUES ('ghost', '')"

# Test: List command doesn't crash
echo "Test: List command with corrupt database"
OUTPUT=$(zjj --db "$TEST_DB" list)
if [[ "$OUTPUT" == *"valid-session"* ]]; then
    echo "✓ List command works with corrupt database"
else
    echo "✗ List command failed"
    exit 1
fi

# Test: Ghost sessions don't appear
if [[ "$OUTPUT" != *"ghost"* ]] && [[ "$OUTPUT" != *""* ]]; then
    echo "✓ Ghost sessions filtered from output"
else
    echo "✗ Ghost sessions not filtered"
    echo "Output: $OUTPUT"
    exit 1
fi

# Test: Focus command handles ghost sessions
if zjj --db "$TEST_DB" focus ghost 2>&1 | grep -q "not found"; then
    echo "✓ Focus command rejects ghost session"
else
    echo "✗ Focus command didn't handle ghost session properly"
    exit 1
fi

# Test: Remove command handles ghost sessions
if zjj --db "$TEST_DB" remove ghost 2>&1 | grep -q "not found"; then
    echo "✓ Remove command rejects ghost session"
else
    echo "✗ Remove command didn't handle ghost session properly"
    exit 1
fi

# Cleanup
rm -rf "$TEST_DIR"

echo "✓ All CLI corruption tests passed"
```

**Fowler's Classification**: **State Verification** (Classical)
- Testing user-facing behavior
- Real CLI invocation
- Real database corruption

---

### 4. Performance Tests

**Purpose**: Ensure validation doesn't significantly slow down operations.

```rust
#[tokio::test]
async fn list_sessions_validation_performance() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Create 100 valid sessions
    for i in 0..100 {
        db.create(&format!("session-{}", i), "/tmp/workspace").await.unwrap();
    }

    // Measure list performance
    let start = std::time::Instant::now();
    let sessions = db.list_sessions().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(sessions.len(), 100);
    assert!(elapsed.as_millis() < 50, "List should complete in <50ms, took {}", elapsed.as_millis());
}

#[tokio::test]
async fn list_sessions_with_corruption_performance() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Create 100 valid sessions
    for i in 0..100 {
        db.create(&format!("session-{}", i), "/tmp/workspace").await.unwrap();
    }

    // Insert 50 corrupt sessions
    for i in 0..50 {
        sqlx::query("INSERT INTO sessions (name, workspace_path) VALUES ('', ?)")
            .bind(format!("/tmp/corrupt{}", i))
            .execute(&db.pool)
            .await
            .unwrap();
    }

    // Measure list performance with filtering
    let start = std::time::Instant::now();
    let sessions = db.list_sessions().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(sessions.len(), 100); // Only valid sessions
    assert!(elapsed.as_millis() < 50, "List with filtering should complete in <50ms, took {}", elapsed.as_millis());
}
```

**Fowler's Classification**: **Performance Test**
- Ensuring non-functional requirement met
- Benchmarking validation overhead

---

### 5. Error Handling Tests

**Purpose**: Verify system doesn't crash on corruption.

```rust
#[tokio::test]
async fn database_with_extreme_corruption_doesnt_crash() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Create valid session
    db.create("valid", "/tmp/workspace").await.unwrap();

    // Insert various corrupt entries
    let corruptions = vec![
        ("", ""),
        ("valid", ""),
        ("", "/tmp"),
        ("   ", "/tmp"),
        ("valid", "   "),
        ("valid", "/tmp", Some("not json")),
        ("valid", "/tmp", Some("")),
        ("valid", "/tmp", Some("{invalid json}")),
    ];

    for (name, path, metadata) in corruptions {
        let mut query = "INSERT INTO sessions (name, workspace_path".to_string();
        let mut values = format!("VALUES ('{}', '{}'", name, path);

        if let Some(meta) = metadata {
            query.push_str(", metadata");
            values.push_str(&format!", '{}'", meta));
        }

        query.push_str(") ");
        values.push(')');

        sqlx::query(&format!("{}{}", query, values))
            .execute(&db.pool)
            .await
            .unwrap();
    }

    // Should not crash, should return only valid session
    let sessions = db.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "valid");
}
```

**Fowler's Classification**: **Robustness Test**
- Testing system resilience
- Extreme edge cases

---

## Test Doubles Strategy

### What We DON'T Mock:

1. **Database**: Real SQLite (`:memory:` or temp file)
2. **Validation Logic**: Pure functions, no mocking needed
3. **CLI**: Real `zjj` binary invocation

### What We MIGHT Mock (If Needed):

1. **Filesystem**: For testing path existence checks
   ```rust
   // Not needed for initial implementation
   // Path existence checks are optional
   ```

**Fowler's Advice**: For database validation tests, real database > mock.

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Line Coverage** | 95% | Validation logic is critical |
| **Branch Coverage** | 90% | All error paths must be tested |
| | | |
| **Specific Coverage** | | |
| `validate_session()` | 100% | All corruption scenarios tested |
| `list_sessions()` | 100% | Filtering logic tested |
| Error paths | 100% | All error variants tested |

---

## Property-Based Tests (Optional)

Using `proptest` to verify validation invariants:

```rust
proptest! {
    #[test]
    fn valid_session_always_passes(name in "[a-zA-Z0-9_-]{1,50}", path in "/tmp/[a-z0-9/_-]{1,100}") {
        let session = SessionRow {
            id: 1,
            name,
            workspace_path: path,
            metadata: None,
            created_at: Utc::now(),
            is_active: false,
        };

        prop_assert!(validate_session(&session).is_ok());
    }
}
```

**Recommendation**: Add property tests if validation logic becomes complex.

---

## Test Smells to Avoid

### 1. **Testing Implementation Details**

❌ **Bad**: Testing internal validation steps
```rust
assert_eq!(session.name.len(), 5); // Implementation detail
```

✅ **Good**: Testing validation contract
```rust
assert!(validate_session(&session).is_ok());
```

### 2. **Brittle Corruption Tests**

❌ **Bad**: Hardcoded database IDs
```rust
sqlx::query("INSERT INTO sessions (id, name) VALUES (123, 'test')")
```

✅ **Good**: Let database auto-assign IDs
```rust
sqlx::query("INSERT INTO sessions (name) VALUES ('test')")
```

### 3. **Over-Specific Error Messages**

❌ **Bad**: Asserting exact error text
```rust
assert_eq!(err.to_string(), "Invalid name: empty");
```

✅ **Good**: Asserting error type
```rust
assert!(matches!(err, SessionError::InvalidName(_)));
```

---

## Regression Test Checklist

Before merging:

- [ ] All validation unit tests pass
- [ ] All filtering integration tests pass
- [ ] CLI tests with corrupt database pass
- [ ] Performance tests meet targets (<50ms for 100 sessions)
- [ ] Error handling tests pass (extreme corruption)
- [ ] Manual testing: Corrupt real database and verify behavior
- [ ] No new unwrap/expect/panic introduced
- [ ] All existing tests still pass
- [ ] `moon run :ci` passes

---

## Continuous Integration (CI) Configuration

```yaml
name: Database Corruption Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1
      - run: moon run :test database-validation
      - run: ./test/integration/corruption_cli_test.sh
```

---

## Manual Testing Checklist

Before closing bead:

- [ ] Create database manually
- [ ] Corrupt it with invalid entries (sqlite3 command)
- [ ] Run `zjj list` - should not crash
- [ ] Run `zjj list --verbose` - should show filtered count
- [ ] Try to focus on ghost session - should error gracefully
- [ ] Try to remove ghost session - should error gracefully
- [ ] Verify valid sessions still work
- [ ] Check logs for corruption warnings

---

## Post-Deployment Monitoring

After merging:

1. **User reports**: "Sessions disappeared"
   - Check if they were corrupt and filtered
   - Add logging to show why filtered

2. **Performance**: "List command slow"
   - Check if validation is expensive
   - Consider making path checks optional

3. **Corruption detection**: Add metrics
   - Track how many corrupt entries filtered
   - Alert if corruption rate spikes

---

## Summary

**Test Approach**: Classical (state verification) + Performance + Robustness

**Test Count**: ~15 tests
- 7 validation unit tests
- 4 filtering integration tests
- 3 CLI integration tests
- 2 performance tests
- 1 robustness test

**Execution Time**: <10 seconds (fast feedback)

**Risk Coverage**: High (all corruption scenarios tested)

**Fowler Compliance**: ✅
- ✅ Realistic testing (real database)
- ✅ No mocking (actual SQLite)
- ✅ Clear intent (tests describe behavior)
- ✅ No test smells (focused on what matters)

---

**Test Plan Status**: ✅ Ready for Builder

**Estimated Test Implementation Time**: 2 hours

**Confidence Level**: High (straightforward validation logic)

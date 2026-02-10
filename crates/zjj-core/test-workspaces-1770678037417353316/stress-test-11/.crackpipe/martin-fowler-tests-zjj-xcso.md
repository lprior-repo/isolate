# Martin Fowler Test Plan: Fix --include-files Flag (zjj-xcso)

**Generated**: 2026-02-07 21:01:30 UTC
**Bead**: zjj-xcso
**Contract**: `/tmp/rust-contract-zjj-xcso.md`
**Issue Type**: Bug fix (dead code removal)

---

## Test Strategy

Since this is a **dead code removal** (not new feature), our test strategy focuses on:

1. **Verification**: Prove the flag is truly gone
2. **Regression**: Ensure export still works
3. **Documentation**: Verify help text honesty
4. **User Experience**: Error messages are clear

**Martin Fowler Principles Applied**:
- **Classical Testing**: Verify state (export still creates JSON)
- **Minimal Mocking**: Test against real filesystem, database
- **Clear Intent**: Test names describe behavior, not implementation

---

## Test Categories

### 1. Compilation Tests (Compile-Time Verification)

**Purpose**: Rust's type system should prevent the field from existing.

```rust
#[test]
fn export_options_struct_no_include_files() {
    // This test is COMPILE-TIME ONLY
    // If include_files field exists, this won't compile
    let options = ExportOptions {
        session: None,
        output: Some("/tmp/test.json".to_string()),
        format: OutputFormat::Text,
        // include_files: true,  // ← This field MUST NOT EXIST
    };

    // If we get here, the field is gone
    assert!(options.include_files, "This line should NOT compile");
}
```

**Test Smell Avoided**: Testing compiler (brittle). Instead, rely on compilation failure.

**Actually**: Delete this test. Let the compiler be the test. If code references `include_files`, it won't compile.

---

### 2. CLI Argument Tests (User-Facing Behavior)

**Purpose**: Verify CLI rejects the removed flag.

```rust
#[test]
fn cli_rejects_include_files_flag() {
    use clap::Parser;

    // Attempt to parse with --include-files flag
    let result = ZjjCli::try_parse_from([
        "zjj", "export", "--include-files"
    ]);

    // Should fail with "unexpected argument" error
    assert!(result.is_err(), "CLI should reject --include-files flag");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("unexpected argument"),
            "Error should mention unexpected argument");
    assert!(err.to_string().contains("--include-files"),
            "Error should mention the flag name");
}
```

**Fowler's Classification**: **Behavior Verification** (Mockist)
- We're verifying CLI parser behavior
- Not checking internal state
- Focused on user-facing error messages

**Test Smell Avoided**:
- ❌ Testing clap internals (fragile across versions)
- ✅ Testing user-visible behavior (stable)

---

### 3. Export Functionality Tests (Regression Prevention)

**Purpose**: Ensure export still works after flag removal.

```rust
#[tokio::test]
async fn export_creates_json_file_without_flag() {
    // Setup: Create temporary database
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = SessionDb::new(&db_path).await.unwrap();

    // Create test session
    let session = db.create("test-session", "/tmp/workspace").await.unwrap();

    // Execute export WITHOUT --include-files (should work as before)
    let options = ExportOptions {
        session: Some("test-session".to_string()),
        output: Some(temp_dir.path().join("export.json").to_str().unwrap().to_string()),
        format: OutputFormat::Text,
    };

    run_export(&options).await.unwrap();

    // Verify: JSON file created
    let export_content = tokio::fs::read_to_string(
        temp_dir.path().join("export.json")
    ).await.unwrap();

    // Verify: Contains session data
    assert!(export_content.contains("\"name\":\"test-session\""));
    assert!(export_content.contains("\"workspace_path\":\"/tmp/workspace\""));

    // Verify: Is valid JSON
    let _parsed: ExportResult = serde_json::from_str(&export_content).unwrap();
}
```

**Fowler's Classification**: **State Verification** (Classical)
- Checking final state (file contents)
- No mocking of filesystem
- Real I/O operations

**Test Smell Avoided**:
- ❌ Mocking filesystem (unnecessary complexity)
- ✅ Using tempdir (realistic, fast)

---

### 4. Help Text Tests (Documentation Verification)

**Purpose**: Ensure help text is honest about what's exported.

```rust
#[test]
fn export_help_text_mentions_json_not_tarball() {
    let cmd = cmd_export();
    let help = cmd.render_help().to_string();

    // Should mention JSON
    assert!(help.contains("json") || help.contains("JSON"),
            "Help should mention JSON output");

    // Should NOT mention tarball
    assert!(!help.contains("tarball"),
            "Help should NOT mention tarball (flag removed)");
    assert!(!help.contains("tar.gz"),
            "Help should NOT mention tar.gz");

    // Should NOT mention --include-files
    assert!(!help.contains("--include-files"),
            "Help should NOT mention --include-files flag");
    assert!(!help.contains("include-files"),
            "Help should NOT mention include-files at all");
}
```

**Fowler's Classification**: **Behavior Verification** (Documentation)
- Testing documentation accuracy
- Prevents future confusion
- Catches "help lies to users" bugs

**Test Smell Avoided**:
- ❌ Asserting exact help text (fragile, changes often)
- ✅ Asserting presence/absence of key terms (stable)

---

### 5. Integration Tests (End-to-End)

**Purpose**: Verify full workflow works.

```bash
#!/bin/bash
# test/integration/export_without_include_files.sh

set -euo pipefail

# Setup: Create test environment
TEST_DIR=$(mktemp -d)
export ZJJ_DB="$TEST_DIR/beads.db"

# Create a session
zjj add test-session --no-open

# Export WITHOUT --include-files (should work)
zjj export test-session -o "$TEST_DIR/export.json"

# Verify: JSON file created
test -f "$TEST_DIR/export.json"
file "$TEST_DIR/export.json" | grep -q "JSON data"

# Verify: Is NOT a tarball
! file "$TEST_DIR/export.json" | grep -q "tar"
! tar -tzf "$TEST_DIR/export.json" 2>/dev/null

# Verify: Flag is rejected
if zjj export --include-files 2>&1 | grep -q "unexpected argument"; then
    echo "✓ --include-files flag correctly rejected"
else
    echo "✗ --include-files flag should be rejected"
    exit 1
fi

# Cleanup
rm -rf "$TEST_DIR"

echo "✓ All integration tests passed"
```

**Fowler's Classification**: **State Verification** (Classical)
- Real CLI invocation
- Real filesystem
- No test doubles

**Test Smell Avoided**:
- ❌ Stubbing out subprocess calls
- ✅ Running real binary (integration test)

---

## Test Doubles Strategy

### What We DON'T Mock:

1. **Filesystem**: Use `tempfile` crate for real temp directories
2. **Database**: Use real SQLite (`:memory:` or temp file)
3. **CLI Parser**: Test real `clap` behavior
4. **Serialization**: Test real `serde_json`

### What We MIGHT Mock (If Needed):

1. **Current Time**: For deterministic timestamps
   ```rust
   // Use chrono::Utc::now() (fast enough, no need to mock)
   ```

2. **External Commands**: If export called `jj` or `zellij`
   ```rust
   // Not applicable - export only reads database
   ```

**Fowler's Advice**: Prefer fakes over mocks. For this bug fix, we don't need either.

---

## Test Coverage Targets

| Metric Type | Target | Rationale |
|-------------|--------|-----------|
| **Line Coverage** | 90% | Simple removal, high coverage achievable |
| **Branch Coverage** | 85% | Few branches in export code |
| | | |
| **Specific Coverage** | | |
| `ExportOptions` usage | 100% | All fields must be tested |
| CLI argument parsing | 100% | All flags must be verified |
| Help text accuracy | 100% | All documented features tested |

---

## Property-Based Tests (Optional but Nice)

Using `proptest` to verify export invariants:

```rust
proptest! {
    #[test]
    fn export_without_files_does_not_read_workspace(
        session_name in "[a-zA-Z0-9_-]{1,50}"
    ) {
        // Property: Export with metadata-only should not touch workspace files
        // This test would need filesystem instrumentation to verify no reads

        // For this bug fix, property testing is overkill
        // Simpler tests are sufficient
    }
}
```

**Recommendation**: Skip property tests for this simple removal. Add if implementing full tarball export later.

---

## Test Smells to Avoid

### 1. **Test-Induced Coupling**

❌ **Bad**: Testing that `ExportOptions` struct has specific field count
```rust
assert_eq!(std::mem::size_of::<ExportOptions>(), 48);
```

✅ **Good**: Testing that export works correctly
```rust
run_export(&options).await.unwrap();
assert!(file_exists(output_path));
```

### 2. **Fragile Tests**

❌ **Bad**: Asserting exact help text
```rust
assert_eq!(help, "Export session state to a file\n\n...");
```

✅ **Good**: Asserting key content
```rust
assert!(help.contains("Export"));
assert!(!help.contains("tarball"));
```

### 3. **Magic Numbers**

❌ **Bad**: Hardcoded sizes
```rust
assert!(json.len() > 100);
```

✅ **Good**: Semantic checks
```rust
assert!(json.contains("\"name\":\"test\""));
```

### 4. **Over-Mocking**

❌ **Bad**: Mocking filesystem for simple write
```rust
let mock_fs = MockFileSystem::new();
when!(mock_fs.write).thenReturn(Ok(()));
```

✅ **Good**: Using tempdir
```rust
let temp = tempfile::tempdir().unwrap();
tokio::fs::write(temp.path().join("test.json"), data).await.unwrap();
```

### 5. **Global State**

❌ **Bad**: Tests depend on execution order
```rust
static mut TEST_DB: Option<SessionDb> = None;
```

✅ **Good**: Each test isolated
```rust
let db = SessionDb::new(":memory:").await.unwrap();
// Test runs, db dropped at end
```

---

## Regression Test Checklist

Before merging, verify:

- [ ] All existing tests pass (`moon run :test`)
- [ ] New tests flag removal verified (`moon run :test export`)
- [ ] Help text updated and tested
- [ ] Documentation updated (README, man pages)
- [ ] CHANGELOG.md entry added
- [ ] Integration tests pass
- [ ] No new clippy warnings (`moon run :quick`)
- [ ] No new dead_code warnings (ironic!)

---

## Continuous Integration (CI) Configuration

Add to `.github/workflows/test.yml`:

```yaml
name: Export Flag Removal Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon-action@v1
      - run: moon run :test
      - run: |
          # Verify --include-files flag is rejected
          if zjj export --include-files 2>&1 | grep -q "unexpected argument"; then
            echo "✓ Flag correctly rejected"
          else
            echo "✗ Flag should be rejected"
            exit 1
          fi
```

---

## Performance Tests (If Concerned About Speed)

Verify export is still fast after changes:

```rust
#[tokio::test]
async fn export_100_sessions_under_100ms() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = SessionDb::new(temp_dir.path().join("test.db")).await.unwrap();

    // Create 100 sessions
    for i in 0..100 {
        db.create(&format!("session-{}", i), "/tmp").await.unwrap();
    }

    let start = std::time::Instant::now();
    run_export(&ExportOptions {
        session: None,
        output: Some(temp_dir.path().join("all.json").to_str().unwrap().to_string()),
        format: OutputFormat::Text,
    }).await.unwrap();
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 100, "Export should complete in <100ms");
}
```

**Recommendation**: Include this test. Export should be fast.

---

## Manual Testing Checklist

Before closing bead:

- [ ] Run `zjj export --help` - no mention of tarball
- [ ] Run `zjj export --include-files` - error message clear
- [ ] Run `zjj export -o test.json` - creates JSON file
- [ ] Verify JSON file is valid: `jq . test.json`
- [ ] Verify JSON file is NOT tarball: `file test.json`
- [ ] Test with real session: `zjj add test-manual && zjj export test-manual`

---

## Post-Deployment Monitoring

After merging, watch for:

1. **User reports**: "Where did --include-files go?"
   - Action: Point to migration guide
   - Consider: Feature request for proper tarball export

2. **CI failures**: Tests referencing removed field
   - Action: Update tests per this plan

3. **Documentation**: Old help text in screenshots, etc.
   - Action: Update docs systematically

---

## Summary

**Test Approach**: Classical (state verification) + Minimal behavior checks

**Test Count**: ~6 tests
- 2 CLI tests (flag rejection, help text)
- 2 functionality tests (export works, JSON valid)
- 2 integration tests (end-to-end, manual)

**Execution Time**: <5 seconds (fast feedback)

**Risk Coverage**: High (regression prevention, documentation accuracy)

**Fowler Compliance**: ✅
- ✅ Minimal mocking
- ✅ Clear intent
- ✅ Realistic test doubles (tempdir, real DB)
- ✅ No test smells (brittleness, coupling, magic numbers)

---

**Test Plan Status**: ✅ Ready for Builder

**Estimated Test Implementation Time**: 30 minutes

**Confidence Level**: High (simple removal, well-understood)

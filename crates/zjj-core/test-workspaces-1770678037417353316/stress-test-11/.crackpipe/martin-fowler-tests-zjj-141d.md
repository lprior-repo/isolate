# Martin Fowler Test Plan: zjj-141d

## Title
config: Fix write-only configuration keys

## Test Strategy
This is a read-after-write bug fix. We need to test that configuration keys can be both written and read back, including arbitrary keys not defined in the Config struct.

## Test Catalog

### TR-1: Read After Write - Known Key
**Scenario**: User sets a known config key, then reads it back
**Given**: A clean zjj environment
**When**: User runs `zjj config set workspace_dir /tmp/workspaces`
**Then**: `zjj config get workspace_dir` returns `/tmp/workspaces`
**And**: `zjj config get workspace_dir --json` returns valid JSON with the value

### TR-2: Read After Write - Arbitrary Nested Key
**Scenario**: User sets an arbitrary key not in Config struct, then reads it back
**Given**: A clean zjj environment
**When**: User runs `zjj config set custom.nested.key value123`
**Then**: `zjj config get custom.nested.key` returns `value123`
**And**: No "key not found" error occurs

### TR-3: Read After Write - Arbitrary Top-Level Key
**Scenario**: User sets an arbitrary top-level key, then reads it back
**Given**: A clean zjj environment
**When**: User runs `zjj config set my_custom_setting true`
**Then**: `zjj config get my_custom_setting` returns `true`
**And**: The value is correctly parsed as a boolean

### TR-4: Project Config Overrides Global Config
**Scenario**: Same key in both global and project, project value wins
**Given**:
- Global config has `workspace_dir = /global`
- Project config has `workspace_dir = /project`
**When**: User runs `zjj config get workspace_dir`
**Then**: Returns `/project` (project takes precedence)

### TR-5: Reading Key That Doesn't Exist
**Scenario**: User tries to read a key that was never set
**Given**: A clean zjj environment
**When**: User runs `zjj config get nonexistent.key`
**Then**: Returns exit code 3 (not found)
**And**: Error message says "Config key 'nonexistent.key' not found"

### TR-6: List All Config Includes Arbitrary Keys
**Scenario**: User sets arbitrary keys, then lists all config
**Given**:
- User ran `zjj config set custom.key1 value1`
- User ran `zjj config set custom.key2 value2`
**When**: User runs `zjj config` (list all)
**Then**: Output includes the arbitrary keys
**Or**: At minimum, the keys can still be read individually

### TR-7: JSON Output Format for Arbitrary Keys
**Scenario**: Reading arbitrary keys with --json flag
**Given**: User ran `zjj config set custom.api_key secret123`
**When**: User runs `zjj config get custom.api_key --json`
**Then**: Returns valid JSON: `"secret123"` or `{"value": "secret123"}`
**And**: JSON parses without error

### TR-8: Concurrent Write Then Read
**Scenario**: Multiple processes set different keys, each can read their own key
**Given**: Multiple zjj instances running
**When**:
- Process A sets `concurrent.a = alpha`
- Process B sets `concurrent.b = beta`
**Then**: Both keys can be read back
**And**: No "key not found" errors

### TR-9: Special Characters in Key Names
**Scenario**: Keys with underscores, numbers, dots
**Given**: A clean zjj environment
**When**: User runs `zjj config set key_with_123.nested.value test`
**Then**: `zjj config get key_with_123.nested.value` returns `test`

### TR-10: Empty String Values
**Scenario**: Setting a key to an empty string
**Given**: A clean zjj environment
**When**: User runs `zjj config set empty.key ""`
**Then**: `zjj config get empty.key` returns empty string
**And**: Does NOT show "key not found" error

## Test Implementation Notes

### Test File Structure
```rust
// crates/zjj/tests/test_config_read_write.rs

#[tokio::test]
async fn read_after_write_known_key() { /* TR-1 */ }

#[tokio::test]
async fn read_after_write_arbitrary_nested_key() { /* TR-2 */ }

#[tokio::test]
async fn read_after_write_arbitrary_top_level_key() { /* TR-3 */ }

#[tokio::test]
async fn project_config_overrides_global() { /* TR-4 */ }

#[tokio::test]
async fn reading_nonexistent_key_returns_error() { /* TR-5 */ }
```

### Test Helpers
```rust
async fn set_config(key: &str, value: &str) -> Result<()>;
async fn get_config(key: &str) -> Result<String>;
async fn with_temp_config<F>(test: F) -> Result<()>
where F: FnOnce(PathBuf) -> Result<()>;
```

## Integration Test Command
```bash
# Run all config read/write tests
moon run :test test_config_read_write

# Run specific test
moon run :test test_config_read_write::read_after_write_arbitrary_nested_key
```

## Success Criteria
- All 10 test scenarios pass
- No regression in existing config functionality
- `moon run :quick` shows no clippy errors

# Martin Fowler Test Plan: zjj-1mch

## Title
LOW-005: Add global --verbose flag

## Test Strategy
Test that the verbose flag works across all commands and doesn't break existing functionality.

## Test Catalog

### TV-1: No Verbose Flag (Default Behavior)
**Scenario**: User runs command without verbose flag
**Given**: Default zjj installation
**When**: User runs `zjj status` (no verbose flag)
**Then**:
- Only warnings and errors are logged
- Normal output appears on stdout
- No debug/info messages

### TV-2: Single -v Shows Info
**Scenario**: User wants to see major operations
**Given**: Default zjj installation
**When**: User runs `zjj -v add my-session`
**Then**:
- Info level messages appear on stderr
- Shows: "Creating workspace", "Cloning repo", etc.
- Normal output still on stdout

### TV-3: Double -vv Shows Debug
**Scenario**: User debugging an issue
**Given**: Default zjj installation
**When**: User runs `zjj -vv sync`
**Then**:
- Debug level messages appear on stderr
- Shows detailed operations, SQL queries, etc.
- Helps identify where operation fails

### TV-4: Triple -vvv Shows Trace
**Scenario**: Developer debugging the code itself
**Given**: Default zjj installation
**When**: User runs `zjj -vvv status`
**Then**:
- Trace level messages appear on stderr
- Shows function calls, internal state changes
- Very verbose output

### TV-5: Verbose Doesn't Break JSON Output
**Scenario**: User wants verbose logging with JSON output
**Given**: Default zjj installation
**When**: User runs `zjj -v status --json`
**Then**:
- JSON output on stdout is still valid
- Logs appear on stderr (not in JSON)
- `jq .` can still parse stdout

### TV-6: Verbose Works With All Commands
**Scenario**: Verbose flag should work globally
**Given**: Default zjj installation
**When**: User runs verbose with various commands
```bash
zjj -v add test-session
zjj -v remove test-session
zjj -v list
zjj -v sync
zjj -v status
```
**Then**:
- All commands show verbose output
- No errors or crashes

### TV-7: Multiple Verbose Flags Combine
**Scenario**: User uses -v -v -v instead of -vvv
**Given**: Default zjj installation
**When**: User runs `zjj -v -v -v status`
**Then**:
- Same behavior as `zjj -vvv status`
- Verbosity level is 3

### TV-8: Verbose Output Goes to Stderr
**Scenario**: Verbose output shouldn't break stdout piping
**Given**: Default zjj installation
**When**: User runs `zjj -v status > output.txt`
**Then**:
- `output.txt` contains normal stdout only
- Verbose logs visible in terminal
- Can grep stderr separately

### TV-9: Verbose Doesn't Impact Performance Significantly
**Scenario**: Verbose off should be as fast as before
**Given**: Default zjj installation
**When**: User runs `time zjj status` vs `time zjj -v status`
**Then**:
- Performance difference is minimal (<5%)
- Verbose checking is fast path when disabled

## Test Implementation Structure

### Test File
```rust
// crates/zjj/tests/test_verbose_flag.rs

#[tokio::test]
async fn default_shows_only_warnings() { /* TV-1 */ }

#[tokio::test]
async fn single_v_shows_info() { /* TV-2 */ }

#[tokio::test]
async fn double_vv_shows_debug() { /* TV-3 */ }

#[tokio::test]
async fn triple_vvv_shows_trace() { /* TV-4 */ }

#[tokio::test]
async fn verbose_doesnt_break_json() { /* TV-5 */ }
```

### Integration Test Commands
```bash
# Run all verbose tests
moon run :test test_verbose_flag

# Manual verification
zjj -v status 2>&1 | tee output.log
zjj -vv status --json | jq .
```

## Success Criteria
- All 9 test scenarios pass
- All commands accept `--verbose` flag
- JSON output still valid with verbose
- No performance regression
- Help text mentions `-v, --verbose` flag

## Estimated Effort
1 hour

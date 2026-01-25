# Bugs Found During Integration Testing

**Date**: 2026-01-25
**Source**: integration-test.sh
**Total Bugs**: 4

---

## BUG #1: list --json Missing Schema Envelope

**Severity**: Medium (P1)
**Component**: crates/zjj/src/commands/list.rs
**Tags**: json, api-contract, schema

### Description
The `jjz list --json` command outputs a raw JSON array instead of wrapping it in a schema envelope.

### Current Behavior
```bash
$ jjz list --json
[
  {
    "name": "test-session",
    "status": "active",
    ...
  }
]
```

### Expected Behavior
```bash
$ jjz list --json
{
  "$schema": "https://jjz.dev/schemas/list-response.json",
  "schema_type": "list_response",
  "version": "1.0",
  "data": {
    "sessions": [
      {
        "name": "test-session",
        "status": "active",
        ...
      }
    ]
  }
}
```

### Impact
- JSON consumers cannot validate schema
- No API versioning
- Breaks consistency with other commands (introspect, doctor use schema envelopes)

### Reproduction
```bash
cargo build --release --bin jjz
./target/release/jjz list --json | jq '."$schema"'  # Returns null
```

### Fix Location
- File: `crates/zjj/src/commands/list.rs`
- Look for JSON output code in `run()` function
- Wrap output in schema envelope using zjj-core JSON utilities

---

## BUG #2: status --json Missing Schema Envelope

**Severity**: Medium (P1)
**Component**: crates/zjj/src/commands/status.rs
**Tags**: json, api-contract, schema

### Description
The `jjz status --json` command outputs raw JSON instead of wrapping it in a schema envelope.

### Current Behavior
```bash
$ jjz status --json
{
  "name": "test-session",
  "status": "active",
  ...
}
```

### Expected Behavior
```bash
$ jjz status --json
{
  "$schema": "https://jjz.dev/schemas/status-response.json",
  "schema_type": "status_response",
  "version": "1.0",
  "data": {
    "name": "test-session",
    "status": "active",
    ...
  }
}
```

### Impact
Same as BUG #1 - no schema validation or versioning

### Reproduction
```bash
./target/release/jjz status --json | jq '."$schema"'  # Returns null
```

### Fix Location
- File: `crates/zjj/src/commands/status.rs`
- Look for JSON output code
- Wrap in schema envelope

---

## BUG #3: Errors Ignore --json Flag

**Severity**: High (P0)
**Component**: crates/zjj/src/main.rs, error handling
**Tags**: json, error-handling, cli-ux

### Description
When `--json` flag is provided, validation errors still output plain text to stderr instead of JSON.

### Current Behavior
```bash
$ jjz add "" --json
Error: Invalid session name: Validation error: Session name cannot be empty
# Exit code: 1
```

### Expected Behavior
```bash
$ jjz add "" --json
{
  "$schema": "https://jjz.dev/schemas/error-response.json",
  "schema_type": "error_response",
  "version": "1.0",
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Session name cannot be empty",
    "exit_code": 1,
    "suggestion": "Provide a non-empty session name starting with a letter"
  }
}
# Exit code: 1
```

### Impact
- Scripts cannot parse error responses reliably
- --json flag has inconsistent behavior
- Breaks automation workflows

### Reproduction
```bash
./target/release/jjz add "" --json 2>&1 | jq .  # Parse error: not JSON
./target/release/jjz remove nonexistent --json 2>&1 | jq .  # Parse error: not JSON
```

### Fix Location
- File: `crates/zjj/src/main.rs`
- Function: `run_cli()` and `main()` error handling
- Need to:
  1. Thread `--json` flag through error handling
  2. Create `ErrorResponse` type with schema envelope
  3. Format errors as JSON when flag is present
  4. Write JSON to stdout (not stderr) for consistency

### Technical Notes
```rust
// Current error handling (simplified)
if let Err(err) = run_cli() {
    eprintln!("Error: {}", format_error(&err));
    process::exit(get_exit_code(&err));
}

// Needed: Check if --json was passed and format accordingly
if let Err(err) = run_cli() {
    if json_flag_was_set {
        let json_err = ErrorResponse {
            schema: "...",
            error: ErrorDetail {
                code: error_code(&err),
                message: err.to_string(),
                exit_code: get_exit_code(&err),
                suggestion: error_suggestion(&err),
            }
        };
        println!("{}", serde_json::to_string_pretty(&json_err)?);
    } else {
        eprintln!("Error: {}", format_error(&err));
    }
    process::exit(get_exit_code(&err));
}
```

---

## BUG #4: Session Creation Panics During Zellij Integration

**Severity**: Critical (P0)
**Component**: crates/zjj/src/commands/add.rs or Zellij integration
**Tags**: panic, zellij, transaction, critical

### Description
Valid session names (with dash/underscore) create database entries successfully but then panic during Zellij integration, leaving orphaned sessions in the database.

### Current Behavior
```bash
$ jjz add test-with-dash
Created session 'test-with-dash'
Launching Zellij with new tab...
[Terminal control codes]
# Exit code: 101 (panic)

$ jjz list
test-with-dash  active  ...  # Session exists in DB!
```

### Expected Behavior
```bash
$ jjz add test-with-dash
Created session 'test-with-dash'
Launching Zellij with new tab...
Session 'test-with-dash' created successfully
# Exit code: 0

# OR if Zellij fails:
$ jjz add test-with-dash
Error: Failed to create Zellij tab
Cause: Zellij is not running
# Exit code: 2
# Session NOT in database (transaction rolled back)
```

### Impact
- CLI crashes with exit code 101 (panic)
- Leaves orphaned sessions in database
- Users can't tell if session was created or not
- Violates "no panics" project rule

### Reproduction
```bash
./target/release/jjz add test-dash
# OR
./target/release/jjz add test_underscore
# Exit code: 101
./target/release/jjz list  # Shows orphaned session
```

### Root Cause Analysis
Likely one of:
1. Unwrap/expect on Zellij command execution
2. No error handling around `zellij action new-tab`
3. Terminal control code parsing fails
4. Missing transaction/rollback on Zellij failure

### Fix Location
- File: `crates/zjj/src/commands/add.rs`
- Check Zellij integration code
- Look for unwrap/expect calls
- Add proper Result handling

### Required Changes
1. **Remove panics**: Replace unwrap/expect with proper error handling
2. **Add transaction**: Wrap session creation + Zellij launch in transaction
3. **Rollback on failure**: If Zellij fails, delete session from DB
4. **Graceful degradation**: Consider --no-open flag behavior (already exists!)

### Testing Strategy
```bash
# Without Zellij running
ZELLIJ_SESSION_NAME="" ./target/release/jjz add test-session --no-open
# Should succeed (--no-open skips Zellij)

# With Zellij integration
./target/release/jjz add test-session
# Should either succeed OR gracefully fail with exit code 2
```

---

## Recommended Fix Order

1. **BUG #4** (Critical): Fix panic, add transaction rollback
2. **BUG #3** (High): Implement JSON error responses
3. **BUG #1** (Medium): Add schema envelope to list
4. **BUG #2** (Medium): Add schema envelope to status

## Testing Checklist After Fixes

- [ ] Run integration-test.sh - all tests pass
- [ ] Test `jjz add valid-name` without Zellij - graceful error
- [ ] Test `jjz add valid-name --no-open` - succeeds without Zellij
- [ ] Test all commands with --json flag - valid JSON output
- [ ] Test error cases with --json flag - valid JSON errors
- [ ] Validate all JSON outputs have schema envelope
- [ ] Check no panics in any error path (search for unwrap/expect)

## References

- Integration Test Report: INTEGRATION-TEST-REPORT.md
- Test Script: integration-test.sh
- Project Rules: CLAUDE.md (zero unwraps, zero panics)

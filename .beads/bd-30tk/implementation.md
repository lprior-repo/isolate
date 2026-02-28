# Implementation: Fix doctor routing to subcommands

## Summary

Successfully implemented subcommand routing for the `doctor` command with backward compatibility for legacy usage.

## Changes Made

### 1. CLI Definition - `object_commands.rs`

Modified `cmd_doctor()` function to:
- Set `.subcommand_required(false)` to allow both subcommand and legacy usage
- Added four subcommands: `check`, `fix`, `integrity`, `clean`
- Added legacy arguments at parent level for backward compatibility: `--json`, `--fix`, `--dry-run`, `--verbose`

### 2. Handler - `handlers/integrity.rs`

Updated `handle_doctor()` function to:
- Route to appropriate handler based on subcommand
- Added `emit_deprecation_warning()` for legacy usage without subcommand
- Support legacy flags when no subcommand is provided
- Return clear error for unknown subcommands

### 3. CLI Definition - `commands.rs`

Updated `cmd_doctor()` function to:
- Added matching subcommand definitions for consistency
- Added `#[allow(clippy::too_many_lines)]` directive due to larger function

## Verified Functionality

| Command | Behavior | Status |
|---------|----------|--------|
| `doctor` | Shows deprecation warning, runs health check | ✅ |
| `doctor check` | Runs health check | ✅ |
| `doctor fix` | Attempts auto-fix | ✅ |
| `doctor integrity` | Runs integrity validation | ✅ |
| `doctor clean` | Cleans up stale sessions | ✅ |
| `doctor invalid` | Shows error, exit code 2 | ✅ |

## Acceptance Tests

- ✅ test_doctor_check: `isolate doctor check` returns health status
- ✅ test_doctor_fix: `isolate doctor fix` attempts repairs  
- ✅ test_doctor_integrity: `isolate doctor integrity` checks DB
- ✅ test_doctor_clean: `isolate doctor clean` removes temp files
- ✅ test_doctor_invalid: `isolate doctor invalid` returns error with clear message
- ✅ test_doctor_missing: `isolate doctor` shows deprecation warning

## Quality Gates

- ✅ moon run :quick passes
- ✅ moon run :test passes (2643 tests)
- ✅ moon run :ci passes
- ✅ No unwrap/expect/panic in source code
- ✅ Uses Result<T, E> pattern throughout

## Files Modified

1. `/crates/isolate/src/cli/object_commands.rs` - CLI definition for doctor subcommands
2. `/crates/isolate/src/cli/commands.rs` - Legacy CLI definition (consistency)
3. `/crates/isolate/src/cli/handlers/integrity.rs` - Handler routing logic

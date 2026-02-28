# Implementation: handlers: Fix doctor routing to subcommands

## Summary

Successfully implemented subcommand routing for the `isolate doctor` command to support:
- `isolate doctor check` - Run all system health checks
- `isolate doctor fix` - Auto-fix issues where possible
- `isolate doctor integrity` - Run database integrity check
- `isolate doctor clean` - Remove stale sessions

Legacy mode (`isolate doctor` without subcommand) continues to work for backward compatibility.

## Files Changed

### 1. `crates/isolate/src/cli/object_commands.rs`
- Modified `cmd_doctor()` to add subcommands (check, fix, integrity, clean)
- Added `subcommand_required(false)` to support legacy mode
- Added legacy flags (--fix, --dry-run, --verbose) at top level for backward compatibility
- Added `#[allow(clippy::too_many_lines)]` directive

### 2. `crates/isolate/src/cli/commands.rs`
- Modified `cmd_doctor()` to add subcommands for consistency
- Added legacy flags at top level to support `isolate doctor --fix` style commands
- Added `#[allow(clippy::too_many_lines)]` directive

### 3. `crates/isolate/src/cli/handlers/integrity.rs`
- Modified `handle_doctor()` to route to appropriate subcommand handlers
- Added `run_db_integrity_check()` function for the integrity subcommand
- Uses `contains_id` to check for legacy flags safely

### 4. `crates/isolate/src/commands/doctor.rs`
- Made `run_integrity_check()` public so it can be called from the handler

## Test Results

- ✅ `moon run :quick` passes
- ✅ `moon run :test` passes (2643 tests)
- ✅ `isolate doctor` (legacy) - works
- ✅ `isolate doctor check` - works
- ✅ `isolate doctor fix` - works  
- ✅ `isolate doctor integrity` - works
- ✅ `isolate doctor clean` - works
- ✅ `isolate doctor --fix` (legacy with flag) - works
- ✅ `isolate doctor invalid` - returns error with clear message

## Verification

All subcommands have been manually tested and verified to work correctly.

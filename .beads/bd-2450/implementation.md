# Implementation: Fix config routing to subcommands

## Bead ID
bd-2450

## Summary
Fixed config routing to subcommands by updating the handle_config function to match on subcommands (list, get, set, schema) and route to the appropriate handler. Added CONFIG_SCHEMA to isolate-core for the `config schema` command.

## Changes Made

### 1. Added CONFIG_SCHEMA to isolate-core
**File:** `crates/isolate-core/src/config.rs`
- Added `ConfigSchemaEntry` struct for schema metadata (type, description, default, validation)
- Added `CONFIG_SCHEMA` constant with all valid configuration keys and their metadata
- Provides introspection for the `config schema` command

**File:** `crates/isolate-core/src/lib.rs`
- Exported `ConfigSchemaEntry` and `CONFIG_SCHEMA` from the config module

### 2. Fixed handle_config in utility.rs
**File:** `crates/isolate/src/cli/handlers/utility.rs`
- Updated `handle_config` to match on subcommands using `subcommand()`:
  - `config list [--global]` - Lists all configuration values
  - `config get <key> [--global]` - Gets a specific config value
  - `config set <key> <value> [--global]` - Sets a config value
  - `config schema` - Shows configuration schema
- Added `#[allow(clippy::too_many_lines, clippy::unnecessary_wraps)]` for show_config_schema_docs

## Verification
- `moon run :quick` - ✅ PASS
- `moon run :test` - ✅ PASS (1698 tests passed)
- `moon run :ci` - ✅ PASS (2643 tests passed)

## Manual Testing
All subcommands verified working:
- `isolate config list` - ✅ Shows all config
- `isolate config list --global` - ✅ Shows global config
- `isolate config get workspace_dir` - ✅ Shows specific value
- `isolate config set session.commit_prefix "test:"` - ✅ Sets value
- `isolate config schema` - ✅ Shows schema documentation

## Subcommand Routing
The implementation follows the same pattern as `handle_task` in `commands/task.rs`:
- Uses `args.subcommand()` to extract the subcommand name
- Matches on subcommand name and routes to appropriate handler
- Clap handles unknown subcommands automatically with helpful error

## Error Handling
- Returns `Result<()>` from all fallible functions
- No unwrap/panic in source code
- Uses functional patterns: map, and_then, ?

## CLI Behavior
- Running `isolate config` (without subcommand) returns error from clap
- Subcommands are required by the CLI definition (`subcommand_required(true)`)

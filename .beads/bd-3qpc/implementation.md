# Implementation: Fix config routing to subcommands

## Bead ID
bd-3qpc

## Summary
Fixed config command routing to properly handle subcommands (list, get, set, schema) in the isolate CLI.

## Changes

### File: `crates/isolate/src/cli/handlers/utility.rs`

**Modified:** `handle_config` function to detect and route to subcommand handlers

**Before:**
```rust
pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    let key = sub_m.get_one::<String>("key").cloned();
    let value = sub_m.get_one::<String>("value").cloned();
    let global = sub_m.get_flag("global");
    let format = get_format(sub_m);
    let options = config::ConfigOptions { key, value, global, format };
    config::run(options).await
}
```

**After:**
```rust
pub async fn handle_config(sub_m: &ArgMatches) -> Result<()> {
    let (subcommand_name, subcommand_args) = sub_m.subcommand()
        .ok_or_else(|| anyhow::anyhow!("Config subcommand required. Use: list, get, set, or schema"))?;

    match subcommand_name {
        "list" => handle_config_list(subcommand_args).await,
        "get" => handle_config_get(subcommand_args).await,
        "set" => handle_config_set(subcommand_args).await,
        "schema" => handle_config_schema(subcommand_args),
        _ => anyhow::bail!("Unknown config subcommand: {subcommand_name}"),
    }
}
```

**Added:** New handler functions:
- `handle_config_list` - Routes to config list (all values)
- `handle_config_get` - Routes to config get <key>
- `handle_config_set` - Routes to config set <key> <value>
- `handle_config_schema` - Generates JSON schema from Config type

## Acceptance Tests Verified

| Test | Status |
|------|--------|
| `isolate config list` | ✅ Returns all config values |
| `isolate config get key` | ✅ Returns specific config value |
| `isolate config set key value` | ✅ Sets config value (with key validation) |
| `isolate config schema` | ✅ Returns JSON schema for config |
| Invalid subcommand | ✅ Returns clear error message |
| `isolate config get missing_key` | ✅ Error with clear message |

## Technical Details

- Uses clap's `subcommand()` method to detect which subcommand was invoked
- Each subcommand has its own handler that builds appropriate `ConfigOptions`
- Follows functional-rust patterns: Result<T,E>, no unwrap/panic, no mut
- All errors use anyhow::Result with proper context messages
- The CLI already has subcommands defined in `object_commands.rs`

## Quality Gates

- ✅ `moon run :quick` - Passed
- ✅ `moon run :test` - 2643 tests passed  
- ✅ `moon run :ci` - Passed
- ✅ Manual verification of all subcommands

## Contract Fulfillment

| Requirement | Status |
|-------------|--------|
| WHEN user runs isolate config list | ✅ THE SYSTEM SHALL call config_list handler |
| WHEN user runs isolate config get key | ✅ THE SYSTEM SHALL return config value |
| IF subcommand is missing | ✅ THE SYSTEM SHALL NOT fail silently (shows error) |
| config schema returns JSON schema | ✅ Implemented |
| config set updates key | ✅ Implemented |

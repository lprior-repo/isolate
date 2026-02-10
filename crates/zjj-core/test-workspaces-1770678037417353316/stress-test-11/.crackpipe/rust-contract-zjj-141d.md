# Rust Contract: zjj-141d

## Title
config: Fix write-only configuration keys

## Type
bug

## Description
Configuration keys can be set but not read back. Set values exist in file but zjj config cannot retrieve them. Error: 'key not found' for keys that exist in config.toml. Impact: Configuration non-functional, users cannot use custom configuration.

## Root Cause Analysis
The bug is in `crates/zjj/src/commands/config.rs` in the `show_config_value()` function. When reading a config value:

1. The function loads a merged `Config` struct via `zjj_core::config::load_config()`
2. This merged Config includes defaults, global, project, and env vars
3. When converting to JSON for nested lookup, only the merged Config is serialized
4. **The problem**: Custom keys that exist in the TOML file but are NOT in the `Config` struct definition are lost during merge

When `set_config_value()` writes to the TOML file, it uses `toml_edit::DocumentMut` which preserves arbitrary keys. But `load_config()` deserializes into the strongly-typed `Config` struct, which drops any unknown fields.

## Preconditions
- A config file exists (global or project)
- User has run `zjj config set <key> <value>` to set a custom key
- The key may or may not be defined in the `Config` struct

## Postconditions
- Reading any key that exists in the config file succeeds
- Keys defined in Config struct return their merged value
- Keys NOT defined in Config struct (arbitrary TOML keys) can still be read
- Error message only appears when key genuinely doesn't exist in any config source

## Invariants
- **I1**: All keys in config files must be readable
- **I2**: Read after write must always succeed for the same key
- **I3**: Unknown keys in TOML files must not be silently dropped
- **I4**: The config hierarchy (defaults -> global -> project -> env) must be preserved

## Implementation Strategy

### Option A: Preserve Unknown Fields in Config Merge (Recommended)
Modify `Config::merge()` to preserve unknown fields from TOML files:

```rust
impl Config {
    fn merge(&mut self, other: Self, unknown_fields: HashMap<String, toml_edit::Item>) {
        // Existing merge logic...
        self.watch.merge(other.watch);
        // ... etc ...

        // NEW: Store unknown fields for later retrieval
        self.unknown_fields.extend(unknown_fields);
    }
}

pub struct Config {
    // ... existing fields ...
    pub unknown_fields: HashMap<String, serde_json::Value>,
}
```

Then in `show_config_value()`, check both the structured Config and unknown_fields.

### Option B: Direct TOML Lookup for Unknown Keys
When `get_nested_value()` fails on the Config struct, fall back to reading the raw TOML file:

```rust
fn show_config_value(config: &Config, key: &str, format: OutputFormat) -> Result<()> {
    // Try structured lookup first
    match get_nested_value(config, key) {
        Ok(value) => println!("{key} = {value}"),
        Err(_) => {
            // Fall back to raw TOML lookup
            let raw_value = lookup_raw_toml(key)?;
            println!("{key} = {raw_value}");
        }
    }
}
```

### Option C: Make Config Fully Dynamic
Replace the strongly-typed `Config` with a `HashMap<String, Value>` for all config operations.

**Reject**: Too much refactoring, loses type safety.

## Recommended Approach
Option B with enhancement:

1. Add `get_raw_value_from_files(key: &str) -> Result<String>` function
2. This function reads both global and project TOML files directly
3. In `show_config_value()`, if structured lookup fails, call raw lookup
4. Raw lookup uses `toml_edit` to navigate the TOML structure by dot-notation

## Test Cases
1. Set arbitrary key `zjj config set custom.arbitrary.key value`, then read it back
2. Set known key `zjj config set workspace_dir /tmp`, verify it reads merged value
3. Set key in project config, read when global config also has same key (project wins)
4. Try to read key that doesn't exist anywhere → proper error message

## Files to Modify
- `crates/zjj/src/commands/config.rs`: Add `lookup_raw_toml()` function, modify `show_config_value()`
- `crates/zjj/tests/test_config_read_write.rs`: New test file for read-after-write

## Dependencies
- None (standalone bug fix)

## Estimated Effort
1 hour

# File Mapping: zjj-viue Implementation

## Existing Files to Modify

### 1. CLI Arguments Definition
**File**: `/home/lewis/src/zjj/crates/zjj/src/cli/args.rs`
**Current State**: Lines 947-1059 contain `cmd_config()` function
**Modification**: Refactor to use `.subcommand()` instead of positional args
**Lines to Change**: ~113 lines
**Scope**: Define 4 subcommands (view, get, set, validate)

```rust
// Current structure:
pub fn cmd_config() -> Command {
    Command::new("config")
        .arg(Arg::new("key")...)      // REMOVE
        .arg(Arg::new("value")...)    // REMOVE
        .arg(Arg::new("global")...)
        .arg(Arg::new("json")...)
        .arg(Arg::new("validate")...) // REMOVE - becomes subcommand
}

// New structure:
pub fn cmd_config() -> Command {
    Command::new("config")
        .subcommand(cmd_config_view())      // NEW
        .subcommand(cmd_config_get())       // NEW
        .subcommand(cmd_config_set())       // NEW
        .subcommand(cmd_config_validate())  // NEW
}
```

### 2. Main Config Command Handler
**File**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/mod.rs`
**Current State**: Lines 29-112 contain `run()` and `run_internal()`
**Modification**: Update to dispatch to subcommands
**Lines to Change**: ~80 lines
**Scope**: Refactor dispatch logic to use subcommand enum

```rust
// Current: Matches on (key, value) tuples
match (&options.key, &options.value) {
    (None, None) => show_all_config(...),      // View
    (Some(key), None) => show_config_value(...), // Get
    (Some(key), Some(value)) => set_config_value(...), // Set
    ...
}

// New: Dispatches to subcommand handlers
match options.subcommand {
    ConfigSubcommand::View => subcommands::view::run(...),
    ConfigSubcommand::Get => subcommands::get::run(...),
    ConfigSubcommand::Set => subcommands::set::run(...),
    ConfigSubcommand::Validate => subcommands::validate::run(...),
}
```

### 3. Config Options Type Definition
**File**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/types.rs`
**Current State**: Lines 1-50+ define `ConfigOptions` struct
**Modification**: Add `subcommand` field to ConfigOptions
**Lines to Change**: ~20 lines
**Scope**: Add enum variant for subcommand

```rust
// Current:
pub struct ConfigOptions {
    pub key: Option<String>,
    pub value: Option<String>,
    pub global: bool,
    pub json: bool,
    pub validate: bool,
}

// New:
pub enum ConfigSubcommand {
    View,
    Get { key: String },
    Set { key: String, value: String },
    Validate,
}

pub struct ConfigOptions {
    pub subcommand: ConfigSubcommand,
    pub global: bool,
    pub json: bool,
}
```

### 4. App Command Dispatcher
**File**: `/home/lewis/src/zjj/crates/zjj/src/app.rs`
**Current State**: Lines 33-42 handle config command
**Modification**: Update to parse subcommand and construct ConfigOptions
**Lines to Change**: ~15 lines
**Scope**: Extract subcommand from ArgMatches

```rust
// Current:
Some(("config", sub_m)) => {
    config::run(config::ConfigOptions {
        key: sub_m.get_one::<String>("key").cloned(),
        value: sub_m.get_one::<String>("value").cloned(),
        global: sub_m.get_flag("global"),
        json: sub_m.get_flag("json"),
        validate: sub_m.get_flag("validate"),
    }).await
}

// New:
Some(("config", sub_m)) => {
    let subcommand = match sub_m.subcommand() {
        Some(("view", ...)) => ConfigSubcommand::View,
        Some(("get", ...)) => ConfigSubcommand::Get { ... },
        Some(("set", ...)) => ConfigSubcommand::Set { ... },
        Some(("validate", ...)) => ConfigSubcommand::Validate,
        _ => return Err(...),
    };
    config::run(config::ConfigOptions {
        subcommand,
        global: sub_m.get_flag("global"),
        json: sub_m.get_flag("json"),
    }).await
}
```

## New Files to Create

### 1. Subcommand Module Dispatcher
**Path**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/subcommands/mod.rs`
**Lines**: ~30 lines
**Purpose**: Route to correct subcommand handler
**Content**:
```rust
pub mod view;
pub mod get;
pub mod set;
pub mod validate;

pub use view::run as run_view;
pub use get::run as run_get;
pub use set::run as run_set;
pub use validate::run as run_validate;
```

### 2. View Subcommand Handler
**Path**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/subcommands/view.rs`
**Lines**: ~80 lines
**Purpose**: Display all configuration
**Extraction From**: `commands/config/mod.rs` lines 59-61 (view logic)
**Reuses**: `loading::show_all_config()`
**Content**: Shows all config in table or JSON format

### 3. Get Subcommand Handler
**Path**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/subcommands/get.rs`
**Lines**: ~80 lines
**Purpose**: Get specific configuration value
**Extraction From**: `commands/config/mod.rs` lines 64-65 (get logic)
**Reuses**: `loading::show_config_value()`, `validation::validate_config_key()`
**Content**: Retrieves and displays single value, with validation

### 4. Set Subcommand Handler
**Path**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/subcommands/set.rs`
**Lines**: ~100 lines
**Purpose**: Set configuration value
**Extraction From**: `commands/config/mod.rs` lines 69-102 (set logic)
**Reuses**: `defaults::set_config_value()`, `loading::global_config_path()`, `loading::project_config_path()`
**Content**: Sets value in appropriate config file (global/project), with validation

### 5. Validate Subcommand Handler
**Path**: `/home/lewis/src/zjj/crates/zjj/src/commands/config/subcommands/validate.rs`
**Lines**: ~90 lines
**Purpose**: Validate configuration integrity
**Extraction From**: `commands/config/mod.rs` lines 115-162 (validation logic)
**Reuses**: `validation::validate_configuration()`, `validation::is_readable()`
**Content**: Validates config, reports issues/warnings, JSON support

## Files NOT to Modify

### These utilities stay as-is (reused by subcommands)
- `/home/lewis/src/zjj/crates/zjj/src/commands/config/loading.rs`
  - `show_all_config()` - Used by view subcommand
  - `show_config_value()` - Used by get subcommand
  - `project_config_path()` - Used by set subcommand
  - `global_config_path()` - Used by set subcommand

- `/home/lewis/src/zjj/crates/zjj/src/commands/config/validation.rs`
  - `validate_config_key()` - Used by get subcommand
  - `validate_configuration()` - Used by validate subcommand
  - `is_readable()` - Used by validate subcommand

- `/home/lewis/src/zjj/crates/zjj/src/commands/config/defaults.rs`
  - `set_config_value()` - Used by set subcommand

## Testing Files to Update

### Existing Test File
**File**: `/home/lewis/src/zjj/crates/zjj/tests/p0_standardization_suite.rs`
**Current Tests**:
- `test_config_view()` (line 519)
- `test_config_view_json()` (line 545)
**Updates Needed**:
- Change from `zjj config` to `zjj config view`
- Change from `zjj config --json` to `zjj config view --json`
- Add new tests for get, set, validate subcommands

### New Tests to Add
In `/home/lewis/src/zjj/crates/zjj/src/commands/config/mod.rs` (tests section):
- `test_config_get_key()`
- `test_config_get_nested_key()`
- `test_config_get_json()`
- `test_config_set_value()`
- `test_config_set_global()`
- `test_config_set_invalid_key()`
- `test_config_validate_pass()`
- `test_config_validate_fail()`
- `test_config_validate_json()`

## Summary Table

| File | Type | Change | Lines | Priority |
|------|------|--------|-------|----------|
| cli/args.rs | Modify | Refactor cmd_config() | 113 | P1 |
| commands/config/mod.rs | Modify | Update run() dispatcher | 80 | P2 |
| commands/config/types.rs | Modify | Add ConfigSubcommand enum | 20 | P1 |
| app.rs | Modify | Update config handler | 15 | P2 |
| config/subcommands/mod.rs | Create | Module dispatcher | 30 | P3 |
| config/subcommands/view.rs | Create | View handler | 80 | P3 |
| config/subcommands/get.rs | Create | Get handler | 80 | P4 |
| config/subcommands/set.rs | Create | Set handler | 100 | P5 |
| config/subcommands/validate.rs | Create | Validate handler | 90 | P6 |
| **TOTAL** | | | **~608** | |

**Total New Code**: ~400 lines
**Total Modified Code**: ~228 lines
**Estimated Implementation Time**: 2-3 hours

## Dependency Graph

```
app.rs (config dispatch)
  └─> config/mod.rs (run)
       └─> config/subcommands/mod.rs (dispatcher)
            ├─> subcommands/view.rs
            │    └─> loading.rs (show_all_config)
            ├─> subcommands/get.rs
            │    ├─> loading.rs (show_config_value)
            │    └─> validation.rs (validate_config_key)
            ├─> subcommands/set.rs
            │    ├─> defaults.rs (set_config_value)
            │    ├─> loading.rs (paths)
            │    └─> validation.rs (validate_config_key)
            └─> subcommands/validate.rs
                 ├─> validation.rs (validate_configuration)
                 └─> loading.rs (config_path_opt)
```

All blue dependencies are existing, tested utilities that will be reused.

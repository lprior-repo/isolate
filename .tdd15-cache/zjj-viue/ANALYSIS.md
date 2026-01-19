# Bead Analysis: zjj-viue

## Issue
"P0: Implement config command subcommands (view/get/set/validate)"

## Current State

### Existing Implementation
The config command is **already partially implemented** but uses a positional argument model rather than true subcommands:

```bash
# Current approach (positional args + flags)
zjj config                 # View all
zjj config KEY             # Get value
zjj config KEY VALUE       # Set value
zjj config --validate      # Validate
```

### Files Already Present
1. **CLI Definition** (`crates/zjj/src/cli/args.rs:947-1059`)
   - `cmd_config()` - Defines the command with positional args and flags
   - Uses clap's `.arg()` for KEY and VALUE as positional arguments
   - Supports `--global`, `--json`, `--validate` flags

2. **Command Logic** (`crates/zjj/src/commands/config/mod.rs`)
   - `run()` - Main async handler that dispatches to view/get/set/validate
   - `run_internal()` - Handles (key, value) pattern matching
   - `run_validate_internal()` - Validation logic
   - Already supports JSON output for all operations

3. **Supporting Modules**
   - `config/defaults.rs` - `set_config_value()` - writes config to files
   - `config/loading.rs` - `show_all_config()`, `show_config_value()` - reads config
   - `config/validation.rs` - `validate_config_key()`, `validate_configuration()` - validates
   - `config/types.rs` - `ConfigOptions` struct

4. **Command Dispatch** (`crates/zjj/src/app.rs:33-42`)
   - Config command is routed directly in the main match statement
   - Extracts positional args: key, value and flags: global, json, validate

## Proposed Refactoring

### New CLI Model (Subcommands)
```bash
# New approach (true subcommands)
zjj config view             # View all
zjj config get KEY          # Get value
zjj config set KEY VALUE    # Set value
zjj config validate         # Validate
```

### Why Refactor?
- **Better CLI Discoverability**: `zjj config --help` will show clear subcommands
- **Cleaner UX**: Each operation has explicit intent
- **Better for Introspection**: Works better with `zjj introspect config`
- **Standard Pattern**: Matches other CLI tools (git config uses subcommands style)
- **Extensibility**: Easy to add new subcommands (e.g., export, import)

### File Changes

#### 1. CLI Definition (`crates/zjj/src/cli/args.rs`)
- Refactor `cmd_config()` to use `.subcommand()`
- Add 4 subcommands: view, get, set, validate
- Move flags (`--global`, `--json`) to appropriate subcommands

#### 2. New Subcommand Handlers
Create new module structure:
```
crates/zjj/src/commands/config/subcommands/
├── mod.rs           # Main subcommand dispatcher
├── view.rs          # zjj config view
├── get.rs           # zjj config get KEY
├── set.rs           # zjj config set KEY VALUE
└── validate.rs      # zjj config validate
```

#### 3. Main Config Handler (`crates/zjj/src/commands/config/mod.rs`)
- Update `run()` to accept subcommand enum
- Route to appropriate subcommand handler
- Update `ConfigOptions` to support subcommand mode

#### 4. App Dispatcher (`crates/zjj/src/app.rs`)
- Update config command handler to parse subcommand from ArgMatches
- Construct new ConfigOptions with subcommand info

## Complexity Analysis

### Rating: **MEDIUM**

### Why MEDIUM (not SIMPLE)?
1. Requires refactoring existing CLI args parsing
2. Creates new module structure with 4 handlers
3. Updates dispatch logic in 2 locations (app.rs + config/mod.rs)
4. Must maintain backward compatibility testing

### Why not COMPLEX?
1. Core logic already exists - just reorganizing
2. No new algorithms or business logic
3. No new external dependencies
4. Validation and loading utilities fully implemented
5. Can reuse ~90% of existing code

## Dependencies

### External
- `clap` - CLI argument parsing (already used)
- `tokio` - Async runtime (already used)
- `anyhow` - Error handling (already used)
- `serde_json` - JSON output (already used)

### Internal
- `zjj_core::config` - Config types and loading
- Existing config module utilities (loading, validation, defaults)
- JSON output types

## Files Overview

| File | Lines | Purpose | Change |
|------|-------|---------|--------|
| crates/zjj/src/cli/args.rs | 2000+ | CLI parsing | Refactor cmd_config() |
| crates/zjj/src/commands/config/mod.rs | 250 | Main handler | Refactor run() dispatch |
| crates/zjj/src/commands/config/types.rs | 50 | ConfigOptions struct | Add subcommand field |
| crates/zjj/src/app.rs | 150 | Command dispatch | Update config handler |
| config/loading.rs | 200 | Show config | Reuse as-is |
| config/validation.rs | 250 | Validate config | Reuse as-is |
| config/defaults.rs | 200 | Set config | Reuse as-is |

**Total Files to Modify**: 4
**Total Files to Create**: 5 (subcommands module + 4 handlers)
**Total Files Affected**: 8

## Recommended Implementation Strategy

### Phase 1: CLI Definition (SIMPLE)
- Add subcommand definitions to cmd_config()
- Define view, get, set, validate subcommands with proper args

### Phase 2: Create View Handler (SIMPLE)
- Extract view logic into `subcommands/view.rs`
- Keep JSON output support

### Phase 3: Create Get Handler (SIMPLE)
- Extract get logic into `subcommands/get.rs`
- Keep JSON output support

### Phase 4: Create Set Handler (SIMPLE)
- Extract set logic into `subcommands/set.rs`
- Support --global flag

### Phase 5: Create Validate Handler (SIMPLE)
- Extract validate logic into `subcommands/validate.rs`
- Keep JSON output support

### Phase 6: Refactor Main Handler (MEDIUM)
- Update config/mod.rs run() to dispatch to subcommands
- Update app.rs to call new dispatch logic
- Update ConfigOptions struct

### Phase 7: Test & Polish (SIMPLE)
- Add tests for each subcommand
- Verify JSON output for all paths
- Test --global flag combinations

## Testing Coverage

### Existing Tests to Update
- `test_config_view()` in p0_standardization_suite.rs
- `test_config_view_json()` in p0_standardization_suite.rs

### New Tests to Add
- test_config_get
- test_config_set
- test_config_validate
- test_config_get_nested_key
- test_config_set_global
- test_config_invalid_key
- test_config_validate_json_output

## Backward Compatibility

Can maintain legacy support via positional args initially, then deprecate in a future version:

```bash
# Phase 1: Support both
zjj config KEY              # Deprecated
zjj config get KEY          # Recommended

# Phase 2: Deprecation warning
zjj config KEY  # Warning: use "zjj config get KEY" instead

# Phase 3: Removal (future release)
```

## Effort Estimate

- **Estimated Lines to Write**: 400
- **Estimated Lines to Modify**: 300
- **Reused Existing Code**: ~1,800 lines (from loading.rs, validation.rs, defaults.rs)
- **Simplification Ratio**: 0.75 (most work is reorganization, not new code)

## Risk Assessment

**Risk Level**: LOW

**Why**:
- Core functionality already proven
- No new external dependencies
- Well-tested utility functions
- Can add subcommands without breaking existing flow
- Errors are surfaced through existing error handling

**Mitigation**:
- Keep existing config loading/validation utilities untouched
- Add subcommand dispatch layer on top of existing logic
- Test each subcommand independently

## Success Criteria

- [ ] All 4 subcommands (view, get, set, validate) work correctly
- [ ] JSON output works for all subcommands
- [ ] --global flag works for get, set, validate
- [ ] Error handling matches existing config command behavior
- [ ] All existing tests pass (with updates for subcommand format)
- [ ] New tests cover all subcommands and flag combinations
- [ ] CLI help (`zjj config --help`) shows clear subcommand list

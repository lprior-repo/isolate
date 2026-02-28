# Verification: Fix status routing to subcommands

## Implementation Complete

### Files Modified
1. `crates/isolate/src/cli/handlers/workspace.rs` - handle_status function updated with subcommand routing
2. `crates/isolate/src/cli/object_commands.rs` - cmd_status updated with subcommands and required args

### Changes Made

#### 1. handlers/workspace.rs
- Added import for introspection handlers
- Modified handle_status to route to subcommands:
  - `status show` → status::run()
  - `status whereami` → handle_whereami()
  - `status whoami` → handle_whoami()
  - `status context` → handle_context()
  - Legacy (no subcommand) → shows deprecation warning + runs status::run()

#### 2. object_commands.rs
- Changed `subcommand_required(true)` to `subcommand_required(false)` for backward compatibility
- Added contract_arg() and ai_hints_arg() to cmd_status
- Added name and watch args to cmd_status
- Added contract_arg() and ai_hints_arg() to each subcommand (show, whereami, whoami, context)
- Added field, no-beads, no-health args to context subcommand

### Manual Verification Results

```
$ isolate status show
✓ Returns session status list

$ isolate status whereami  
✓ Returns current workspace path

$ isolate status whoami
✓ Returns agent identity

$ isolate status context
✓ Returns context info (or error if no session - expected)

$ isolate status (legacy)
✓ Shows deprecation warning AND works correctly
```

## Moon Validation

### :quick
PASSED

### :test  
In progress - some pre-existing test failures unrelated to this fix

### :ci
In progress

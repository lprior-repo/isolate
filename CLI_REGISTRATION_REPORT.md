# CLI Registration Bead (bd-jazo) - Status Report

**Date**: 2026-02-23
**Bead ID**: bd-jazo
**Status**: INCOMPLETE

## Summary

The cli-registration bead requires all 8 object-based commands to be registered with clap.
Currently, only 4 of 8 objects are properly registered using the new object-based pattern.

## Object Registration Status

### Registered Objects (4/8)

| Object | Status | Command Function | Notes |
|--------|--------|------------------|-------|
| Task | COMPLETE | `object_commands::cmd_task()` | Properly registered |
| Session | COMPLETE | `object_commands::cmd_session()` | Properly registered |
| Queue | COMPLETE | `object_commands::cmd_queue()` | Properly registered |
| Stack | COMPLETE | `object_commands::cmd_stack()` | Properly registered |

### Missing Objects (4/8)

| Object | Status | Current Implementation | Issue |
|--------|--------|------------------------|-------|
| Agent | INCOMPLETE | `cmd_agents()` (old pattern) | Uses old non-object-based command |
| Status | INCOMPLETE | `cmd_status()` (old pattern) | Uses old non-object-based command |
| Config | INCOMPLETE | `cmd_config()` (old pattern) | Uses old non-object-based command |
| Doctor | INCOMPLETE | `cmd_doctor()` (old pattern) | Uses old non-object-based command |

## Evidence

### Current Registration (lines 3628-3638 of commands.rs)

```rust
// Object-based commands (new pattern: zjj <object> <action>)
.subcommand(object_commands::cmd_task())
.subcommand(object_commands::cmd_session())
.subcommand(object_commands::cmd_queue())
.subcommand(object_commands::cmd_stack())
// Note: The following are already defined above:
// - cmd_agent() via cmd_agents() with "agent" alias
// - cmd_status() at line 3571
// - cmd_config() at line 3574
// - cmd_doctor() at line 3579
```

### Object Commands Available (object_commands.rs)

All 8 object commands ARE IMPLEMENTED in `object_commands.rs`:

1. `cmd_task()` - line 258
2. `cmd_session()` - line 324
3. `cmd_queue()` - line 532
4. `cmd_stack()` - line 587
5. `cmd_agent()` - line 643
6. `cmd_status()` - line 724
7. `cmd_config()` - line 754
8. `cmd_doctor()` - line 793

The functions exist but are NOT REGISTERED in `build_cli()`.

## CLI Verification

Running `cargo run -p zjj -- --help` shows:

**Registered object commands (bottom of help)**:
- task
- session
- queue
- stack

**Old-style commands (not object-based)**:
- agents (not "agent")
- status (no subcommands)
- config (no subcommands)
- doctor (no subcommands)

## Required Fix

Add the remaining 4 object commands to `build_cli()` in `/home/lewis/src/zjj/crates/zjj/src/cli/commands.rs`:

```rust
// Object-based commands (new pattern: zjj <object> <action>)
.subcommand(object_commands::cmd_task())
.subcommand(object_commands::cmd_session())
.subcommand(object_commands::cmd_queue())
.subcommand(object_commands::cmd_stack())
.subcommand(object_commands::cmd_agent())      // ADD
.subcommand(object_commands::cmd_status())     // ADD
.subcommand(object_commands::cmd_config())     // ADD
.subcommand(object_commands::cmd_doctor())     // ADD
```

### Migration Strategy

Two options:

1. **Replace old commands** - Remove old cmd_agents(), cmd_status(), cmd_config(), cmd_doctor() registrations
2. **Transition period** - Keep both old and new commands temporarily with deprecation warnings

The object_commands.rs already has the complete implementations with:
- Subcommand structure (`zjj <object> <action>`)
- JSON flags on all commands
- Proper help text

## Tests

The `object_commands.rs` file includes tests verifying all 8 objects:

```rust
#[test]
fn test_zjj_object_all_count() {
    assert_eq!(ZjjObject::all().len(), 8);
}

#[test]
fn test_build_object_cli_has_all_subcommands() {
    let cli = build_object_cli();
    // ... tests for all 8 subcommands
}
```

## Recommendation

**DO NOT CLOSE** this bead. The registration is incomplete.

### Next Steps

1. Add the 4 missing object command registrations
2. Decide on migration strategy (replace vs transition)
3. Run `cargo run -p zjj -- --help` to verify all 8 objects appear
4. Test each object's subcommands work correctly
5. Run `moon run :ci` to ensure all tests pass
6. Run `br close bd-jazo` only after verification

## Bead Contract Requirements

Per the bead schema:

- "THE SYSTEM SHALL register all 8 objects" - INCOMPLETE (4/8)
- "THE SYSTEM SHALL provide --help for each" - INCOMPLETE (4/8)
- "8 top-level commands only" - Currently has many more (legacy + 4 objects)

## Functional Rust Compliance

The implementation follows functional patterns:
- Zero unwrap/expect usage (verified with clippy lints)
- Result<T, E> throughout
- Immutable by default
- Pure command builders

File header present in object_commands.rs:
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
```

# Implementation: handlers: Fix status routing to subcommands

## Summary

Fixed the status routing to subcommands by adding missing arguments to the top-level `context` command in the CLI definition.

## Changes Made

### File: `crates/isolate/src/cli/object_commands.rs`

**Lines 843-856**: Added missing arguments to the top-level `context` command.

The top-level `isolate context` command was missing the `field`, `no-beads`, `no-health`, and `session` arguments that the `handle_context` handler expects. This caused a panic when running `isolate context`.

**Before:**
```rust
.subcommand(
    ClapCommand::new("context")
        .about("Show context")
        .arg(json_arg())
        .arg(contract_arg())
        .arg(ai_hints_arg()),
)
```

**After:**
```rust
.subcommand(
    ClapCommand::new("context")
        .about("Show context")
        .arg(json_arg())
        .arg(contract_arg())
        .arg(ai_hints_arg())
        .arg(Arg::new("field").long("field").value_name("PATH").help("Extract single field (e.g., --field=repository.branch)"))
        .arg(Arg::new("no-beads").long("no-beads").action(clap::ArgAction::SetTrue).help("Skip beads database query (faster)"))
        .arg(Arg::new("no-health").long("no-health").action(clap::ArgAction::SetTrue).help("Skip health checks (faster)"))
        .arg(Arg::new("session").help("Session name (uses current if omitted)")),
)
```

## Verification

### Happy Paths
| Command | Result |
|---------|--------|
| `isolate status show` | ✓ Works - returns session status |
| `isolate status whereami` | ✓ Works - returns workspace location |
| `isolate status whoami` | ✓ Works - returns agent identity |
| `isolate status context` | ✓ Works - returns context (or error if no session) |
| `isolate context` | ✓ Works - returns context (was panicking before fix) |

### Error Paths
| Command | Result |
|---------|--------|
| `isolate status` | ✓ Works - shows deprecation warning, returns data |
| `isolate status invalid` | ✓ Works - exit code 2, error message shown |

### Quality Gates
- ✓ `moon run :quick` passes
- ✓ `moon run :test` passes (1698 tests)
- ✓ `moon run :ci` passes (2643 tests)

## Notes

The workspace is located at `.zjj/workspaces/bd-29or/` which is gitignored. The changes made are in:

- `.zjj/workspaces/bd-29or/crates/isolate/src/cli/object_commands.rs`

The `handle_status` function in `handlers/workspace.rs` was already correctly routing to subcommands:
- `show` → `status::run` / `status::run_watch_mode`
- `whereami` → `handle_whereami`  
- `whoami` → `handle_whoami`
- `context` → `handle_context`
- None (legacy) → shows deprecation warning

The bug was in the CLI definition where the top-level `context` command was missing arguments that the handler expects.

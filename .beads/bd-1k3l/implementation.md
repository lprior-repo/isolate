# Implementation Plan: Fix status routing to subcommands

## Summary

The `isolate status` command currently does not route subcommands (show, whereami, whoami, context) to their respective handlers. The CLI structure already defines these subcommands in `object_commands.rs`, but the handler only processes flags like `--contract`, `--ai-hints`, `--watch`.

## Files Modified

### 1. `crates/isolate/src/cli/handlers/workspace.rs`

**Location:** Lines 132-151 (`handle_status` function)

**Current Implementation:**
```rust
pub async fn handle_status(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::status());
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let watch = sub_m.get_flag("watch");

    if watch {
        status::run_watch_mode(name).await
    } else {
        status::run(name).await
    }
}
```

**Required Changes:**
- Add import for introspection handlers: `handle_context`, `handle_whereami`, `handle_whoami`
- Add subcommand matching using `sub_m.subcommand()`
- Route to appropriate handlers based on subcommand
- Add deprecation warning for legacy `isolate status` (no subcommand)

## Implementation Pattern

Following the pattern from `commands/task.rs:731` and `handlers/session.rs:200`:

```rust
pub async fn handle_status(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first (global flag)
    if sub_m.get_flag("contract") {
        println!("{}", crate::cli::json_docs::ai_contracts::status());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", crate::cli::json_docs::ai_contracts::command_flow());
        return Ok(());
    }

    // Route to subcommand handlers
    match sub_m.subcommand() {
        Some(("show", show_m)) => {
            let name = show_m.get_one::<String>("session").map(String::as_str);
            status::run(name).await
        }
        Some(("whereami", whereami_m)) => {
            handle_whereami(whereami_m).await
        }
        Some(("whoami", whoami_m)) => {
            handle_whoami(whoami_m)
        }
        Some(("context", context_m)) => {
            handle_context(context_m).await
        }
        None => {
            // Legacy: isolate status (no subcommand)
            // Show deprecation warning
            eprintln!("warning: 'isolate status' without subcommand is deprecated, use 'isolate status show' instead.");
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            let watch = sub_m.get_flag("watch");
            if watch {
                status::run_watch_mode(name).await
            } else {
                status::run(name).await
            }
        }
        Some((unknown, _)) => {
            Err(anyhow::anyhow!("Unknown status subcommand: '{}'. Use 'show', 'whereami', 'whoami', or 'context'", unknown))
        }
    }
}
```

## Handler Functions to Use

From `handlers/introspection.rs`:

| Subcommand | Handler Function | Signature |
|------------|------------------|-----------|
| show | `status::run(name)` | `async fn run(name: Option<&str>) -> Result<()>` |
| whereami | `handle_whereami` | `async fn handle_whereami(sub_m: &ArgMatches) -> Result<()>` |
| whoami | `handle_whoami` | `fn handle_whoami(sub_m: &ArgMatches) -> Result<()>` |
| context | `handle_context` | `async fn handle_context(sub_m: &ArgMatches) -> Result<()>` |

## Acceptance Tests

### Happy Paths
| Test | Command | Expected Behavior |
|------|---------|------------------|
| test_status_show | `isolate status show` | Exit code 0, workspace status output |
| test_status_whereami | `isolate status whereami` | Exit code 0, current path |
| test_status_whoami | `isolate status whoami` | Exit code 0, current agent |
| test_status_context | `isolate status context` | Exit code 0, context info |

### Error Paths
| Test | Command | Expected Behavior |
|------|---------|------------------|
| test_invalid_subcommand | `isolate status invalid` | Exit code non-zero, clear error message |
| test_missing_subcommand | `isolate status` | Deprecation warning + runs legacy status |

## Verification

After implementation:
1. Run `moon run :quick` - should pass
2. Run `moon run :test` - should pass
3. Run `moon run :ci` - should pass
4. Manual verification:
   - `isolate status show` - shows workspace status
   - `isolate status whereami` - shows current path
   - `isolate status whoami` - shows current agent
   - `isolate status context` - shows context info
   - `isolate status` - shows deprecation warning + status

## Constraints

- Use functional patterns: map, and_then, ?
- Return Result<T, Error> from all fallible functions
- Zero unwrap/expect - use proper error handling
- Keep existing contract/ai-hints flag handling

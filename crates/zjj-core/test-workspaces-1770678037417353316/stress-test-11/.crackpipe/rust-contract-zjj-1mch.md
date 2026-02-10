# Rust Contract: zjj-1mch

## Title
LOW-005: Add global --verbose flag

## Type
feature

## Description
Consider adding global --verbose flag for debugging. Would provide detailed logging across all commands.

## Scope
Add a global `--verbose` / `-v` flag that enables detailed logging for all commands.

## Preconditions
- Commands currently use `tracing` crate for logging
- No existing verbose flag
- Logging goes to stderr

## Postconditions
- All commands accept `--verbose` flag
- Verbose mode shows detailed operation logs
- Verbosity levels: `-v`, `-vv`, `-vvv` (optional)
- Helps with debugging user issues

## Invariants
- Verbose output goes to stderr (doesn't break stdout parsing)
- Default behavior unchanged (no verbose = normal output)
- Performance impact minimal when not verbose

## Implementation

### CLI Setup
```rust
// In main.rs, add global flag:
.arg(
    Arg::new("verbose")
        .short('v')
        .long("verbose")
        .action(clap::ArgAction::Count)
        .help("Increase verbosity (-v, -vv, -vvv)")
        .global(true)
)
```

### Logging Configuration
```rust
// Initialize logging based on verbosity level
fn init_logging(verbosity: u8) {
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_writer(std::io::stderr)
        .with_max_level(level)
        .init();
}
```

### Verbosity Levels
- **0 (default)**: Warnings and errors only
- **1 (-v)**: Info level - shows major operations
- **2 (-vv)**: Debug level - shows detailed operations
- **3 (-vvv)**: Trace level - shows everything

## Use Cases

### User: Debug why add command failed
```bash
zjj -v add my-session
# Shows: Creating workspace, cloning repo, setting up Zellij...
```

### User: Understand what sync is doing
```bash
zjj -vv sync
# Shows: Fetching from remote, checking for conflicts, updating beads.db...
```

### Developer: Full trace during development
```bash
zjj -vvv status
# Shows: Every function call, SQL query, file operation...
```

## Files to Modify
- `crates/zjj/src/main.rs` - Add global flag, init logging
- `crates/zjj-core/src/lib.rs` - Maybe add logging helpers

## Files to Test
- `crates/zjj/tests/test_verbose_flag.rs` - New test file

## Test Cases

### TV-1: Default (No Verbose)
- Run `zjj status` with no verbose flag
- Only warnings/errors shown
- Normal output unchanged

### TV-2: Single -v
- Run `zjj -v status`
- Info level logs shown
- Major operations visible

### TV-3: Double -vv
- Run `zjj -vv status`
- Debug level logs shown
- Detailed operations visible

### TV-4: Triple -vvv
- Run `zjj -vvv status`
- Trace level logs shown
- Everything logged

### TV-5: Verbose Doesn't Break JSON
- Run `zjj -v status --json`
- JSON output still valid on stdout
- Logs on stderr

## Estimated Effort
1 hour

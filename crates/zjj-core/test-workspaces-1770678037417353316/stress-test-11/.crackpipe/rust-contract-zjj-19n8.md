# Rust Contract: zjj-19n8

## Title
LOW-006: Document callback execution behavior

## Type
chore

## Description
Document callback execution behavior in user guide. Explain --on-success, --on-failure, --on-complete behavior, output capture, and execution context.

## Scope
This is a documentation-only task. No code changes required.

## Preconditions
- Callback flags exist in the CLI (`--on-success`, `--on-failure`, `--on-complete`)
- Callback execution logic is implemented
- User guide exists

## Postconditions
- User guide documents callback behavior
- Examples show how to use callbacks
- Edge cases are explained
- Output capture behavior is clear

## Invariants
- Documentation matches actual implementation
- Examples are copy-paste runnable

## Documentation Sections

### Callback Overview
- What are callbacks?
- When do they run?
- Execution order

### Callback Flags
- `--on-success <command>`: Runs when command succeeds (exit 0)
- `--on-failure <command>`: Runs when command fails (exit != 0)
- `--on-complete <command>`: Always runs, regardless of exit code

### Execution Context
- Working directory: Current zjj session directory
- Environment variables: What's available to callback
- Exit code propagation: How callback exit codes affect overall result

### Output Capture
- Stdout from command is NOT captured by callbacks
- Callbacks receive no input from parent command
- Callback output goes directly to terminal

### Examples
```bash
# Sync beads after successful add
zjj add my-session --on-success "br sync"

# Cleanup on failure
zjj add my-session --on-failure "rm -rf workspaces/my-session"

# Always log completion
zjj add my-session --on-complete "echo 'Session setup complete' >> ~/.zjj.log"
```

### Edge Cases
- Callback fails: What happens?
- Callback doesn't exist: Error or silent skip?
- Multiple callbacks: Execution order
- Nested callbacks: Supported or not?

## Files to Modify
- `docs/user-guide.md` (or equivalent)
- `README.md` (if quick reference needed)

## Estimated Effort
30 minutes

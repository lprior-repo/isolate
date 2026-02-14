# Dry-Run Implementation Pattern

## Overview

This document describes the pattern for implementing `--dry-run` functionality across zjj commands. The pattern was established with the `remove` command and should be followed for all mutating commands.

## Completed

- ✅ `remove` - Full implementation with preview output

## Pending

- ⏳ `init` - Initialize zjj
- ⏳ `sync` - Sync workspace with main
- ⏳ `spawn` - Spawn agent workspace
- ⏳ `batch` - Execute batch commands

## Implementation Steps

### Step 1: Add dry_run field to Options struct

**Location**: `crates/zjj/src/commands/<command>.rs` or `crates/zjj/src/commands/<command>/types.rs`

**Pattern**:
```rust
pub struct <Command>Options {
    // ... existing fields ...
    /// Preview without executing
    pub dry_run: bool,
    /// Output format
    pub format: OutputFormat,
}
```

**Example** (from `remove.rs`):
```rust
pub struct RemoveOptions {
    pub force: bool,
    pub merge: bool,
    pub keep_branch: bool,
    pub idempotent: bool,
    pub dry_run: bool,  // ← Add this field
    pub format: OutputFormat,
}
```

### Step 2: Extract dry-run flag in handler

**Location**: `crates/zjj/src/cli/handlers/<handler>.rs`

**Pattern**:
```rust
pub async fn handle_<command>(sub_m: &ArgMatches) -> Result<()> {
    // ... extract other arguments ...
    let options = <Command>Options {
        // ... other fields ...
        dry_run: sub_m.get_flag("dry-run"),  // ← Add this line
        format,
    };
    <command>::run_with_options(&options).await
}
```

**Example** (from `workspace.rs`):
```rust
pub async fn handle_remove(sub_m: &ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
    let format = get_format(sub_m);
    let options = remove::RemoveOptions {
        force: sub_m.get_flag("force"),
        merge: sub_m.get_flag("merge"),
        keep_branch: sub_m.get_flag("keep-branch"),
        idempotent: sub_m.get_flag("idempotent"),
        dry_run: sub_m.get_flag("dry-run"),  // ← Added
        format,
    };
    remove::run_with_options(name, &options).await
}
```

### Step 3: Implement dry-run logic in command

**Location**: `crates/zjj/src/commands/<command>.rs` or `crates/zjj/src/commands/<command>/mod.rs`

**Pattern**:
```rust
pub async fn run_with_options(/* args */, options: &<Command>Options) -> Result<()> {
    // 1. Perform validation (same as normal execution)
    // 2. Gather information about what would happen
    
    // 3. DRY-RUN MODE: Exit early with preview
    if options.dry_run {
        let preview_message = build_dry_run_preview(/* ... */);
        
        if options.format.is_json() {
            let output = <Command>Output {
                // ... fields ...
                message: preview_message,
            };
            let envelope = SchemaEnvelope::new("<command>-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(std::io::stdout(), "{}", preview_message)?;
        }
        return Ok(());
    }
    
    // 4. Continue with normal execution
    // ...
}

fn build_dry_run_preview(/* args */) -> String {
    let mut preview = String::new();
    preview.push_str(&format!("DRY-RUN: Would <action>\n"));
    // Add details about what would happen
    preview.push_str("\nNo changes made (dry-run mode)");
    preview
}
```

**Example** (from `remove.rs`):
```rust
pub async fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()> {
    let db = get_session_db().await?;

    // Validation happens first (same as normal execution)
    let session = match db.get(name).await? {
        Some(session) => session,
        None if options.idempotent => {
            // Handle idempotent case
            return Ok(());
        }
        None => {
            return Err(anyhow::Error::new(zjj_core::Error::NotFound(
                format!("Session '{name}' not found")
            )));
        }
    };

    // DRY-RUN MODE: Show preview without executing
    if options.dry_run {
        let preview_message = build_dry_run_preview(name, &session.workspace_path, options);
        
        if options.format.is_json() {
            let output = RemoveOutput {
                name: name.to_string(),
                message: preview_message,
            };
            let envelope = SchemaEnvelope::new("remove-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(std::io::stdout(), "{}", preview_message)?;
        }
        return Ok(());
    }

    // Normal execution continues...
    // ...
}

fn build_dry_run_preview(name: &str, workspace_path: &str, options: &RemoveOptions) -> String {
    let mut preview = String::new();
    preview.push_str(&format!("DRY-RUN: Would remove session '{name}'\n"));
    preview.push_str(&format!("  Workspace path: {workspace_path}\n"));
    preview.push_str(&format!("  Database record: {name}\n"));
    
    if options.merge {
        preview.push_str("  Action: Squash-merge to main before removal\n");
    }
    
    if !options.force {
        preview.push_str("  Confirmation: Would prompt for confirmation\n");
        preview.push_str("  Hooks: Would run pre_remove hooks\n");
    } else {
        preview.push_str("  Confirmation: Skipped (--force)\n");
        preview.push_str("  Hooks: Skipped (--force)\n");
    }
    
    preview.push_str("\nNo changes made (dry-run mode)");
    preview
}
```

## Key Principles

### 1. Early Exit with Preview
- Dry-run should exit **after validation but before any modifications**
- This ensures errors are caught (e.g., "session not found") even in dry-run mode
- Preview shows exactly what **would** happen

### 2. Zero Side Effects
Dry-run mode MUST NOT:
- ❌ Create, modify, or delete files
- ❌ Modify database records
- ❌ Execute hooks
- ❌ Make network requests
- ❌ Spawn processes
- ❌ Modify environment state

Dry-run mode SHOULD:
- ✅ Perform validation (check if session exists, validate inputs)
- ✅ Read current state (to show what would change)
- ✅ Return appropriate exit codes (0 for valid operation, non-zero for errors)

### 3. Consistent Output Format
- **Text mode**: Human-readable preview starting with "DRY-RUN: "
- **JSON mode**: Same JSON structure as normal execution, with preview in message field
- **Exit code**: 0 if operation would succeed, non-zero if it would fail

### 4. Show Conditional Actions
Preview should indicate:
- What files/records would be affected
- Whether optional actions would trigger (e.g., hooks, confirmations)
- What flags affect behavior (e.g., --force, --merge)

## Testing Guidelines

### Manual Testing
```bash
# Test with existing session
zjj <command> <args> --dry-run

# Test with non-existent resource (should error even in dry-run)
zjj <command> nonexistent --dry-run

# Test JSON output
zjj <command> <args> --dry-run --json

# Test with various flag combinations
zjj <command> <args> --dry-run --force
zjj <command> <args> --dry-run --merge
```

### Verification Checklist
- [ ] Command accepts `--dry-run` flag
- [ ] Preview shows what would happen
- [ ] No files are created/modified/deleted
- [ ] No database records are changed
- [ ] Validation errors are still shown
- [ ] Exit code 0 for valid operation
- [ ] Exit code non-zero for invalid operation
- [ ] JSON output is valid and follows schema
- [ ] Text output starts with "DRY-RUN: "

## Command-Specific Notes

### remove
- ✅ Implemented
- Shows: workspace path, database record, merge intent, confirmation, hooks

### init (Pending)
- Should show: .zjj directory structure, config files, database initialization
- No database or files should be created

### sync (Pending)
- Should show: commits to rebase, conflicts (if any), workspace state changes
- No git/jj operations should execute

### spawn (Pending)
- Should show: workspace to create, agent command, bead association
- No workspace creation, no agent spawning

### batch (Pending)
- Should show: list of all commands that would execute
- No actual command execution

## Files Modified

For each command, you'll typically modify:
1. `crates/zjj/src/commands/<command>.rs` or `<command>/mod.rs` - Add dry_run field and logic
2. `crates/zjj/src/commands/<command>/types.rs` - If Options struct is in separate file
3. `crates/zjj/src/cli/handlers/<handler>.rs` - Extract dry-run flag
4. `crates/zjj/src/cli/commands.rs` - CLI arg already defined ✓

## Success Criteria

A dry-run implementation is complete when:
1. ✅ Code compiles without errors
2. ✅ `zjj <command> --help` shows `--dry-run` flag
3. ✅ `zjj <command> <valid-args> --dry-run` shows preview without side effects
4. ✅ `zjj <command> <invalid-args> --dry-run` returns error
5. ✅ `zjj <command> --dry-run --json` outputs valid JSON
6. ✅ Manual testing confirms zero side effects
7. ✅ Pattern documented for next implementer

## Example Usage

```bash
# Remove command (implemented)
$ zjj remove test-session --dry-run
DRY-RUN: Would remove session 'test-session'
  Workspace path: /path/to/workspace
  Database record: test-session
  Confirmation: Would prompt for confirmation
  Hooks: Would run pre_remove hooks

No changes made (dry-run mode)

$ zjj remove test-session --dry-run --force --merge
DRY-RUN: Would remove session 'test-session'
  Workspace path: /path/to/workspace
  Database record: test-session
  Action: Squash-merge to main before removal
  Confirmation: Skipped (--force)
  Hooks: Skipped (--force)

No changes made (dry-run mode)
```

## Next Steps

To implement dry-run for the remaining commands:
1. Choose a command (init, sync, spawn, or batch)
2. Follow the 3-step pattern above
3. Test thoroughly
4. Update this document
5. Commit changes
6. Move to next command

---

**Pattern established by**: Phase 2 dry-run implementation (bead bd-b6q)
**Reference implementation**: `remove` command
**Date**: 2026-02-14

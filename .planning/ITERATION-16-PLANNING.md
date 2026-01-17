# Ralph Loop Iteration 16 - Planning

**Date:** 2026-01-16
**Focus:** Phase 8/9 (AI-Native Features) - Output Composability
**Status:** PLANNING
**Priority:** P3 (unblocked enhancement work)

---

## Context

**Previous Iterations:**
- Iterations 1-11: Technical debt cleanup COMPLETE
- Iterations 12-13: Machine-readable exit codes COMPLETE (zjj-8en6)
- Iterations 14-15: Machine-readable help COMPLETE (zjj-g80p)
- Current: Continue Phase 8/9 AI-native enhancements

**Available Work:**
- P2: zjj-2a4, zjj-so2 (BLOCKED - require profiling)
- P3: zjj-t157 (UNBLOCKED - output composability)
- P3: zjj-eca (documentation - CODE_OF_CONDUCT.md)
- P4: zjj-im1 (documentation - async migration changelog)

---

## Selected Work: zjj-t157 (Output Composability)

**Bead:** zjj-t157
**Priority:** P3
**Type:** Feature (AI-native composability)
**Status:** In progress

**Description:**
"Command output must be pipe-friendly: silent mode, parseable format, no ANSI in pipes. Add --silent flag, detect TTY vs pipe. Success: commands compose well with | and >."

**Why This Work:**
1. Natural follow-up to zjj-g80p (both AI-native features)
2. Complements existing --json modes
3. No blockers or dependencies
4. Improves automation and scripting experience
5. Common Unix philosophy: "programs that work together"

---

## Current State Analysis

### Existing Infrastructure
**TTY Detection (cli.rs):**
```rust
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

pub fn is_stdin_tty() -> bool {
    std::io::stdin().is_terminal()
}
```

**Already Used In:**
- add.rs:860 - Check TTY before Zellij operations
- focus.rs:94 - Check TTY before Zellij operations
- remove.rs:283 - Check stdin TTY before confirmation prompt

### Current Output Patterns

**list command (decorated):**
```
NAME                 STATUS       BRANCH          CHANGES    BEADS
----------------------------------------------------------------------
feature-auth         active       my-branch       2 files    3 open
```

**list command (JSON mode):**
```json
[
  {
    "name": "feature-auth",
    "status": "active",
    "branch": "my-branch",
    "changes": "2 files",
    "beads": "3 open"
  }
]
```

### Decorative Elements Found
1. **Headers** - Column names (NAME, STATUS, BRANCH, etc.)
2. **Separators** - Dash lines ("-".repeat(70))
3. **Formatting** - Column alignment ("{:<20}")
4. **Messages** - "No sessions found." type messages

---

## Requirements

### Pipe-Friendly Behavior
When output is piped (not a TTY) or --silent flag is used:
1. **Suppress decorations** - No headers, no separators, no formatting
2. **Minimal output** - Just the data, one item per line
3. **No ANSI codes** - Plain text only (we don't use ANSI currently, but future-proof)
4. **Parseable format** - Tab or space-separated values, easy to parse

### Three Output Modes
1. **Normal (TTY)** - Decorated output with headers, alignment, formatting
2. **Pipe mode (!is_tty())** - Minimal, parseable output (auto-detected)
3. **Silent (--silent)** - Minimal output even in TTY (explicit)

### Command Behavior

**list command:**
- Normal: Table with headers and formatting
- Pipe/Silent: Tab-separated values, one session per line
- JSON: Unchanged (already machine-readable)

**add command:**
- Normal: "Created session 'name' in workspace '/path'"
- Pipe/Silent: Just output the session name or nothing
- JSON: Unchanged

**remove command:**
- Normal: "Removed session 'name'"
- Pipe/Silent: Nothing (or just status code)
- JSON: Unchanged

**focus command:**
- Normal: "Focused on session 'name'"
- Pipe/Silent: Nothing (or just status code)
- JSON: Unchanged

**status command:**
- Normal: Decorated status with formatting
- Pipe/Silent: Minimal key-value pairs
- JSON: Unchanged

---

## Implementation Plan

### Step 1: Add --silent Flag to Commands
Add --silent flag to commands that produce decorative output:
- list
- status
- (add, remove, focus already use --json for scripting)

**Option struct pattern:**
```rust
pub struct ListOptions {
    pub all: bool,
    pub json: bool,
    pub silent: bool,  // NEW
}
```

### Step 2: Create Output Helper Functions
Create helper functions to decide output mode:

```rust
// In cli.rs or new output.rs module
pub fn should_use_minimal_output(json: bool, silent: bool) -> bool {
    json || silent || !is_tty()
}
```

### Step 3: Update list Command
Modify list.rs to support minimal output:

**Current:**
```rust
if options.json {
    output_json(&items)?;
} else {
    output_table(&items);
}
```

**New:**
```rust
if options.json {
    output_json(&items)?;
} else if options.silent || !is_tty() {
    output_minimal(&items);  // NEW
} else {
    output_table(&items);
}
```

**New function:**
```rust
fn output_minimal(items: &[SessionListItem]) {
    for item in items {
        println!("{}\t{}\t{}\t{}\t{}",
            item.name, item.status, item.branch, item.changes, item.beads
        );
    }
}
```

### Step 4: Update Other Commands
Apply similar pattern to:
- status command - minimal key-value output
- (add/remove/focus use JSON mode for scripting, may not need --silent)

### Step 5: Testing Strategy
Test scenarios:
1. **TTY mode:** Decorated output as expected
2. **Pipe mode:** `jjz list | grep feature` - minimal output
3. **Silent flag:** `jjz list --silent` - minimal output in terminal
4. **JSON mode:** `jjz list --json | jq` - unchanged
5. **Redirect:** `jjz list > output.txt` - minimal output

---

## Success Criteria

- [ ] --silent flag added to list command
- [ ] --silent flag added to status command
- [ ] Pipe detection using is_tty() works correctly
- [ ] Minimal output is parseable (tab-separated)
- [ ] No decorations in pipe mode (auto-detected)
- [ ] No decorations with --silent flag
- [ ] JSON mode unchanged
- [ ] All 202/202 tests passing
- [ ] Manual testing with pipes and redirects passes
- [ ] zjj-t157 bead can be closed

---

## Estimated Effort

**Time:** 2-3 hours

**Breakdown:**
- Planning and analysis: 30 minutes (current)
- Implementation: 1-2 hours
  - Add --silent flags: 15 minutes
  - Update list command: 30 minutes
  - Update status command: 30 minutes
  - Helper functions: 15 minutes
- Testing: 30 minutes
- Documentation: 15 minutes

---

## Related Work

**Completed:**
- zjj-8en6 (exit codes) - AI agents can interpret outcomes
- zjj-g80p (help JSON) - AI agents can parse command structure

**Builds On:**
- Existing TTY detection in cli.rs
- Existing --json modes in all commands

**Follow-up:**
- Could add color support with auto-detection (but not required for zjj-t157)
- Could add --format option (json|table|minimal) for explicit control

---

## Decision Points

### Output Format for Minimal Mode
**Options:**
1. Tab-separated values (TSV) - `name\tstatus\tbranch`
2. Space-separated values (SSV) - `name status branch`
3. JSON Lines (JSONL) - `{"name":"...","status":"..."}`

**Recommendation:** Tab-separated values (TSV)
- Easy to parse with cut, awk, etc.
- Handles spaces in values
- Common Unix pattern
- Can be imported into spreadsheets

### Commands to Update
**Must Update:**
- list (table output)
- status (formatted output)

**Maybe Update:**
- add (just success message, --json already available)
- remove (just success message, --json already available)
- focus (just success message, --json already available)

**Recommendation:** Start with list and status, assess others based on testing

---

**Status:** READY TO IMPLEMENT
**Next:** Implement --silent flag and minimal output for list command

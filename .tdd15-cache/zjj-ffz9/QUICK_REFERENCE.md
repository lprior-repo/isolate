# Quick Reference: Help Text Update for zjj-ffz9

## One-Minute Overview

**What**: Update all CLI help text to reflect v0.2.0 naming changes
- **File**: `crates/zjj/src/cli/args.rs` (25 command builders)
- **Change**: Replace `jjz` → `zjj`, `.jjz/` → `.zjj/`
- **Time**: 3-3.5 hours
- **Risk**: LOW (documentation only)

---

## Search & Replace Patterns

### Pattern 1: Binary Name (Most Common)
```bash
# Find: jjz init, jjz add, etc. in help text
# Replace with: zjj init, zjj add, etc.

# Command to find all:
grep -n '\.about.*".*jjz\|\.long_about.*".*jjz\|\.after_help.*".*jjz' \
  crates/zjj/src/cli/args.rs
```

### Pattern 2: Directory Paths
```bash
# Find: .jjz/ in help text examples
# Replace with: .zjj/

# Command to find all:
grep -n '\.jjz' crates/zjj/src/cli/args.rs
```

### Pattern 3: Tab Prefix
```bash
# Find: jjz: (tab naming)
# Replace/Note: zjj: (or mention as configurable)

grep -n 'jjz:' crates/zjj/src/cli/args.rs
```

---

## Key Locations by Priority

### HIGHEST PRIORITY
1. **build_cli()** (line 2068)
   - Root command `.long_about()`
   - Most visible to users
   - Examples showing full workflow

### HIGH PRIORITY
2. **cmd_init()** (line 8) - Already mostly correct, spot-check examples
3. **cmd_add()** (line 110) - Most frequently used
4. **cmd_list()** (line 327)
5. **cmd_focus()** (line 579)

### MEDIUM PRIORITY
6. **cmd_remove()** (line 458)
7. **cmd_sync()** (line 745)
8. **cmd_diff()** (line 851)
9. **cmd_status()** (line 649)
10. **cmd_config()** (line 948)

### OTHER COMMANDS
11-25: Remaining command builders (less frequent use)

---

## Common Issues to Watch For

### Issue 1: Hidden "jjz" References
These will appear in middle of text strings:
- `"jjz-1234"` (bead ID - KEEP AS IS, don't change)
- `"jjz agent"` (command name in examples - CHANGE to `zjj agent`)
- `"jjz:session"` (tab prefix example - CHANGE to `zjj:session`)

### Issue 2: Directory References
```rust
// WRONG (v0.1.0)
.zjj/config.toml      # Already using .zjj/ - OK
.jjz/config.toml      # OLD - CHANGE TO .zjj/

// WRONG (should be)
~/.jjz/layouts        # CHANGE to ~/.zjj/layouts
.jjz/state.db         # CHANGE to .zjj/state.db
```

### Issue 3: Workflow Examples
```rust
// WRONG
"jjz init → jjz add → [work] → jjz sync → jjz remove"

// CORRECT
"zjj init → zjj add → [work] → zjj sync → zjj remove"
```

---

## Testing Checklist

After updates, verify:

### Automated Tests
```bash
moon run :test              # Run all tests
# Specifically check:
# - test_help_flag
# - test_init_help
# - test_add_help
# - test_all_commands_have_help
# - test_help_has_examples
# - test_help_has_ai_agent_sections
```

### Manual Verification
```bash
# Verify no jjz references (except bead IDs)
zjj --help | grep jjz         # Should be empty
zjj init --help | grep jjz    # Should be empty
zjj add --help | grep jjz     # Should be empty

# Verify zjj references present
zjj --help | grep "zjj init"  # Should find examples

# Check JSON output validity
zjj --help --json | jq .      # Should parse without errors
zjj init --help --json | jq . # Should parse without errors
```

---

## Red Flags (Stop & Review)

🚨 If you see:
- `"jjz agent"` or `"jjz query"` in help text (but not `jjz-XXXX` IDs)
- `.jjz/` directory in examples (should be `.zjj/`)
- `"Initialize jjz in a JJ repository"` (should be "zjj")
- `"jjz:"` as tab prefix in examples (should be `"zjj:"`)
- Bead ID like `"jjz-ffz9"` being changed (WRONG - keep bead IDs as is)

---

## Implementation Template

Each update follows same pattern:

```rust
// BEFORE (v0.1.0 style)
pub fn cmd_example() -> Command {
    Command::new("example")
        .about("Do something with jjz setup")
        .long_about(
            "Initialize ZJJ in a Repository\n\
             \n\
             WHAT IT DOES:\n\
             Setting up jjz infrastructure:\n  \
             1. Creates .jjz/ directory\n  \
             2. Initializes .jjz/config.toml\n  \
             3. Sets up .jjz/layouts/\n\
             \n\
             WORKFLOW:\n\
             jjz init → jjz add → jjz sync\n\
             \n\
             RELATED:\n\
             • jjz doctor - Check system health\n\
             • jjz config - View/modify configuration"
        )
        .after_help(
            "EXAMPLES:\n  \
             jjz init\n\
             jjz init --repair\n\
             jjz init --force"
        )
}

// AFTER (v0.2.0 style)
pub fn cmd_example() -> Command {
    Command::new("example")
        .about("Do something with zjj setup")
        .long_about(
            "Initialize ZJJ in a Repository\n\
             \n\
             WHAT IT DOES:\n\
             Setting up ZJJ infrastructure:\n  \
             1. Creates .zjj/ directory\n  \
             2. Initializes .zjj/config.toml\n  \
             3. Sets up .zjj/layouts/\n\
             \n\
             WORKFLOW:\n\
             zjj init → zjj add → zjj sync\n\
             \n\
             RELATED:\n\
             • zjj doctor - Check system health\n\
             • zjj config - View/modify configuration"
        )
        .after_help(
            "EXAMPLES:\n  \
             zjj init\n\
             zjj init --repair\n\
             zjj init --force"
        )
}
```

---

## Commands Needing Updates (Complete List)

| # | Function | Lines | Status |
|---|----------|-------|--------|
| 1 | build_cli | 2068-2150+ | CRITICAL |
| 2 | cmd_init | 8-107 | REVIEW |
| 3 | cmd_add | 110-228 | REVIEW |
| 4 | cmd_add_batch | 231-325 | REVIEW |
| 5 | cmd_list | 327-456 | REVIEW |
| 6 | cmd_remove | 458-577 | REVIEW |
| 7 | cmd_focus | 579-647 | REVIEW |
| 8 | cmd_status | 649-743 | REVIEW |
| 9 | cmd_sync | 745-849 | REVIEW |
| 10 | cmd_diff | 851-946 | REVIEW |
| 11 | cmd_config | 948-1060 | REVIEW |
| 12 | cmd_dashboard | 1062-1137 | REVIEW |
| 13 | cmd_context | 1139-1226 | REVIEW |
| 14 | cmd_prime | 1228-1295 | REVIEW |
| 15 | cmd_introspect | 1297-1386 | REVIEW |
| 16 | cmd_doctor | 1388-1414 | REVIEW |
| 17 | cmd_query | 1416-1537 | REVIEW |
| 18 | cmd_completions | 1539-1611 | REVIEW |
| 19 | cmd_backup | 1649-1671 | REVIEW |
| 20 | cmd_restore | 1673-1702 | REVIEW |
| 21 | cmd_verify_backup | 1704-1807 | REVIEW |
| 22 | cmd_essentials | 1809-1865 | REVIEW |
| 23 | cmd_version | 1867-1883 | REVIEW |
| 24 | cmd_onboard | 1885-1949 | REVIEW |
| 25 | cmd_hooks | 1951-1970 | REVIEW |
| 26 | cmd_agent | 1972-2063 | REVIEW |

---

## Time Estimates

| Phase | Task | Time |
|-------|------|------|
| 1 | Audit for outdated refs | 30 min |
| 2 | Update root command | 15 min |
| 3 | Update core commands (7) | 45 min |
| 4 | Update sync/config (6) | 30 min |
| 5 | Update introspection (6) | 30 min |
| 6 | Test & validate | 15 min |
| **Total** | | **2.5-3.5 hrs** |

---

## Success Criteria

- ✅ All 26 command builders reviewed
- ✅ No "jjz" references in help text (except bead IDs)
- ✅ All paths use `.zjj/`
- ✅ All examples use `zjj` command
- ✅ All help text tests passing
- ✅ Manual verification: `zjj --help` outputs correctly
- ✅ JSON help output valid
- ✅ Commit includes v0.2.0 reference

---

## Related Documents

- Full analysis: `ANALYSIS.md` (this directory)
- Machine-readable: `triage.json` (this directory)
- Changelog: `/home/lewis/src/zjj/CHANGELOG.md` (lines 40-100)
- Code file: `/home/lewis/src/zjj/crates/zjj/src/cli/args.rs`

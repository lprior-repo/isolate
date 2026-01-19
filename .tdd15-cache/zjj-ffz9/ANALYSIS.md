# Bead Analysis: zjj-ffz9 - Update help text for v0.2.0 release

## Bead Details
- **ID**: zjj-ffz9
- **Title**: P0: Update help text for v0.2.0 release
- **Status**: OPEN
- **Priority**: P0 (Critical for release)
- **Type**: Documentation/Help Text Update

---

## Executive Summary

This bead requires updating all CLI help text across 25+ command builders to reflect the v0.2.0 release. The primary change is that the project was renamed from `jjz` to `zjj` (both the binary name and directory structure).

**Scope**: Single file with systematic updates
- **File**: `/home/lewis/src/zjj/crates/zjj/src/cli/args.rs` (2,180 lines)
- **Changes**: Replace outdated examples and path references
- **Complexity**: MEDIUM - Repetitive but straightforward pattern matching

**Status Assessment**: READY FOR IMPLEMENTATION

---

## v0.2.0 Breaking Changes

Per CHANGELOG.md (lines 40-100):

### Binary & Directory Rename
- Binary: `jjz` → `zjj`
- Config Directory: `.jjz/` → `.zjj/`
- Session Prefix: `jjz:` → `zjj:` (configurable)
- Database: `.jjz/state.db` → `.zjj/state.db`
- Layouts: `.jjz/layouts/` → `.zjj/layouts/`

### Example Migration
```bash
# OLD (v0.1.0)
jjz init
jjz add feature-x
jjz list

# NEW (v0.2.0)
zjj init
zjj add feature-x
zjj list
```

---

## Help Text Locations

All help text is defined in a single file with 25 command builder functions:

### Session Lifecycle Commands (7 functions)
1. **cmd_init()** (line 8)
   - Long help: lines 11-54
   - Status: ✓ Already uses `.zjj/` (correct for v0.2.0)

2. **cmd_add()** (line 110)
   - Long help: lines 113-228
   - Status: REVIEW NEEDED

3. **cmd_add_batch()** (line 231)
   - Long help: lines 234-325
   - Status: REVIEW NEEDED

4. **cmd_list()** (line 327)
   - Long help: lines 330-456
   - Status: REVIEW NEEDED

5. **cmd_remove()** (line 458)
   - Long help: lines 461-577
   - Status: REVIEW NEEDED

6. **cmd_focus()** (line 579)
   - Long help: lines 582-647
   - Status: REVIEW NEEDED

7. **cmd_status()** (line 649)
   - Long help: lines 652-743
   - Status: REVIEW NEEDED

### Workspace Sync Commands (2 functions)
8. **cmd_sync()** (line 745)
   - Long help: lines 748-849
   - Status: REVIEW NEEDED

9. **cmd_diff()** (line 851)
   - Long help: lines 854-946
   - Status: REVIEW NEEDED

### Configuration & System (2 functions)
10. **cmd_config()** (line 948)
    - Long help: lines 952-1060
    - Status: REVIEW NEEDED

11. **cmd_doctor()** (line 1388)
    - Help: lines 1390-1414
    - Status: REVIEW NEEDED

### Introspection & UI (3 functions)
12. **cmd_dashboard()** (line 1062)
    - Long help: lines 1066-1137
    - Status: REVIEW NEEDED

13. **cmd_context()** (line 1139)
    - Long help: lines 1143-1226
    - Status: REVIEW NEEDED

14. **cmd_introspect()** (line 1297)
    - Long help: lines 1300-1386
    - Status: REVIEW NEEDED

15. **cmd_prime()** (line 1228)
    - Long help: lines 1231-1295
    - Status: REVIEW NEEDED

### Utilities & Backup (6 functions)
16. **cmd_backup()** (line 1649)
    - Help: lines 1651-1671
    - Status: REVIEW NEEDED

17. **cmd_restore()** (line 1673)
    - Help: lines 1675-1702
    - Status: REVIEW NEEDED

18. **cmd_verify_backup()** (line 1704)
    - Long help: lines 1707-1807
    - Status: REVIEW NEEDED

19. **cmd_query()** (line 1416)
    - Long help: lines 1419-1537
    - Status: REVIEW NEEDED

20. **cmd_completions()** (line 1539)
    - Long help: lines 1542-1611
    - Status: REVIEW NEEDED

21. **cmd_version()** (line 1867)
    - Help: lines 1868-1883
    - Status: REVIEW NEEDED

### Onboarding & AI (4 functions)
22. **cmd_onboard()** (line 1885)
    - Help: lines 1886-1949
    - Status: REVIEW NEEDED

23. **cmd_hooks()** (line 1951)
    - Help: lines 1952-1970
    - Status: REVIEW NEEDED

24. **cmd_agent()** (line 1972)
    - Long help: lines 1973-2063
    - Status: REVIEW NEEDED

25. **cmd_essentials()** (line 1809)
    - Help: lines 1810-1865
    - Status: REVIEW NEEDED

### Root Command (CRITICAL)
26. **build_cli()** (line 2068)
    - Version: line 2070 uses `env!("CARGO_PKG_VERSION")`
    - About: line 2072 - "ZJJ - Manage JJ workspaces with Zellij sessions"
    - Long help: lines 2073-2150+
    - Status: CRITICAL - Root command description and examples

---

## Search Patterns

### Pattern 1: Old Binary Name
```bash
grep -n "jjz " /home/lewis/src/zjj/crates/zjj/src/cli/args.rs
# Should find: 0 results (if already updated)
# Example: "jjz init", "jjz add", etc.
```

### Pattern 2: Old Directory
```bash
grep -n '\.jjz' /home/lewis/src/zjj/crates/zjj/src/cli/args.rs
# Should find: Examples that still reference .jjz/
# Expected to be replaced with .zjj/
```

### Pattern 3: Tab Prefix References
```bash
grep -n 'jjz:' /home/lewis/src/zjj/crates/zjj/src/cli/args.rs
# May find: Tab naming examples
# Note: Should mention "zjj:" with note about configurable prefix
```

### Pattern 4: Version References
```bash
grep -n '0\.1\.0' /home/lewis/src/zjj/crates/zjj/src/cli/args.rs
# Should find: 0 results (version is dynamic via env macro)
```

---

## Update Checklist

### Critical Updates
- [ ] Root command `.about()` reflects "ZJJ" branding
- [ ] All examples use `zjj` command (not `jjz`)
- [ ] All paths reference `.zjj/` (not `.jjz/`)
- [ ] Tab naming examples reference `zjj:` prefix
- [ ] All `WORKFLOW POSITION` sections reference correct binary name

### Per-Command Verification
- [ ] cmd_init: init sequence examples
- [ ] cmd_add: session creation examples
- [ ] cmd_add_batch: batch operations examples
- [ ] cmd_list: output description
- [ ] cmd_remove: cleanup examples
- [ ] cmd_focus: tab switching description
- [ ] cmd_status: status display examples
- [ ] cmd_sync: rebase workflow examples
- [ ] cmd_diff: diff command examples
- [ ] cmd_config: configuration file path (.zjj/config.toml)
- [ ] cmd_dashboard: UI description
- [ ] cmd_context: environment context examples
- [ ] cmd_introspect: introspection command examples
- [ ] cmd_prime: prime command examples
- [ ] cmd_query: query system examples
- [ ] cmd_backup: backup examples
- [ ] cmd_restore: restore examples
- [ ] cmd_verify_backup: verification examples
- [ ] cmd_completions: shell completion examples
- [ ] cmd_version: version display format
- [ ] cmd_onboard: onboarding flow examples
- [ ] cmd_hooks: hooks management examples
- [ ] cmd_agent: agent listing examples
- [ ] cmd_essentials: essentials command examples
- [ ] build_cli: root command examples

---

## Testing Requirements

### Automated Tests
These tests verify help text integrity:

1. **test_help_flag** (test_cli_parsing.rs:13)
   - Validates: Help flag functionality

2. **test_init_help** (test_cli_parsing.rs:36)
   - Validates: `zjj init --help` output

3. **test_add_help** (test_cli_parsing.rs:47)
   - Validates: `zjj add --help` output

4. **test_all_commands_have_help** (p0_standardization_suite.rs:388)
   - Validates: Every command has help text

5. **test_help_section_headers_uppercase** (p0_standardization_suite.rs:424)
   - Validates: Section headers are uppercase

6. **test_help_has_examples** (p0_standardization_suite.rs:464)
   - Validates: Help text includes examples

7. **test_help_has_ai_agent_sections** (p0_standardization_suite.rs:491)
   - Validates: AI agent documentation sections

### Manual Verification
```bash
# Test help output
zjj --help
zjj init --help
zjj add --help
zjj list --help

# Test JSON help
zjj --help --json
zjj init --help --json

# Verify no "jjz" references
zjj init --help | grep -i "jjz"  # Should return empty
```

---

## Implementation Strategy

### Phase 1: Audit (30 minutes)
- Search for all outdated references
- Generate list of specific edits needed
- Verify no critical references missed

### Phase 2: Root Command Update (15 minutes)
- Update `build_cli()` function
- Update main command description and examples
- Most impactful change (root entry point)

### Phase 3: Session Lifecycle Commands (45 minutes)
- Update: init, add, add_batch, list, remove, focus, status
- Highest user impact commands

### Phase 4: Workspace & Utility Commands (30 minutes)
- Update: sync, diff, config, backup, restore, verify_backup
- Intermediate frequency commands

### Phase 5: Introspection & AI Commands (30 minutes)
- Update: context, introspect, dashboard, prime, query, agent
- Advanced/programmatic commands

### Phase 6: Testing & Validation (15 minutes)
- Run help text tests
- Manual verification of output
- Verify JSON help format

**Total Estimated Time**: 3-3.5 hours

---

## Validation Checklist

Before marking complete:
- [ ] No "jjz" references in help text (except historical notes)
- [ ] All paths use `.zjj/`
- [ ] All examples use `zjj` command
- [ ] All tests pass (`moon run :test`)
- [ ] Manual verification: `zjj --help` shows correct content
- [ ] JSON help output valid: `zjj --help --json`
- [ ] Commit message references v0.2.0 breaking changes

---

## File References

**Primary File**:
- `/home/lewis/src/zjj/crates/zjj/src/cli/args.rs` (2,180 lines)

**Related Files**:
- `/home/lewis/src/zjj/CHANGELOG.md` - v0.2.0 release notes
- `/home/lewis/src/zjj/crates/zjj/tests/test_cli_parsing.rs` - Help text tests
- `/home/lewis/src/zjj/crates/zjj/tests/p0_standardization_suite.rs` - Help text validation

**Related Codanna Symbols**:
- `build_cli()` - Root command builder
- `cmd_init()`, `cmd_add()`, `cmd_list()`, etc. - Command builders
- `output_help_json()` - JSON help output

---

## Risk Assessment

**Severity**: LOW
- Documentation-only changes
- No logic modifications
- Existing tests validate structure

**Regression Risk**: MINIMAL
- Automated tests catch missing help text
- Format validation prevents corruption
- Examples are cosmetic (don't affect execution)

**Impact**: HIGH (User-facing)
- First impression for new users
- Examples guide workflow understanding
- Help text is critical for adoption

---

## Notes

1. **Version Macro**: The `env!("CARGO_PKG_VERSION")` macro automatically pulls version from Cargo.toml, so no hardcoded version updates needed in help text.

2. **Config Format**: Help text references `.zjj/config.toml` - verify this path is correct in actual implementation.

3. **Tab Prefix**: While "zjj:" is the default, help text should note this is configurable via `.zjj/config.toml` for advanced users.

4. **Consistency**: All commands use the same help text format:
   - `.about()` - One-line description
   - `.long_about()` - Multi-line with WHAT IT DOES, WHEN TO USE, etc.
   - `.after_help()` - EXAMPLES section

5. **AI Features**: Several commands include "AI AGENT" sections documenting JSON output and structured parsing for programmatic use.

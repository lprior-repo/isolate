# Bead Analysis: zjj-ffz9 Triage

This directory contains the complete analysis and triage for bead **zjj-ffz9**: "P0: Update help text for v0.2.0 release"

## Analysis Documents

### 1. **QUICK_REFERENCE.md** (START HERE)
- **Best for**: Implementers who want to get started quickly
- **Contains**: 
  - One-minute overview
  - Search & replace patterns
  - Common issues to watch
  - Testing checklist
  - Commands needing updates (table)

### 2. **ANALYSIS.md** (COMPREHENSIVE)
- **Best for**: Understanding full scope and context
- **Contains**:
  - Executive summary
  - v0.2.0 breaking changes explained
  - All 26 help text locations listed
  - Search patterns and grep commands
  - Implementation strategy (6 phases)
  - Complete testing requirements
  - Validation checklist
  - Risk assessment

### 3. **triage.json** (MACHINE-READABLE)
- **Best for**: Programmatic analysis and tracking
- **Contains**:
  - Bead metadata
  - Overview and complexity assessment
  - Complete list of 25+ command builders
  - v0.2.0 key changes
  - Implementation phases
  - Search results summary
  - Time estimates and effort breakdown

## Quick Facts

| Aspect | Details |
|--------|---------|
| **Bead ID** | zjj-ffz9 |
| **Title** | P0: Update help text for v0.2.0 release |
| **Status** | OPEN |
| **Priority** | P0 (Critical) |
| **Scope** | Single file: `crates/zjj/src/cli/args.rs` (2,180 lines) |
| **Complexity** | MEDIUM - Systematic updates across 25+ command builders |
| **Risk Level** | LOW - Documentation only, no logic changes |
| **Estimated Time** | 3-3.5 hours |
| **Test Coverage** | HIGH - Automated tests verify help text structure |

## Primary Change

The project was renamed:
- **Binary**: `jjz` → `zjj`
- **Directory**: `.jjz/` → `.zjj/`
- **Tab Prefix**: `jjz:` → `zjj:`
- **All help text must reflect these changes**

## Implementation Path

1. Read **QUICK_REFERENCE.md** (5 min)
2. Search for patterns using grep commands (15 min)
3. Update command builders (2.5 hours)
   - Root command first (build_cli)
   - Then core commands
   - Then utilities
4. Run tests (15 min)
5. Manual verification (15 min)

## File Structure

```
.tdd15-cache/zjj-ffz9/
├── README.md                 # This file
├── QUICK_REFERENCE.md        # Start here for implementation
├── ANALYSIS.md              # Complete detailed analysis
├── triage.json              # Machine-readable metadata
├── bead.json                # Original bead data
└── progress.json            # Implementation progress tracking
```

## Key Statistics

- **Command builders to update**: 26 (25 commands + root)
- **Help text sections**: ~100+ `.about()` and `.long_about()` calls
- **Lines of help text**: ~1,000+ lines across the file
- **Test coverage**: 7 dedicated help text validation tests
- **Most critical**: `build_cli()` - root command (most visible)

## Success Metrics

Before marking complete, verify:
- [ ] No "jjz" in help text (except bead IDs like `jjz-XXXX`)
- [ ] All paths use `.zjj/`
- [ ] All examples use `zjj` command
- [ ] All tests pass: `moon run :test`
- [ ] Manual check: `zjj --help | grep jjz` returns nothing
- [ ] JSON output valid: `zjj --help --json | jq .`

## Next Steps

1. **For Quick Start**: Open `QUICK_REFERENCE.md`
2. **For Full Context**: Read `ANALYSIS.md`
3. **For Programmatic Use**: Parse `triage.json`
4. **For Code**: Edit `/home/lewis/src/zjj/crates/zjj/src/cli/args.rs`

---

Generated: 2026-01-18
Analysis Tool: Codanna (semantic code search)

# Rust Contract Specification: Fix --include-files Flag (zjj-xcso)

**Generated**: 2026-02-07 21:01:00 UTC
**Bead**: zjj-xcso
**Title**: export: Fix misleading --include-files flag
**Issue Type**: Bug fix

---

## Problem Statement

The `--include-files` flag on the `zjj export` command is misleading:

**Current Behavior**:
- Help text claims: "Include workspace files in export (creates tarball)"
- Flag is accepted but does nothing (marked `#[allow(dead_code)]`)
- Export always creates JSON file, never tarball
- User impact: Misleading behavior, poor UX

**Expected Behavior** (Contract Decision):

Two possible approaches:

### Option A: Implement the Flag (RECOMMENDED)
Actually implement tarball creation with workspace files.

**Pros**:
- Matches documented behavior
- Useful feature for backup/transfer
- No breaking changes

**Cons**:
- Implementation complexity
- File size considerations
- Performance concerns for large workspaces

### Option B: Remove the Flag
Remove flag and update help text to reflect actual behavior.

**Pros**:
- Simple, immediate fix
- No confusion
- No performance concerns

**Cons**:
- Feature removal
- Breaking change for users expecting it

**CONTRACT DECISION**: **Option B - Remove the flag**

**Rationale**:
1. Session metadata export is sufficient for most use cases
2. Workspace files are typically in git/jj anyway
3. Tarball duplication is unnecessary complexity
4. Fast, lean export aligns with zjj philosophy
5. If needed, implement later as separate feature with proper design

---

## Module Structure

**File**: `crates/zjj/src/commands/export_import.rs`

**Changes Required**:
1. Remove `include_files` field from `ExportOptions` struct
2. Remove `#[allow(dead_code)]` attribute
3. Update CLI help text to remove tarball mention
4. Verify all tests pass after removal

---

## Public API (No Changes)

The export command API remains **unchanged** - this is purely internal cleanup.

```rust
// NO CHANGES to public API
// This is purely removing a dead, misleading field

pub async fn run_export(options: &ExportOptions) -> Result<()> {
    // Implementation unchanged - include_files was never used
}
```

---

## Type Changes

### Before (MISLEADING):
```rust
/// Options for the export command
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub session: Option<String>,
    pub output: Option<String>,
    /// Include workspace files
    #[allow(dead_code)]  // ⚠️ NEVER IMPLEMENTED
    pub include_files: bool,  // ⚠️ MISLEADING
    pub format: OutputFormat,
}
```

### After (HONEST):
```rust
/// Options for the export command
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Session to export (or all if None)
    pub session: Option<String>,

    /// Output file path (stdout if None)
    pub output: Option<String>,

    /// Output format
    pub format: OutputFormat,

    // REMOVED: include_files field (was dead code, misleading)
}
```

---

## CLI Changes

### Before (MISLEADING):
```rust
// crates/zjj/src/cli/commands.rs:2295-2298
.arg(
    Arg::new("include-files")
        .long("include-files")
        .action(clap::ArgAction::SetTrue)
        .help("Include workspace files in export (creates tarball)"),  // ⚠️ LIE
)
```

### After (HONEST):
```rust
// REMOVED: The entire --include-files argument
// Export is metadata-only (session state, not workspace files)
```

### Updated Examples:
```rust
.after_help(after_help_text(
    &[
        "zjj export feature-x -o state.json  Export session to file",
        "zjj export --json                  Export all sessions as JSON",
        "zjj export                         Export to stdout",
    ],
    None,
))
```

---

## Error Types (No Changes)

No new error types. This is a removal of dead code.

---

## Performance Constraints

- Export must complete in <100ms for 100 sessions
- Export must not read workspace files (metadata only)
- Export output size: <1KB per session typical

---

## Testing Requirements

### Unit Tests Required:

1. **Verify flag removed**:
   ```rust
   #[test]
   fn export_options_no_include_files_field() {
       // ExportOptions struct should NOT have include_files field
       // This is a compile-time verification
   }
   ```

2. **Verify CLI doesn't accept flag**:
   ```rust
   #[test]
   fn cli_rejects_include_files_flag() {
       // `zjj export --include-files` should error:
       // "error: unexpected argument '--include-files' found"
   }
   ```

3. **Export still works**:
   ```rust
   #[tokio::test]
   async fn export_without_include_files_still_works() {
       // Export should work exactly as before (JSON-only)
   }
   ```

### Integration Tests Required:

1. **Help text verification**:
   ```bash
   zjj export --help | grep -q "include-files"
   # Should NOT find this text
   ```

2. **Export creates JSON, not tarball**:
   ```bash
   zjj export -o /tmp/test.json
   file /tmp/test.json  # Should be "JSON data"
   # Should NOT be "tar archive"
   ```

---

## Migration Guide (For Users)

### Breaking Change Notification:

**If you were using `--include-files`**:
- The flag never worked anyway (dead code)
- Your exports were always JSON-only
- No action needed - remove the flag from scripts

**Example Migration**:
```bash
# Before (did nothing, but didn't error)
zjj export --include-files -o backup.tar

# After (honest, explicit)
zjj export -o backup.json

# If you need workspace files too:
# 1. Export session metadata
zjj export -o sessions.json
# 2. Export workspace files via git/jj
jj git export
# or: tar -czf workspace.tar.gz .jj/
```

---

## Implementation Checklist

- [ ] Remove `include_files: bool` field from `ExportOptions`
- [ ] Remove `#[allow(dead_code)]` attribute
- [ ] Remove `--include-files` argument from CLI definition
- [ ] Update help text examples
- [ ] Verify all existing tests pass
- [ ] Add tests verifying flag doesn't exist
- [ ] Update documentation (if tarball mentioned elsewhere)
- [ ] Run `moon run :quick` (6-7ms)
- [ ] Run `moon run :ci` (full pipeline)
- [ ] Update CHANGELOG.md

---

## Zero Unwrap/Expect/Panic Requirements

**CRITICAL**: This is Rust code. Follow **Rule 4** of CLAUDE.md:

```rust
// ❌ FORBIDDEN
let session = db.get(name).await.unwrap();
let count = sessions.len().expect("non-empty");

// ✅ REQUIRED
use zjj_core::Result;
let session = db.get(name).await?
    .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;
```

**However**: This contract is about **removing** code, not adding it. No new unwrap/expect patterns introduced.

---

## Success Criteria

1. `--include-files` flag completely removed
2. Help text no longer mentions tarball
3. All existing tests pass
4. New tests verify flag rejection
5. No breaking changes to working functionality
6. Export behavior unchanged (still creates JSON)

---

## Future Enhancements (Out of Scope)

If tarball export is truly needed, implement as **separate feature**:

```bash
zjj export-backup --include-workspace -o backup.tar.gz
```

**Design for future**:
- Separate command (not flag on `export`)
- Explicit about what it includes
- Compression options
- Size estimation
- Progress reporting
- Incremental backup support

---

**Contract Status**: ✅ Ready for Builder

**Estimated Implementation Time**: 15 minutes (simple removal)

**Risk Level**: Low (removing dead code, well-tested)

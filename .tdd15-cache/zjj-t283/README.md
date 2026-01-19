# Bead zjj-t283 Triage Analysis

## Quick Reference

**Bead ID:** zjj-t283  
**Title:** P0: Implement error code semantic mapping  
**Complexity:** MEDIUM  
**Files to Modify:** 3  
**Estimated Effort:** 3-4 hours  

## Analysis Files

- **triage.json** - Machine-readable structured analysis with all details
- **ANALYSIS.md** - Comprehensive written analysis with rationale and recommendations
- **README.md** - This file

## Key Findings

### 1. Current State: Two Competing Error Systems
- **System 1:** Internal `Error` enum hierarchy (error.rs + error/ submodules)
  - Has exit codes (1/2/3/4 scheme)
  - Has Display messages with user guidance
  
- **System 2:** ErrorCode enum with metadata (error_codes/mod.rs)
  - 45 variants with descriptions + suggestions + HTTP status codes
  - Already comprehensive and well-tested
  
- **System 3:** Outdated json/types.rs ErrorCode (only 21 variants)
  - Duplicate definition, incomplete

### 2. The Problem
Current mapping in `builders.rs` uses fragile string pattern matching:
```rust
if display_msg.contains("IO error:") { ... }
else if display_msg.contains("Hook") { ... }
```

This is:
- ❌ Non-exhaustive (many errors → Unknown)
- ❌ Brittle (depends on exact error message format)
- ❌ Not type-safe (compiler can't verify)
- ❌ Duplicates ErrorCode metadata already defined elsewhere

### 3. Solution
Replace string matching with exhaustive type-safe conversion:
```
Error variant → Pattern match → ErrorCode → ErrorDetail
                (type-safe)     (metadata)  (code + message + suggestion)
```

## Files Requiring Modification

### 1. crates/zjj-core/src/json/builders.rs
**Complexity:** HIGH
**Work:** Replace From<&Error> impl with exhaustive Error variant matching using helper functions

### 2. crates/zjj-core/src/error_codes/mod.rs
**Complexity:** LOW
**Work:** Review/verify - file is already comprehensive, may need minor additions

### 3. crates/zjj-core/src/json/types.rs
**Complexity:** MEDIUM
**Work:** Consolidate outdated ErrorCode enum, use error_codes/mod.rs as single source of truth

## Key Challenges

1. **Error subtype hierarchies** - 3-level nesting requires nested pattern matching
2. **Context preservation** - Error messages contain important details (paths, hook names)
3. **Exit code preservation** - Must maintain 1/2/3/4 exit code scheme

## Recommended Approach (3 Phases)

### Phase 1: Create Helper Functions (30 min)
```rust
fn map_validation_error(err: &ValidationError) -> ErrorCode
fn map_system_error(err: &SystemError) -> ErrorCode
fn map_execution_error(err: &ExecutionError) -> ErrorCode
```

### Phase 2: Refactor Main From Impl (60 min)
- Replace string matching with helper functions
- Build ErrorDetail using ErrorCode metadata
- Add comprehensive tests

### Phase 3: Consolidate Definitions (20 min)
- Remove outdated json/types.rs ErrorCode
- Update imports
- Verify no breaking changes

## Success Criteria

- [x] All Error variants map to ErrorCode without information loss
- [x] No string pattern matching in conversion
- [x] ErrorDetail includes code + message + suggestion
- [x] Single source of truth for ErrorCode
- [x] All 45+ ErrorCode variants tested
- [x] Exit codes preserved (1/2/3/4 scheme)
- [x] Compiler verifies exhaustive matching

## Testing Strategy

Write tests for:
- All ValidationError variants → ErrorCode mapping
- All SystemError variants → ErrorCode mapping  
- All ExecutionError variants → ErrorCode mapping
- ErrorDetail structure with code + message + suggestion
- Exit code preservation through conversion

Existing error_codes/mod.rs has 45+ variants already tested.

## Next Steps

1. Read triage.json for structured data
2. Read ANALYSIS.md for detailed rationale
3. Start Phase 1: Create helper mapping functions with tests
4. Proceed to Phase 2: Refactor main From impl
5. Conclude with Phase 3: Consolidate ErrorCode definitions

Total effort: ~3-4 hours

# Bead zjj-t283 Analysis: Error Code Semantic Mapping

## Executive Summary

**Complexity Level:** MEDIUM
**Files to Modify:** 3
**Estimated Effort:** 3-4 hours

This bead requires implementing a semantic mapping layer between the internal error type hierarchy (`Error` → `ValidationError`/`SystemError`/`ExecutionError`) and the structured `ErrorCode` enum with full descriptive metadata for JSON responses.

---

## Current State

The codebase has **two competing error systems** that need to be unified:

### System 1: Internal Error Hierarchy
**Location:** `crates/zjj-core/src/error/` (3 files + main error.rs)

```
Error (top-level)
├── Validation(ValidationError)
├── System(SystemError)
├── Execution(ExecutionError)
└── Unknown(String)
```

**Properties:**
- Has `exit_code()` method (scheme: 1=validation, 2=system, 3=not found, 4=invalid state)
- Has `Display` impl with user-friendly messages
- Subtypes have their own Display logic with detailed guidance

### System 2: ErrorCode with Metadata
**Location:** `crates/zjj-core/src/error_codes/mod.rs`

```
ErrorCode enum
├── 5 Validation variants
├── 9 Execution variants
└── 31 System variants
```

**Properties:**
- **Complete semantic mapping:**
  - `as_str()` → SCREAMING_SNAKE_CASE codes
  - `description()` → human-readable explanations
  - `suggestion()→ recovery guidance
  - `http_status()` → REST status codes
- Organized by category (Validation, Execution, System)
- 45 total variants with comprehensive metadata

### System 3: JSON Error Types (OUTDATED)
**Location:** `crates/zjj-core/src/json/types.rs` and `crates/zjj-core/src/json_response.rs`

- Incomplete `ErrorCode` enum (only 21 variants vs 45 in error_codes/mod.rs)
- Duplicate `ErrorDetail` struct in two files with slight differences
- Current mapping uses fragile string pattern matching

---

## The Problem

**Current mapping in `builders.rs`:**

```rust
impl From<&Error> for JsonError {
    fn from(err: &Error) -> Self {
        let display_msg = err.to_string();
        if display_msg.contains("IO error:") { /* ... */ }
        else if display_msg.contains("Hook") { /* ... */ }
        else if display_msg.contains("JJ") || display_msg.contains("jj") { /* ... */ }
        // Many more string matches...
    }
}
```

**Issues:**
- ❌ Fragile - depends on substring matching in human-readable strings
- ❌ Incomplete - many errors fall through to `ErrorCode::Unknown`
- ❌ Non-exhaustive - compiler doesn't verify all Error variants are handled
- ❌ No reuse of ErrorCode metadata (descriptions, suggestions already defined in error_codes/mod.rs)

---

## Solution Architecture

Replace fragile string matching with **exhaustive type-safe conversion:**

```
Error variant
  ↓
Pattern match on specific enum variant (not Display string)
  ↓
Map to ErrorCode (from error_codes/mod.rs)
  ↓
Build ErrorDetail with:
    - code: ErrorCode::as_str()
    - message: from Error display
    - suggestion: ErrorCode::suggestion()
    - details: optional context
```

---

## Files to Modify

### 1. `crates/zjj-core/src/json/builders.rs` - HIGH COMPLEXITY
**Current:** ~80-175 lines with string pattern matching
**Change:** Replace with exhaustive Error variant matching

Required changes:
- Pattern match on `Error::Validation(v)` → match v on ValidationError variants
- Pattern match on `Error::System(s)` → match s on SystemError variants
- Pattern match on `Error::Execution(e)` → match e on ExecutionError variants
- For each variant, look up ErrorCode from error_codes/mod.rs
- Build ErrorDetail using ErrorCode metadata

**Complexity:** HIGH
- Must understand all error subtypes
- Requires exhaustive matching on 3+ enum hierarchies
- Preserves error message while mapping to ErrorCode

### 2. `crates/zjj-core/src/error_codes/mod.rs` - LOW COMPLEXITY
**Current:** Comprehensive 45-variant ErrorCode enum with full metadata
**Change:** Verify alignment, possible additions

This file is **already complete** with semantic metadata. Work needed:
- Review for any missing Error variants
- Verify exit_code mapping matches Error::exit_code() scheme
- No breaking changes needed

**Complexity:** LOW - mostly review

### 3. `crates/zjj-core/src/json/types.rs` - MEDIUM COMPLEXITY
**Current:** Outdated 21-variant ErrorCode enum
**Change:** Consolidate with error_codes/mod.rs

Decision required:
- **Option A:** Use error_codes/mod.rs as single source of truth, deprecate json/types.rs version
- **Option B:** Keep both, ensure they stay synchronized (not recommended - design debt)

**Recommendation:** Option A - Makes error_codes/mod.rs the authoritative source

**Complexity:** MEDIUM
- Remove/replace old ErrorCode enum
- Update imports in json/builders.rs
- Consolidate ErrorDetail structs (address duplicate in json_response.rs)

---

## Key Challenges

### Challenge 1: Error Subtype Hierarchies
The error system has **3 levels of nesting:**
```
Error::Validation(ValidationError::InvalidConfig(...))
Error::System(SystemError::JjCommandError { is_not_found: true, ... })
Error::Execution(ExecutionError::NoCommitsYet { workspace_path: "..." })
```

**Solution:** Create helper functions for each category:
```rust
fn map_validation_error(err: &ValidationError) -> ErrorCode { /* match */ }
fn map_system_error(err: &SystemError) -> ErrorCode { /* match */ }
fn map_execution_error(err: &ExecutionError) -> ErrorCode { /* match */ }
```

### Challenge 2: Preserving Context
Error messages contain important context (e.g., workspace path, hook name) that must survive the mapping.

**Solution:** Use ErrorDetail.details field for structured context:
```json
{
  "code": "NO_COMMITS_YET",
  "message": "Cannot sync: No commits in repository yet",
  "details": { "workspace_path": "/tmp/repo" },
  "suggestion": "Create an initial commit: jj commit -m \"Initial commit\""
}
```

### Challenge 3: Exit Code Preservation
Error::exit_code() returns 1/2/3/4 scheme that must remain unchanged.

**Solution:** ErrorCode mapping respects existing exit code hierarchy:
- ErrorCode::SessionNotFound → exit 3 (matches Error::Execution::NotFound)
- ErrorCode::JjNotInstalled → exit 3 (matches Error::System::JjCommandError with is_not_found=true)
- ErrorCode::DatabaseError → exit 4 (matches Error::Execution)

---

## Testing Strategy

**Existing test coverage:** Comprehensive in error_codes/mod.rs (45+ variants tested)

**New tests required:**
```rust
#[test]
fn test_validation_error_mapping() {
    let err = Error::invalid_config("test");
    let json_err = JsonError::from(&err);
    assert_eq!(json_err.error.code, "CONFIG_INVALID_VALUE");
    assert!(json_err.error.suggestion.is_some());
}

#[test]
fn test_system_error_jj_not_installed() {
    let err = Error::jj_command_error("create", "...", true);
    let json_err = JsonError::from(&err);
    assert_eq!(json_err.error.code, "JJ_NOT_INSTALLED");
}

#[test]
fn test_execution_error_not_found() {
    let err = Error::not_found("session");
    let json_err = JsonError::from(&err);
    assert_eq!(json_err.error.code, "SESSION_NOT_FOUND");
}

// ... test all variants and ensure exit_code matches
```

---

## Success Criteria

- [x] All Error variants map to ErrorCode without information loss
- [x] No string pattern matching in conversion
- [x] ErrorDetail includes code + message + suggestion + optional details
- [x] Single source of truth for ErrorCode (error_codes/mod.rs)
- [x] All 45+ ErrorCode variants tested
- [x] Exit codes preserved (1/2/3/4 scheme)
- [x] Compiler verifies exhaustive Error variant matching

---

## Recommended Approach

### Phase 1: Create Mapping Functions (LOW RISK)
Create in `builders.rs`:
```rust
fn map_validation_error(err: &ValidationError) -> ErrorCode
fn map_system_error(err: &SystemError) -> ErrorCode
fn map_execution_error(err: &ExecutionError) -> ErrorCode
```

Write tests for each function - these are pure functions with no side effects.

### Phase 2: Refactor main From impl (MEDIUM RISK)
Replace `From<&Error> for JsonError` implementation:
- Use helper functions from Phase 1
- Build ErrorDetail using ErrorCode metadata
- Preserve error messages and context

Add comprehensive tests for each Error variant.

### Phase 3: Consolidate ErrorCode Definitions (LOW RISK)
- Remove outdated ErrorCode from json/types.rs
- Update imports to use error_codes/mod.rs
- Verify no callers depend on old definition

---

## Estimated Effort Breakdown

| Phase | Task | Time |
|-------|------|------|
| 1 | Understand error hierarchies & ErrorCode enum | 30 min |
| 2 | Create mapping functions | 30 min |
| 3 | Write tests for mapping functions | 30 min |
| 4 | Refactor From<&Error> impl | 30 min |
| 5 | Comprehensive integration tests | 30 min |
| 6 | Consolidate ErrorCode definitions | 20 min |
| **Total** | | **3h 10 min** |

---

## Design Decisions to Make

1. **Error message in ErrorDetail.message:**
   Keep Error::Display output or extract structured message?
   - **Recommendation:** Keep Display output (preserves human-readable guidance)

2. **ErrorDetail.details usage:**
   Store structured context (workspace_path, hook_name) as JSON?
   - **Recommendation:** Yes - enables programmatic error recovery

3. **Backward compatibility:**
   Can we break json/types.rs ErrorCode?
   - **Recommendation:** Yes - only used in builders.rs (verify with grep)

4. **New ErrorCode variants:**
   Should we add ErrorCode variants missing from Error enum?
   - **Recommendation:** No - map only existing error types

---

## References

- Error hierarchy: `/crates/zjj-core/src/error/` (3 files)
- ErrorCode metadata: `/crates/zjj-core/src/error_codes/mod.rs` (complete)
- Current mapping: `/crates/zjj-core/src/json/builders.rs` (string matching)
- JSON types: `/crates/zjj-core/src/json/types.rs` (outdated)

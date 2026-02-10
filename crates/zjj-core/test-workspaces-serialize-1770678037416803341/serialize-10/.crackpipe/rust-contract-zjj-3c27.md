# Contract Specification: Fix done/types.rs E0063 - Missing Field session_updated

```jsonl
{"kind":"contract","id":"zjj-3c27","status":"draft","created":"2026-02-08"}
{"kind":"issue","type":"compile_error","code":"E0063","location":"crates/zjj/src/commands/done/types.rs:340"}
{"kind":"scope","fix":"single_field_initialization","files":["crates/zjj/src/commands/done/types.rs"]}
{"kind":"pattern","error":"missing_field","struct":"DoneOutput","field":"session_updated"}
```

## Context

- **Feature**: Fix compilation error in test for `DoneOutput` struct initialization
- **Domain terms**:
  - `DoneOutput`: Result struct containing operation metadata (workspace name, bead ID, commit counts, status flags)
  - `Default` trait: Provides default values for all fields in a struct
  - Struct update syntax: `..Default::default()` pattern for filling unspecified fields
- **Assumptions**:
  - The test at line 340 is meant to verify `DoneOutput` field values
  - `DoneOutput` derives `Default`, so all fields have sensible defaults
  - The `session_updated` field was added after the test was written
  - Test intentionally sets specific fields and relies on defaults for others
- **Open questions**: None

## Preconditions

- [ ] `DoneOutput` struct must derive `Default` trait
- [ ] All fields of `DoneOutput` must implement `Default`
- [ ] Test file `types.rs` must compile without E0063 errors

## Postconditions

- [ ] Test function `test_done_output_serialization` compiles successfully
- [ ] `DoneOutput` instance is created with all required fields specified
- [ ] Test assertions verify expected field values
- [ ] No E0063 missing field errors in test module

## Invariants

- [ ] `DoneOutput` struct literal initializations must either:
  - Specify all fields explicitly, OR
  - Use struct update syntax `..Default::default()` for unspecified fields
- [ ] Tests must not rely on partial field specification without update syntax

## Error Taxonomy

**Note**: This fix addresses a compile-time error, not runtime errors. The error types below are for documentation purposes only.**

- `CompileError::E0063MissingField { struct_name, field_name }` - when struct literal does not specify all non-defaulted fields
- `CompileError::MissingStructUpdateSyntax` - when partial field specification is used without `..Default::default()` pattern

## Contract Signatures

```rust
// Test function (no signature change - fix implementation only)
#[test]
fn test_done_output_serialization() {
    // Before (FAILS - missing session_updated):
    // let output = DoneOutput {
    //     workspace_name: "test-ws".to_string(),
    //     bead_id: Some("zjj-test".to_string()),
    //     files_committed: 2,
    //     commits_merged: 1,
    //     merged: true,
    //     cleaned: true,
    //     bead_closed: true,
    //     // session_updated: true,  // MISSING - causes E0063
    //     pushed_to_remote: false,
    //     dry_run: false,
    //     preview: None,
    //     error: None,
    // };

    // After (PASSES - all fields specified with update syntax):
    let output = DoneOutput {
        workspace_name: "test-ws".to_string(),
        bead_id: Some("zjj-test".to_string()),
        files_committed: 2,
        commits_merged: 1,
        merged: true,
        cleaned: true,
        bead_closed: true,
        session_updated: true,  // ADDED
        pushed_to_remote: false,
        dry_run: false,
        preview: None,
        error: None,
    };

    // OR using update syntax (if only testing specific fields):
    // let output = DoneOutput {
    //     workspace_name: "test-ws".to_string(),
    //     session_updated: true,
    //     ..Default::default()
    // };

    assert_eq!(output.workspace_name, "test-ws");
    assert!(output.merged);
    assert!(output.cleaned);
}
```

## Non-goals

- [ ] No changes to `DoneOutput` struct definition
- [ ] No changes to production code logic
- [ ] No new test cases (only fix existing test)
- [ ] No changes to serialization behavior
- [ ] No changes to other tests in the module

## Implementation Approach

**Option 1: Explicit field specification (RECOMMENDED)**
- Add `session_updated: true,` to the existing struct literal
- Maintains test clarity - all fields visible
- Consistent with existing test style

**Option 2: Struct update syntax**
- Keep explicit fields, add `..Default::default()` at end
- Less verbose but hides default values
- Better if test only cares about specific fields

**Recommendation**: Use Option 1 (explicit specification) since the test already explicitly sets most fields and the pattern is consistent with the test's intent to verify specific values.

## Verification Checklist

- [ ] Test compiles without errors
- [ ] `moon run :quick` passes (format + lint check)
- [ ] `moon run :test` passes for `types.rs` module
- [ ] No new clippy warnings introduced
- [ ] Test still verifies expected field values

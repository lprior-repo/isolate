# Plan: fix SURVIVOR-002 path parser bug for files named "a -> b.txt"

**Date:** 2026-02-21
**Source:** `.agents/research/2026-02-21-survivor-002-path-parser-bug.md`

## Context
The JJ diff summary parser in the conflict detector is responsible for extracting file paths from `jj diff --summary` output. Files containing the literal substring " -> " can be misinterpreted as renames if parsing is too permissive. We need to ensure filenames like "a -> b.txt" are preserved unless the status is an explicit rename (`R`).

## Files to Modify

| File | Change |
| --- | --- |
| `crates/zjj/src/commands/done/conflict.rs` | Add regression coverage for filenames containing " -> " with non-rename status. |

## Boundaries

**Always:** Preserve current rename parsing for status `R`; avoid changing CLI output formats.
**Ask First:** Any change to JJ diff parsing beyond the conflict detector.
**Never:** Alter lint config or introduce unwrap/expect/panic.

## Baseline Audit

| Metric | Command | Result |
| --- | --- | --- |
| parse_diff_summary references | `rg -n "parse_diff_summary" crates/zjj/src/commands/done/conflict.rs | wc -l` | 12 |
| diff summary tests | `rg -n "test_parse_diff_summary" crates/zjj/src/commands/done/conflict.rs | wc -l` | 4 |

## Implementation

### 1. Add regression test for literal " -> " filenames

In `crates/zjj/src/commands/done/conflict.rs`:

- **Add test** near existing diff parsing tests:
  - `fn test_parse_diff_summary_with_arrow_filename()`
  - Use output line: `"M a -> b.txt"` and assert the set contains `"a -> b.txt"`.

## Tests

**`crates/zjj/src/commands/done/conflict.rs`** — add:
- `test_parse_diff_summary_with_arrow_filename`: Non-rename entries with " -> " keep the full filename.

## Conformance Checks

| Issue | Check Type | Check |
| --- | --- | --- |
| Issue 1 | content_check | `{file: "crates/zjj/src/commands/done/conflict.rs", pattern: "test_parse_diff_summary_with_arrow_filename"}` |
| Issue 1 | content_check | `{file: "crates/zjj/src/commands/done/conflict.rs", pattern: "M a -> b.txt"}` |

## Verification

1. **Unit tests**: `moon run :test -- crates/zjj/src/commands/done/conflict.rs`
2. **Targeted test**: `moon run :test -- -p zjj test_parse_diff_summary_with_arrow_filename`

## Issues

### Issue 1: Add regression test for arrow filenames
**Dependencies:** None
**Acceptance:** New test exists and ensures "M a -> b.txt" preserves the full filename.
**Description:** Add `test_parse_diff_summary_with_arrow_filename` in `crates/zjj/src/commands/done/conflict.rs` alongside existing diff summary tests, using the current parser behavior to validate the edge case.

## Execution Order

**Wave 1** (parallel): Issue 1

## Next Steps
- Run `/pre-mortem` to validate the plan
- Then `/crank` to implement

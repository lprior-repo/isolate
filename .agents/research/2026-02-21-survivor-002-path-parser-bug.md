# Research: SURVIVOR-002 path parser bug for files named "a -> b.txt"

**Date:** 2026-02-21
**Backend:** inline
**Scope:** JJ diff summary parsing and conflict detection path extraction.

## Summary
The JJ diff summary parser lives in the conflict detector and already includes logic to only treat " -> " as a rename when the status is "R". The current parser splits the diff summary line into status and file part, then only applies rename parsing for status "R"; other statuses return the full file part unchanged.

## Key Files
| File | Purpose |
| --- | --- |
| crates/zjj/src/commands/done/conflict.rs | JJ diff summary parsing and conflict detection logic for overlapping files. |

## Findings
- The diff summary parser splits each line into a status token and the rest of the line, then only applies rename parsing when status is "R" and the file part contains " -> ". Other statuses return the file part as-is, which should preserve filenames containing " -> " for non-rename entries. `crates/zjj/src/commands/done/conflict.rs:359`
- Rename parsing currently uses `split(" -> ")` and keeps the last segment to capture the new name. `crates/zjj/src/commands/done/conflict.rs:381`
- Existing tests cover basic parsing, rename parsing, whitespace handling, and empty input; there is no explicit test for filenames containing " -> " with a non-rename status. `crates/zjj/src/commands/done/conflict.rs:561`

## Recommendations
- Add a regression test that includes a non-rename status with a filename containing " -> " (e.g., "M a -> b.txt") and assert the full filename is preserved.
- If failures reproduce, confirm JJ diff summary format for such filenames and update parsing logic accordingly.

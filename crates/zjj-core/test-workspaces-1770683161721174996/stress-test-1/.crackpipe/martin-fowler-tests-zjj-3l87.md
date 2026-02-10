# Martin Fowler Test Plan: Fix Bookmark Delete (zjj-3l87)

## Test Strategy
Focus on parser robustness and delete command correctness.

## Unit Tests

### `test_delete_existing_bookmark_succeeds`
**Given**: Bookmark "feature-x" exists in JJ
**When**: `delete(&DeleteOptions { name: "feature-x", ... })`
**Then**: Returns Ok(()) and bookmark removed

### `test_delete_nonexistent_bookmark_fails`
**Given**: Bookmark "ghost" does not exist
**When**: `delete(&DeleteOptions { name: "ghost", ... })`
**Then**: Returns Err(BookmarkError::NotFound("ghost"))

### `test_parse_bookmark_list_with_multiline_format`
**Given**: JJ output with multiline bookmarks
```text
main: ntzomurw e553bf6b feat: Implement
  @origin: ntzomurw e553bf6b feat: Implement
```
**When**: `parse_bookmark_list(output)`
**Then**: Returns vec![BookmarkInfo { name: "main", ... }]

### `test_parse_bookmark_list_filters_deleted`
**Given**: JJ output with deleted bookmark
```text
main: abc123
old: xyz789 (deleted)
```
**When**: `parse_bookmark_list(output)`
**Then**: Returns vec![BookmarkInfo { name: "main" }] (only non-deleted)

### `test_parse_bookmark_list_skips_remotes`
**Given**: JJ output with indented remotes
```text
main: abc123
  @origin: abc123
```
**When**: `parse_bookmark_list(output)`
**Then**: Returns vec![BookmarkInfo { name: "main", remote: false }]

## Integration Tests

### `test_full_delete_workflow`
1. Create bookmark via JJ
2. List bookmarks (verify exists)
3. Delete via zjj
4. List bookmarks (verify gone)
5. Attempt delete again (error)

## Edge Cases

### `test_delete_bookmark_with_special_chars`
**Given**: Bookmark name "feature-123_test"
**When**: Validate and delete
**Then**: Succeeds (valid chars)

### `test_parse_empty_list`
**Given**: Empty JJ output
**When**: `parse_bookmark_list(b"")`
**Then**: Returns Ok(vec![])

## Contract Tests

### `test_precondition_bookmark_name_validated`
**Given**: Invalid name "bad name!"
**When**: `delete(&DeleteOptions { name: "bad name!", ... })`
**Then**: Returns Err(BookmarkError::InvalidName)

### `test_postcondition_bookmark_removed`
**Given**: Existing bookmark
**When**: Delete succeeds
**Then**: JJ query confirms bookmark gone

## Test Coverage Goals
- Line: >90%
- Branch: >85%
- All error variants tested

## Files
- Contract: `/tmp/rust-contract-zjj-3l87.md`
- Code: `/home/lewis/src/zjj/crates/zjj/src/commands/bookmark.rs`

## Generated
2026-02-08 by architect-1

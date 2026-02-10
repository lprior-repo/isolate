# Contract Specification: Fix Bookmark Delete (zjj-3l87)

## Problem
The `bookmark delete` command fails, likely due to parsing issues from CRITICAL-007.

## Current Implementation
- `delete()` (line 198) calls `list()` to check if bookmark exists
- `list()` uses `parse_bookmark_list()` (line 426) to parse JJ output
- Delete proceeds only if bookmark found in parsed list

## Root Cause Analysis
The `parse_bookmark_list()` function has known issues (CRITICAL-007) with:
1. Multi-line format handling
2. Remote bookmark filtering
3. Deleted bookmark detection

If parsing fails or returns incomplete results, delete fails with "bookmark not found" even if it exists.

## Preconditions
- Bookmark name is valid (alphanumeric, -, _)
- Session exists (if specified)
- Workspace path exists

## Postconditions
- Bookmark removed from JJ repository
- Bookmark no longer appears in `jj bookmark list` output
- Delete response confirms deletion

## Invariants
- Delete only succeeds if bookmark exists
- Delete idempotent: deleting non-existent bookmark errors
- Parser robustness: handle all JJ output formats

## Error Taxonomy
- `BookmarkError::NotFound` - bookmark doesn't exist
- `BookmarkError::InvalidName` - bookmark name invalid
- `BookmarkError::SessionNotFound` - session doesn't exist
- `BookmarkError::WorkspaceNotFound` - workspace path invalid
- `BookmarkError::JjCommandFailed` - JJ command failed

## Implementation Fix
**Option 1**: Fix `parse_bookmark_list()` (addresses CRITICAL-007)
- Improve multi-line parsing
- Better remote filtering
- Robust deleted detection

**Option 2**: Bypass parser for delete check
- Use direct JJ query before delete
- Still need parser for list command
- Partial fix

**Decision**: Option 1 - Fix the parser (proper solution)

## Test Requirements
1. Delete existing bookmark → succeeds
2. Delete non-existent bookmark → NotFound error
3. Delete with invalid name → InvalidName error
4. Parser handles all JJ formats correctly
5. Parser filters deleted bookmarks
6. Parser skips remote bookmarks

## Files
- `/home/lewis/src/zjj/crates/zjj/src/commands/bookmark.rs`

## Generated
2026-02-08 by architect-1

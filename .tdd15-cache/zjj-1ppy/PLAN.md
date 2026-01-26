# PLAN.md - Add --bead flag to list command

## Overview
Add --bead <BEAD_ID> flag to filter sessions by bead ID stored in metadata.

## Files to Modify
1. `crates/zjj/src/main.rs` - Add flag definition (lines 84-99), update handle_list (lines 408-412)
2. `crates/zjj/src/commands/list.rs` - Add bead parameter, implement filtering
3. `crates/zjj/src/commands/introspect.rs` - Update help metadata (lines 472-515)
4. `crates/zjj/tests/test_cli_parsing.rs` - Add tests (after line 170)

## Key Design
- Flag definition: --bead <BEAD_ID> (no short form)
- Parameter: Option<String> passed to list::run()
- Filtering: Check metadata.bead_id field, exact string match
- Storage: Sessions store bead_id in metadata as {"bead_id": "zjj-xxxx"}
- Filtering: retain() logic that checks metadata.get("bead_id").as_str()

## Test Plan
- test_list_with_bead_flag - Basic filtering
- test_list_with_bead_no_matches - No matching sessions
- test_list_with_bead_excludes_others - Filter excludes other sessions
- test_list_without_bead_shows_all - Backward compatibility
- test_list_with_bead_and_all_flags - Combined flags
- test_list_with_bead_json_output - JSON output works

## Implementation Order
1. Add flag to cmd_list()
2. Update handle_list() parameter extraction
3. Update list::run() signature and filtering logic
4. Update introspection metadata
5. Add comprehensive tests

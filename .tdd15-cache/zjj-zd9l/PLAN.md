# PLAN.md - Add --agent flag to list command

## Overview
Add --agent <NAME> flag to filter sessions by agent owner stored in metadata.

## Files to Modify
1. `crates/zjj/src/main.rs` - Add flag definition (lines 84-99), update handle_list (lines 408-412)
2. `crates/zjj/src/commands/list.rs` - Add agent parameter, implement filtering
3. `crates/zjj/src/commands/introspect.rs` - Update help metadata (lines 472-515)
4. `crates/zjj/tests/test_cli_parsing.rs` - Add flag combination tests
5. `crates/zjj/tests/test_session_lifecycle.rs` - Add filtering logic tests

## Key Design
- Flag definition: --agent <NAME> (string value, no short form)
- Parameter: Option<&str> passed to list::run()
- Filtering: Check metadata.owner field, exact string match (case-sensitive)
- Storage: Sessions store owner in metadata as {"owner": "agent-name"}
- Filtering: retain() logic with JSON navigation and comparison

## Test Plan
- test_list_with_agent_flag - Basic flag parsing
- test_list_agent_with_no_matches - No matching sessions
- test_list_agent_combined_with_all - Combined with --all flag
- test_list_agent_combined_with_json - Combined with --json flag
- test_list_filters_by_agent_owner - Core filtering logic
- Backward compatibility: Sessions without owner excluded when filtering

## Implementation Order
1. Add flag to cmd_list()
2. Update handle_list() parameter extraction to Option<&str>
3. Update list::run() signature and filtering logic
4. Update introspection metadata with examples
5. Add comprehensive tests for all scenarios

## Edge Cases
- Empty agent name: Treated as valid filter (no sessions match)
- Case-sensitive matching
- Sessions without owner metadata excluded when filtering

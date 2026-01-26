# PLAN.md - Wrap SyncOutput in SchemaEnvelope

## Overview
Wrap all 4 SyncOutput JSON serializations with SchemaEnvelope to comply with protocol.

## Files to Modify
1. `crates/zjj-core/src/json.rs` - SchemaEnvelope (shared with jakw)
2. `crates/zjj/src/commands/sync.rs` - Wrap 4 serialization points (lines 55, 73, 97, 132)
3. `crates/zjj/tests/schema_tests.rs` - Add envelope validation

## Key Points
- Reuse SchemaEnvelope from zjj-jakw
- Wrap lines: 55 (single success), 73 (single failure), 97 (all empty), 132 (all with results)
- schema_type="single" for all (SyncOutput is single object, not array)

## Test Plan
- test_sync_json_has_envelope - Verify structure
- test_sync_schema_type_single - Verify schema format

## Dependency: Requires zjj-jakw complete first

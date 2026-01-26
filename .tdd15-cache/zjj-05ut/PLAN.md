# PLAN.md - SchemaEnvelope wrapper on most JSON outputs

## Overview
Wrap all remaining JSON outputs (8 files, ~20 locations) with SchemaEnvelope.

## Files in Scope
1. focus.rs (2 locations) - Lines 43, 59
2. remove.rs (2 locations) - Lines 57, 101
3. clean.rs (4 locations) - Lines 94, 112, 137, 154
4. list.rs (2 locations) - Lines 59, 182 (array type)
5. query.rs (4 locations) - Lines 220, 264, 330, 349
6. doctor.rs (2 locations) - Lines 342, 452
7. introspect.rs (2 locations) - Lines 141, 235
8. config.rs (3 locations) - Lines 88, 137, 176

## Phase Breakdown
1. Phase 1: Simple single outputs (focus, remove) - 2 tests
2. Phase 2: Array outputs (list) - 2 tests
3. Phase 3: Multiple variants (clean) - 4 tests
4. Phase 4: Query outputs (query) - 4 tests
5. Phase 5: Diagnostic outputs (doctor, introspect) - 4 tests
6. Phase 6: Config outputs (config) - 2 tests

## Key Design
- Reuse SchemaEnvelope from zjj-jakw
- Import in each file via: use zjj_core::json::SchemaEnvelope
- Use SchemaEnvelope::new("command-variant", "single"/"array", output)
- schema_type="array" only for list.rs (SessionListItem vec)
- All others use "single"

## Dependency: Requires zjj-jakw complete first

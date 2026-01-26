# PLAN.md - Wrap StatusOutput in SchemaEnvelope

## Overview
Create a generic SchemaEnvelope<T> wrapper in zjj-core and use it to wrap status command JSON output.

## Files to Modify
1. `crates/zjj-core/src/json.rs` - Add SchemaEnvelope struct
2. `crates/zjj/src/commands/status.rs` - Wrap output with envelope
3. `crates/zjj/tests/` - Add validation tests

## Key Design
- SchemaEnvelope<T> with fields: $schema, _schema_version, schema_type, success, #[serde(flatten)] data
- Schema format: "zjj://status-response/v1"
- schema_type="single" for single-object responses

## Test Plan
- test_status_json_has_envelope - Verify envelope wrapper
- test_status_schema_type_single - Verify schema_type field
- test_status_empty_sessions_wrapped - Edge case handling

## Phase Sequence
1. Create SchemaEnvelope in zjj-core/src/json.rs with impl and unit tests
2. Add StatusResponseData wrapper struct
3. Update output_json() to use envelope
4. Add integration tests with schema validation

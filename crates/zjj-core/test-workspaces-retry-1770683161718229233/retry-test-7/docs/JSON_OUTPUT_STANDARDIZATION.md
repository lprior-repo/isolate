# JSON Output Standardization

This document describes the JSON output standardization implemented for zjj commands.

## Overview

All zjj commands that output JSON follow a consistent envelope-based structure that provides:

1. **Schema Reference** - `$schema` field pointing to schema URI
2. **Version Tracking** - `_schema_version` for compatibility management
3. **Type Information** - `schema_type` indicating "single" or "array"
4. **Success Flag** - `success` boolean indicating operation outcome
5. **HATEOAS Navigation** - Optional `_links` for discoverability
6. **Metadata** - Optional `_meta` for debugging/tracing

## Schema Envelope Types

### SchemaEnvelope<T> (Single Object Response)

Used for commands that return a single object.

### SchemaEnvelopeArray<T> (Array Response)

Used for commands that return collections.

### Error Responses

Errors use the same envelope structure with `success: false`.

## Implementation Pattern

See `crates/zjj-core/src/json.rs` for full implementation details.

## Tests

Integration tests in `crates/zjj/tests/test_json_*.rs` verify envelope compliance.

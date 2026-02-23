# Serde Validation Tests Summary

Comprehensive serde validation tests have been added for all domain types to ensure proper JSON serialization/deserialization.

## Test File

**Location**: `/home/lewis/src/zjj/crates/zjj-core/tests/serde_validation_tests.rs`

## Test Coverage (55 tests, all passing)

### 1. Identifier Type Tests (15 tests)

Tests for all semantic identifier types:

- **SessionName**: JSON roundtrip, invalid input rejection, whitespace handling
- **AgentId**: JSON roundtrip, invalid input rejection
- **WorkspaceName**: JSON roundtrip, path separator rejection
- **TaskId/BeadId**: JSON roundtrip, prefix validation, hex validation
- **SessionId**: JSON roundtrip, ASCII-only validation
- **QueueEntryId**: JSON roundtrip, negative/zero rejection
- **AbsolutePath**: JSON roundtrip, relative path rejection
- **IssueId**: JSON roundtrip validation

### 2. Value Object Tests (6 tests)

Tests for validated value objects:

- **Title**: JSON roundtrip, empty/whitespace handling
- **Description**: JSON roundtrip, empty string allowed
- **IssueId**: JSON roundtrip
- **Assignee**: JSON roundtrip

### 3. State Enum Tests (7 tests)

Tests for state enumeration types:

- **IssueState**: All variants roundtrip, Closed with timestamp
- **Priority**: All variants, lowercase serialization
- **IssueType**: All variants, lowercase serialization (MergeRequest → "mergerequest")
- **BranchState**: Detached and OnBranch variants, snake_case serialization
- **ParentState**: Root and ChildOf variants

### 4. Collection Type Tests (4 tests)

Tests for collection wrappers:

- **Labels**: JSON roundtrip, empty collection
- **DependsOn**: JSON roundtrip
- **BlockedBy**: JSON roundtrip

### 5. Domain Event Tests (3 tests)

Tests for event sourcing types:

- **DomainEvent**: Multiple event types roundtrip, tagged enum structure
- **StoredEvent**: Event with metadata serialization
- **Binary serialization**: Bytes roundtrip for all events

### 6. Edge Case Tests (6 tests)

Tests for boundary conditions:

- **Null values**: Proper handling in Option fields
- **Unicode**: Rejection in ASCII-only identifiers
- **Long strings**: Rejection of over-length strings
- **Special characters**: Null bytes in paths rejected
- **Numeric edge cases**: Boundary values for QueueEntryId
- **Timestamp serialization**: DateTime precision preservation

### 7. Invalid JSON Structure Tests (4 tests)

Tests for malformed input:

- **Invalid syntax**: Proper error reporting
- **Wrong type**: Number instead of string rejected
- **Array/Object**: Wrong container types rejected

### 8. Complex Scenario Tests (3 tests)

Integration tests:

- **Nested domain types**: ParentState with SessionName
- **Event with all fields**: SessionFailed with reason
- **Multiple events**: Type preservation in arrays

## Key Findings

### Serialization Behavior

1. **Identifiers**: All serialize as JSON strings with validation on deserialization
2. **Enums**: Use lowercase snake_case (e.g., `"p0"`, `"open"`, `"on_branch"`)
3. **Events**: Use tagged enum with PascalCase type discriminator (`"SessionCreated"`)
4. **Timestamps**: Serialize as RFC3339 strings, microsecond precision preserved

### Validation Rules

1. **Empty strings**: Rejected for required fields (title, identifiers)
2. **Whitespace**: Trimmed during construction, not preserved in serialization
3. **Prefixes**: Task/Bead IDs must have "bd-" prefix
4. **ASCII**: Session/Agent IDs must be ASCII-only
5. **Length limits**: Enforced at serialization boundary

### Known Limitations

Some aggregate roots (Session, Workspace, Bead, QueueEntry) don't have Serialize/Deserialize derived:
- This is intentional - these are typically constructed through builders
- Event sourcing uses domain events for persistence instead
- Future work could add serialization for snapshotting

## Running the Tests

```bash
# Run all serde validation tests
cargo test -p zjj-core --test serde_validation_tests

# Run specific test category
cargo test -p zjj-core --test serde_validation_tests test_identifier
cargo test -p zjj-core --test serde_validation_tests test_state_enum
cargo test -p zjj-core --test serde_validation_tests test_event
```

## Files Covered

- `crates/zjj-core/src/domain/identifiers.rs`: All identifier types
- `crates/zjj-core/src/domain/session.rs`: BranchState, ParentState
- `crates/zjj-core/src/beads/domain.rs`: Value objects, enums, collections
- `crates/zjj-core/src/domain/events.rs`: All domain event types

## Benefits

1. **Type Safety**: All domain types validated at serialization boundary
2. **Documentation**: Tests serve as usage examples
3. **Regression Prevention**: Ensures serialization changes don't break
4. **JSON Output**: Validates CLI JSON output format is correct
5. **Interoperability**: Ensures external systems can exchange data

## Future Work

1. Add MessagePack/CBOR serialization tests for binary protocols
2. Add performance benchmarks for serialization hot paths
3. Consider adding Serialize/Deserialize to aggregates for snapshotting
4. Add fuzzing tests for robustness against malformed input

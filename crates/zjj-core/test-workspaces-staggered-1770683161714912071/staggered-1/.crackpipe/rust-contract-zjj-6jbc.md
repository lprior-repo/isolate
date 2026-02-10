# Contract Specification: Broadcast Command Implementation

## Context
- **Feature**: Inter-agent messaging system for zjj multi-agent workflows
- **Bead ID**: zjj-6jbc
- **Domain Terms**:
  - *Active Agent*: An agent with a heartbeat timestamp within the timeout window (default 60 seconds)
  - *Broadcast*: One-to-many message from sender to all active agents except sender
  - *Recipient List*: Set of agent IDs who should receive the message (all active agents minus sender)
  - *Message Storage*: Persistent log of broadcasts in SQLite broadcasts table

## Assumptions
1. AgentRegistry (zjj-mitf) is already implemented and functional
2. Database is initialized (`state.db` exists in zjj data directory)
3. Agent IDs are unique strings (e.g., "architect-1", "builder-2")
4. Messages are plain text (future: may support structured data)
5. No message delivery confirmation - broadcast is fire-and-forget
6. No read/unread tracking - messages are stored for audit only
7. No filtering - all active agents except sender receive all broadcasts

## Open Questions
1. Should messages be TTL-expired? **Decision**: No, keep forever for audit
2. Should there be a message size limit? **Decision**: Not in MVP (SQLite TEXT handles large strings)
3. Should agents be able to query broadcasts for themselves? **Decision**: Out of scope (future query command)
4. Should empty recipient list be an error? **Decision**: No, valid case (only agent active)

---

## Preconditions

### Database Preconditions
- [P1] `state.db` must exist in zjj data directory
- [P2] Database must be accessible (no locks, permissions OK)
- [P3] AgentRegistry agents table must exist

### Agent Registry Preconditions
- [P4] AgentRegistry must be initialized with valid timeout (default 60 seconds)
- [P5] At least one agent (the sender) must be registered (or none - valid edge case)

### Input Preconditions
- [P6] `args.message` must be a non-empty string
- [P7] `args.agent_id` must be a valid registered agent ID (or future: any string for anonymous broadcast)

---

## Postconditions

### Database Postconditions
- [PO1] Broadcasts table is created if it did not exist
- [PO2] New row inserted into broadcasts table with:
  - `id`: Auto-incrementing INTEGER PRIMARY KEY
  - `message`: Exact message string from args
  - `sender_id`: Exact agent_id from args
  - `sent_to`: JSON array of agent IDs (may be empty array `[]`)
  - `timestamp`: RFC3339 timestamp at insertion time

### Output Postconditions
- [PO3] Function returns `Ok(())` on success
- [PO4] If format is JSON: prints SchemaEnvelope with `type: "broadcast-response"`, `flavor: "single"`
- [PO5] If format is human: prints human-readable summary
- [PO6] Response contains:
  - `success: true`
  - `message`: Original message
  - `sent_to`: Vec<String> of recipient agent IDs (may be empty)
  - `timestamp`: RFC3339 string

### Filtering Postconditions
- [PO7] Sender agent_id is NOT in sent_to list
- [PO8] All active agents (within heartbeat timeout) ARE in sent_to list
- [PO9] Stale agents (outside heartbeat timeout) are NOT in sent_to list

---

## Invariants

- [I1] sent_to list is always a Vec<String> (never null, may be empty)
- [I2] Timestamp is always valid RFC3339 format
- [I3] sent_to JSON in database is always a valid JSON array
- [I4] sent_to list contains no duplicates
- [I5] sent_to list is sorted alphabetically (for deterministic output)
- [I6] success field is always true for successful broadcast
- [I7] broadcasts table schema is immutable once created

---

## Error Taxonomy

### Database Errors
- **Error::DatabaseNotInitialized** - When `state.db` does not exist (missing init)
  - User action: Run `zjj init` first
  - HTTP-like: 404 Not Found

- **Error::DatabaseConnectionFailed** - When SQLite connection fails (lock, permissions, corruption)
  - User action: Check database file, close other processes, run integrity check
  - HTTP-like: 503 Service Unavailable

- **Error::TableCreationFailed** - When CREATE TABLE IF NOT EXISTS fails
  - User action: Check disk space, permissions, database corruption
  - HTTP-like: 500 Internal Server Error

- **Error::MessageStorageFailed** - When INSERT into broadcasts fails
  - User action: Check disk space, database integrity
  - HTTP-like: 500 Internal Server Error

### Agent Registry Errors
- **Error::AgentRegistryInitFailed** - When AgentRegistry::new fails
  - User action: Check agents table exists, database integrity
  - HTTP-like: 500 Internal Server Error

- **Error::GetActiveAgentsFailed** - When registry.get_active() fails (query error)
  - User action: Check agents table integrity
  - HTTP-like: 500 Internal Server Error

### Input Validation Errors
- **Error::EmptyMessage** - When args.message is empty or whitespace-only
  - User action: Provide non-empty message
  - HTTP-like: 400 Bad Request

- **Error::InvalidAgentId** - When args.agent_id is empty or malformed
  - User action: Provide valid agent ID string
  - HTTP-like: 400 Bad Request

### Serialization Errors
- **Error::RecipientListSerializationFailed** - When serde_json::to_string(sent_to) fails
  - User action: Report bug (should never happen with Vec<String>)
  - HTTP-like: 500 Internal Server Error

- **Error::ResponseSerializationFailed** - When serde_json::to_string_pretty fails
  - User action: Report bug (should never happen with valid structs)
  - HTTP-like: 500 Internal Server Error

---

## Contract Signatures

### Primary Function
```rust
/// Send a broadcast message to all active agents except sender
///
/// # Preconditions
/// - Database must be initialized (state.db exists)
/// - AgentRegistry must be accessible
/// - args.message must be non-empty
///
/// # Postconditions
/// - Broadcast stored in database with timestamp and recipient list
/// - Response printed in requested format (JSON or human)
///
/// # Errors
/// - Error::DatabaseNotInitialized
/// - Error::DatabaseConnectionFailed
/// - Error::EmptyMessage
/// - Error::InvalidAgentId
pub async fn run(
    args: &BroadcastArgs,
    format: OutputFormat,
) -> Result<(), Error>
```

### Helper Functions
```rust
/// Get SQLite pool from zjj data directory
///
/// # Errors
/// - Error::DatabaseNotInitialized
/// - Error::DatabaseConnectionFailed
async fn get_db_pool() -> Result<SqlitePool, Error>

/// Store broadcast message in database
///
/// # Preconditions
/// - pool is valid connection
/// - message is non-empty
/// - sender_id is valid string
///
/// # Postconditions
/// - broadcasts table exists
/// - New row inserted with all fields
///
/// # Errors
/// - Error::TableCreationFailed
/// - Error::MessageStorageFailed
/// - Error::RecipientListSerializationFailed
async fn store_broadcast(
    pool: &SqlitePool,
    message: &str,
    sender_id: &str,
    sent_to: &[String],
) -> Result<(), Error>

/// Print human-readable broadcast summary
///
/// # Invariants
/// - Always prints exactly one message line
/// - Always prints timestamp and recipient count
/// - If recipients > 0, prints bulleted list
/// - If recipients = 0, prints "No other active agents"
fn print_human_readable(response: &BroadcastResponse)
```

---

## Non-goals

These are explicitly out of scope for this implementation:

- [NG1] Message delivery confirmation or read receipts
- [NG2] Querying broadcasts for specific agents
- [NG3] Deleting or expiring old broadcasts
- [NG4] Filtering broadcasts by agent or time range
- [NG5] Structured messages (JSON, protobuf) - plain text only
- [NG6] Message size limits or validation beyond non-empty
- [NG7] Reply-to or threading messages
- [NG8] Broadcast to specific subset (unicast/multicast) - always all-active-minus-sender
- [NG9] Real-time push notifications - store-only, polling required
- [NG10] Encryption or authentication of messages

---

## Design by Type Summary

### Input Types
- `BroadcastArgs { message: String, agent_id: String }` - Validated, non-empty fields
- `OutputFormat` - Enum: `Human` or `Json`

### Output Types
- `BroadcastResponse { success: bool, message: String, sent_to: Vec<String>, timestamp: String }`
- SchemaEnvelope wrapper for JSON output

### Error Types
- All errors propagate via `Result<T, Error>` (anyhow::Error in current implementation)
- Future: Custom error enum with semantic variants listed above

### Database Schema
```sql
CREATE TABLE IF NOT EXISTS broadcasts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    sent_to TEXT NOT NULL,  -- JSON array of strings
    timestamp TEXT NOT NULL  -- RFC3339
)
```

---

## Testing Strategy

Contracts will be verified through:
1. **Unit tests** for each helper function (get_db_pool, store_broadcast)
2. **Integration tests** for full run() with mock AgentRegistry
3. **Property tests** for invariants (sorted sent_to, no duplicates, RFC3339 validity)
4. **Error injection tests** for each error variant (database lock, corruption, etc.)
5. **Golden output tests** for JSON and human-readable formats

See `martin-fowler-tests.md` for complete test scenarios.

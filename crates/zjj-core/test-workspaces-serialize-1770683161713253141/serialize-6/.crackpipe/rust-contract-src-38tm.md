# Contract Specification: oya-ipc

## Context

**Feature**: Inter-Process Communication (IPC) protocol crate for host-guest messaging between oya-orchestrator (host) and Zellij WASM plugin (guest).

**Domain Terms**:
- **Host**: oya-orchestrator process running the agent swarm and workflow engine
- **Guest**: Zellij WASM plugin (oya-ui) running in the terminal UI
- **Serialization**: Encoding/decoding Rust types to binary format using bincode
- **Round-trip**: Serialize → Deserialize cycle that must preserve data integrity
- **Size bounds**: Maximum serialized message size (1MB) to prevent memory exhaustion
- **Latency targets**: Performance constraints for serialization/deserialization

**Assumptions**:
1. Bincode 2.0 is the chosen serialization format (not JSON, MessagePack, etc.)
2. Transport layer (stdio, socket, shared memory) is handled by a separate crate
3. Guest runs in WASM sandbox with limited memory (needs compact messages)
4. Host and guest must maintain protocol version compatibility
5. All message types are immutable after creation (no internal mutation)

**Open Questions**:
1. [Q1] Should we include protocol version field in every message for future compatibility?
2. [Q2] Do we need compression for large payloads (e.g., WorkflowGraph with 1000+ nodes)?
3. [Q3] Should messages include timestamps for debugging/lag detection?
4. [Q4] How do we handle schema evolution (adding new message variants)?
5. [Q5] Do we need message chunking for payloads approaching 1MB limit?

---

## Preconditions

### For Message Serialization
- [P1] All message fields must be serializable by serde (implement `Serialize`)
- [P2] Message size when serialized must not exceed 1MB (1,048,576 bytes)
- [P3] All string fields must be valid UTF-8
- [P4] All numeric types must be within their valid ranges (e.g., `u8` 0-255)
- [P5] Collection types (Vec, HashMap) must not be empty unless explicitly allowed

### For Message Deserialization
- [P6] Input byte slice must be non-empty
- [P7] Input byte slice must be valid bincode 2.0 format
- [P8] Input byte slice must contain a complete message (not partial/truncated)

### For Message Creation
- [P9] Required fields (non-Option) must not be empty/zero unless semantically valid
- [P10] IDs (BeadId, WorkflowId, AgentId) must match expected format patterns

---

## Postconditions

### After Successful Serialization
- [PS1] Output byte slice is non-empty
- [PS2] Output byte slice size ≤ 1MB
- [PS3] Output can be deserialized back to identical message value (round-trip property)
- [PS4] Serialization completes within 500ns for simple messages (e.g., GetBeadList)
- [PS5] Serialization completes within 2µs for complex messages (e.g., WorkflowGraph)

### After Successful Deserialization
- [PD1] Result contains valid `HostMessage` or `GuestMessage` enum
- [PD2] All fields are correctly reconstructed (no data loss)
- [PD3] Collections maintain order and cardinality
- [PD4] Deserialization completes within 500ns for simple messages
- [PD5] Deserialization completes within 2µs for complex messages

### After Failed Operations
- [PF1] `Error::SerializationFailed` is returned if serialization violates bincode invariants
- [PF2] `Error::DeserializationFailed` is returned if input is invalid bincode
- [PF3] `Error::SizeLimitExceeded` is returned if serialized size > 1MB
- [PF4] `Error::InvalidData` is returned if post-deserialization validation fails
- [PF5] Error messages include diagnostic context (what failed, why)

---

## Invariants

### Global Invariants
- [I1] All message variants have unique discriminators in binary format
- [I2] All message types are `Send + Sync + 'static` (required for actor passing)
- [I3] All message types implement `Debug`, `Clone`, `Serialize`, `Deserialize`
- [I4] No message variant contains raw file descriptors, handles, or pointers
- [I5] All messages are self-contained (no external state references)

### Size Invariants
- [I6] Max serialized message size: 1MB (enforced at serialization boundary)
- [I7] String field max length: 64KB (to prevent single-field exhaustion)
- [I8] Collection max items: 10,000 elements (Vec, HashMap, etc.)

### Semantic Invariants
- [I9] All `Option<T>` fields default to `None` when not applicable
- [I10] Empty collections (`Vec::new()`, `HashMap::new()`) are semantically different from `None`
- [I11] Message IDs (where present) must be unique within a session
- [I12] Timestamps (where present) must be monotonic increasing per sender

---

## Error Taxonomy

```rust
/// IPC protocol errors.
///
/// All errors are recoverable and provide diagnostic context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    /// Serialization failed due to bincode invariant violation.
    ///
    /// Caused by:
    /// - Invalid UTF-8 strings
    /// - Out-of-range numeric values
    /// - Non-serializable embedded types
    SerializationFailed {
        message_type: String,
        cause: String,
    },

    /// Deserialization failed due to invalid or corrupted bincode.
    ///
    /// Caused by:
    /// - Truncated input (incomplete message)
    /// - Invalid bincode format
    /// - Unknown message variant (version mismatch)
    /// - Size limit exceeded
    DeserializationFailed {
        cause: String,
        bytes_read: usize,
        total_bytes: usize,
    },

    /// Message exceeds size limit (1MB).
    ///
    /// Caused by:
    /// - Large payloads (e.g., WorkflowGraph with 10K nodes)
    /// - Embedded base64 or binary data
    SizeLimitExceeded {
        message_type: String,
        actual_size: usize,
        max_size: usize,
    },

    /// Post-deserialization validation failed.
    ///
    /// Caused by:
    /// - Empty required fields
    /// - Invalid ID format
    /// - Out-of-bounds values
    InvalidData {
        field_path: String,
        reason: String,
    },

    /// Protocol version mismatch between host and guest.
    ///
    /// Caused by:
    /// - Host running newer protocol version than guest supports
    /// - Guest running newer protocol version than host supports
    VersionMismatch {
        host_version: u32,
        guest_version: u32,
    },
}
```

---

## Contract Signatures

### Core Serialization Functions

```rust
/// Serialize a HostMessage to bincode format.
///
/// # Preconditions
/// - msg: Valid HostMessage variant
/// - All fields satisfy type constraints
///
/// # Postconditions
/// - Returns Ok(bytes) where bytes.len() ≤ 1MB
/// - Returns Err(IpcError::SerializationFailed) if serialization fails
/// - Returns Err(IpcError::SizeLimitExceeded) if serialized size > 1MB
///
/// # Performance
/// - Must complete within 500ns for simple messages (GetBeadList, etc.)
/// - Must complete within 2µs for complex messages (WorkflowGraph, etc.)
pub fn serialize_host_message(msg: &HostMessage) -> Result<Vec<u8>, IpcError> {
    // Implementation checks size limit during serialization
    // Uses bincode::serialize() with configuration
}

/// Deserialize a HostMessage from bincode format.
///
/// # Preconditions
/// - data: Non-empty byte slice
/// - data: Valid bincode 2.0 format
/// - data: Complete message (not truncated)
///
/// # Postconditions
/// - Returns Ok(HostMessage) with all fields reconstructed
/// - Returns Err(IpcError::DeserializationFailed) if input is invalid
/// - Returns Err(IpcError::InvalidData) if validation fails
///
/// # Performance
/// - Must complete within 500ns for simple messages
/// - Must complete within 2µs for complex messages
pub fn deserialize_host_message(data: &[u8]) -> Result<HostMessage, IpcError> {
    // Implementation uses bincode::deserialize()
    // Validates all fields after deserialization
}

/// Serialize a GuestMessage to bincode format.
///
/// Same contract as serialize_host_message but for GuestMessage.
pub fn serialize_guest_message(msg: &GuestMessage) -> Result<Vec<u8>, IpcError> {
    // Symmetric implementation
}

/// Deserialize a GuestMessage from bincode format.
///
/// Same contract as deserialize_host_message but for GuestMessage.
pub fn deserialize_guest_message(data: &[u8]) -> Result<GuestMessage, IpcError> {
    // Symmetric implementation
}
```

### Message Creation Functions

```rust
/// Create a BeadList response message.
///
/// # Preconditions
/// - beads: Vector of bead summaries (0-10,000 items)
/// - Each bead has valid BeadId format
///
/// # Postconditions
/// - Returns HostMessage::BeadList(beads)
/// - Total serialized size ≤ 1MB
pub fn host_message_bead_list(beads: Vec<BeadSummary>) -> HostMessage {
    // Constructor with validation
}

/// Create a GetBeadList query message.
///
/// # Preconditions
/// - filter: Optional filter string (max 256 chars if present)
///
/// # Postconditions
/// - Returns GuestMessage::GetBeadList { filter }
pub fn guest_message_get_bead_list(filter: Option<String>) -> GuestMessage {
    // Constructor with validation
}

/// Create an Error response message.
///
/// # Preconditions
/// - message: Non-empty error description (max 1KB)
///
/// # Postconditions
/// - Returns HostMessage::Error(message)
/// - Message length ≤ 1KB enforced
pub fn host_message_error(message: String) -> HostMessage {
    // Constructor with validation
}

// Additional constructors for each message variant...
```

### Validation Functions

```rust
/// Validate a message before serialization.
///
/// # Preconditions
/// - msg: Reference to any message type
///
/// # Postconditions
/// - Returns Ok(()) if message is valid
/// - Returns Err(IpcError::InvalidData) with field_path if validation fails
///
/// # Invariants Checked
/// - String field lengths ≤ 64KB
/// - Collection sizes ≤ 10,000
/// - Required fields are non-empty
/// - ID format patterns match
pub fn validate_message<T: MessageValidator>(msg: &T) -> Result<(), IpcError> {
    // Generic validation using trait
}

/// Estimate serialized message size.
///
/// # Preconditions
/// - msg: Reference to any message type
///
/// # Postconditions
/// - Returns estimated size in bytes
/// - Estimate is ≥ actual serialized size (conservative)
pub fn estimate_serialized_size<T: Serialize>(msg: &T) -> usize {
    // Uses bincode size calculation or heuristic
}
```

---

## Non-Goals

1. **Transport Layer**: This crate does NOT handle:
   - Socket I/O, stdio, or shared memory communication
   - Connection lifecycle (connect, disconnect, reconnect)
   - Multiplexing multiple concurrent connections
   - Flow control or backpressure

2. **Message Routing**: This crate does NOT define:
   - How messages are routed to specific actors
   - Request-reply correlation logic
   - Broadcast or pub/sub semantics

3. **Compression**: This crate does NOT include:
   - Compression algorithms (zstd, lz4, etc.)
   - Delta encoding for incremental updates
   - Binary diff formats

4. **Encryption**: This crate does NOT provide:
   - Encryption or authentication
   - Message signing or integrity verification
   - Secure channel establishment

5. **Compatibility Shim**: Version 1.0 does NOT include:
   - Protocol version negotiation
   - Backward compatibility for older message formats
   - Schema migration tools

---

## Performance Verification

### Criterion Benchmarks (Required)

```rust
// Benchmark: Simple message round-trip
fn bench_host_message_bead_list_roundtrip(c: &mut Criterion) {
    let msg = HostMessage::BeadList(vec![
        // 10 bead summaries
    ]);

    group.bench_function("serialize_10_beads", |b| {
        b.iter(|| serialize_host_message(&msg))
    });

    group.bench_function("deserialize_10_beads", |b| {
        let bytes = serialize_host_message(&msg).unwrap();
        b.iter(|| deserialize_host_message(&bytes))
    });
}

// Benchmark: Complex message round-trip
fn bench_host_message_workflow_graph_roundtrip(c: &mut Criterion) {
    let msg = HostMessage::WorkflowGraph(/* 1000 nodes, 2000 edges */);

    // Same pattern...
}

// Target: <1µs round-trip for simple messages
// Target: <4µs round-trip for complex messages
```

---

## Type Definitions (Contractual)

```rust
/// Messages sent from Host (oya-orchestrator) to Guest (Zellij plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostMessage {
    // Query Responses
    BeadList(Vec<BeadSummary>),
    BeadDetail(Option<Box<BeadDetail>>),
    WorkflowGraph(Box<WorkflowGraph>),
    AgentPool(Box<AgentPoolStats>),
    SystemHealth(Box<SystemHealth>),

    // Events
    BeadStateChanged(BeadStateChangedEvent),
    PhaseProgress(PhaseProgressEvent),
    AgentHeartbeat(AgentHeartbeatEvent),
    SystemAlert(SystemAlertEvent),

    // Errors
    Error(String),
}

/// Messages sent from Guest (Zellij plugin) to Host (oya-orchestrator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuestMessage {
    // Queries
    GetBeadList { filter: Option<String> },
    GetBeadDetail { bead_id: String },
    GetWorkflowGraph { workflow_id: String },
    GetAgentPool,
    GetSystemHealth,

    // Commands
    StartBead { bead_id: String },
    CancelBead { bead_id: String },
    RetryBead { bead_id: String },

    // Subscriptions
    SubscribeEvents,
    UnsubscribeEvents,
}

// Supporting types (must be defined in implementation)
// - BeadSummary, BeadDetail, WorkflowGraph, AgentPoolStats, SystemHealth
// - BeadStateChangedEvent, PhaseProgressEvent, etc.
```

---

## Protocol Versioning Strategy

### Version 1.0 Assumptions
- Single protocol version (no version field in messages)
- Host and guest must be built from same oya-ipc crate version
- Breaking changes require coordinated deployment

### Future Path (Post-1.0)
- Add `protocol_version: u32` field to each message enum
- Implement variant migration for backward compatibility
- Define deprecation policy for old message variants

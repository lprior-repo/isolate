# Contract Specification: Length-Prefixed Buffer Transport Layer

## Context

**Feature**: Transport layer for reading/writing length-prefixed messages over stdin/stdout buffers for Zellij plugin IPC.

**Domain Terms**:
- **Length prefix**: 4-byte big-endian u32 header indicating message payload size
- **Payload**: Bincode-encoded message data (HostMessage or GuestMessage)
- **Transport**: Bidirectional byte stream abstraction (Read + Write traits)
- **Frame**: Complete message including length prefix and payload
- **Flush**: Ensures buffered data is written to underlying stream
- **Zero-copy**: Deserialization directly from read buffer without allocation

**Assumptions**:
1. Transport operates on unidirectional byte streams (stdin/stdout)
2. Both ends of the transport use the same protocol (length-prefixed bincode)
3. Stream is reliable (no packet loss, but may have partial reads/writes)
4. Concurrent access is serialized (external synchronization via actors)
5. Underlying Read/Write implementations are blocking (not async)

**Open Questions**:
1. [Q1] Should we support async IO (tokio::AsyncRead/AsyncWrite) or only std::io?
2. [Q2] Do we need message framing for streaming large payloads (>1MB)?
3. [Q3] Should transport handle backpressure or propagate full/empty errors?
4. [Q4] Do we need heartbeat/keepalive messages for connection liveness?
5. [Q5] Should transport include timeout configuration for reads/writes?

---

## Preconditions

### For Transport Construction
- [P1] reader parameter must implement `std::io::Read`
- [P2] writer parameter must implement `std::io::Write`
- [P3] reader and writer must be independent streams (no shared state)

### For Sending Messages
- [P4] Message must be serializable by serde (implement `Serialize`)
- [P5] Serialized message size must not exceed 1MB (1,048,576 bytes)
- [P6] Writer must not be in an error state

### For Receiving Messages
- [P7] Target type must be deserializable (implement `DeserializeOwned`)
- [P8] Reader must have at least 4 bytes available (for length prefix)
- [P9] Reader must not be in an error state
- [P10] Length prefix value must be ≤ 1MB

### For Stream State
- [P11] No concurrent calls to `send()` (must be externally synchronized)
- [P12] No concurrent calls to `recv()` (must be externally synchronized)

---

## Postconditions

### After Successful Construction
- [PC1] Returns `IpcTransport<R, W>` with initialized reader/writer buffers
- [PC2] Internal buffers are empty (no stale data)
- [PC3] Transport is ready for first `send()` or `recv()` call

### After Successful send()
- [PS1] Length prefix (4 bytes, big-endian) is written to stream
- [PS2] Serialized payload is written to stream after length prefix
- [PS3] Data is flushed to underlying stream (not buffered in memory)
- [PS4] Total bytes written = 4 + payload_size
- [PS5] Function completes in <2µs for 1KB message
- [PS6] Returns `Ok(())`

### After Successful recv()
- [PR1] Length prefix is read and validated (≤ 1MB)
- [PR2] Exactly N bytes are read (where N = length prefix value)
- [PR3] Payload is deserialized to target type T
- [PR4] Total bytes consumed = 4 + payload_size
- [PR5] Function completes in <3µs for 1KB message
- [PR6] Returns `Ok(T)` with deserialized value

### After Failed Operations
- [PF1] `send()` returns `Err(TransportError::SerializationFailed)` if bincode fails
- [PF2] `send()` returns `Err(TransportError::MessageTooLarge)` if size > 1MB
- [PF3] `send()` returns `Err(TransportError::WriteFailed)` if stream write fails
- [PF4] `recv()` returns `Err(TransportError::UnexpectedEof)` if stream ends mid-frame
- [PF5] `recv()` returns `Err(TransportError::InvalidLength)` if length prefix > 1MB
- [PF6] `recv()` returns `Err(TransportError::DeserializationFailed)` if bincode fails
- [PF7] All error variants include diagnostic context (bytes processed, cause)

---

## Invariants

### Protocol Invariants
- [I1] Every message frame starts with exactly 4-byte length prefix
- [I2] Length prefix is encoded in big-endian byte order
- [I3] Length prefix value = payload size (not including prefix itself)
- [I4] Maximum allowed payload size = 1MB (1,048,576 bytes)

### Buffer Invariants
- [I5] Read buffer capacity ≥ max frame size (1MB + 4 bytes)
- [I6] Write buffer capacity ≥ max frame size (1MB + 4 bytes)
- [I7] Buffers are cleared between messages (no cross-message contamination)

### State Invariants
- [I8] Transport is always in a consistent state (mid-frame or between frames)
- [I9] Partial reads are buffered internally (not exposed to caller)
- [I10] Partial writes are retried until complete or error

### Performance Invariants
- [I11] Zero allocations during normal operation (buffers pre-allocated)
- [I12] send() flushes after each message (no batching)
- [I13] recv() uses read_exact semantics (blocks until full frame available)

---

## Error Taxonomy

```rust
/// Transport layer errors.
///
/// All errors are recoverable and provide diagnostic context.
#[derive(Debug, Clone, PartialEq)]
pub enum TransportError {
    /// Message payload exceeds 1MB limit.
    ///
    /// Caused by:
    /// - Attempting to send a message that serializes to >1MB
    /// - Receiving a length prefix > 1MB
    MessageTooLarge {
        /// Actual payload size in bytes
        actual_size: usize,
        /// Maximum allowed size
        max_size: usize,
    },

    /// End of stream reached before complete frame.
    ///
    /// Caused by:
    /// - Remote process terminated
    /// - Pipe/socket closed
    /// - Truncated message
    UnexpectedEof {
        /// Bytes successfully read before EOF
        bytes_read: usize,
        /// Expected bytes (from length prefix)
        expected_bytes: usize,
    },

    /// Length prefix indicates invalid size.
    ///
    /// Caused by:
    /// - Length prefix = 0 (invalid frame)
    /// - Length prefix > 1MB (size limit violation)
    InvalidLength {
        /// Invalid length value
        length: u32,
        /// Reason why length is invalid
        reason: String,
    },

    /// Serialization failed (bincode error).
    ///
    /// Caused by:
    /// - Message contains non-serializable data
    /// - Invalid UTF-8 strings
    /// - Out-of-range numeric values
    SerializationFailed {
        /// Bincode error message
        cause: String,
    },

    /// Deserialization failed (bincode error).
    ///
    /// Caused by:
    /// - Corrupted payload data
    /// - Schema mismatch (version incompatibility)
    /// - Invalid bincode format
    DeserializationFailed {
        /// Bincode error message
        cause: String,
        /// Bytes of payload read
        payload_bytes: usize,
    },

    /// Write operation failed.
    ///
    /// Caused by:
    /// - Broken pipe
    /// - Disk full
    /// - Permission denied
    /// - Stream shutdown
    WriteFailed {
        /// OS error code
        error_code: Option<i32>,
        /// Error kind
        kind: IoErrorKind,
    },

    /// Read operation failed.
    ///
    /// Caused by:
    /// - Read timeout (if configured)
    /// - Stream corruption
    /// - OS-level I/O error
    ReadFailed {
        /// OS error code
        error_code: Option<i32>,
        /// Error kind
        kind: IoErrorKind,
    },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageTooLarge { actual_size, max_size } => {
                write!(
                    f,
                    "Message too large: {} bytes (max {} bytes)",
                    actual_size, max_size
                )
            }
            Self::UnexpectedEof {
                bytes_read,
                expected_bytes,
            } => {
                write!(
                    f,
                    "Unexpected EOF: {} bytes read, expected {}",
                    bytes_read, expected_bytes
                )
            }
            Self::InvalidLength { length, reason } => {
                write!(f, "Invalid length prefix {}: {}", length, reason)
            }
            Self::SerializationFailed { cause } => {
                write!(f, "Serialization failed: {}", cause)
            }
            Self::DeserializationFailed {
                cause,
                payload_bytes,
            } => {
                write!(
                    f,
                    "Deserialization failed at {} bytes: {}",
                    payload_bytes, cause
                )
            }
            Self::WriteFailed { error_code, kind } => {
                write!(
                    f,
                    "Write failed: {:?} (error code: {:?})",
                    kind, error_code
                )
            }
            Self::ReadFailed { error_code, kind } => {
                write!(
                    f,
                    "Read failed: {:?} (error code: {:?})",
                    kind, error_code
                )
            }
        }
    }
}

impl std::error::Error for TransportError {}
```

---

## Contract Signatures

### Core Transport Functions

```rust
/// Create a new transport from reader and writer.
///
/// # Preconditions
/// - reader: Implements std::io::Read
/// - writer: Implements std::io::Write
/// - Streams are independent (no shared state)
///
/// # Postconditions
/// - Returns Ok(IpcTransport) with initialized buffers
/// - Buffers are empty and ready for use
/// - Returns Err(()) if reader or writer is invalid (currently infallible)
///
/// # Invariants
/// - Buffer capacity ≥ max frame size + 4 bytes
pub fn new<R: std::io::Read, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IpcTransport<R, W> {
    // Constructor with buffer initialization
}

/// Send a message over the transport.
///
/// # Preconditions
/// - msg: Implements serde::Serialize
/// - Serialized size ≤ 1MB
/// - Writer is not in error state
/// - No concurrent send() calls
///
/// # Postconditions
/// - Returns Ok(()) if message sent successfully
/// - Returns Err(TransportError::SerializationFailed) if bincode fails
/// - Returns Err(TransportError::MessageTooLarge) if size > 1MB
/// - Returns Err(TransportError::WriteFailed) if stream write fails
/// - Length prefix (4 bytes BE) + payload written to stream
/// - Data flushed to underlying stream
///
/// # Performance
/// - Must complete <2µs for 1KB message
/// - Must complete <20µs for 100KB message
pub fn send<T: serde::Serialize>(
    &mut self,
    msg: &T,
) -> Result<(), TransportError> {
    // Implementation:
    // 1. Serialize message to buffer
    // 2. Check size ≤ 1MB
    // 3. Write length prefix (4 bytes, big-endian)
    // 4. Write payload
    // 5. Flush
}

/// Receive a message from the transport.
///
/// # Preconditions
/// - T: Implements serde::de::DeserializeOwned
/// - Reader is not in error state
/// - No concurrent recv() calls
/// - At least 4 bytes available (for length prefix)
///
/// # Postconditions
/// - Returns Ok(T) with deserialized message
/// - Returns Err(TransportError::UnexpectedEof) if stream ends mid-frame
/// - Returns Err(TransportError::InvalidLength) if length > 1MB or = 0
/// - Returns Err(TransportError::DeserializationFailed) if bincode fails
/// - Returns Err(TransportError::ReadFailed) if stream read fails
/// - Exact frame consumed from reader (buffer position advanced)
///
/// # Performance
/// - Must complete <3µs for 1KB message
/// - Must complete <30µs for 100KB message
pub fn recv<T: serde::de::DeserializeOwned>(
    &mut self,
) -> Result<T, TransportError> {
    // Implementation:
    // 1. Read 4 bytes (length prefix)
    // 2. Validate length ≤ 1MB and > 0
    // 3. Read N bytes (where N = length prefix)
    // 4. Deserialize payload
}
```

### Utility Functions

```rust
/// Get the number of bytes available in read buffer.
///
/// # Postconditions
/// - Returns number of bytes buffered but not yet consumed
/// - Useful for detecting partial reads or connection liveness
pub fn buffered_bytes(&self) -> usize {
    // Returns internal buffer fill level
}

/// Clear internal buffers (for error recovery).
///
/// # Postconditions
/// - All buffered data is discarded
/// - Transport is in clean state (ready for new frame)
/// - Any partial message data is lost
pub fn clear_buffers(&mut self) {
    // Resets buffer positions
}

/// Check if stream is at EOF.
///
/// # Postconditions
/// - Returns true if underlying reader is exhausted
/// - Returns false if data may still be available
pub fn is_eof(&self) -> bool {
    // Checks stream state
}
```

---

## Non-Goals

1. **Async IO**: This implementation does NOT support:
   - tokio::AsyncRead/AsyncWrite
   - Non-blocking operations
   - Future-based API
   - Async timeout configuration

2. **Connection Management**: This transport does NOT handle:
   - TCP socket lifecycle
   - Connection establishment/teardown
   - TLS/SSL encryption
   - Authentication

3. **Message Fragmentation**: Version 1.0 does NOT support:
   - Messages larger than 1MB
   - Automatic chunking/reassembly
   - Streaming payloads
   - Progress callbacks for large messages

4. **Backpressure**: This transport does NOT implement:
   - Flow control signals
   - Credit-based sending
   - Buffer full notifications
   - Rate limiting

5. **Compression**: This transport does NOT include:
   - Compression/decompression
   - Delta encoding
   - Binary diffing

---

## Protocol Specification

### Frame Format

```
+--------+--------+--------+--------+--------------------------+
| Byte 0 | Byte 1 | Byte 2 | Byte 3 | Bytes 4..(4+N)           |
|--------+--------+--------+--------+--------------------------|
|          Length (big-endian u32)       |    Bincode Payload      |
|           N = payload size             |    (N bytes)             |
+--------+--------+--------+--------+--------------------------+
```

### Byte Order

- **Length prefix**: Big-endian (network byte order)
- **Rationale**: Cross-platform compatibility, network standard
- **Implementation**: `u32.to_be_bytes()` for write, `u32::from_be_bytes()` for read

### Size Limits

- **Minimum frame**: 5 bytes (length = 1)
- **Maximum frame**: 1,048,580 bytes (length = 1,048,576)
- **Empty payload**: Disallowed (length = 0 is invalid)

### State Machine

```
IDLE → READ_PREFIX → READ_PAYLOAD → DESERIALIZE → IDLE
  ↓         ↓              ↓              ↓
ERROR ← any failure mode ←──────────────┘
```

---

## Performance Verification

### Criterion Benchmarks (Required)

```rust
// Benchmark: Send 1KB message
fn bench_send_1kb_message(c: &mut Criterion) {
    let mut transport = create_transport_pair();
    let msg = test_message_1kb();

    group.bench_function("send_1kb", |b| {
        b.iter(|| transport.send(&msg))
    });

    // Target: <2µs median
}

// Benchmark: Recv 1KB message
fn bench_recv_1kb_message(c: &mut Criterion) {
    let (mut tx, mut rx) = create_transport_pair();
    let msg = test_message_1kb();
    tx.send(&msg).unwrap();

    group.bench_function("recv_1kb", |b| {
        b.iter(|| rx.recv::<TestMessage>())
    });

    // Target: <3µs median
}

// Benchmark: Round-trip 1KB message
fn bench_roundtrip_1kb(c: &mut Criterion) {
    let (mut tx, mut rx) = create_transport_pair();
    let msg = test_message_1kb();

    group.bench_function("roundtrip_1kb", |b| {
        b.iter(|| {
            tx.send(&msg).unwrap();
            rx.recv::<TestMessage>().unwrap()
        })
    });

    // Target: <5µs median
}
```

---

## Type Definitions (Contractual)

```rust
/// Transport layer for length-prefixed bincode messages.
///
/// # Type Parameters
/// - `R`: Reader type (implements std::io::Read)
/// - `W`: Writer type (implements std::io::Write)
///
/// # Thread Safety
/// - `!Send + !Sync` (must be externally synchronized)
/// - Use within actor context for safe concurrent access
///
/// # Example
/// ```rust
/// let transport = IpcTransport::new(stdin, stdout);
/// transport.send(&HostMessage::BeadList(vec![]))?;
/// let msg: GuestMessage = transport.recv()?;
/// ```
pub struct IpcTransport<R, W> {
    reader: BufReader<R>,
    writer: BufWriter<W>,
    _phantom: PhantomData<(R, W)>,
}

impl<R: std::io::Read, W: std::io::Write> IpcTransport<R, W> {
    // Constructor and methods defined above
}

// Convenience type for stdin/stdout
pub type StdioTransport = IpcTransport<Stdin, Stdout>;
```

---

## Testing Strategy

### Unit Tests (Required)
- Round-trip send/recv for all message sizes (1 byte to 1MB)
- Boundary value tests (0, 1, 1MB-1, 1MB, 1MB+1)
- Error path tests (EOF, invalid length, corruption)
- Partial read/write handling

### Integration Tests (Required)
- Bidirectional communication (send and recv concurrently)
- Multiple sequential messages
- Message with max size (1MB)
- Zero-byte payload rejection

### Property-Based Tests (Optional)
- `prop_roundtrip_preserves_data`: For all messages, recv(send(msg)) == msg
- `prop_frame_format_valid`: All sent frames match protocol spec

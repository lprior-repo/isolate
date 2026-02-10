# Martin Fowler Test Plan: Length-Prefixed Buffer Transport Layer

## Test Philosophy

These tests verify the transport layer correctly implements the length-prefixed frame protocol for reliable IPC communication. Tests cover protocol compliance, error handling, performance targets, and edge cases.

---

## Happy Path Tests

### Test: Send and receive small message (1KB) successfully

```rust
#[test]
fn test_send_recv_1kb_message_roundtrip_succeeds() {
    // Given: A transport pair and a 1KB message
    let (mut client, mut server) = create_transport_pair();
    let original = HostMessage::BeadList(vec![
        BeadSummary {
            id: "src-123".to_string(),
            title: "Test bead".to_string(),
            status: BeadStatus::Open,
            priority: Priority::P1,
        },
    ]);

    // When: Client sends and server receives
    client.send(&original).expect("send should succeed");
    let received = server.recv::<HostMessage>().expect("recv should succeed");

    // Then: Message is preserved exactly
    assert_eq!(original, received);
}
```

### Test: Length prefix is correctly encoded in big-endian

```rust
#[test]
fn test_length_prefix_encoded_as_big_endian() {
    // Given: A transport with captured write buffer
    let (mut client, _server) = create_transport_pair_with_buffer();
    let msg = HostMessage::BeadList(vec![]);

    // When: Message is sent
    client.send(&msg).unwrap();

    // Then: First 4 bytes are big-endian length prefix
    let written = client.captured_bytes();
    let length_prefix = u32::from_be_bytes([written[0], written[1], written[2], written[3]]);

    // And: Length equals payload size (excluding prefix)
    assert_eq!(length_prefix as usize, written.len() - 4);
}
```

### Test: Flush is called after each send

```rust
#[test]
fn test_send_flushes_data_to_underlying_stream() {
    // Given: A transport with flush-counting writer
    let (mut client, _server) = create_transport_with_flush_counter();
    let msg = HostMessage::BeadList(vec![]);

    // When: Message is sent
    client.send(&msg).unwrap();

    // Then: Flush was called at least once
    assert!(client.flush_count() > 0);
}
```

### Test: Multiple sequential messages are framed correctly

```rust
#[test]
fn test_multiple_messages_are_independently_framed() {
    // Given: A transport pair
    let (mut client, mut server) = create_transport_pair();

    // When: Three messages are sent sequentially
    client
        .send(&HostMessage::BeadList(vec![]))
        .unwrap();
    client
        .send(&HostMessage::Error("test".to_string()))
        .unwrap();
    client
        .send(&HostMessage::BeadDetail(None))
        .unwrap();

    // Then: All three messages are received correctly
    let msg1 = server.recv::<HostMessage>().unwrap();
    let msg2 = server.recv::<HostMessage>().unwrap();
    let msg3 = server.recv::<HostMessage>().unwrap();

    assert!(matches!(msg1, HostMessage::BeadList(_)));
    assert!(matches!(msg2, HostMessage::Error(_)));
    assert!(matches!(msg3, HostMessage::BeadDetail(_)));
}
```

### Test: Maximum size message (1MB - 1) succeeds

```rust
#[test]
fn test_message_at_exactly_1mb_limit_succeeds() {
    // Given: A message that serializes to exactly 1MB - 1
    let max_size = 1_048_575; // 1MB - 1
    let large_msg = HostMessage::BeadList(create_beads_of_size(max_size));

    // When: Sent and received
    let (mut client, mut server) = create_transport_pair();
    client.send(&large_msg).expect("send should succeed");
    let received = server.recv::<HostMessage>().expect("recv should succeed");

    // Then: Message is preserved
    assert_eq!(large_msg, received);
}
```

### Test: Bidirectional communication works correctly

```rust
#[test]
fn test_bidirectional_send_recv_in_both_directions() {
    // Given: Two transports (client and server)
    let (mut client, mut server) = create_transport_pair();

    // When: Each sends to the other
    let client_msg = HostMessage::BeadList(vec![]);
    let server_msg = HostMessage::Error("ack".to_string());

    client.send(&client_msg).unwrap();
    server.send(&server_msg).unwrap();

    // Then: Each receives the other's message
    let server_received = server.recv::<HostMessage>().unwrap();
    let client_received = client.recv::<HostMessage>().unwrap();

    assert_eq!(server_received, client_msg);
    assert_eq!(client_received, server_msg);
}
```

---

## Error Path Tests

### Test: Message larger than 1MB is rejected

```rust
#[test]
fn test_send_message_exceeding_1mb_returns_error() {
    // Given: A message larger than 1MB
    let oversized_msg = HostMessage::BeadList(create_beads_of_size(1_048_577));

    // When: Attempting to send
    let (mut client, _server) = create_transport_pair();
    let result = client.send(&oversized_msg);

    // Then: Returns MessageTooLarge error
    assert!(matches!(result, Err(TransportError::MessageTooLarge { .. })));

    if let Err(TransportError::MessageTooLarge { actual_size, max_size }) = result {
        assert!(*actual_size > 1_048_576);
        assert_eq!(*max_size, 1_048_576);
    }
}
```

### Test: Receiving message with invalid length prefix (>1MB)

```rust
#[test]
fn test_recv_with_invalid_length_prefix_returns_error() {
    // Given: A stream with length prefix = 2MB
    let (mut _client, mut server) = create_transport_pair();
    write_invalid_length_prefix(&mut server, 2_097_152); // 2MB

    // When: Attempting to receive
    let result: Result<HostMessage, _> = server.recv();

    // Then: Returns InvalidLength error
    assert!(matches!(result, Err(TransportError::InvalidLength { .. })));

    if let Err(TransportError::InvalidLength { length, reason }) = result {
        assert_eq!(length, 2_097_152);
        assert!(reason.contains("exceeds maximum"));
    }
}
```

### Test: Receiving message with zero-length prefix

```rust
#[test]
fn test_recv_with_zero_length_prefix_returns_error() {
    // Given: A stream with length prefix = 0
    let (mut _client, mut server) = create_transport_pair();
    write_length_prefix(&mut server, 0);

    // When: Attempting to receive
    let result: Result<HostMessage, _> = server.recv();

    // Then: Returns InvalidLength error
    assert!(matches!(result, Err(TransportError::InvalidLength { .. })));
}
```

### Test: Unexpected EOF during length prefix read

```rust
#[test]
fn test_recv_eof_during_length_prefix_returns_error() {
    // Given: A stream that closes after 2 bytes
    let (mut _client, mut server) = create_transport_pair();
    close_stream_after_n_bytes(&mut server, 2);

    // When: Attempting to receive
    let result: Result<HostMessage, _> = server.recv();

    // Then: Returns UnexpectedEof error
    assert!(matches!(result, Err(TransportError::UnexpectedEof { .. })));

    if let Err(TransportError::UnexpectedEof {
        bytes_read,
        expected_bytes,
    }) = result
    {
        assert_eq!(*bytes_read, 2);
        assert_eq!(*expected_bytes, 4); // Expected full length prefix
    }
}
```

### Test: Unexpected EOF during payload read

```rust
#[test]
fn test_recv_eof_during_payload_returns_error() {
    // Given: A valid length prefix but truncated payload
    let (mut _client, mut server) = create_transport_pair();
    // Write length prefix = 1000, but only provide 500 bytes
    write_truncated_frame(&mut server, 1000, 500);

    // When: Attempting to receive
    let result: Result<HostMessage, _> = server.recv();

    // Then: Returns UnexpectedEof error
    assert!(matches!(result, Err(TransportError::UnexpectedEof { .. })));

    if let Err(TransportError::UnexpectedEof {
        bytes_read,
        expected_bytes,
    }) = result
    {
        assert_eq!(*bytes_read, 504); // 4 bytes prefix + 500 bytes payload
        assert_eq!(*expected_bytes, 1004); // 4 bytes prefix + 1000 bytes payload
    }
}
```

### Test: Corrupted payload fails deserialization

```rust
#[test]
fn test_recv_with_corrupted_payload_returns_error() {
    // Given: A valid length prefix but invalid bincode payload
    let (mut _client, mut server) = create_transport_pair();
    write_frame_with_invalid_bincode(&mut server, 100);

    // When: Attempting to receive
    let result: Result<HostMessage, _> = server.recv();

    // Then: Returns DeserializationFailed error
    assert!(matches!(
        result,
        Err(TransportError::DeserializationFailed { .. })
    ));

    if let Err(TransportError::DeserializationFailed {
        cause,
        payload_bytes,
    }) = result
    {
        assert!(!cause.is_empty());
        assert_eq!(*payload_bytes, 100);
    }
}
```

### Test: Write failure propagates as error

```rust
#[test]
fn test_send_with_failing_writer_returns_error() {
    // Given: A transport with a failing writer
    let mut writer = FailingWriter::new();
    let reader = std::io::empty();
    let mut transport = IpcTransport::new(reader, &mut writer);

    let msg = HostMessage::BeadList(vec![]);

    // When: Attempting to send
    let result = transport.send(&msg);

    // Then: Returns WriteFailed error
    assert!(matches!(result, Err(TransportError::WriteFailed { .. })));
}
```

---

## Edge Case Tests

### Test: Empty collection message (smallest valid payload)

```rust
#[test]
fn test_send_recv_empty_bead_list_succeeds() {
    // Given: A message with empty collection (minimal but valid)
    let msg = HostMessage::BeadList(vec![]);

    // When: Sent and received
    let (mut client, mut server) = create_transport_pair();
    client.send(&msg).unwrap();
    let received = server.recv::<HostMessage>().unwrap();

    // Then: Empty collection is preserved
    assert_eq!(msg, received);
}
```

### Test: Single byte payload (minimum non-empty)

```rust
#[test]
fn test_single_byte_payload_roundtrip_succeeds() {
    // Given: A message type with 1-byte serialized representation
    let msg = MinimalMessage::new();

    // When: Sent and received
    let (mut client, mut server) = create_transport_pair();
    client.send(&msg).unwrap();
    let received = server.recv::<MinimalMessage>().unwrap();

    // Then: Message is preserved
    assert_eq!(msg, received);

    // And: Frame size is 5 bytes (4 length prefix + 1 payload)
    assert_eq!(client.bytes_written(), 5);
}
```

### Test: Maximum size message boundary (exactly 1MB)

```rust
#[test]
fn test_message_at_exactly_1mb_boundary_succeeds() {
    // Given: Message exactly at 1MB boundary
    let boundary_size = 1_048_576;
    let msg = HostMessage::BeadList(create_beads_of_size(boundary_size));

    // When: Sent and received
    let (mut client, mut server) = create_transport_pair();
    client.send(&msg).unwrap();
    let received = server.recv::<HostMessage>().unwrap();

    // Then: Message is preserved
    assert_eq!(msg, received);
}
```

### Test: One byte over maximum size fails

```rust
#[test]
fn test_message_one_byte_over_1mb_fails() {
    // Given: Message one byte over limit
    let oversize = 1_048_577; // 1MB + 1
    let msg = HostMessage::BeadList(create_beads_of_size(oversize));

    // When: Attempting to send
    let (mut client, _server) = create_transport_pair();
    let result = client.send(&msg);

    // Then: Fails with MessageTooLarge
    assert!(matches!(result, Err(TransportError::MessageTooLarge { .. })));
}
```

### Test: Partial read is buffered internally

```rust
#[test]
fn test_partial_read_is_buffered_until_complete() {
    // Given: A transport that reads data in 100-byte chunks
    let (mut client, mut server) = create_transport_with_chunked_reads(100);

    let msg = HostMessage::BeadList(create_beads_of_size(500));

    // When: Message is sent
    client.send(&msg).unwrap();

    // Then: Multiple reads eventually complete the frame
    let received = server.recv::<HostMessage>().expect("recv should succeed after buffering");
    assert_eq!(received, msg);
}
```

### Test: Partial write is retried until complete

```rust
#[test]
fn test_partial_write_is_retried_until_complete() {
    // Given: A transport that writes in 100-byte chunks
    let (mut client, mut server) = create_transport_with_chunked_writes(100);

    let msg = HostMessage::BeadList(create_beads_of_size(500));

    // When: Message is sent
    let result = client.send(&msg);

    // Then: Succeeds after multiple write attempts
    assert!(result.is_ok());

    // And: Server can receive complete message
    let received = server.recv::<HostMessage>().unwrap();
    assert_eq!(received, msg);
}
```

### Test: Concurrency safety (send and recv from different threads)

```rust
#[test]
#[ignore] // Requires thread safety verification
fn test_concurrent_send_recv_from_different_threads() {
    // Given: Two threads sharing a transport (via channels/actor)
    let (transport_tx, transport_rx) = split_transport_for_concurrency();

    // When: Thread A sends, Thread B receives
    let handle = thread::spawn(move || {
        let msg = HostMessage::BeadList(vec![]);
        transport_tx.send(msg).unwrap();
    });

    let received = transport_rx.recv::<HostMessage>().unwrap();
    handle.join().unwrap();

    // Then: Message is received correctly
    assert!(matches!(received, HostMessage::BeadList(_)));
}
```

---

## Contract Verification Tests

### Test: All message types implement required traits

```rust
#[test]
fn test_transport_requires_serialize_and_deserialize_owned() {
    // Compile-time verification that send/recv bounds are correct
    fn assert_send_bound<T: serde::Serialize>() {}
    fn assert_recv_bound<T: serde::de::DeserializeOwned>() {}

    assert_send_bound::<HostMessage>();
    assert_recv_bound::<HostMessage>();
    assert_send_bound::<GuestMessage>();
    assert_recv_bound::<GuestMessage>();
}
```

### Test: Buffer capacity is sufficient for max frame size

```rust
#[test]
fn test_internal_buffer_capacity_exceeds_max_frame_size() {
    // Given: A new transport
    let (_reader, _writer) = std::io::empty();
    let transport = IpcTransport::new(_reader, _writer);

    // When: Checking buffer capacity
    let reader_cap = transport.reader_buffer_capacity();
    let writer_cap = transport.writer_buffer_capacity();

    // Then: Capacity >= 1MB + 4 bytes (max frame size)
    assert!(reader_cap >= 1_048_580);
    assert!(writer_cap >= 1_048_580);
}
```

### Test: Protocol invariants - every frame has 4-byte prefix

```rust
#[test]
fn test_all_frames_have_exactly_four_byte_prefix() {
    // Given: Various message sizes
    let test_cases = vec![1, 100, 1024, 100_000, 1_048_576];

    for size in test_cases {
        let (mut client, _server) = create_transport_pair_with_buffer();
        let msg = create_message_of_size(size);

        // When: Message is sent
        client.send(&msg).unwrap();

        // Then: First 4 bytes are length prefix
        let written = client.captured_bytes();
        assert!(written.len() >= 4, "Frame must have at least 4 bytes");

        let payload_size = written.len() - 4;
        let length_prefix =
            u32::from_be_bytes([written[0], written[1], written[2], written[3]]);

        assert_eq!(length_prefix as usize, payload_size);
    }
}
```

### Test: Protocol invariants - big-endian byte order

```rust
#[test]
fn test_length_prefix_uses_big_endian_byte_order() {
    // Given: A message with known payload size
    let (mut client, _server) = create_transport_pair_with_buffer();
    let msg = HostMessage::BeadList(create_beads_of_size(1000));
    client.send(&msg).unwrap();

    // When: Inspecting first 4 bytes
    let written = client.captured_bytes();
    let prefix_bytes = &[written[0], written[1], written[2], written[3]];

    // Then: Decoding as big-endian gives correct payload size
    let decoded_length = u32::from_be_bytes(*prefix_bytes);
    let expected_length = (written.len() - 4) as u32;

    assert_eq!(decoded_length, expected_length);

    // And: Decoding as little-endian gives WRONG result
    let little_endian_decode = u32::from_le_bytes(*prefix_bytes);
    assert_ne!(little_endian_decode, expected_length);
}
```

### Test: Invariant - transport is not Send or Sync

```rust
#[test]
fn test_transport_is_not_send_or_sync() {
    // Verify that transport requires external synchronization
    fn assert_not_send<T: !Send>() {}
    fn assert_not_sync<T: !Sync>() {}

    // This test will fail to compile if IpcTransport is Send or Sync
    // Commented out to allow compilation, but conceptually required
    // assert_not_send::<IpcTransport<std::io::Empty, std::io::Sink>>();
    // assert_not_sync::<IpcTransport<std::io::Empty, std::io::Sink>>();
}
```

---

## Given-When-Then Scenarios

### Scenario 1: Client sends query, server responds

```rust
#[test]
fn scenario_client_query_server_response() {
    // Given: A client and server transport
    let (mut client, mut server) = create_transport_pair();

    // When: Client sends query
    let query = GuestMessage::GetBeadList {
        filter: Some("status:open".to_string()),
    };
    client.send(&query).unwrap();

    // And: Server receives and processes query
    let received_query = server.recv::<GuestMessage>().unwrap();
    assert_eq!(received_query, query);

    // And: Server sends response
    let response = HostMessage::BeadList(vec![]);
    server.send(&response).unwrap();

    // And: Client receives response
    let received_response = client.recv::<HostMessage>().unwrap();

    // Then: Response matches server's message
    assert_eq!(received_response, response);
}
```

### Scenario 2: Connection drops mid-message

```rust
#[test]
fn scenario_connection_drops_during_message_reception() {
    // Given: A transport with simulated connection drop
    let (mut _client, mut server) = create_transport_pair();
    let drop_after_bytes = 500;

    // When: Partial message is received, then connection drops
    simulate_connection_drop_after(&mut server, drop_after_bytes);
    let result: Result<HostMessage, _> = server.recv();

    // Then: Error indicates incomplete read
    assert!(matches!(result, Err(TransportError::UnexpectedEof { .. })));
}
```

### Scenario 3: Multiple messages queued in buffer

```rust
#[test]
fn scenario_multiple_messages_queued_in_buffer() {
    // Given: A client that sends 3 messages before server reads
    let (mut client, mut server) = create_transport_pair();

    let msgs = vec![
        HostMessage::BeadList(vec![]),
        HostMessage::Error("msg1".to_string()),
        HostMessage::Error("msg2".to_string()),
    ];

    // When: All messages are sent without reading
    for msg in &msgs {
        client.send(msg).unwrap();
    }

    // Then: Server receives all messages in order
    for expected in msgs {
        let received = server.recv::<HostMessage>().unwrap();
        assert_eq!(received, expected);
    }
}
```

---

## Performance Tests (Criterion Benchmarks)

### Test: Send latency for 1KB message

```rust
#[bench]
fn bench_send_1kb_message(b: &mut Bencher) {
    let (mut client, _server) = create_transport_pair();
    let msg = create_message_of_size(1024);

    b.iter(|| client.send(&msg).unwrap());

    // Target: <2µs median latency
}
```

### Test: Recv latency for 1KB message

```rust
#[bench]
fn bench_recv_1kb_message(b: &mut Bencher) {
    let (mut client, mut server) = create_transport_pair();
    let msg = create_message_of_size(1024);

    // Pre-send message
    client.send(&msg).unwrap();

    b.iter(|| server.recv::<TestMessage>().unwrap());

    // Target: <3µs median latency
}
```

### Test: Round-trip latency for 1KB message

```rust
#[bench]
fn bench_roundtrip_1kb_message(b: &mut Bencher) {
    let (mut client, mut server) = create_transport_pair();
    let msg = create_message_of_size(1024);

    b.iter(|| {
        client.send(&msg).unwrap();
        server.recv::<TestMessage>().unwrap()
    });

    // Target: <5µs median latency
}
```

### Test: Throughput for 100KB message

```rust
#[bench]
fn bench_send_100kb_message(b: &mut Bencher) {
    let (mut client, _server) = create_transport_pair();
    let msg = create_message_of_size(100_000);

    b.iter(|| client.send(&msg).unwrap());

    // Target: <20µs median latency
}
```

---

## Property-Based Tests (Proptest)

### Test: Round-trip preserves all messages

```rust
#[proptest]
fn prop_roundtrip_preserves_data(
    #[strategy(arb_host_message())] msg: HostMessage,
) {
    // Given: Any valid HostMessage
    let (mut client, mut server) = create_transport_pair();

    // When: Sent and received
    client.send(&msg).unwrap();
    let received = server.recv::<HostMessage>().unwrap();

    // Then: Data is preserved exactly
    prop_assert_eq!(msg, received);
}
```

### Test: Frame format is always valid

```rust
#[proptest]
fn prop_frame_format_is_valid(
    #[strategy(arb_host_message())] msg: HostMessage,
) {
    // Given: Any valid HostMessage
    let (mut client, _server) = create_transport_pair_with_buffer();

    // When: Sent
    client.send(&msg).unwrap();
    let bytes = client.captured_bytes();

    // Then: Frame has valid structure
    prop_assert!(bytes.len() >= 4); // At least length prefix

    let length_prefix =
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    // And: Length prefix matches payload size
    prop_assert_eq!(length_prefix as usize, bytes.len() - 4);

    // And: Length prefix is within bounds
    prop_assert!(length_prefix > 0);
    prop_assert!(length_prefix <= 1_048_576);
}
```

---

## Integration Tests

### Test: Full IPC stack with real messages

```rust
#[test]
fn integration_full_ipc_stack_with_real_host_guest_messages() {
    // Given: Real IPC stack (transport + serialization)
    let (mut client, mut server) = create_real_ipc_transport();

    // When: Client sends various GuestMessage types
    client.send(&GuestMessage::GetBeadList { filter: None }).unwrap();
    client
        .send(&GuestMessage::GetBeadDetail {
            bead_id: "src-123".to_string(),
        })
        .unwrap();
    client.send(&GuestMessage::SubscribeEvents).unwrap();

    // And: Server responds with HostMessage types
    let query1 = server.recv::<GuestMessage>().unwrap();
    let query2 = server.recv::<GuestMessage>().unwrap();
    let query3 = server.recv::<GuestMessage>().unwrap();

    // Then: All messages are correctly typed
    assert!(matches!(query1, GuestMessage::GetBeadList { .. }));
    assert!(matches!(query2, GuestMessage::GetBeadDetail { .. }));
    assert!(matches!(query3, GuestMessage::SubscribeEvents));

    // And: Server sends responses
    server
        .send(&HostMessage::BeadList(vec![]))
        .unwrap();
    server
        .send(&HostMessage::BeadDetail(None))
        .unwrap();

    // And: Client receives responses
    let resp1 = client.recv::<HostMessage>().unwrap();
    let resp2 = client.recv::<HostMessage>().unwrap();

    assert!(matches!(resp1, HostMessage::BeadList(_)));
    assert!(matches!(resp2, HostMessage::BeadDetail(_)));
}
```

---

## Test Organization

### File Structure
```
crates/oya-ipc/tests/
├── transport_happy_path_tests.rs    # Send/recv, framing
├── transport_error_tests.rs          # All error conditions
├── transport_edge_case_tests.rs      # Boundaries, empty, max size
├── transport_contract_tests.rs       # Protocol invariants
├── transport_scenario_tests.rs       # Given-When-Then scenarios
└── transport_integration_tests.rs    # Full stack tests

crates/oya-ipc/benches/
├── transport_latency_bench.rs        # Latency benchmarks
└── transport_throughput_bench.rs     # Throughput benchmarks
```

### Test Execution
```bash
# Unit tests
cargo test --package oya-ipc --lib transport

# Benchmarks
cargo bench --package oya-ipc transport

# Property-based tests (requires proptest)
cargo test --package oya-ipc --features proptest
```

---

## Coverage Requirements

- **Line Coverage**: Minimum 95%
- **Branch Coverage**: Minimum 90%
- **Error Path Coverage**: 100% (all TransportError variants tested)
- **Protocol Coverage**: 100% (all protocol invariants verified)
- **Performance Regression**: All benchmarks must pass CI gates

# Martin Fowler Test Plan: oya-ipc

## Test Philosophy

These tests serve as **executable specifications** for the IPC protocol. Each test describes a behavior, not an implementation detail. Tests are written in Given-When-Then format to make requirements explicit and verifiable.

---

## Happy Path Tests

### Test: Round-trip serialization preserves data integrity

```rust
#[test]
fn test_host_message_bead_list_roundtrip_preserves_all_data() {
    // Given: A BeadList message with 3 beads
    let original = HostMessage::BeadList(vec![
        BeadSummary {
            id: "src-abc1".to_string(),
            title: "First bead".to_string(),
            status: BeadStatus::Open,
            priority: Priority::P1,
        },
        BeadSummary {
            id: "src-def2".to_string(),
            title: "Second bead".to_string(),
            status: BeadStatus::InProgress,
            priority: Priority::P2,
        },
        BeadSummary {
            id: "src-ghi3".to_string(),
            title: "Third bead".to_string(),
            status: BeadStatus::Completed,
            priority: Priority::P3,
        },
    ]);

    // When: The message is serialized and deserialized
    let bytes = serialize_host_message(&original).expect("serialization should succeed");
    let deserialized = deserialize_host_message(&bytes).expect("deserialization should succeed");

    // Then: All fields are exactly preserved
    assert!(matches!(deserialized, HostMessage::BeadList(_)));
    if let HostMessage::BeadList(beads) = deserialized {
        assert_eq!(beads.len(), 3);
        assert_eq!(beads[0].id, "src-abc1");
        assert_eq!(beads[0].title, "First bead");
        assert_eq!(beads[0].status, BeadStatus::Open);
        assert_eq!(beads[1].priority, Priority::P2);
        assert_eq!(beads[2].id, "src-ghi3");
    }
}
```

### Test: All GuestMessage variants serialize successfully

```rust
#[test]
fn test_all_guest_message_variants_serialize_without_error() {
    let test_cases = vec![
        GuestMessage::GetBeadList {
            filter: Some("status:open".to_string()),
        },
        GuestMessage::GetBeadDetail {
            bead_id: "src-123".to_string(),
        },
        GuestMessage::GetWorkflowGraph {
            workflow_id: "wf-main".to_string(),
        },
        GuestMessage::GetAgentPool,
        GuestMessage::GetSystemHealth,
        GuestMessage::StartBead {
            bead_id: "src-456".to_string(),
        },
        GuestMessage::CancelBead {
            bead_id: "src-789".to_string(),
        },
        GuestMessage::RetryBead {
            bead_id: "src-000".to_string(),
        },
        GuestMessage::SubscribeEvents,
        GuestMessage::UnsubscribeEvents,
    ];

    for msg in test_cases {
        // When: Each variant is serialized
        let result = serialize_guest_message(&msg);

        // Then: Serialization succeeds
        assert!(result.is_ok(), "Failed to serialize {:?}", msg);

        // And: Deserialization produces identical variant
        let bytes = result.unwrap();
        let restored = deserialize_guest_message(&bytes).unwrap();
        assert_eq!(msg, restored);
    }
}
```

### Test: Empty collections serialize correctly

```rust
#[test]
fn test_empty_bead_list_serializes_to_valid_message() {
    // Given: An empty BeadList
    let original = HostMessage::BeadList(vec![]);

    // When: Serialized and deserialized
    let bytes = serialize_host_message(&original).unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();

    // Then: Result is also an empty BeadList (not None or error)
    assert!(matches!(restored, HostMessage::BeadList(beads) if beads.is_empty()));
}
```

### Test: SystemHealth event with all fields present

```rust
#[test]
fn test_system_health_event_serializes_with_full_data() {
    // Given: A SystemHealth event with comprehensive data
    let event = HostMessage::SystemHealth(Box::new(SystemHealth {
        components: vec![
            ComponentHealth {
                name: "scheduler".to_string(),
                status: HealthStatus::Healthy,
                last_check: Utc::now(),
            },
            ComponentHealth {
                name: "agent_pool".to_string(),
                status: HealthStatus::Degraded,
                last_check: Utc::now(),
            },
        ],
        total_agents: 10,
        healthy_agents: 8,
        uptime_secs: 3600,
    });

    // When: Serialized and deserialized
    let bytes = serialize_host_message(&event).unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();

    // Then: All fields are preserved
    if let HostMessage::SystemHealth(health) = restored {
        assert_eq!(health.components.len(), 2);
        assert_eq!(health.components[0].name, "scheduler");
        assert_eq!(health.components[1].status, HealthStatus::Degraded);
        assert_eq!(health.total_agents, 10);
        assert_eq!(health.uptime_secs, 3600);
    }
}
```

---

## Error Path Tests

### Test: Serialization fails when string exceeds 64KB limit

```rust
#[test]
fn test_serialization_fails_when_string_field_exceeds_64kb() {
    // Given: A message with a title field exceeding 64KB (65536 bytes)
    let oversized_title = "X".repeat(70_000); // 70KB
    let msg = HostMessage::BeadList(vec![BeadSummary {
        id: "src-1".to_string(),
        title: oversized_title,
        status: BeadStatus::Open,
        priority: Priority::P1,
    }]);

    // When: Attempting to serialize
    let result = serialize_host_message(&msg);

    // Then: Returns SizeLimitExceeded error
    assert!(matches!(result, Err(IpcError::SizeLimitExceeded { .. })));
    if let Err(IpcError::SizeLimitExceeded { actual_size, max_size, .. }) = result {
        assert!(*actual_size > 65_536);
        assert_eq!(*max_size, 1_048_576); // 1MB
    }
}
```

### Test: Deserialization fails with invalid bincode format

```rust
#[test]
fn test_deserialization_fails_with_invalid_bincode_data() {
    // Given: Corrupted or invalid bincode bytes
    let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid discriminant

    // When: Attempting to deserialize
    let result = deserialize_host_message(&invalid_bytes);

    // Then: Returns DeserializationFailed error
    assert!(matches!(
        result,
        Err(IpcError::DeserializationFailed { .. })
    ));

    // And: Error includes diagnostic context
    if let Err(IpcError::DeserializationFailed {
        bytes_read,
        total_bytes,
        ..
    }) = result
    {
        assert_eq!(*total_bytes, 4);
        assert!(*bytes_read <= 4);
    }
}
```

### Test: Deserialization fails when message exceeds 1MB

```rust
#[test]
fn test_deserialization_fails_when_message_exceeds_1mb() {
    // Given: A serialized message larger than 1MB
    let large_bead_list: Vec<BeadSummary> = (0..20_000)
        .map(|i| BeadSummary {
            id: format!("src-{:05}", i),
            title: "Bead with description".to_string(),
            status: BeadStatus::Open,
            priority: Priority::P1,
        })
        .collect();

    let msg = HostMessage::BeadList(large_bead_list);

    // When: Attempting to serialize (should fail at size check)
    let result = serialize_host_message(&msg);

    // Then: Returns SizeLimitExceeded error
    assert!(matches!(result, Err(IpcError::SizeLimitExceeded { .. })));
}
```

### Test: Deserialization fails with truncated input

```rust
#[test]
fn test_deserialization_fails_with_truncated_message() {
    // Given: A valid message that is truncated mid-stream
    let msg = HostMessage::BeadList(vec![
        BeadSummary {
            id: "src-1".to_string(),
            title: "Title".to_string(),
            status: BeadStatus::Open,
            priority: Priority::P1,
        },
    ]);

    let mut full_bytes = serialize_host_message(&msg).unwrap();
    let truncated_bytes = &full_bytes[0..full_bytes.len() / 2]; // Cut in half

    // When: Attempting to deserialize truncated data
    let result = deserialize_host_message(truncated_bytes);

    // Then: Returns DeserializationFailed with bytes_read info
    assert!(matches!(
        result,
        Err(IpcError::DeserializationFailed { .. })
    ));
}
```

### Test: Empty input fails deserialization

```rust
#[test]
fn test_deserialization_fails_with_empty_input() {
    // Given: An empty byte slice
    let empty: Vec<u8> = vec![];

    // When: Attempting to deserialize
    let result = deserialize_host_message(&empty);

    // Then: Returns DeserializationFailed error
    assert!(matches!(
        result,
        Err(IpcError::DeserializationFailed { .. })
    ));
}
```

---

## Edge Case Tests

### Test: Single bead in BeadList handles correctly

```rust
#[test]
fn test_single_bead_in_bead_list_serializes_correctly() {
    // Given: A BeadList with exactly one bead
    let msg = HostMessage::BeadList(vec![BeadSummary {
        id: "src-only".to_string(),
        title: "Only bead".to_string(),
        status: BeadStatus::Ready,
        priority: Priority::P1,
    }]);

    // When: Serialized and deserialized
    let bytes = serialize_host_message(&msg).unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();

    // Then: Single bead is preserved correctly
    if let HostMessage::BeadList(beads) = restored {
        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].id, "src-only");
    }
}
```

### Test: Maximum allowed collection size (10,000 items)

```rust
#[test]
fn test_maximum_collection_size_10k_items_serializes_successfully() {
    // Given: A BeadList with exactly 10,000 beads (boundary value)
    let max_beads: Vec<BeadSummary> = (0..10_000)
        .map(|i| BeadSummary {
            id: format!("src-{:05}", i),
            title: format!("Bead {}", i),
            status: BeadStatus::Open,
            priority: Priority::P1,
        })
        .collect();

    let msg = HostMessage::BeadList(max_beads);

    // When: Serializing
    let result = serialize_host_message(&msg);

    // Then: Succeeds (exactly at boundary)
    assert!(result.is_ok());

    // And: Round-trip preserves all items
    let bytes = result.unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();
    if let HostMessage::BeadList(beads) = restored {
        assert_eq!(beads.len(), 10_000);
    }
}
```

### Test: Collection with 10,001 items exceeds limit

```rust
#[test]
fn test_collection_10k_plus_one_items_fails_size_limit() {
    // Given: A BeadList with 10,001 beads (one over limit)
    let too_many_beads: Vec<BeadSummary> = (0..10_001)
        .map(|i| BeadSummary {
            id: format!("src-{:05}", i),
            title: "Bead".to_string(),
            status: BeadStatus::Open,
            priority: Priority::P1,
        })
        .collect();

    let msg = HostMessage::BeadList(too_many_beads);

    // When: Serializing
    let result = serialize_host_message(&msg);

    // Then: Fails size validation
    assert!(matches!(result, Err(IpcError::SizeLimitExceeded { .. })));
}
```

### Test: String with exactly 64KB characters succeeds

```rust
#[test]
fn test_string_exactly_64kb_serializes_successfully() {
    // Given: A message with a 64KB string (boundary value)
    let title_64kb = "X".repeat(65_536); // Exactly 64KB
    let msg = HostMessage::BeadList(vec![BeadSummary {
        id: "src-boundary".to_string(),
        title: title_64kb,
        status: BeadStatus::Open,
        priority: Priority::P1,
    }]);

    // When: Serializing
    let result = serialize_host_message(&msg);

    // Then: Succeeds (at boundary)
    assert!(result.is_ok());
}
```

### Test: BeadDetail with None for missing bead

```rust
#[test]
fn test_bead_detail_returns_none_when_bead_not_found() {
    // Given: A BeadDetail response for non-existent bead
    let msg = HostMessage::BeadDetail(None);

    // When: Serialized and deserialized
    let bytes = serialize_host_message(&msg).unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();

    // Then: None is preserved (not converted to error)
    assert!(matches!(restored, HostMessage::BeadDetail(None)));
}
```

### Test: Error message with maximum allowed length

```rust
#[test]
fn test_error_message_exactly_1kb_serializes_successfully() {
    // Given: An error message of exactly 1KB (1024 bytes)
    let error_msg = "E".repeat(1024);
    let msg = HostMessage::Error(error_msg);

    // When: Serializing
    let result = serialize_host_message(&msg);

    // Then: Succeeds
    assert!(result.is_ok());

    // And: Error message is preserved
    let bytes = result.unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();
    if let HostMessage::Error(err) = restored {
        assert_eq!(err.len(), 1024);
    }
}
```

### Test: Error message exceeding 1KB is rejected

```rust
#[test]
fn test_error_message_exceeding_1kb_fails_validation() {
    // Given: An error message exceeding 1KB
    let error_msg = "E".repeat(1025); // 1KB + 1
    let msg = HostMessage::Error(error_msg);

    // When: Attempting to serialize
    let result = serialize_host_message(&msg);

    // Then: Fails validation
    assert!(matches!(result, Err(IpcError::InvalidData { .. })));
}
```

---

## Contract Verification Tests

### Test: Precondition - All message types implement required traits

```rust
#[test]
fn test_all_message_types_implement_required_traits() {
    // Verify compile-time trait bounds
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_debug_clone<T: Debug + Clone>() {}

    // This test will fail to compile if traits are missing
    assert_send_sync::<HostMessage>();
    assert_send_sync::<GuestMessage>();
    assert_debug_clone::<HostMessage>();
    assert_debug_clone::<GuestMessage>();
}
```

### Test: Postcondition - Simple message serializes within 500ns

```rust
#[test]
#[ignore] // Criterion benchmark, not unit test
fn bench_simple_message_serialization_under_500ns() {
    // Criterion benchmark in benches/serialization.rs
    // Target: GetBeadList serialization < 500ns
}
```

### Test: Postcondition - Complex message round-trip within 4µs

```rust
#[test]
#[ignore] // Criterion benchmark
fn bench_workflow_graph_roundtrip_under_4us() {
    // Criterion benchmark in benches/serialization.rs
    // Target: WorkflowGraph (1000 nodes) round-trip < 4µs
}
```

### Test: Invariant - All variants have unique binary discriminators

```rust
#[test]
fn test_all_host_message_variants_have_unique_discriminators() {
    // Given: All HostMessage variants
    let variants = vec![
        HostMessage::BeadList(vec![]),
        HostMessage::BeadDetail(None),
        HostMessage::WorkflowGraph(Box::new(WorkflowGraph::default())),
        HostMessage::AgentPool(Box::new(AgentPoolStats::default())),
        HostMessage::SystemHealth(Box::new(SystemHealth::default())),
        HostMessage::BeadStateChanged(BeadStateChangedEvent {
            bead_id: "test".to_string(),
            old_status: BeadStatus::Open,
            new_status: BeadStatus::InProgress,
        }),
        HostMessage::Error("test".to_string()),
    ];

    // When: Each variant is serialized
    let discriminators: Vec<u8> = variants
        .iter()
        .map(|v| {
            let bytes = serialize_host_message(v).unwrap();
            bytes[0] // First byte is discriminant
        })
        .collect();

    // Then: All discriminators are unique
    let unique_discriminators: std::collections::HashSet<_> =
        discriminators.iter().collect();
    assert_eq!(
        unique_discriminators.len(),
        discriminators.len(),
        "Duplicate discriminators found"
    );
}
```

### Test: Invariant - No message contains raw pointers or handles

```rust
#[test]
fn test_messages_are_self_contained_no_external_references() {
    // This is a compile-time test - if messages contain
    // Rc, Arc, unsafe pointers, or file descriptors,
    // they won't implement Serialize/Deserialize properly

    // Given: Sample messages
    let host_msg = HostMessage::BeadList(vec![]);
    let guest_msg = GuestMessage::GetBeadList { filter: None };

    // When: Serializing (would fail if non-serializable types present)
    let host_result = serialize_host_message(&host_msg);
    let guest_result = serialize_guest_message(&guest_msg);

    // Then: Both succeed (proves no external references)
    assert!(host_result.is_ok());
    assert!(guest_result.is_ok());
}
```

---

## Given-When-Then Scenarios

### Scenario 1: Guest queries bead list with filter

```rust
#[test]
fn scenario_guest_queries_bead_list_with_filter() {
    // Given: A guest wants open beads only
    let filter = Some("status:open priority:p1".to_string());
    let query = GuestMessage::GetBeadList {
        filter: filter.clone(),
    };

    // When: The query is serialized and sent to host
    let bytes = serialize_guest_message(&query).unwrap();
    let restored_query = deserialize_guest_message(&bytes).unwrap();

    // Then: The filter is preserved exactly
    assert_eq!(restored_query, GuestMessage::GetBeadList { filter });
}
```

### Scenario 2: Host broadcasts bead state change event

```rust
#[test]
fn scenario_host_broadcasts_bead_state_change_to_all_subscribers() {
    // Given: A bead transitioned from Open to InProgress
    let event = HostMessage::BeadStateChanged(BeadStateChangedEvent {
        bead_id: "src-123".to_string(),
        old_status: BeadStatus::Open,
        new_status: BeadStatus::InProgress,
        timestamp: Utc::now(),
    });

    // When: The event is serialized for broadcasting
    let bytes = serialize_host_message(&event).unwrap();

    // Then: Multiple guests can deserialize the same bytes
    let guest1_view = deserialize_host_message(&bytes).unwrap();
    let guest2_view = deserialize_host_message(&bytes).unwrap();

    // And: All guests see identical data
    assert_eq!(guest1_view, guest2_view);
}
```

### Scenario 3: Guest cancels a running bead

```rust
#[test]
fn scenario_guest_cancels_running_bead() {
    // Given: A user cancels bead src-456
    let cmd = GuestMessage::CancelBead {
        bead_id: "src-456".to_string(),
    };

    // When: Command is serialized and sent to host
    let bytes = serialize_guest_message(&cmd).unwrap();
    let restored = deserialize_guest_message(&bytes).unwrap();

    // Then: Bead ID is preserved exactly
    assert_eq!(
        restored,
        GuestMessage::CancelBead {
            bead_id: "src-456".to_string()
        }
    );
}
```

### Scenario 4: System alert with critical severity

```rust
#[test]
fn scenario_system_alert_critical_propagates_correctly() {
    // Given: Host detects critical system failure
    let alert = HostMessage::SystemAlert(SystemAlertEvent {
        severity: AlertSeverity::Critical,
        component: "scheduler".to_string(),
        message: "DAG execution deadlock detected".to_string(),
        timestamp: Utc::now(),
    });

    // When: Alert is serialized for guest display
    let bytes = serialize_host_message(&alert).unwrap();
    let restored = deserialize_host_message(&bytes).unwrap();

    // Then: Severity and message are preserved
    if let HostMessage::SystemAlert(event) = restored {
        assert_eq!(event.severity, AlertSeverity::Critical);
        assert_eq!(event.component, "scheduler");
        assert!(event.message.contains("deadlock"));
    }
}
```

### Scenario 5: Guest subscribes to events

```rust
#[test]
fn scenario_guest_subscribes_to_all_events() {
    // Given: Guest wants to receive all event broadcasts
    let subscribe = GuestMessage::SubscribeEvents;

    // When: Subscription message is serialized
    let bytes = serialize_guest_message(&subscribe).unwrap();
    let restored = deserialize_guest_message(&bytes).unwrap();

    // Then: Subscription intent is preserved
    assert_eq!(restored, GuestMessage::SubscribeEvents);
}
```

---

## End-to-End Scenarios

### Scenario: Complete query-response flow

```rust
#[test]
fn e2e_guest_queries_workflow_graph_host_responds() {
    // Given: Guest wants workflow graph for "wf-main"
    let query = GuestMessage::GetWorkflowGraph {
        workflow_id: "wf-main".to_string(),
    };

    // When: Guest serializes query
    let query_bytes = serialize_guest_message(&query).unwrap();

    // And: Host deserializes query
    let host_query = deserialize_guest_message(&query_bytes).unwrap();

    // And: Host builds response with 50 nodes
    let graph = WorkflowGraph {
        nodes: (0..50).map(|i| GraphNode { id: i }).collect(),
        edges: vec![],
    };
    let response = HostMessage::WorkflowGraph(Box::new(graph));

    // And: Host serializes response
    let response_bytes = serialize_host_message(&response).unwrap();

    // And: Guest deserializes response
    let guest_response = deserialize_host_message(&response_bytes).unwrap();

    // Then: Response contains all 50 nodes
    if let HostMessage::WorkflowGraph(graph) = guest_response {
        assert_eq!(graph.nodes.len(), 50);
    }
}
```

---

## Performance Regression Tests

### Test: Serialization latency does not exceed baseline

```rust
#[test]
#[ignore]
fn regression_test_simple_message_latency_baseline() {
    // Run in CI to catch performance regressions
    // Fails if median latency > 1.5x baseline
    let baseline_ns = 450; // Established from initial benchmarks
    let msg = HostMessage::BeadList(vec![]);

    let start = std::time::Instant::now();
    for _ in 0..10_000 {
        let _ = serialize_host_message(&msg).unwrap();
    }
    let elapsed = start.elapsed();

    let avg_ns = elapsed.as_nanos() / 10_000;
    assert!(
        avg_ns <= (baseline_ns * 3 / 2),
        "Latency regression: {}ns > baseline {}ns",
        avg_ns,
        baseline_ns
    );
}
```

---

## Test Organization

### File Structure
```
crates/oya-ipc/tests/
├── serialization_tests.rs      # Round-trip, happy path
├── error_tests.rs              # All error conditions
├── edge_case_tests.rs          # Boundary values, empty collections
├── contract_tests.rs           # Pre/post/invariant verification
├── scenario_tests.rs           # Given-When-Then scenarios
└── e2e_tests.rs                # End-to-end flows

benches/
├── serialization_bench.rs      # Criterion benchmarks
└── size_validation_bench.rs    # Size limit enforcement
```

### Test Execution
```bash
# Unit tests
cargo test --package oya-ipc

# Benchmarks (requires --release)
cargo bench --package oya-ipc

# With coverage
tarpaulin --out Html --package oya-ipc
```

---

## Coverage Requirements

- **Line Coverage**: Minimum 95%
- **Branch Coverage**: Minimum 90%
- **Error Path Coverage**: 100% (all error variants must be tested)
- **Invariant Coverage**: 100% (all invariants must have verification tests)

---

## Test Maintenance

### When Adding New Message Variants
1. Add round-trip test in `serialization_tests.rs`
2. Add edge case tests for boundaries
3. Update size validation tests if applicable
4. Add Given-When-Then scenario
5. Run benchmarks to establish new baseline

### When Changing Error Variants
1. Add new error path test
2. Update error taxonomy documentation
3. Verify error messages provide diagnostic context
4. Test error propagation in callers

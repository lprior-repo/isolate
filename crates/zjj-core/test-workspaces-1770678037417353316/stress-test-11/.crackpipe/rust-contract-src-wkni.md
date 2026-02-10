# Rust Contract: src-wkni

## Overview
Generated from bead: src-wkni

## Functional Requirements
# Host IPC Worker

## Overview
Create IPC worker actor in oya-orchestrator that manages Zellij plugin connections, handles GuestMessage commands, and broadcasts HostMessage events to subscribers.

## Architecture

```
IpcWorker (Actor)
  ├─ Transport: IpcTransport<ChildStdout, ChildStdin>
  ├─ Orchestrator: Arc<Orchestrator>
  └─ Subscribers: Vec<Sender<HostMessage>>
```

## Message Handlers

Implement handle() for all GuestMessage types:
- **GetBeadList**: Query BeadStore, return BeadList
- **GetBeadDetail**: Query bead by ID, return BeadDetail
- **GetWorkflowGraph**: Query DAG, return WorkflowGraph
- **GetAgentPool**: Query agent pool, return AgentPoolStats
- **GetSystemHealth**: Query health, return SystemHealth
- **StartBead/CancelBead/RetryBead**: Execute command, return Ack

## Event Broadcasting

Subscribe to orchestrator events and broadcast:
- BeadStateChanged → all subscribers
- PhaseProgress → all subscribers
- AgentHeartbeat → all subscribers
- SystemAlert → all subscribers

## Implementation

Create crates/orchestrator/src/actors/ipc_worker.rs:

```rust
pub struct IpcWorker {
    transport: IpcTransport<...>,
    orchestrator: Arc<Orchestrator>,
    event_tx: broadcast::Sender<HostMessage>,
}

#[async_trait]
impl Actor for IpcWorker {
    async fn start(&mut self) -> Result<()>;
    async fn handle(&mut self, msg: GuestMessage) -> Result<HostMessage>;
}
```

## Lifecycle
1. Accept Zellij connection (spawn plugin process)
2. Spawn IpcWorker actor
3. Message loop: recv → handle → send response
4. Event loop: subscribe to events → broadcast
5. Graceful shutdown on disconnect

## Performance Targets
- Query handling: <10µs (p99)
- Event broadcast: <5µs per subscriber
- Max 100 concurrent connections

## Testing
- Unit tests for each GuestMessage handler
- Integration test with mock Zellij guest
- Event broadcast to multiple subscribers
- Connection lifecycle (spawn, crash, reconnect)

## API Contract

### Types
- Define all public structs and enums
- Must derive: Debug, Clone, Serialize, Deserialize
- Zero unwraps, zero panics

### Functions
- All functions return Result<T, E>
- Use functional patterns (map, and_then, ?)
- Document error cases

## Performance Constraints
- Specify latency targets
- Memory constraints
- Throughput requirements

## Testing Requirements
- Unit tests for all public functions
- Integration tests for workflows
- Property-based tests for invariants

## Implementation Notes
- Use functional-rust patterns
- Railway-oriented programming
- Error handling over panics

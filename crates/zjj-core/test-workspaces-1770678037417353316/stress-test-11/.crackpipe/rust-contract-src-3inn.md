# Rust Contract: Guest IPC Client for Zellij WASM Plugin

**Bead ID**: src-3inn  
**Component**: oya-ui/src/ipc/client.rs  
**Actor**: architect-replacement-1  
**Date**: 2026-02-07

## 1. Overview

Implement a robust IPC client for the oya-ui WASM plugin that enables bidirectional communication with the oya-orchestrator host via Zellij's stdin/stdout buffers.

### Key Constraints
- **WASM Environment**: No async runtime available - use sync I/O with blocking reads
- **Performance**: Send <2µs, Receive <3µs, Reconnect <1s
- **Reliability**: Exponential backoff reconnection, command preservation across reconnects
- **Zero Panic**: All error paths must return `Result`, never panic

## 2. Dependencies

### Existing Crates
- `oya-ipc`: Provides `IpcTransport<R, W>` for length-prefixed bincode messages
- `serde`: For message serialization/deserialization
- `bincode`: For binary encoding

### External Dependencies (to be added)
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
thiserror = "1.0"
tokio = { version = "1.0", optional = true }  # For mock host testing
```

## 3. Message Protocol Definition

### File: `crates/oya-ipc/src/messages.rs` (NEW)

```rust
//! Message protocol for guest-host IPC

use serde::{Deserialize, Serialize};

/// Commands sent from guest (oya-ui plugin) to host (oya-orchestrator)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GuestMessage {
    /// Subscribe to workflow updates for a specific bead
    SubscribeBead { bead_id: String },
    
    /// Unsubscribe from bead updates
    UnsubscribeBead { bead_id: String },
    
    /// Request current workflow graph state
    GetWorkflowGraph,
    
    /// Request pipeline status for all agents
    GetPipelineStatus,
    
    /// Request agent health metrics
    GetAgentMetrics,
    
    /// Ping for connection health check
    Ping,
}

/// Events sent from host (oya-orchestrator) to guest (oya-ui plugin)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HostMessage {
    /// Bead status update
    BeadUpdate {
        bead_id: String,
        status: String,
        stage: String,
        timestamp: u64,
    },
    
    /// Workflow graph state
    WorkflowGraph {
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
    },
    
    /// Pipeline status update
    PipelineStatus {
        agent_id: String,
        status: String,
        current_task: Option<String>,
    },
    
    /// Agent metrics update
    AgentMetrics {
        agent_id: String,
        cpu_usage: f64,
        memory_usage: u64,
        task_count: usize,
    },
    
    /// Response to GuestMessage commands
    Ack { success: bool, message: String },
    
    /// Pong response to ping
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}
```

## 4. Client Implementation

### File: `crates/oya-ui/src/ipc/client.rs` (NEW)

### 4.1 Core Structure

```rust
//! IPC client for Zellij WASM plugin

use crate::ipc::messages::{GuestMessage, HostMessage};
use oya_ipc::{IpcTransport, TransportError};
use std::io::{Stdin, Stdout};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// IPC client for guest-host communication
/// 
/// # Thread Safety
/// - `!Send + !Sync` (WASM single-threaded)
/// - Must be externally synchronized if used in multi-threaded context
pub struct IpcClient {
    /// Transport layer for message I/O
    transport: IpcTransport<Stdin, Stdout>,
    
    /// Reconnection state
    reconnect_state: ReconnectState,
    
    /// Connection health flag
    connected: bool,
}

/// Reconnection state with exponential backoff
#[derive(Debug)]
struct ReconnectState {
    /// Current backoff delay in milliseconds
    current_delay_ms: u64,
    
    /// Maximum backoff delay (5 seconds)
    max_delay_ms: u64,
    
    /// Minimum backoff delay (100ms)
    min_delay_ms: u64,
    
    /// Reconnection attempt count
    attempt_count: u32,
}

impl ReconnectState {
    pub fn new() -> Self {
        Self {
            current_delay_ms: 100,
            max_delay_ms: 5000,
            min_delay_ms: 100,
            attempt_count: 0,
        }
    }
    
    /// Calculate next backoff delay with exponential increase
    pub fn next_delay(&mut self) -> Duration {
        self.attempt_count += 1;
        
        // Exponential backoff: 100ms * 2^attempt_count
        let delay = self.min_delay_ms * 2u64.pow(self.attempt_count.saturating_sub(1));
        
        // Cap at max_delay
        let delay = delay.min(self.max_delay_ms);
        self.current_delay_ms = delay;
        
        Duration::from_millis(delay)
    }
    
    /// Reset backoff state (called on successful connection)
    pub fn reset(&mut self) {
        self.current_delay_ms = self.min_delay_ms;
        self.attempt_count = 0;
    }
    
    /// Get current backoff delay
    pub fn current_delay(&self) -> Duration {
        Duration::from_millis(self.current_delay_ms)
    }
}
```

### 4.2 Error Handling

```rust
use std::io;

/// IPC client errors
#[derive(Debug, thiserror::Error)]
pub enum IpcClientError {
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Connection lost - host disconnected")]
    ConnectionLost,
    
    #[error("Reconnection failed after {0} attempts")]
    ReconnectFailed(u32),
    
    #[error("Unexpected message type: expected {expected}, got {actual}")]
    UnexpectedMessage { expected: String, actual: String },
    
    #[error("Client not connected")]
    NotConnected,
}

pub type IpcClientResult<T> = Result<T, IpcClientError>;
```

### 4.3 API Implementation

```rust
impl IpcClient {
    /// Create new IPC client connected to Zellij stdin/stdout
    /// 
    /// # Preconditions
    /// - Zellij stdin is readable
    /// - Zellij stdout is writable
    /// 
    /// # Postconditions
    /// - Returns Ok(IpcClient) if initialization succeeds
    /// - Returns Err if stream initialization fails
    /// - Client is in connected state
    pub fn new() -> IpcClientResult<Self> {
        let transport = IpcTransport::new(std::io::stdin(), std::io::stdout());
        
        Ok(Self {
            transport,
            reconnect_state: ReconnectState::new(),
            connected: true,
        })
    }
    
    /// Send command to host and wait for response
    /// 
    /// # Preconditions
    /// - Client is in connected state
    /// - cmd is a valid GuestMessage
    /// 
    /// # Postconditions
    /// - Returns Ok(HostMessage) with response
    /// - Returns Err if send fails or connection lost
    /// - Updates connection state based on result
    /// 
    /// # Performance
    /// - Must complete <2µs for small messages
    /// - Must complete <20µs for large messages (100KB)
    pub fn send_command(&mut self, cmd: GuestMessage) -> IpcClientResult<HostMessage> {
        if !self.connected {
            return Err(IpcClientError::NotConnected);
        }
        
        // Send command
        self.transport.send(&cmd)?;
        
        // Wait for response
        let response = self.recv_message()?;
        
        Ok(response)
    }
    
    /// Receive a single message from host (blocking)
    /// 
    /// # Preconditions
    /// - Client is in connected state
    /// 
    /// # Postconditions
    /// - Returns Ok(HostMessage) when message received
    /// - Returns Err(IpcClientError::ConnectionLost) on EOF
    /// - Blocks until message available or timeout
    /// 
    /// # Performance
    /// - Must complete <3µs for 1KB message
    /// - Must complete <30µs for 100KB message
    pub fn recv_message(&mut self) -> IpcClientResult<HostMessage> {
        if !self.connected {
            return Err(IpcClientError::NotConnected);
        }
        
        match self.transport.recv::<HostMessage>() {
            Ok(msg) => Ok(msg),
            Err(TransportError::UnexpectedEof { .. }) => {
                self.connected = false;
                Err(IpcClientError::ConnectionLost)
            }
            Err(e) => Err(e.into()),
        }
    }
    
    /// Check if client is connected to host
    /// 
    /// # Postconditions
    /// - Returns true if connection is active
    /// - Returns false if disconnected or reconnecting
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    /// Attempt to reconnect to host with exponential backoff
    /// 
    /// # Preconditions
    /// - Client is in disconnected state
    /// 
    /// # Postconditions
    /// - Returns Ok(()) if reconnection succeeds
    /// - Returns Err if max attempts exceeded
    /// - Resets backoff state on success
    /// 
    /// # Performance
    /// - Must complete <1s for successful reconnect
    /// - Must use exponential backoff: 100ms → 5s
    pub fn reconnect(&mut self) -> IpcClientResult<()> {
        const MAX_ATTEMPTS: u32 = 10;
        
        for attempt in 0..MAX_ATTEMPTS {
            let delay = self.reconnect_state.next_delay();
            
            // Wait before attempt (skip for first attempt)
            if attempt > 0 {
                std::thread::sleep(delay);
            }
            
            // Attempt to detect liveness by sending ping
            match self.attempt_reconnect() {
                Ok(_) => {
                    self.reconnect_state.reset();
                    self.connected = true;
                    return Ok(());
                }
                Err(_) if attempt < MAX_ATTEMPTS - 1 => {
                    // Continue retrying
                    continue;
                }
                Err(e) => {
                    return Err(IpcClientError::ReconnectFailed(MAX_ATTEMPTS));
                }
            }
        }
        
        Err(IpcClientError::ReconnectFailed(MAX_ATTEMPTS))
    }
    
    /// Attempt single reconnection
    fn attempt_reconnect(&mut self) -> IpcClientResult<()> {
        // Check if stdin is still open
        if self.transport.is_eof() {
            return Err(IpcClientError::ConnectionLost);
        }
        
        // Try to receive a message (will fail if still disconnected)
        // Use small timeout to avoid blocking
        // Note: In WASM, we can't use timeout, so we do a non-blocking check
        
        Ok(())
    }
}
```

### 4.4 Module Declaration

Update `crates/oya-ui/src/lib.rs`:

```rust
pub mod components;
pub mod layout;
pub mod plugin;
pub mod render;

// NEW: IPC module
pub mod ipc;

// Re-exports
pub use ipc::client::{IpcClient, IpcClientError, IpcClientResult};
```

Create `crates/oya-ui/src/ipc/mod.rs`:

```rust
//! IPC module for guest-host communication

pub mod client;
pub mod messages;  // Re-export from oya-ipc

pub use client::{IpcClient, IpcClientError, IpcClientResult};
pub use oya_ipc::messages::{GuestMessage, HostMessage};
```

## 5. Performance Requirements

### 5.1 Timing Constraints
All methods MUST meet these performance targets:

| Operation | Target | Measurement |
|-----------|--------|-------------|
| `send_command` (1KB) | <2µs | Wall-clock time |
| `send_command` (100KB) | <20µs | Wall-clock time |
| `recv_message` (1KB) | <3µs | Wall-clock time |
| `recv_message` (100KB) | <30µs | Wall-clock time |
| `reconnect` (success) | <1s | Including backoff |
| `is_connected` | <100ns | Direct flag check |

### 5.2 Memory Constraints
- Max pending commands: 100 (during reconnect)
- Buffer size: 1MB (enforced by IpcTransport)
- Zero allocations on hot path (send/recv)

## 6. Testing Requirements

### 6.1 Unit Tests (覆盖率 >90%)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_construction() {
        let client = IpcClient::new();
        assert!(client.is_ok());
        assert!(client.unwrap().is_connected());
    }
    
    #[test]
    fn test_send_command_success() {
        // Test with mock transport
    }
    
    #[test]
    fn test_send_command_when_disconnected_returns_error() {
        // Test error handling
    }
    
    #[test]
    fn test_reconnect_with_exponential_backoff() {
        // Test backoff timing: 100ms, 200ms, 400ms, 800ms, 1600ms, 3200ms, 5000ms
    }
    
    #[test]
    fn test_reconnect_resets_backoff_on_success() {
        // Test state reset
    }
    
    #[test]
    fn test_recv_message_returns_eof_on_disconnect() {
        // Test connection loss detection
    }
}
```

### 6.2 Integration Tests

```rust
// File: crates/oya-ui/tests/ipc_integration_tests.rs

#[test]
fn test_bidirectional_communication_with_mock_host() {
    // Create mock host, send commands, verify responses
}

#[test]
fn test_reconnection_after_host_crash() {
    // Simulate host crash, verify reconnect succeeds
}

#[test]
fn test_concurrent_command_sending() {
    // Test multiple commands in sequence
}

#[test]
fn test_large_message_within_limits() {
    // Test 1MB message
}

#[test]
fn test_message_exceeding_1mb_returns_error() {
    // Test error handling for oversized messages
}
```

## 7. Acceptance Criteria

### 7.1 Functional Requirements
- [ ] Client successfully sends GuestMessage commands
- [ ] Client successfully receives HostMessage events
- [ ] Reconnection works with exponential backoff (100ms → 5s)
- [ ] Connection loss detected via EOF
- [ ] Pending commands preserved across reconnect
- [ ] All error paths return Result, no panics

### 7.2 Non-Functional Requirements
- [ ] `send_command` <2µs for 1KB messages (measured with criterion)
- [ ] `recv_message` <3µs for 1KB messages (measured with criterion)
- [ ] `reconnect` <1s for successful reconnect
- [ ] Zero clippy warnings (except allowed lints)
- [ ] Zero unsafe code
- [ ] Test coverage >90%

### 7.3 Integration Requirements
- [ ] Works with oya-ipc IpcTransport
- [ ] Compatible with GuestMessage/HostMessage protocol
- [ ] Integrates with oya-ui plugin lifecycle
- [ ] Mock host for testing works correctly

## 8. Constraints & Invariants

### 8.1 MUST (Violations are bugs)
- Zero panics in production code
- Zero unwrap/expect calls
- All errors use IpcClientResult<T>
- WASM-compatible (no async runtime)
- Reconnect MUST use exponential backoff
- Message size MUST NOT exceed 1MB

### 8.2 MUST NOT (Violations are bugs)
- MUST NOT use async/await (WASM limitation)
- MUST NOT block indefinitely on recv
- MUST NOT lose pending commands on reconnect
- MUST NOT use unsafe code

### 8.3 SHOULD (Best practices)
- SHOULD prefer functional patterns (map, and_then)
- SHOULD provide clear error messages
- SHOULD log reconnection attempts
- SHOULD expose connection state for monitoring

## 9. Dependencies on Other Beads

This bead depends on:
- **src-38tm**: oya-ipc crate with message protocol (creates GuestMessage/HostMessage types)
- **src-1xvj**: IpcTransport implementation (provides transport layer)

## 10. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test Coverage | >90% | cargo-llvm-cov |
| Performance | All targets met | criterion benchmarks |
| Clippy | Zero warnings | cargo clippy |
| Documentation | All public items documented | rustdoc |
| Integration Tests | All passing | cargo test |

## 11. Open Questions

1. **Question**: How to handle timeout in WASM environment without async runtime?
   - **Answer**: Use non-blocking I/O checks in event loop

2. **Question**: Should pending commands be persisted to disk?
   - **Answer**: No, keep in memory for simplicity (WASM storage limited)

3. **Question**: How to detect host liveness beyond EOF check?
   - **Answer**: Implement periodic ping/pong health checks

## 12. Implementation Phases

### Phase 1: Foundation (Week 1)
- Create message types in oya-ipc
- Implement basic IpcClient structure
- Add error types

### Phase 2: Core Functionality (Week 1-2)
- Implement send_command
- Implement recv_message
- Add connection state tracking

### Phase 3: Reliability (Week 2)
- Implement reconnection logic
- Add exponential backoff
- Handle connection loss detection

### Phase 4: Testing (Week 2-3)
- Write unit tests
- Write integration tests
- Add benchmarks
- Validate performance targets

## 13. Review Checklist

Before marking this bead as complete, verify:

- [ ] All public APIs have documentation
- [ ] All error cases are tested
- [ ] Performance benchmarks meet targets
- [ ] Zero clippy warnings
- [ ] Test coverage >90%
- [ ] Integration tests pass
- [ ] Code review approved
- [ ] Documentation complete

---

**Contract Version**: 1.0  
**Last Updated**: 2026-02-07  
**Status**: Ready for Implementation

# Rust Contract: src-3ax5

## Overview
Generated from bead: src-3ax5

## Functional Requirements
# Zellij WASM Plugin Scaffold

## Overview
Create oya-ui Zellij WASM plugin with proper Cargo.toml configuration, layout system, and basic pane rendering using Zellij plugin SDK.

## Project Structure

```
crates/oya-ui/
├── Cargo.toml              # WASM configuration
├── src/
│   ├── main.rs             # Plugin entry point
│   ├── ipc/
│   │   └── client.rs       # IPC client (from NEW-004)
│   ├── layout.rs           # Pane layout definitions
│   ├── components/         # UI components
│   └── render.rs           # Terminal rendering
└── oya.kll                 # Zellij layout file
```

## Cargo.toml Configuration

```toml
[package]
name = "oya-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
zellij = "0.40"              # Zellij plugin SDK
bincode = "2.0"
serde = { version = "1.0", features = ["derive"] }
oya-ipc = { path = "../oya-ipc" }
crossterm = "0.28"           # Terminal rendering

[profile.release]
opt-level = "z"             # Optimize for size
lto = true
codegen-units = 1
```

## Plugin Implementation

src/main.rs:
```rust
use zellij::plugin::{ZellijPlugin, PluginInfo};
use crossterm::terminal::Size;

struct OyaPlugin {
    client: IpcClient,
    size: Size,
}

impl ZellijPlugin for OyaPlugin {
    fn new(info: PluginInfo) -> Self {
        Self {
            client: IpcClient::new().unwrap(),
            size: info.size,
        }
    }

    fn render(&mut self, _rows: usize, _cols: usize) -> String {
        // Initial layout: 3 panes
        // ┌─────────────────────────────────┐
        // │ BeadList      │ BeadDetail       │
        // │               ├─────────────────┤
        // │               │ PipelineView     │
        // ├───────────────┴─────────────────┤
        // │ WorkflowGraph                   │
        └─────────────────────────────────┘
        
        self.render_layout()
    }
    
    fn update(&mut self, event: Event) {
        // Handle keyboard input, resize, etc.
    }
}
```

## Layout System

Implement 3-pane layout:
1. **Left**: BeadList (40% width)
2. **Right Top**: BeadDetail + PipelineView (60% width, 60% height)
3. **Bottom**: WorkflowGraph (full width, 40% height)

Use ANSI boxdrawing characters:
```
┌─┬─┐
│ │ │
├─┼─┤
│ │ │
└─┴─┘
```

## Compilation

```bash
# Add wasm32-wasi target
rustup target add wasm32-wasi

# Build WASM
cargo build --release --target wasm32-wasi

# Output: target/wasm32-wasi/release/oya_ui.wasm
```

## Zellij Integration

Create oya.kll layout file:
```kll
layout {
    default_tab {
        pane name="oya" {
            plugin location="file:target/wasm32-wasi/release/oya_ui.wasm"
        }
    }
}
```

Load in Zellij:
```bash
zellij --layout oya.kll
```

## Performance Targets
- WASM binary: <5MB
- Plugin load: <100ms
- Initial render: <50ms
- Responsive to input: <10ms

## Testing
- WASM compiles without errors
- Loads in Zellij
- Renders 3-pane layout
- Handles terminal resize
- Basic keyboard input works

## Next Steps
After scaffold:
- Integrate IpcClient (from NEW-004)
- Implement BeadList component
- Add vim navigation
- Wire up real data from orchestrator

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

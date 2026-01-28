# ZJJ - JJ Workspace + Zellij Session Manager

ZJJ is a workspace isolation and setup tool that combines [JJ (Jujutsu)](https://github.com/martinvonz/jj) version control with [Zellij](https://zellij.dev/) terminal multiplexing for focused development sessions.

## What ZJJ Does

ZJJ creates **isolated workspaces** for parallel development tasks:
- Each workspace is a separate JJ branch with a dedicated Zellij tab
- Seamlessly switch between tasks with `zjj focus`
- Keep your main branch clean while working on multiple features
- Built-in agent workflow support with `zjj spawn` and `zjj done`

## Commands

### Core Session Management
| Command | Description |
|---------|-------------|
| `zjj init` | Initialize zjj in a JJ repository |
| `zjj add <name>` | Create a new session with JJ workspace + Zellij tab |
| `zjj list` | List all sessions |
| `zjj remove <name>` | Remove a session and its workspace |
| `zjj focus <name>` | Switch to a session's Zellij tab |

### Session Operations
| Command | Description |
|---------|-------------|
| `zjj status [name]` | Show detailed session status |
| `zjj sync [name]` | Sync session workspace with main (rebase) |
| `zjj diff <name>` | Show diff between session and main |
| `zjj attach <name>` | Attach to an existing Zellij session |
| `zjj clean` | Remove stale sessions |

### Agent Workflow
| Command | Description |
|---------|-------------|
| `zjj spawn <bead-id>` | Spawn isolated workspace for a bead and run agent |
| `zjj done` | Complete work and merge workspace to main |

### System & Diagnostics
| Command | Description |
|---------|-------------|
| `zjj config [key] [value]` | View or modify configuration |
| `zjj doctor` | Run system health checks |
| `zjj introspect [cmd]` | Discover zjj capabilities and command details |
| `zjj query <type>` | Query system state programmatically |
| `zjj context` | Show complete environment context |
| `zjj dashboard` | Launch interactive TUI dashboard |

All commands support `--json` flag for machine-readable output.

## Quick Start

```bash
# Initialize ZJJ in a JJ repository
zjj init

# Create a session for a feature
zjj add auth-refactor

# List all sessions
zjj list

# Switch to the session
zjj focus auth-refactor

# When done, clean up
zjj remove auth-refactor
```

## ⚡ Hyper-Fast CI/CD Pipeline

This project uses **Moon** + **bazel-remote** for a production-grade CI/CD pipeline with **98.5% faster** cached builds:

### 🚀 Performance
- **6-7ms** cached task execution (vs ~450ms cold)
- **100GB local cache** with zstd compression
- **Parallel task execution** across all crates
- **Persistent cache** survives clean/rebuild cycles

### 🛠️ Build System
- **Moon v1.41.8**: Modern build orchestrator
- **bazel-remote v2.6.1**: High-performance cache backend
- **Native binary**: No Docker overhead
- **User service**: Auto-starts on login, no sudo required

### ✅ Pipeline Stages
1. **Format Check** (`moon run :fmt`) - Verify code formatting
2. **Linting** (`moon run :clippy`) - Strict Clippy checks
3. **Type Check** (`moon run :check`) - Fast compilation check
4. **Testing** (`moon run :test`) - Full test suite with nextest
5. **Build** (`moon run :build`) - Release builds
6. **Security** (`moon run :audit`) - Dependency audits

### 📊 Typical Development Loop
```bash
# Edit code...
moon run :fmt :check  # 6-7ms with cache! ⚡
```

See [docs/CI-CD-PERFORMANCE.md](docs/CI-CD-PERFORMANCE.md) for detailed benchmarks and optimization guide.

## Installation

### Prerequisites

- **Moon** - Install from https://moonrepo.dev/docs/install
- **bazel-remote** - For local caching (setup below)
- **JJ** (Jujutsu) - Install from https://github.com/martinvonz/jj#installation
- **Zellij** - Install from https://zellij.dev/download
- **Rust** 1.80 or later

### From Source (with Moon)

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Install Moon (if not already installed)
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Build with Moon
moon run :build

# Run the binary
./target/release/zjj --help
```

**Important**: All commands in this project must be run through Moon. Do not use `cargo` directly.

### Usage

```bash
# Initialize ZJJ in a JJ repository
zjj init

# Create a new session
zjj add my-session

# List all sessions
zjj list

# Focus on a session
zjj focus my-session

# Remove a session
zjj remove my-session
```

## Development

**All commands must be run through Moon.** This project uses Moon for build orchestration with bazel-remote for hyper-fast local caching.

### Async Architecture & Database

ZJJ uses **async/await** with Tokio runtime and **sqlx** for all database operations. This provides:

- **Non-blocking database access** - UI remains responsive during queries
- **Connection pooling** - Efficient reuse of database connections
- **Better error handling** - Railway-oriented error propagation

**For Contributors:**
- All command functions accessing database are `async fn`
- Use `.await` on all database operations
- Database connection is via `SqlitePool` (not direct `Connection`)
- Main function uses `#[tokio::main]` to provide async runtime

**Example Pattern:**
```rust
// In command handler
pub async fn run(args: Args, ctx: &Context) -> Result<()> {
    let db = get_session_db().await?;
    let sessions = db.list(None).await?;
    ctx.output_json(&sessions)
}
```

### Quick Development Loop

```bash
# Format and type-check (6-7ms with cache!)
moon run :quick

# Full pipeline (parallel execution)
moon run :ci

# Individual tasks
moon run :fmt        # Check formatting
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check
moon run :test       # Run tests
moon run :build      # Release build
```

### Cache Setup (bazel-remote)

The project uses bazel-remote for local caching at `http://localhost:9092`:

```bash
# View cache stats
curl http://localhost:9090/status | jq

# Monitor cache in real-time
watch -n 1 'curl -s http://localhost:9090/status | jq'

# Restart cache service (if needed)
systemctl --user restart bazel-remote

# View cache logs
journalctl --user -u bazel-remote -f
```

### Available Moon Tasks

```bash
moon run :fmt        # Format check
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Type check only
moon run :test       # Run all tests
moon run :build      # Release build
moon run :ci         # Full CI pipeline
moon run :quick      # fmt + check (fastest)
```

## Contributing

Contributions are welcome! Please follow the existing code style and submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.
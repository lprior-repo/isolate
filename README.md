# ZJJ - JJ Workspace + Zellij Session Manager

ZJJ is a powerful tool that combines [JJ (Just Join)](https://github.com/martinvonz/jj) version control with [Zellij](https://zellij.dev/) terminal sessions for an enhanced development workflow.

## Features

- **JJ Integration**: Seamlessly manage JJ workspaces
- **Zellij Sessions**: Create and manage Zellij tabs for each session
- **Session Management**: Add, remove, list, and focus sessions
- **JSON Output Support**: All commands support JSON output for scripting
- **Error Handling**: Comprehensive error handling with user-friendly messages

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

## Getting Started

### Prerequisites

- Rust 1.80 or later
- **Moon** (install from https://moonrepo.dev/docs/install)
- **bazel-remote** (auto-installed via setup script)
- Zellij (for session management)
- SQLite (for database operations)
- CUE (for schema validation)

### Installation

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Install Moon (if not already installed)
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Setup hyper-fast local cache (one-time setup)
bash /tmp/install-bazel-remote-user.sh  # Created during development

# Build the project with Moon
moon run :build

# Install the binary
moon run :install  # Copies to ~/.local/bin/zjj
```

**Note**: The bazel-remote cache service runs as a systemd user service and auto-starts on login.

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
moon run :test       # Run tests with nextest
moon run :clippy     # Linting
moon run :build      # Release build
```

### Cache Management

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

### Performance Benchmarking

```bash
# Benchmark cache performance
time moon run :fmt :check  # First run (cache miss)
time moon run :fmt :check  # Second run (cache hit - should be <10ms!)
```

## CI/CD Configuration

The project uses **Moon** for all CI/CD operations with hyper-fast caching:

### Local Development
- **bazel-remote** runs as systemd user service
- **gRPC cache** at `localhost:9092` (zero network latency)
- **100GB cache** with zstd compression
- **6-7ms** task execution with cache hits

### CI Environment (Future)
```yaml
# .github/workflows/ci.yml (example)
- uses: moonrepo/setup-toolchain@v0
- run: moon ci --base origin/main --head HEAD
  env:
    CACHE_TOKEN: ${{ secrets.CACHE_TOKEN }}  # Optional remote cache
```

### Pipeline Stages
1. **Format** (`~:fmt`) - Parallel formatting check
2. **Lint** (`~:clippy`) - Parallel linting
3. **Test** (`~:test`) - Parallel test execution
4. **Build** (`build`) - Sequential release build
5. **Docs** (`build-docs`) - Generate documentation

All stages with `~:` prefix run **in parallel** for maximum speed.

See [docs/CI-CD-PERFORMANCE.md](docs/CI-CD-PERFORMANCE.md) for detailed configuration and optimization guide.

## Contributing

Contributions are welcome! Please follow the existing code style and submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.
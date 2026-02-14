# ZJJ — Parallel Workspace Isolation + Queue Coordination (JJ + Zellij)

[![Coverage](https://codecov.io/gh/lprior-repo/zjj/branch/main/graph/badge.svg)](https://codecov.io/gh/lprior-repo/zjj)
[![CI](https://github.com/lprior-repo/zjj/actions/workflows/ci.yml/badge.svg)](https://github.com/lprior-repo/zjj/actions/workflows/ci.yml)

ZJJ helps you run **multiple parallel workstreams (humans or AI agents)** against a single repo **without stepping on each other**.

It combines:
- **JJ (Jujutsu) workspaces** for hard isolation
- **Zellij** tabs/sessions for fast context switching
- A **SQLite-backed state + merge/processing queue** for coordination, retries, and recovery

If you want "run 6–12 coding agents safely" *or* "work on 5 features at once without trashing main," ZJJ is for that.

---

## Why ZJJ exists

Parallel work is easy to start and hard to finish cleanly:

- two workers edit the same area → conflicts
- multiple tasks get half-done → you lose track
- "who is working on what?" becomes tribal knowledge
- agents can duplicate work or race each other

ZJJ fixes this by making parallelism **explicit, isolated, and coordinated**.

**Before**: 6 agents race on the same working copy → duplicated effort + conflicts  
**After**: each agent gets an isolated workspace + the queue enforces safe claiming/landing

---

## Mental model

**Session** = a named isolated workspace (+ optional bead/issue) + optional Zellij tab  
**Queue entry** = a unit of work tied to a workspace that a worker/agent can claim and process  
**Done** = finish the work and land it back to main

## Quick reference

### Core session commands
- `zjj init` — initialize ZJJ in a JJ repo
- `zjj add <name>` — create an isolated session (workspace + Zellij tab)
- `zjj add <name> --bead <BEAD_ID>` — create a session tied to an issue/bead
- `zjj list [--verbose]` — list sessions
- `zjj status [name]` — detailed status + changes
- `zjj focus <name>` — jump to that session's Zellij tab
- `zjj sync [name]` — rebase/sync workspace onto main
- `zjj done` — complete work and merge to main
- `zjj remove <name>` — remove session + workspace
- `zjj whereami` — show current location (main or workspace)
- `zjj switch [name]` — switch between workspaces (interactive if no name)
- `zjj diff <name>` — show diff between session and main
- `zjj attach <name>` — attach to an existing Zellij session
- `zjj clean` — remove stale sessions

> All commands support `--json` for machine-readable output.

### Queue (multi-worker / multi-agent coordination)
- `zjj queue --add <workspace> --bead <BEAD_ID> [--priority N] [--agent AGENT_ID]`
- `zjj queue --list`
- `zjj queue --next`
- `zjj queue --status <workspace>`
- `zjj queue --retry <ID>`
- `zjj queue --cancel <ID>`
- `zjj queue --remove <ID>`
- `zjj queue --reclaim-stale [seconds]`
- `zjj queue worker --once | --loop`

### System & Diagnostics
- `zjj config [key] [value]` — view or modify configuration
- `zjj doctor` — run system health checks
- `zjj introspect [cmd]` — discover zjj capabilities and command details
- `zjj query <type>` — query system state programmatically
- `zjj context` — show complete environment context
- `zjj dashboard` — launch interactive TUI dashboard

---

## 60-second quick start

```bash
# 1) inside a JJ repo
zjj init

# 2) create an isolated session
zjj add auth-refactor --bead BD-123

# 3) jump into it (Zellij tab)
zjj focus auth-refactor

# 4) keep it synced with main
zjj sync auth-refactor

# 5) finish and land the work
zjj done

# 6) optionally clean it up
zjj remove auth-refactor
```

### Multi-agent workflow (example)
```bash
# Add multiple work items
zjj queue --add feature-a --bead BD-101 --priority 3
zjj queue --add feature-b --bead BD-102 --priority 5 --agent agent-002

# Start workers (these can be human-driven or agent-driven wrappers)
zjj queue worker --loop
```

ZJJ ensures only one worker claims a given entry at a time, and provides:
- retries for failures
- cancel/remove operations
- reclaiming stale leases when workers crash

---

## Reliability notes

ZJJ stores its state in a local database and includes a corruption recovery policy:
- `warn` (default), `silent`, or `fail-fast`
- Configurable via flag/env/config, with recovery logging.

See [Recovery Policy](#recovery-policy) section below for details.

---

## Documentation

Comprehensive documentation is available in the `/docs` directory:

- **[docs/00_START_HERE.md](docs/00_START_HERE.md)** - Start here
- **[docs/INDEX.md](docs/INDEX.md)** - Complete documentation index

### Key Topics

| Topic | Document |
|-------|----------|
| Getting Started | [00_START_HERE.md](docs/00_START_HERE.md) |
| Error Handling | [01_ERROR_HANDLING.md](docs/01_ERROR_HANDLING.md) |
| Build & Test | [02_MOON_BUILD.md](docs/02_MOON_BUILD.md) |
| Zellij Layouts | [11_ZELLIJ.md](docs/11_ZELLIJ.md) |
| JJ Workspaces | [09_JUJUTSU.md](docs/09_JUJUTSU.md) |
| Issue Triage | [08_BEADS.md](docs/08_BEADS.md) |

---

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



## Development & Contributing

**All commands must be run through Moon.** This project uses Moon for build orchestration with bazel-remote for hyper-fast local caching.

### ⚡ Hyper-Fast CI/CD Pipeline

This project uses **Moon** + **bazel-remote** for a production-grade CI/CD pipeline with **98.5% faster** cached builds:

#### 🚀 Performance
- **6-7ms** cached task execution (vs ~450ms cold)
- **100GB local cache** with zstd compression
- **Parallel task execution** across all crates
- **Persistent cache** survives clean/rebuild cycles

#### 🛠️ Build System
- **Moon v1.41.8**: Modern build orchestrator
- **bazel-remote v2.6.1**: High-performance cache backend
- **Native binary**: No Docker overhead
- **User service**: Auto-starts on login, no sudo required

#### ✅ Pipeline Stages
1. **Format Check** (`moon run :fmt`) - Verify code formatting
2. **Linting** (`moon run :clippy`) - Strict Clippy checks
3. **Type Check** (`moon run :check`) - Fast compilation check
4. **Testing** (`moon run :test`) - Full test suite with nextest
5. **Build** (`moon run :build`) - Release builds
6. **Security** (`moon run :audit`) - Dependency audits

#### 📊 Typical Development Loop
```bash
# Edit code...
moon run :fmt :check  # 6-7ms with cache! ⚡
```

See [docs/CI-CD-PERFORMANCE.md](docs/CI-CD-PERFORMANCE.md) for detailed benchmarks and optimization guide.

### Recovery Policy

ZJJ detects and handles database corruption based on a **recovery policy** that controls whether corruption is silently fixed, warned about, or treated as a fatal error.

#### Policy Modes

| Mode | Behavior | Use Case |
|-------|-----------|-----------|
| `silent` | Recovers from corruption without warning (default in older versions) | Development, testing |
| `warn` | ⚠ Shows warning message, then recovers (new default) | Production systems where auto-recovery is acceptable |
| `fail-fast` | ✗ Fails immediately on corruption, no recovery | CI/CD, strict production environments |

#### Configuration

Recovery policy can be configured in three ways (higher priority overrides lower):

1. **CLI flag**: `zjj --strict <command>` (sets fail-fast)
2. **Environment variable**: `ZJJ_RECOVERY_POLICY=silent|warn|fail-fast`
3. **Config file**: Add to `.zjj/config.toml`:
   ```toml
   [recovery]
   policy = "warn"  # or "silent", "fail-fast"
   ```

#### Recovery Logging

When recovery occurs, ZJJ logs all recovery actions to `.zjj/recovery.log`:
```
[2026-01-27T20:30:00Z] Database corruption detected at: .zjj/state.db. Recovered silently.
[2026-01-27T20:31:15Z] Database corruption detected at: .zjj/state.db. Recovering by recreating database file.
```

Logging can be controlled via:
- Environment variable: `ZJJ_RECOVERY_LOG=1` (default) or `ZJJ_RECOVERY_LOG=0` (disable)
- Config file: `recovery.log_recovered = true` in `.zjj/config.toml`

#### Doctor Exit Codes

The `zjj doctor` command uses these exit codes:
- `0`: System healthy (all checks passed)
- `1`: System unhealthy (one or more checks failed)
- `2`: Recovery occurred (system recovered from corruption, review `.zjj/recovery.log`)

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

### Combative Ralph Loop

Run a long Red-Queen hardening loop that keeps stress-testing `zjj`, patching failures, and re-running quality gates.

```bash
# Default (claude-code + Claude Opus, min=30, max=200)
bash scripts/run_ralph_combative_loop.sh

# Or via Moon
moon run :ralph-combative-loop
```

Useful overrides:

- `RALPH_AGENT` (default: `claude-code`)
- `RALPH_MODEL` (default: `anthropic/claude-opus-4-5`)
- `RALPH_MIN_ITERATIONS` (default: `30`)
- `RALPH_MAX_ITERATIONS` (default: `200`)
- `RALPH_NO_COMMIT` (default: `1`)

Completion promise default: `COMBATIVE_LOOP_COMPLETE`.

Contributions are welcome! Please follow the existing code style and submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.

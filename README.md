# ZJJ - Session-Based Development with JJ + Zellij

[![Build Status](https://img.shields.io/github/actions/workflow/status/lprior-repo/zjj/ci.yml?branch=main)](https://github.com/lprior-repo/zjj/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-nightly-orange)](https://www.rust-lang.org)

> Seamlessly manage isolated development sessions using JJ (Jujutsu) workspaces and Zellij terminal multiplexing.

ZJJ combines the power of [Jujutsu](https://github.com/martinvonz/jj) version control with [Zellij](https://zellij.dev) terminal sessions, giving you isolated workspaces for each feature, bug fix, or experiment.

## Platform Support

**Supported Platforms**:
- ✅ **Linux** - Fully supported and tested (primary platform)
- ✅ **macOS** - Expected to work (untested, please report issues)
- ⚠️ **Windows** - Limited support (use WSL2 recommended)

**Windows Limitation**: Zellij does not support Windows. ZJJ requires Zellij for terminal multiplexing. Windows users should use WSL2.

See [docs/14_PLATFORM_SUPPORT.md](docs/14_PLATFORM_SUPPORT.md) for complete platform compatibility matrix.

## Quick Start

### Prerequisites

#### Required Versions

- **[JJ (Jujutsu)](https://github.com/martinvonz/jj) 0.8.0 or later**
  - Required features: `workspace add`, `workspace forget`, `workspace list`, `root`
  - Workspace support introduced in v0.8.0 (July 2023)
  - Check version: `jj --version`

- **[Zellij](https://zellij.dev) 0.35.1 or later**
  - Required features: KDL layouts (v0.32.0+), `go-to-tab-name` action (v0.35.1+)
  - KDL layout format introduced in v0.32.0 (October 2022)
  - Tab switching by name introduced in v0.35.1 (March 2023)
  - Check version: `zellij --version`

- **Rust Nightly** (required for building from source)
  - ZJJ requires nightly Rust due to advanced tracing features
  - The project includes `rust-toolchain.toml` which automatically uses the correct version
  - Install: `rustup toolchain install nightly`
  - Check version: `rustc --version`

#### Optional

- **[Beads](https://github.com/beadorg/beads)** - Issue tracker (recommended)
- **SQLite 3.x** - For session database (typically pre-installed)

### Installation

```bash
# Clone repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Build with Moon (recommended)
moon run :build

# Or build with Cargo
cargo build --release

# Install binary
cargo install --path crates/zjj

# Verify installation
jjz --version
```

### 5-Minute Tutorial

```bash
# 1. Initialize ZJJ in your JJ repository
cd /path/to/your/jj/repo
jjz init

# 2. Create your first session (creates JJ workspace + Zellij tab)
jjz add feature-auth

# 3. Work on your feature (JJ tracks changes automatically)
# ZJJ switches to a dedicated Zellij tab named "jjz:feature-auth"
vim src/auth.rs
jj describe -m "feat: add authentication"

# 4. List all sessions
jjz list

# 5. Switch between sessions
jjz focus another-session

# 6. Sync with main branch
jjz sync feature-auth

# 7. When done, cleanup the session
jjz remove feature-auth
```

## Core Concepts

### What is a Session?

A **session** is a unified development context consisting of:

- **JJ Workspace**: Isolated working directory for your changes
- **Zellij Tab**: Dedicated terminal tab (named `jjz:<session-name>`)
- **Database Record**: Session metadata stored in `.jjz/sessions.db`

```
Session "feature-auth"
├── JJ workspace: /workspaces/feature-auth/
├── Zellij tab: jjz:feature-auth
└── Database: .jjz/sessions.db
```

### Why Use Sessions?

- **Isolation**: Each feature/bug lives in its own workspace
- **Context Switching**: Jump between tasks without mental overhead
- **Clean History**: Keep branches focused and rebased on main
- **Terminal Organization**: Dedicated Zellij tabs per session

## Commands

### Core Commands

| Command | Description | Example |
|---------|-------------|---------|
| `init` | Initialize ZJJ in a JJ repository | `jjz init` |
| `add <name>` | Create a new session | `jjz add feature-auth` |
| `list` | Show all sessions | `jjz list` |
| `focus <name>` | Switch to session's Zellij tab | `jjz focus feature-auth` |
| `remove <name>` | Delete session and cleanup | `jjz remove feature-auth` |

### Additional Commands

| Command | Description |
|---------|-------------|
| `status [name]` | Show session status |
| `sync [name]` | Rebase session on main |
| `diff <name>` | Show changes vs main |
| `config [key] [value]` | View/modify configuration |
| `dashboard` | Launch TUI kanban view |
| `doctor` | Run health checks |

### Command Examples

```bash
# Create session and link to Beads issue
jjz add feature-oauth --issue BD-456

# List only active sessions
jjz list --status active

# Remove session and merge changes
jjz remove feature-auth --merge

# Show diff between session and main
jjz diff feature-auth

# View configuration
jjz config

# Set workspace directory
jjz config workspace_dir /custom/path

# Launch interactive dashboard (TUI)
jjz dashboard

# Check system dependencies
jjz doctor
```

## Typical Workflow

```bash
# 1. Start with an issue (using Beads)
bd claim BD-789

# 2. Create session for the work
jjz add fix-validation

# 3. Make changes (JJ automatically tracks)
vim src/validator.rs
jj describe -m "fix: improve validation logic

Closes BD-789"

# 4. Test your changes
moon run :test

# 5. Keep in sync with main
jjz sync fix-validation

# 6. Need to switch tasks? Create another session
jjz add urgent-hotfix
# Work on hotfix...
jjz focus fix-validation  # Back to original work

# 7. Done? Cleanup
jjz remove fix-validation --merge

# 8. Close issue
bd complete BD-789
```

## Configuration

ZJJ uses a layered configuration system (higher priority first):

1. Command-line flags
2. Environment variables (`ZJJ_*`)
3. Project config (`.jjz/config.toml`)
4. Global config (`~/.config/zjj/config.toml`)
5. Defaults

### Example Configuration

Create `.jjz/config.toml`:

```toml
# Custom workspace directory
workspace_dir = "/home/user/dev/workspaces"

[zellij]
# Use Zellij tabs for sessions
use_tabs = true

[hooks]
# Run script after creating session
post_create = "echo 'Session created: $SESSION_NAME'"

# Cleanup hook before removing session
pre_remove = "./scripts/backup-session.sh $SESSION_NAME"
```

### Available Hooks

| Hook | Triggered | Environment Variables |
|------|-----------|----------------------|
| `post_create` | After `jjz add` | `$SESSION_NAME`, `$WORKSPACE_PATH` |
| `pre_remove` | Before `jjz remove` | `$SESSION_NAME` |
| `post_sync` | After `jjz sync` | `$SESSION_NAME` |
| `on_focus` | After `jjz focus` | `$SESSION_NAME` |

## Project Structure

```
zjj/
├── crates/
│   ├── zjj-core/       # Core library (JJ/Zellij/Beads integrations)
│   └── zjj/            # CLI binary (commands + persistence)
├── docs/               # Documentation (architecture, patterns, guides)
├── schemas/            # CUE schemas for validation
└── README.md           # This file
```

## Building from Source

ZJJ uses [Moon](https://moonrepo.dev/) for builds (required):

```bash
# Quick lint check
moon run :quick

# Run tests
moon run :test

# Release build
moon run :build

# Full CI pipeline
moon run :ci

# Format code
moon run :fmt-fix
```

**Never use raw cargo commands** - always use Moon for consistency.

## Session Lifecycle

```
Creating → Active → [Paused] → Completed
   ↓         ↓          ↓
 Failed ← Failed ← Failed
```

- **Creating**: Session is being initialized
- **Active**: Workspace and tab are ready
- **Paused**: Session is inactive but preserved
- **Completed**: Work is merged and session archived
- **Failed**: Error occurred during lifecycle

## Code Quality

ZJJ follows strict Rust standards:

- **Zero Unwrap Law**: No `.unwrap()`, `.expect()`, `panic!()`, or `unsafe` code
- **Functional Patterns**: Railway-oriented programming with `Result` combinators
- **Type Safety**: Strong types prevent invalid states at compile time
- **Comprehensive Testing**: All core logic has unit and integration tests

See [docs/05_RUST_STANDARDS.md](docs/05_RUST_STANDARDS.md) for details.

## Documentation

| Document | Purpose |
|----------|---------|
| [00_START_HERE.md](docs/00_START_HERE.md) | Quick onboarding guide |
| [11_ARCHITECTURE.md](docs/11_ARCHITECTURE.md) | System architecture |
| [12_AI_GUIDE.md](docs/12_AI_GUIDE.md) | AI-assisted development |
| [14_PLATFORM_SUPPORT.md](docs/14_PLATFORM_SUPPORT.md) | Platform compatibility matrix |
| [01_ERROR_HANDLING.md](docs/01_ERROR_HANDLING.md) | Error handling patterns |
| [02_MOON_BUILD.md](docs/02_MOON_BUILD.md) | Build system usage |
| [08_BEADS.md](docs/08_BEADS.md) | Beads integration |
| [09_JUJUTSU.md](docs/09_JUJUTSU.md) | JJ integration details |

See [docs/INDEX.md](docs/INDEX.md) for the complete documentation index.

## FAQ

### Why use JJ instead of Git?

JJ provides automatic change tracking, safer rebasing, and better handling of work-in-progress. Every edit is tracked without manual commits.

### Do I need to be in Zellij?

Yes. ZJJ is designed to work inside Zellij sessions and manages tabs for you.

### Can I use this with Git?

ZJJ is designed for JJ, but JJ has Git interop. You can use `jj git push/fetch` for Git remotes.

### What if I delete a session by accident?

Session removal is intentional and requires confirmation. Use `jjz status <name>` before removing.

### How do I integrate with CI/CD?

ZJJ sessions are local development tools. For CI, use standard JJ/Git workflows.

## Troubleshooting

```bash
# Check system dependencies and configuration
jjz doctor

# View session details
jjz status <session-name>

# Check ZJJ database location
jjz config

# Enable debug logging
RUST_LOG=debug jjz <command>
```

### Common Issues

**Error: Database file does not exist**
```bash
# Solution: Initialize ZJJ first
jjz init
```

**Error: Not running in Zellij**
```bash
# Solution: Start Zellij first
zellij
jjz <command>
```

**Error: JJ repository not found**
```bash
# Solution: Initialize JJ repository
jj init --git
jjz init
```

## Contributing

We welcome contributions! Please see our development guides:

1. Read [docs/00_START_HERE.md](docs/00_START_HERE.md)
2. Check [docs/03_WORKFLOW.md](docs/03_WORKFLOW.md) for the development workflow
3. Follow the [Zero Unwrap Law](docs/05_RUST_STANDARDS.md)
4. Use Moon for all builds: `moon run :ci`

## License

MIT License - see [LICENSE](LICENSE) for details.

Copyright (c) 2026 ZJJ Contributors

## Acknowledgments

- [Jujutsu](https://github.com/martinvonz/jj) - Next-generation version control
- [Zellij](https://zellij.dev) - Terminal workspace manager
- [Beads](https://github.com/beadorg/beads) - Integrated issue tracking

---

**Get Started**: `jjz init && jjz add my-first-session`

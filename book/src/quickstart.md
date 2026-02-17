# Quick Start

Get up and running with ZJJ in 5 minutes.

---

## Prerequisites

Before you start, ensure you have these installed:

| Tool | Version | Install |
|------|---------|---------|
| **JJ (Jujutsu)** | 0.20+ | [Install guide](https://github.com/martinvonz/jj#installation) |
| **Zellij** | 0.39+ | [Install guide](https://zellij.dev/download) |
| **Rust** | 1.80+ | [Install via rustup](https://rustup.rs/) |

---

## Installation

### From source (recommended)

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Build with Moon
moon run :build

# Install the binary
moon run :install

# Verify
zjj --version
```

### From crates.io

```bash
cargo install zjj
```

---

## Your first workspace

### 1. Initialize ZJJ

```bash
# Inside a JJ repository
zjj init
```

This creates the `.zjj/` directory with the state database.

### 2. Create a workspace

```bash
zjj add feature-auth --bead BD-123
```

This creates:
- An isolated JJ workspace
- A Zellij tab for quick switching
- Association with bead `BD-123` for tracking

### 3. Jump into it

```bash
zjj focus feature-auth
```

Or check where you are:

```bash
zjj whereami
# workspace:feature-auth
```

### 4. Do your work

Edit files in your isolated workspace. Main stays untouched.

```bash
vim src/auth.rs
```

### 5. Keep synced

```bash
zjj sync
```

This rebases your workspace onto the latest main.

### 6. Complete and land

```bash
zjj done --message "Add authentication" --push
```

---

## Common commands

| What you want to do | Command |
|---------------------|---------|
| Check where you are | `zjj whereami` |
| Create a workspace | `zjj add <name>` |
| List all workspaces | `zjj list` |
| Switch workspaces | `zjj focus <name>` |
| Sync with main | `zjj sync` |
| Complete work | `zjj done` |
| Clean up workspace | `zjj remove <name>` |

---

## Multi-agent quick start

Run multiple AI agents in parallel.

### 1. Add tasks to the queue

```bash
zjj queue --add feature-a --bead BD-101 --priority 5
zjj queue --add feature-b --bead BD-102 --priority 3
zjj queue --add feature-c --bead BD-103 --priority 1
```

### 2. View the queue

```bash
zjj queue --list
```

### 3. Claim work

```bash
# Agent 1
zjj work --agent agent-001

# Agent 2
zjj work --agent agent-002
```

### 4. Monitor progress

```bash
zjj queue --status
```

---

## Keyboard shortcuts

When using Zellij:

| Shortcut | Action |
|----------|--------|
| `Ctrl+p` then `t` | New tab |
| `Ctrl+p` then `n` | Next tab |
| `Ctrl+p` then `h/l` | Navigate tabs |
| `Ctrl+p` then `w` | Close tab |

Use `zjj focus <name>` to jump directly to a workspace tab.

---

## Configuration

ZJJ stores configuration in `.zjj/config.toml`.

### View configuration

```bash
zjj config
```

### Common settings

```toml
[recovery]
policy = "warn"
log_recovered = true

[queue]
default_priority = 5
stale_timeout_seconds = 3600
```

---

## Troubleshooting

### "Not in a JJ repository"

Initialize JJ first:

```bash
jj init && zjj init
```

### "Database corruption detected"

Run diagnostics:

```bash
zjj doctor
```

### "Zellij not found"

Install Zellij:

```bash
# macOS
brew install zellij

# Linux
cargo install zellij
```

---

## Next steps

<div class="quickstart-cards">
  <div class="card">
    <h4>Workspace Management</h4>
    <p>Learn the full workspace lifecycle</p>
    <a href="./guide/workspaces.html">Read guide →</a>
  </div>
  
  <div class="card">
    <h4>Queue Coordination</h4>
    <p>Coordinate multiple agents and tasks</p>
    <a href="./guide/queue.html">Learn queues →</a>
  </div>
  
  <div class="card">
    <h4>AI Agent Guide</h4>
    <p>Set up parallel AI workflows</p>
    <a href="./ai/overview.html">AI guide →</a>
  </div>
  
  <div class="card">
    <h4>Command Reference</h4>
    <p>All commands and options</p>
    <a href="./reference/commands.html">Browse →</a>
  </div>
</div>

---

## Getting help

- 📖 [User Guide](./guide/workspaces.md)
- 🐛 [Report issues](https://github.com/lprior-repo/zjj/issues)
- 💬 [Discussions](https://github.com/lprior-repo/zjj/discussions)

# Quick Start

Get up and running with ZJJ in 5 minutes.

---

## Prerequisites

Before you start, make sure you have:

- ✅ **JJ (Jujutsu)** - [Install from here](https://github.com/martinvonz/jj#installation)
- ✅ **Zellij** - [Install from here](https://zellij.dev/download)
- ✅ **Rust** 1.80 or later - [Install from here](https://rustup.rs/)

<div class="info">
💡 <strong>Tip</strong>: ZJJ requires a JJ repository to work. If you don't have one yet, run <code>jj init</code> in your project directory.
</div>

---

## Installation

### Option 1: From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Install Moon (if not already installed)
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Build with Moon
moon run :build

# Install the binary
moon run :install

# Verify installation
zjj --version
```

### Option 2: From crates.io

```bash
cargo install zjj
```

<div class="warning">
⚠️ <strong>Note</strong>: The crates.io version may lag behind the latest features. Building from source is recommended for the best experience.
</div>

---

## 60-Second Workflow

Here's a complete workflow from start to finish:

### 1. Initialize ZJJ

```bash
# Inside a JJ repository
zjj init
```

This creates the `.zjj/` directory with the state database.

### 2. Create an Isolated Session

```bash
# Create a session for your work
zjj add auth-refactor --bead BD-123
```

This creates:
- A new JJ workspace named `auth-refactor`
- A Zellij tab for this workspace
- Optional association with bead/issue `BD-123`

### 3. Jump Into Your Workspace

```bash
# Switch to the session's Zellij tab
zjj focus auth-refactor
```

Or check where you are:

```bash
zjj whereami
# Output: workspace:auth-refactor
```

### 4. Do Your Work

Make changes in your isolated workspace. Your main branch stays clean!

```bash
# Edit files
vim src/auth.rs

# Check status
zjj status auth-refactor
```

### 5. Keep Synced with Main

```bash
# Rebase your workspace onto main
zjj sync auth-refactor
```

This keeps your workspace up to date with the latest changes from main.

### 6. Complete and Land Your Work

```bash
# Finish and merge to main
zjj done
```

This:
- Merges your changes to main
- Pushes to remote (if configured)
- Cleans up the workspace

### 7. Clean Up (Optional)

```bash
# Remove the session
zjj remove auth-refactor
```

---

## Multi-Agent Quick Start

Want to run multiple agents in parallel? Here's how:

### 1. Add Work Items to the Queue

```bash
zjj queue --add feature-a --bead BD-101 --priority 3
zjj queue --add feature-b --bead BD-102 --priority 5
zjj queue --add feature-c --bead BD-103 --priority 1
```

### 2. List Queue Items

```bash
zjj queue --list
```

### 3. Start a Queue Worker

```bash
# Run once
zjj queue worker --once

# Or run continuously
zjj queue worker --loop
```

### 4. Monitor Progress

```bash
# Check queue status
zjj queue --status
```

---

## Common Commands

Here are the commands you'll use most:

| Command | What It Does |
|---------|-------------|
| `zjj whereami` | Check current location (main or workspace) |
| `zjj add <name>` | Create new session + workspace + tab |
| `zjj list` | List all sessions |
| `zjj status [name]` | Show detailed session status |
| `zjj focus <name>` | Switch to session's Zellij tab |
| `zjj sync [name]` | Rebase workspace onto main |
| `zjj done` | Complete work and merge to main |
| `zjj remove <name>` | Remove session and workspace |

<div class="note">
✨ <strong>Pro Tip</strong>: All commands support <code>--json</code> for machine-readable output!
</div>

---

## Keyboard Shortcuts

When working with Zellij:

| Shortcut | Action |
|----------|--------|
| `Ctrl+p` then `t` | New tab |
| `Ctrl+p` then `n` | Next tab |
| `Ctrl+p` then `h/l` | Navigate tabs |
| `Ctrl+p` then `w` | Close tab |

<div class="info">
💡 <strong>Tip</strong>: Use <code>zjj focus &lt;name&gt;</code> to jump directly to a session's tab instead of navigating manually.
</div>

---

## Configuration

ZJJ stores configuration in `.zjj/config.toml`. You can modify it directly or use:

```bash
# View configuration
zjj config

# Set a value
zjj config key value
```

### Common Settings

```toml
[recovery]
policy = "warn"  # Options: silent, warn, fail-fast
log_recovered = true

[queue]
default_priority = 5
stale_timeout_seconds = 3600
```

---

## Troubleshooting

### "Database corruption detected"

ZJJ has built-in recovery. Check `.zjj/recovery.log` for details.

```bash
zjj doctor
```

### "Not in a JJ repository"

Make sure you're in a JJ repository:

```bash
jj init  # If starting fresh
# or
jj git clone <url>  # If cloning an existing repo
```

### "Zellij not found"

Install Zellij:

```bash
# macOS
brew install zellij

# Linux
cargo install zellij

# Or download from https://zellij.dev/download
```

---

## Next Steps

Now that you're set up, explore:

- **[Workspace Management](./guide/workspaces.md)** - Deep dive into sessions
- **[Queue Coordination](./guide/queue.md)** - Learn queue operations
- **[AI Agent Guide](./ai/overview.md)** - Set up AI agents
- **[Command Reference](./reference/commands.md)** - Complete command list

---

## Getting Help

- 📖 Read the [User Guide](./guide/workspaces.md)
- 🐛 [Report issues](https://github.com/lprior-repo/zjj/issues)
- 💬 [Discussions](https://github.com/lprior-repo/zjj/discussions)
- 📧 [Email support](mailto:lewis@example.com)

---

<div class="page-footer">
  <p>Ready to dive deeper? Check out the <a href="./guide/workspaces.html">User Guide</a>.</p>
</div>

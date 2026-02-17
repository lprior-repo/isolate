# ZJJ

<div class="hero">
  <h1>Run parallel workstreams without conflicts</h1>
  <p>Create isolated workspaces for each task. Switch instantly. Land cleanly.</p>
</div>

---

## What you can do with ZJJ

<div class="features">
  <div class="feature">
    <div class="feature-icon">🔒</div>
    <h3>Work on multiple features</h3>
    <p>Each task gets its own isolated workspace. No more stashing, branching, or context switching pain.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">⚡</div>
    <h3>Switch in seconds</h3>
    <p>Jump between workspaces instantly. Your environment, editor state, and terminal history stay intact.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">🤖</div>
    <h3>Run AI agents in parallel</h3>
    <p>Coordinate 6-12 coding agents safely. Each agent works in isolation with queue-based coordination.</p>
  </div>
  
  <div class="feature">
    <div class="feature-icon">✅</div>
    <h3>Land work cleanly</h3>
    <p>One command merges your changes to main. Built-in sync keeps you up to date with teammates.</p>
  </div>
</div>

---

## How it works

ZJJ combines three tools into a unified workflow:

| Component | What it does |
|-----------|--------------|
| **JJ Workspaces** | Hard isolation for each task |
| **Zellij Tabs** | Fast context switching between workspaces |
| **SQLite Queue** | Coordination, retries, and recovery |

### The workflow

```bash
# 1. Create an isolated workspace
zjj add feature-auth --bead BD-123

# 2. Jump into it
zjj focus feature-auth

# 3. Do your work (main stays untouched)
vim src/auth.rs

# 4. Sync with main anytime
zjj sync

# 5. Land your changes
zjj done --message "Add auth"
```

---

## Before and after

**Without ZJJ:**

- 6 agents race on the same working copy → duplicated effort, conflicts
- You switch branches → lose editor state, stash changes, forget context
- Multiple features in progress → hard to track what's where

**With ZJJ:**

- Each agent gets an isolated workspace → no conflicts, clear ownership
- Switch workspaces in 1 second → state preserved, instant context
- See all work at a glance → `zjj list` shows everything

---

## When to use ZJJ

| If you are | ZJJ helps you |
|------------|---------------|
| A developer | Work on multiple features without branch chaos |
| An AI operator | Run multiple agents safely in parallel |
| A team | Coordinate workstreams without stepping on each other |
| A maintainer | Review and merge PRs in isolated workspaces |

---

## Quick comparison

| Feature | Git branches | Git worktrees | ZJJ |
|---------|--------------|---------------|-----|
| Isolation | Shared .git | Separate dirs | Separate dirs |
| Switching | `git checkout` | `cd ../dir` | `zjj focus name` |
| State preserved | No | Partial | Yes (Zellij) |
| Queue coordination | No | No | Yes |
| AI-ready | No | No | Yes |

---

## Get started in 5 minutes

<div class="quickstart-cards">
  <div class="card">
    <h4>🚀 Quick Start</h4>
    <p>Set up ZJJ and create your first workspace</p>
    <a href="./quickstart.html">Start here →</a>
  </div>
  
  <div class="card">
    <h4>📖 User Guide</h4>
    <p>Learn the complete workflow</p>
    <a href="./guide/workspaces.html">Read guide →</a>
  </div>
  
  <div class="card">
    <h4>🤖 AI Agents</h4>
    <p>Run parallel AI coding agents</p>
    <a href="./ai/overview.html">AI guide →</a>
  </div>
  
  <div class="card">
    <h4>📚 Reference</h4>
    <p>All commands and configuration</p>
    <a href="./reference/commands.html">Browse →</a>
  </div>
</div>

---

## Key concepts

### Session

A named, isolated workspace that optionally links to a bead (issue/task).

```bash
zjj add auth-refactor --bead BD-123
```

### Queue entry

A unit of work that agents can claim and process.

```bash
zjj queue --add feature-a --bead BD-101 --priority 3
```

### Done

Complete your work and merge it back to main.

```bash
zjj done --message "Implement auth refactor" --push
```

---

## Prerequisites

Before you start, ensure you have:

- **JJ (Jujutsu)** 0.20+ — [Install guide](https://github.com/martinvonz/jj#installation)
- **Zellij** 0.39+ — [Install guide](https://zellij.dev/download)
- **Rust** 1.80+ — [Install via rustup](https://rustup.rs/)

---

## License

MIT License — see [LICENSE](https://github.com/lprior-repo/zjj/blob/main/LICENSE) for details.

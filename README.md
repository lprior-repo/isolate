# Isolate

Isolate is a workspace isolation tool built on top of [JJ (Jujutsu)](https://github.com/jj-vcs/jj) version control. It provides a robust solution for managing isolated development workspaces, particularly designed for AI agents working in parallel.

## The Problem

Running agents in swarms is painful. When you have multiple agents working simultaneously:

- **Code gets lost** — agents overwrite each other's changes
- **File locks** — agents block each other out
- **Merge conflicts** — agents step on each other's toes constantly
- **Noisy state** — agents see each other's work-in-progress
- **Chaos** — total fucking chaos

## The Solution

Isolate solves this by creating clean, isolated workspaces where each agent has:

- **Complete isolation** — no file locks, no conflicts, no stepping on toes
- **Clean state** — each workspace sees only its own changes
- **Proper versioning** — built on JJ's powerful undo/changeset model
- **Session tracking** — knows who's working where and when
- **Clean merge path** — easy to sync and merge back to main

## Key Commands

```bash
# Initialize Isolate in a repo
isolate init

# Spawn a new isolated workspace for a task
isolate spawn <bead-id>

# Switch between workspaces
isolate switch <workspace-name>

# List all workspaces
isolate list

# Sync workspace with main
isolate sync

# Merge completed work back to main
isolate done

# Abort and clean up a workspace
isolate abort

# Check status of your workspace
isolate status
```

## Requirements

- **JJ (Jujutsu)** must be installed. Isolate is built on top of JJ and requires it to function.
- Install via: `cargo install jj-cli` or `brew install jj`

## Architecture

Isolate leverages JJ's workspace model to provide:

1. **Multiple workspaces** — each agent gets its own JJ workspace
2. **Bookmarks** — track which workspace maps to which "bead" (task)
3. **Session state** — SQLite-backed tracking of workspace metadata
4. **Sync operations** — intelligent merging between isolated workspaces and main
5. **Recovery** — robust handling of interrupted sessions

## Why JJ?

JJ provides the perfect foundation for isolation:

- **Anonymous commits** — workspaces don't need to share branch names
- **Undo capability** — completely safe operations with easy rollback
- **Sparse checkouts** — only see what you need
- **Conflict resolution** — sane merge handling
- **Operation log** — full history of workspace changes

## Installation

```bash
cargo install isolate
```

Or build from source:

```bash
cargo install --path crates/isolate
```

## Getting Started

```bash
# Initialize in your repo
cd your-project
isolate init

# Create an isolated workspace for a task
isolate spawn feature-123

# Do your work...

# Sync with main if needed
isolate sync

# When done, merge back
isolate done
```

## Documentation

See the `docs/` directory for:

- [AI Agent Guide](./docs/AI_AGENT_GUIDE.md) — how to use Isolate with AI agents
- [Rollout/Rollback](./docs/ROLLOUT_ROLLBACK.md) — deployment strategies
- [Error Troubleshooting](./docs/ERROR_TROUBLESHOOTING.md) — common issues and fixes

## License

MIT

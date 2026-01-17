# ZJJ Architecture

ZJJ integrates JJ (Jujutsu), Zellij, and Beads for session-based development.

## Core Concept

A **session** = JJ workspace + Zellij tab + SQLite record

```
Session "feature-auth"
├── JJ workspace: /workspaces/feature-auth/
├── Zellij tab: jjz:feature-auth
└── Database record: .jjz/sessions.db
```

## Project Structure

```
zjj/
├── crates/
│   ├── zjj-core/          # Library (integrations, config, errors)
│   │   ├── error.rs       # Error types
│   │   ├── config.rs      # Configuration
│   │   ├── jj.rs          # JJ integration
│   │   ├── zellij.rs      # Zellij integration
│   │   └── beads.rs       # Beads integration
│   │
│   └── zjj/               # Binary (CLI + persistence)
│       ├── main.rs        # CLI entry point
│       ├── db.rs          # SQLite persistence
│       ├── session.rs     # Session model
│       └── commands/      # Command implementations
│
├── docs/                  # Documentation (00-11)
└── CLAUDE.md              # Project rules
```

## Session Model

```rust
pub struct Session {
    pub id: Option<i64>,
    pub name: String,              // [a-zA-Z][a-zA-Z0-9_-]{0,63}
    pub status: SessionStatus,     // Creating|Active|Paused|Completed|Failed
    pub workspace_path: String,
    pub zellij_tab: String,        // "jjz:<name>"
    pub branch: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub last_synced: Option<u64>,
    pub metadata: Option<Value>,   // Extensible JSON
}
```

**Lifecycle**:
```
Creating → Active → [Paused] → Completed
   ↓          ↓         ↓
  Failed ← Failed ← Failed
```

## Database Schema

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
    workspace_path TEXT NOT NULL,
    branch TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_synced INTEGER,
    metadata TEXT
);

CREATE INDEX idx_status ON sessions(status);
CREATE INDEX idx_name ON sessions(name);
```

**Concurrency**: `Arc<Mutex<Connection>>` ensures thread safety

## Commands

| Command | Purpose |
|---------|---------|
| `init` | Initialize `.jjz/sessions.db` |
| `add <name>` | Create session (workspace + tab + record) |
| `list` | List sessions |
| `remove <name>` | Cleanup session |
| `focus <name>` | Switch to session's Zellij tab |
| `status [name]` | Show session status |
| `sync [name]` | Rebase session on main |
| `diff <name>` | Show diff vs main |
| `config [key] [value]` | View/modify config |
| `dashboard` | TUI kanban view |
| `doctor` | Health checks |
| `introspect [cmd]` | Machine-readable API docs |
| `query <type>` | Programmatic queries |

## Integrations

### JJ (Jujutsu)
- `jj workspace add <name>`: Create isolated workspace per session
- `jj rebase -d main`: Sync strategy
- `jj status`, `jj diff`: Status reporting

### Zellij
- `zellij action go-to-tab-name jjz:<name>`: Tab switching
- `zellij action new-tab --name jjz:<name>`: Create tab
- Tab naming: `jjz:<session-name>` for isolation

### Beads
- Session metadata stores Beads issue ID
- Commit messages link to issues: `Closes BD-123`
- Dashboard shows Beads context

## Validation

**Session names**:
- Must start with letter (a-z, A-Z)
- Only ASCII alphanumeric, dash, underscore
- 1-64 characters
- Prevents: path traversal, SQL injection, shell metacharacters

See `crates/zjj/src/session.rs::validate_session_name` for implementation.

## Error Handling

All operations return `Result<T, zjj_core::Error>`:

```rust
pub enum Error {
    DatabaseError(String),
    ValidationError(String),
    NotFound(String),
    AlreadyExists(String),
    ConfigError(String),
    IoError(String),
    ParseError(String),
    ProcessError(String),
    Unknown(String),
}
```

**User messages** include context, cause, and solution:
```
Error: Database file does not exist: /path/to/.jjz/sessions.db

Run 'jjz init' to initialize ZJJ in this repository.
```

## Configuration

**Hierarchy** (highest to lowest priority):
1. Command-line flags
2. Environment variables (`ZJJ_*`)
3. Project config (`.jjz/config.toml`)
4. Global config (`~/.config/zjj/config.toml`)
5. Defaults

**Example** (`.jjz/config.toml`):
```toml
workspace_dir = "/path/to/workspaces"

[zellij]
use_tabs = true

[hooks]
post_create = "echo 'Session created'"
```

## Hooks

| Hook | When | Environment Variables |
|------|------|----------------------|
| `post_create` | After `jjz add` | `$SESSION_NAME`, `$WORKSPACE_PATH` |
| `pre_remove` | Before `jjz remove` | `$SESSION_NAME` |
| `post_sync` | After `jjz sync` | `$SESSION_NAME` |
| `on_focus` | After `jjz focus` | `$SESSION_NAME` |

## Example Workflow

```bash
# 1. Claim issue
bd claim BD-456

# 2. Create session
jjz add feature-new-api

# 3. Work (JJ tracks changes automatically)
vim src/api.rs
jj describe -m "feat: add new API endpoint

Closes BD-456"

# 4. Test
moon run :test

# 5. Sync with main
jjz sync feature-new-api

# 6. Merge and cleanup
jjz remove feature-new-api -m

# 7. Close issue
bd complete BD-456
```

## Design Principles

1. **Zero Unwrap**: No `.unwrap()`, `.expect()`, `panic!()`, or `unsafe`
2. **Functional**: Railway-oriented programming with `Result` and combinators
3. **Type-Safe**: Strong types prevent invalid states
4. **Isolated**: Each session is completely independent
5. **Fail-Safe**: Graceful error handling with helpful messages

---

**See Also**:
- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - Zero unwrap law
- [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - Error patterns
- [08_BEADS.md](08_BEADS.md) - Beads integration
- [09_JUJUTSU.md](09_JUJUTSU.md) - JJ integration

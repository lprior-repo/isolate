# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

### Code Searching: Codanna Only
**ABSOLUTE RULE: Use Codanna for ALL code and documentation searching. FORBIDDEN: bash grep, find, Glob, Grep, and Read tools for code exploration.**

Use only:
- `mcp__codanna__semantic_search_with_context` - Find and understand symbols with full context
- `mcp__codanna__semantic_search_docs` - Search project documentation
- `mcp__codanna__find_symbol` - Locate exact symbols by name
- `mcp__codanna__search_symbols` - Search symbols with filters
- `mcp__codanna__analyze_impact` - Understand symbol dependencies

**FORBIDDEN TOOLS FOR CODE WORK:**
- ❌ bash grep, find, awk, sed
- ❌ Glob tool
- ❌ Grep tool
- ❌ Read tool for code exploration

**Read tool may ONLY be used for:**
- Reading CLAUDE.md or project documentation files
- Reading configuration files (Cargo.toml, moon.yml, etc.)
- Reading test data or examples
- NOT for code file exploration or analysis

Codanna is the authoritative source of truth for all code intelligence.

### NEVER Touch Clippy/Lint Configuration
**ABSOLUTE RULE: DO NOT MODIFY clippy or linting configuration files. EVER.**

This includes but is not limited to:
- `.clippy.toml`
- `clippy.toml`
- Any `#![allow(...)]` or `#![deny(...)]` attributes in `lib.rs` or `main.rs`
- Clippy-related sections in `Cargo.toml`
- Any lint configuration in `moon.yml` or build scripts

If clippy reports warnings or errors, fix the **code**, not the lint rules.
The user has explicitly configured these rules. Do not second-guess them.

### Build System: Moon Only
**NEVER use raw cargo commands.** Always use Moon for all build operations:

```bash
# Correct
moon run :quick      # Format + lint check
moon run :test       # Run tests
moon run :build      # Release build
moon run :ci         # Full pipeline
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check

# WRONG - Never do this
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
```

### Code Quality
- Zero unwraps: `unwrap()` and `expect()` are forbidden
- Zero panics: `panic!`, `todo!`, `unimplemented!` are forbidden
- All errors must use `Result<T, Error>` with proper propagation
- Use functional patterns: `map`, `and_then`, `?` operator

### Project Structure
```
crates/
  zjj-core/     # Core library (error handling, types, functional utils)
  zjj/          # CLI binary (MVP: init, add, list, remove, focus)
```

### MVP Commands
1. `zjj init` - Initialize zjj in a JJ repository
2. `zjj add <name>` - Create session with JJ workspace + Zellij tab
3. `zjj list` - Show all sessions
4. `zjj remove <name>` - Cleanup session and workspace
5. `zjj focus <name>` - Switch to session's Zellij tab

### Key Decisions
- **Sync strategy**: Rebase (`jj rebase -d main`)
- **Zellij tab naming**: `zjj:<session-name>`
- **Beads**: Hard requirement, always integrate with `.beads/beads.db`
- **zjj runs inside Zellij**: Tab switching via `zellij action go-to-tab-name`

### Dependencies
- JJ (Jujutsu) for workspace management
- Zellij for terminal multiplexing
- Beads for issue tracking integration
- SQLite for session state persistence

## JJ (Jujutsu) Mastery - No Git Commands

**ABSOLUTE RULE: Use JJ exclusively. Never use git commands directly.**

### Revset Mastery
Use flexible revsets for selection:
- `@` - Current working copy
- `root()` - Start of history
- `x::y` - Ancestors of y that are descendants of x
- `x-` - Parents of x
- `x+` - Children of x
- `x|y` - Union
- `x&y` - Intersection
- `::` - All visible commits

**Examples:**
```bash
jj log -r 'main..@'    # See your stack (not just recent commits)
jj diff -r @-          # Changes against parent
```

### Workspaces for Multitasking
**Never use 'git stash' or create temporary WIP commits to switch contexts.**

Use JJ workspaces for parallel work streams:
```bash
jj workspace add ../fix-bug main    # New working copy at main
jj workspace list                   # See all workspaces
```

This allows parallel work without disturbing your current state (`@`).

### Conflicts as Data
Conflicts are not errors - they are valid, committable states.
- You can still `jj git push` with conflicts
- To resolve: edit the file (standard conflict markers), then `jj squash` or `jj new`
- Never need to 'abort' a rebase - just `jj undo` if unwanted

### Bookmark Management
Bookmarks = Git Branches. Anonymous heads are fine for local work.

**Workflow:**
```bash
# Work on anonymous revisions first
jj new main
# ... make changes ...

# Only assign bookmark when ready to push
jj bookmark set feature-x -r @
jj git push

# Move a bookmark
jj bookmark set feature-x -r <new-revision>

# Delete remote bookmark
jj bookmark delete feature-x
jj git push
```

### Safety Net: Operation Log
Every action creates an operation entry - infinite undo buffer.

```bash
jj op log                    # View all operations
jj op restore <op-id>        # Revert entire repo state instantly
```

If you destroy history or make a bad rebase, recover with `jj op restore`.

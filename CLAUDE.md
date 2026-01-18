# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

### Code Searching: Codanna Only
**ABSOLUTE RULE: Use Codanna for ALL code and documentation searching. NEVER use bash grep, find, or native Claude search tools.**

Use only:
- `mcp__codanna__semantic_search_with_context` - Find and understand symbols with full context
- `mcp__codanna__semantic_search_docs` - Search project documentation
- `mcp__codanna__find_symbol` - Locate exact symbols by name
- `mcp__codanna__search_symbols` - Search symbols with filters
- `mcp__codanna__analyze_impact` - Understand symbol dependencies

No bash grep, no Glob, no Grep tool. Codanna is the source of truth.

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

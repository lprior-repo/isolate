# AI Assistant Guide

How to brief AI assistants (Claude, ChatGPT, etc.) for effective collaboration on ZJJ.

## Quick Briefing Template

```markdown
I'm working on ZJJ, a Rust CLI that manages sessions (JJ workspace + Zellij tab + SQLite record).

**Critical Rules**:
1. ❌ NO `.unwrap()`, `.expect()`, `panic!()`, or `unsafe` (compiler enforced)
2. ✅ Return `Result<T, zjj_core::Error>` for all fallible operations
3. ✅ Use Moon for builds: `moon run :test` (NEVER raw `cargo`)
4. ✅ Use functional patterns: `?` operator, combinators
5. ❌ NEVER modify clippy/lint config (fix code instead)

**Docs**: See `CLAUDE.md` and `docs/01_ERROR_HANDLING.md`

**Task**: [YOUR TASK HERE]
```

## Common Scenarios

### Error Handling
```markdown
This code uses `.unwrap()` which is forbidden:

[PASTE CODE]

Refactor to follow ZJJ patterns (see docs/01_ERROR_HANDLING.md).
Must return Result<T> and use ? operator or combinators.
```

### New Command
```markdown
Add `jjz <command>` following pattern in crates/zjj/src/commands/add.rs:

1. Return Result<()>
2. Add to main.rs (command definition + match arm)
3. Add tests for success + error paths
4. Use zjj_core::Error for errors

[DESCRIBE COMMAND]
```

### Database Changes
```markdown
Add field to Session model:

**Field**: `last_focused: Option<u64>`

Provide:
1. Schema SQL
2. Session struct update
3. CRUD updates in db.rs
4. Concurrency tests

Follow patterns in crates/zjj/src/db.rs.
```

### Code Review
```markdown
Review this ZJJ code for:
- ✅ No .unwrap() or .expect()
- ✅ Returns Result<T>
- ✅ Uses ? or combinators
- ✅ User-friendly error messages
- ✅ Tests for error paths

[PASTE CODE]
```

## Key Project Info

**Structure**:
```
crates/
  zjj-core/     # Library (errors, config, integrations)
  zjj/          # Binary (CLI, db, commands)
```

**Session Model**:
```rust
pub struct Session {
    pub name: String,              // [a-zA-Z][a-zA-Z0-9_-]{0,63}
    pub status: SessionStatus,     // Creating|Active|Paused|Completed|Failed
    pub workspace_path: String,
    pub zellij_tab: String,        // "jjz:<name>"
    // ...
}
```

**Commands**: init, add, list, remove, focus, status, sync, diff, config, dashboard, doctor, introspect, query

**Error Type**:
```rust
pub enum Error {
    DatabaseError(String),
    ValidationError(String),
    NotFound(String),
    // ...
}
```

## Common Pitfalls to Warn AI

### ❌ The Unwrap Trap
```rust
// AI might suggest:
let value = result.unwrap();

// Correct:
let value = result?;
```

**Warn**: "ZJJ has `#![deny(unwrap_used)]` - compiler will reject `.unwrap()`"

### ❌ The Cargo Trap
```bash
# AI might suggest:
cargo test

# Correct:
moon run :test
```

**Warn**: "Always use Moon, never raw cargo"

### ❌ The String Error Trap
```rust
// AI might suggest:
fn op() -> Result<T, String>

// Correct:
fn op() -> Result<T>  // Uses zjj_core::Error
```

**Warn**: "Use `zjj_core::Result<T>` alias"

### ❌ The Panic Trap
```rust
// AI might suggest:
if !valid { panic!("error"); }

// Correct:
if !valid { return Err(Error::ValidationError("error".into())); }
```

**Warn**: "`panic!` is forbidden. Return errors."

## Effective Questions

**Good**:
- "How do I handle the error case where the database is locked?"
- "What combinator transforms `Option<Result<T>>` to `Result<Option<T>>`?"
- "How do I test concurrent database operations?"

**Bad**:
- "How do I fix this error?" (too vague)
- "What should I do?" (no context)
- "Help me with Rust" (not specific)

## Reference Docs

| Topic | Doc |
|-------|-----|
| Error handling | docs/01_ERROR_HANDLING.md |
| Build system | docs/02_MOON_BUILD.md |
| Functional patterns | docs/04_FUNCTIONAL_PATTERNS.md |
| Testing | docs/07_TESTING.md |
| Architecture | docs/11_ARCHITECTURE.md |

## Quick Reference

| Need | Tell AI |
|------|---------|
| Error handling | "Return Result<T>, no .unwrap(), see docs/01_ERROR_HANDLING.md" |
| Building | "Use moon run :test, never cargo" |
| New command | "Follow crates/zjj/src/commands/add.rs pattern" |
| Database | "Thread-safe (Arc<Mutex>), test concurrency, see db.rs" |
| Validation | "See session.rs::validate_session_name for patterns" |

---

**Key Point**: Always reference `CLAUDE.md` and relevant docs. State constraints upfront. Point to similar code examples.

# 🚀 ZJJ - Start Here

Welcome to ZJJ - a strictly typed, functional Rust project with **zero panics, zero unwraps, and zero unsafe code**.

## The Sacred Law

```
❌ .unwrap()     ← Compiler forbids
❌ .expect()     ← Compiler forbids
❌ panic!()      ← Compiler forbids
❌ unsafe { }    ← Compiler forbids

All fallible operations return Result<T, Error>
The compiler enforces this.
```

## What You Need to Know (2 minutes)

### Errors are Results
```rust
fn operation(input: &str) -> Result<Output> {
    let parsed = parse(input)?;      // ? early returns on error
    let valid = validate(&parsed)?;  // Chain with ?
    Ok(transform(valid))             // Return success
}
```

### Building
```bash
moon run :ci       # Full pipeline (lint, test, build)
moon run :test     # Just tests
moon run :quick    # Just lint
# Never: cargo build, cargo test, etc.
```

### Work Flow
```bash
bd list            # See work to do
bd claim BD-123    # Claim an issue
# ... make changes ...
moon run :ci       # Validate locally
jj describe -m "feat: description"  # Commit
jj git push        # Push
bd complete BD-123 # Close issue
```

## Documentation

All documentation in `/docs/`:

| Read This | For |
|-----------|-----|
| [docs/INDEX.md](docs/INDEX.md) | Master index + navigation |
| [docs/00_START_HERE.md](docs/00_START_HERE.md) | 5-minute crash course |
| [docs/01_ERROR_HANDLING.md](docs/01_ERROR_HANDLING.md) | How to handle errors (10 patterns) |
| [docs/02_MOON_BUILD.md](docs/02_MOON_BUILD.md) | Building & testing |
| [docs/03_WORKFLOW.md](docs/03_WORKFLOW.md) | Daily workflow |
| [docs/08_BEADS.md](docs/08_BEADS.md) | Using `bv` to pick work |
| [docs/09_JUJUTSU.md](docs/09_JUJUTSU.md) | Committing & pushing |

All docs are numbered (00-09) and searchable. Start with [docs/INDEX.md](docs/INDEX.md).

## The Tools

### bv (Triage Engine)
```bash
bv --robot-triage   # Get recommendations on what to work on
```

See [docs/08_BEADS.md](docs/08_BEADS.md) for full guide.

### Moon (Build System)
```bash
moon run :ci        # Build, test, lint (everything)
moon run :test      # Tests only
```

See [docs/02_MOON_BUILD.md](docs/02_MOON_BUILD.md) for full guide.

### Jujutsu (Git Alternative)
```bash
jj describe -m "feat: description"  # Commit
jj git push                         # Push
```

See [docs/09_JUJUTSU.md](docs/09_JUJUTSU.md) for full guide.

### Beads (Issue Tracking)
```bash
bd list             # All issues
bd claim BD-123     # Start work
bd complete BD-123  # Done
```

See [docs/08_BEADS.md](docs/08_BEADS.md) for full guide.

## Project Structure

```
zjj/
├── Cargo.toml              # Workspace + strict lints
├── rustfmt.toml            # Code formatting rules
├── rust-toolchain.toml     # Nightly Rust
├── .clippy.toml            # Lint config
│
├── docs/                   # All documentation (indexed 00-09)
│   ├── INDEX.md           # Master index
│   ├── 00_START_HERE.md   # 5-minute start
│   ├── 01_ERROR_HANDLING.md
│   ├── 02_MOON_BUILD.md
│   ├── ... (complete set)
│
├── crates/
│   └── zjj-core/          # Example library crate
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── error.rs
│           ├── result.rs
│           └── functional.rs
│
└── START.md               # This file
```

## Quick Command Reference

```bash
# Pick work
bv --robot-triage

# Claim issue
bd claim BD-123

# Validate locally
moon run :test

# Commit
jj describe -m "feat: description"

# Push
jj git push

# Close issue
bd complete BD-123
```

## The 5-Second Version

1. **Pick work** → `bv --robot-triage`
2. **Claim it** → `bd claim BD-123`
3. **Code** → Edit files (tracked automatically)
4. **Test** → `moon run :test`
5. **Commit** → `jj describe -m "feat: ..."`
6. **Push** → `jj git push`
7. **Close** → `bd complete BD-123`

If your code doesn't compile, the compiler tells you to:
- ✅ Return `Result` instead of panicking
- ✅ Handle all error cases
- ✅ Handle all Option cases
- ✅ Use idiomatic Rust

**That's a feature, not a bug.** Trust the compiler.

## Next Steps

1. **Read [docs/INDEX.md](docs/INDEX.md)** (5 minutes) - Understand what docs exist
2. **Read [docs/00_START_HERE.md](docs/00_START_HERE.md)** (5 minutes) - Crash course
3. **Read [docs/01_ERROR_HANDLING.md](docs/01_ERROR_HANDLING.md)** (20 minutes) - How errors work
4. **Bookmark [docs/06_COMBINATORS.md](docs/06_COMBINATORS.md)** - Reference while coding

## Key Rules

| Rule | Why |
|------|-----|
| All errors return `Result` | Type safety - compiler catches bugs |
| No `unwrap()` | Prevents panics in production |
| No `panic!()` | Graceful failure, not crashes |
| No `unsafe` | Memory safety guaranteed |
| Use Moon to build | Caching + consistency |
| Use Jujutsu to commit | Clean history + stacking |
| Use Beads for tracking | Dependencies + triage |

## The Mantra

> "In ZJJ, we don't panic. All fallible operations return `Result<T, Error>`. The compiler enforces this law. Idiomatic, functional, zero-panic Rust—that's the ZJJ way."

## Help

- **Quick question?** → [docs/00_START_HERE.md](docs/00_START_HERE.md)
- **Error handling?** → [docs/01_ERROR_HANDLING.md](docs/01_ERROR_HANDLING.md)
- **Lost?** → [docs/INDEX.md](docs/INDEX.md) - Full navigation
- **Reference?** → [docs/06_COMBINATORS.md](docs/06_COMBINATORS.md) - Combinator cheatsheet

---

**You're ready. Start with [docs/INDEX.md](docs/INDEX.md).**

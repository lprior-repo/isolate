//! Project Scaffolding Templates
//!
//! Provides the core templates for bootstrapping new projects with ZJJ.
//! Includes AI instructions, core rules, and workflow documentation.

use std::collections::HashMap;

/// Unified AI Instructions for AGENTS.md and CLAUDE.md
pub const AI_INSTRUCTIONS: &str = r"# Agent Instructions: Autonomous Development Loop

This project uses **Beads** for triage, **ZJJ** for isolation, and **Moon** for absolute quality.

## Core Workflow (The Loop)

1. **Triage & Pull**: Use `bv --robot-triage` to find the highest-impact bead. Claim it with `bd claim <bead-id>`.
2. **Isolate**: Invoke `zjj spawn <bead-id>`. This provisions an isolated workspace at `.zjj/workspaces/<bead-id>/`.
3. **Execute Skills**:
   - **tdd15**: Drive development through small, failing tests.
   - **red-queen**: Adhere to rigorous adversarial verification standards.
   - **functional**: Ensure purely functional Rust (ROP, zero unwraps).
4. **Absolute Quality**: Run `moon run :ci --force` (the `--force` flag is mandatory to bypass cache and ensure absolute correctness).
5. **Merge & Close**: Run `zjj done`. This merges your work into `main` and marks the bead as completed.

## Build System (Moon Only)

**NEVER use raw cargo.**
- ✅ `moon run :quick` (Fast check)
- ✅ `moon run :ci --force` (Absolute verification)
- ❌ `cargo build/test`

## Zero-Policy (Enforced)

- No `.unwrap()` or `.expect()`
- No `panic!()` or `unsafe`
- All errors use `Result<T, Error>` with proper combinators (`map`, `and_then`).

## Landing Rules

Work is not complete until:
1. `moon run :ci --force` passes.
2. `zjj done` has been executed.
3. `git push` succeeds.
";

/// Template for `docs/01_ERROR_HANDLING.md`
pub const DOC_01_ERROR_HANDLING: &str = r"# Error Handling: Zero Policy

## The Sacred Law
All fallible operations return `Result<T, Error>`. Capturing error information is a requirement, not a suggestion.

## combinators
Use `map`, `and_then`, and `?` to propagate errors idiomatically.
";

/// Template for `docs/02_MOON_BUILD.md`
pub const DOC_02_MOON_BUILD: &str = r"# Build Pipeline: Moon

## Absolute Verification
To ensure no cached success masks a subtle regression, always run:
```bash
moon run :ci --force
```
";

/// Template for `docs/03_WORKFLOW.md`
pub const DOC_03_WORKFLOW: &str = r"# Workflow: Pull -> Isolate -> Verify -> Merge

1. **Pull**: `bv` discover.
2. **Isolate**: `zjj spawn`.
3. **Verify**: `moon run :ci --force`.
4. **Merge**: `zjj done`.
";

/// Template for `docs/05_RUST_STANDARDS.md`
pub const DOC_05_RUST_STANDARDS: &str = r"# Rust Standards

- **KIRK**: Keep It Robust and Klean.
- **Contract Based Testing**: Verify boundaries.
- **Invariants**: Document and enforce state consistency.
";

/// Template for `docs/08_BEADS.md`
pub const DOC_08_BEADS: &str = r"# Beads Integration

Issues are nodes in a graph. Prioritize using `bv --robot-triage`.
";

/// Template for `docs/09_JUJUTSU.md`
pub const DOC_09_JUJUTSU: &str = r"# Jujutsu Workspaces

Instant isolation via `jj workspace add`. Managed automatically by `zjj`.
";

/// Get all documentation templates mapped to their relative paths
#[must_use]
pub fn get_docs_templates() -> HashMap<String, &'static str> {
    let mut docs = HashMap::new();
    docs.insert("01_ERROR_HANDLING.md".to_string(), DOC_01_ERROR_HANDLING);
    docs.insert("02_MOON_BUILD.md".to_string(), DOC_02_MOON_BUILD);
    docs.insert("03_WORKFLOW.md".to_string(), DOC_03_WORKFLOW);
    docs.insert("05_RUST_STANDARDS.md".to_string(), DOC_05_RUST_STANDARDS);
    docs.insert("08_BEADS.md".to_string(), DOC_08_BEADS);
    docs.insert("09_JUJUTSU.md".to_string(), DOC_09_JUJUTSU);
    docs
}

/// Template for .moon/workspace.yml
pub const MOON_WORKSPACE: &str = r#"$schema: "https://moonrepo.dev/schemas/workspace.json"
vcs:
  manager: "git"
  defaultBranch: "main"
"#;

/// Template for .moon/toolchain.yml
pub const MOON_TOOLCHAIN: &str = r#"$schema: "https://moonrepo.dev/schemas/toolchain.json"
"#;

/// Template for .moon/tasks.yml
pub const MOON_TASKS: &str = r#"$schema: "https://moonrepo.dev/schemas/tasks.json"
tasks:
  ci:
    command: "cargo check && cargo test"
    options:
      cache: true
"#;

/// Get all moon templates mapped to their relative paths
#[must_use]
pub fn get_moon_templates() -> HashMap<String, &'static str> {
    let mut moon = HashMap::new();
    moon.insert("workspace.yml".to_string(), MOON_WORKSPACE);
    moon.insert("toolchain.yml".to_string(), MOON_TOOLCHAIN);
    moon.insert("tasks.yml".to_string(), MOON_TASKS);
    moon
}
pub mod askama;

# Agent Instructions: Autonomous Development Loop

This project uses **Beads** for triage, **Isolate** for isolation, and **Moon** for absolute quality.

## Core Workflow (The Loop)

1. **Triage & Pull**: Use `bv --robot-triage` to find the highest-impact bead. Claim it with `br claim <bead-id>`.
2. **Isolate**: Invoke `isolate spawn <bead-id>`. This provisions an isolated workspace at `.isolate/workspaces/<bead-id>/`.
3. **Execute Skills**:
   - **tdd15**: Drive development through small, failing tests.
   - **red-queen**: Adhere to rigorous adversarial verification standards.
   - **functional**: Ensure purely functional Rust (ROP, zero unwraps).
4. **Absolute Quality**: Run `moon run :ci --force` (the `--force` flag is mandatory to bypass cache and ensure absolute correctness).
5. **Merge & Close**: Run `isolate done`. This merges your work into `main` and marks the bead as completed.

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
2. `isolate done` has been executed.
3. `git push` succeeds.

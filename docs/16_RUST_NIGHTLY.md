# Rust Nightly Requirement

## Status: Required

ZJJ **requires Rust nightly** for compilation. This document explains why and what features depend on it.

## Why Nightly?

ZJJ uses nightly-only Rust features for advanced telemetry and error tracking capabilities:

### 1. Dynamic Level in Tracing Macros

The `tracing` crate's `event!` and `span!` macros require compile-time constant `Level` values in stable Rust. However, ZJJ's telemetry system (`zjj-core/src/telemetry.rs`) needs runtime-determined log levels for flexible error classification.

**Example from `telemetry.rs`:**
```rust
let level: Level = self.severity.into();  // Runtime conversion

event!(
    target: "zjj::telemetry",
    level,  // Error: not constant in stable Rust
    error.message = %self.message,
    "Error occurred"
);
```

This pattern appears in:
- `ErrorEvent::emit()` - Error event emission with dynamic severity
- Performance tracking macros
- Context-aware logging

**Why we use it:**
- Errors have different severities (Debug, Info, Warning, Error, Critical)
- Severity determines the appropriate log level at runtime
- Allows flexible error classification without code duplication

### 2. Advanced Type System Features

Some trait implementations use type alias patterns that are more permissive in nightly.

## Automatic Toolchain Management

ZJJ includes `rust-toolchain.toml` at the project root:

```toml
[toolchain]
channel = "nightly-2025-12-15"
components = ["rustfmt", "clippy", "rust-analyzer"]
targets = ["x86_64-unknown-linux-gnu"]
```

**Note**: ZJJ pins to a specific nightly version (`nightly-2025-12-15`) because later nightlies (2026-01-08+) broke dynamic log level support in tracing macros. This pin will be updated periodically as we verify compatibility.

**When you run any cargo/rustc command in the ZJJ directory, rustup automatically:**
1. Detects `rust-toolchain.toml`
2. Downloads nightly if not installed
3. Uses the correct nightly toolchain
4. Ensures consistent builds across environments

You don't need to manually switch toolchains.

## Installation

### First-Time Setup

```bash
# Install nightly toolchain (if not already installed)
rustup toolchain install nightly

# Verify installation
rustup toolchain list
# Should show: nightly-x86_64-unknown-linux-gnu

# Clone and build ZJJ
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# rust-toolchain.toml will automatically activate nightly
cargo --version  # Should show: cargo 1.XX.0-nightly

# Build project (uses nightly automatically)
moon run :build
```

### Verifying Nightly is Active

```bash
cd /path/to/zjj
rustc --version
# Should show: rustc 1.XX.0-nightly (hash YYYY-MM-DD)

rustup show active-toolchain
# Should show: nightly-x86_64-unknown-linux-gnu (overridden by '/path/to/zjj/rust-toolchain.toml')
```

## Migration to Stable (Future)

Migrating to stable Rust would require:

### 1. Refactor Telemetry System

Replace dynamic level selection with macro-based dispatch:

```rust
// Current (nightly-only):
fn emit_with_level(level: Level, message: &str) {
    event!(target: "zjj", level, message = %message);  // ❌ Not constant
}

// Stable alternative:
fn emit_with_level(severity: Severity, message: &str) {
    match severity {
        Severity::Debug => event!(target: "zjj", Level::DEBUG, message = %message),
        Severity::Info => event!(target: "zjj", Level::INFO, message = %message),
        Severity::Warn => event!(target: "zjj", Level::WARN, message = %message),
        Severity::Error => event!(target: "zjj", Level::ERROR, message = %message),
        Severity::Critical => event!(target: "zjj", Level::ERROR, message = %message),
    }
}
```

**Trade-offs:**
- More verbose code (5x duplication per log site)
- Harder to maintain (changes require updating multiple match arms)
- No functional benefit for users

### 2. Update Workspace Configuration

```toml
# Cargo.toml
[workspace.package]
rust-version = "1.80"  # Update to stable version
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"  # Change from nightly
```

### 3. CI/CD Updates

Update `.github/workflows/ci.yml` to use stable toolchain.

## Decision: Nightly Required

**As of 2026-01-11, we are staying on nightly Rust.**

**Reasoning:**
1. **Telemetry is Core**: Dynamic log levels are essential for error tracking and debugging
2. **No User Impact**: Nightly is transparent (automatic via `rust-toolchain.toml`)
3. **No Stability Issues**: ZJJ has not experienced nightly-related breakage
4. **Migration Cost > Benefits**: Refactoring would add complexity without user value
5. **Ecosystem Precedent**: Many Rust projects (especially dev tools) use nightly

## FAQ

### Does nightly affect runtime stability?

No. Nightly is a **build-time** requirement. The compiled binary is identical to a stable-compiled binary in terms of runtime behavior and stability.

### Will ZJJ break when nightly updates?

Occasionally, yes. ZJJ currently pins to `nightly-2025-12-15` because later nightlies (2026-01-08+) introduced breaking changes to the tracing macro system.

**Current Pin**: `2025-12-15` (verified working)

The `rust-toolchain.toml` pins to a specific nightly version:

```toml
[toolchain]
channel = "nightly-2025-12-15"  # Pinned to known-working version
```

We periodically test newer nightlies and update the pin when compatibility is verified.

### Can I use stable Rust?

No, compilation will fail with errors like:

```
error[E0435]: attempt to use a non-constant value in a constant
   --> crates/zjj-core/src/telemetry.rs:139:21
    |
139 |                     level,
    |                     ^^^^^ non-constant value
```

### What if I don't have nightly installed?

When you run `cargo build` in the ZJJ directory:
1. rustup reads `rust-toolchain.toml`
2. Automatically prompts: "nightly not installed, download now? [Y/n]"
3. Downloads and installs nightly
4. Proceeds with build

It's completely automated.

### Can I build ZJJ on stable with a feature flag?

Not currently. Telemetry is core functionality and cannot be disabled without breaking compilation.

Future work could add a `--no-telemetry` feature flag that removes the problematic code, but this is not a priority.

## References

- **Tracing Documentation**: https://docs.rs/tracing/latest/tracing/
- **Rustup Toolchain Management**: https://rust-lang.github.io/rustup/overrides.html
- **rust-toolchain.toml Spec**: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file

## Related Documents

- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - Code quality standards
- [02_MOON_BUILD.md](02_MOON_BUILD.md) - Build system usage
- [11_ARCHITECTURE.md](11_ARCHITECTURE.md) - System architecture

## Current Nightly Version

**Pinned Version**: `nightly-2025-12-15` (rustc 1.94.0-nightly)
**Reason for Pin**: Later nightlies (2026-01-08+) broke dynamic log levels in tracing
**Next Pin Update**: When newer nightlies are verified compatible

To test a newer nightly:
```bash
# Edit rust-toolchain.toml and update channel to "nightly-YYYY-MM-DD"
# channel = "nightly-2026-02-01"
rustup toolchain install nightly-YYYY-MM-DD
moon run :build  # If successful, update the pin in rust-toolchain.toml
```

---

**Last Updated**: 2026-01-11
**Status**: Active (Nightly Required, Pinned to 2025-12-15)
**Next Review**: 2026-04-01 (Quarterly review of nightly requirement)

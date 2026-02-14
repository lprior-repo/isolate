# Building from Source

Build ZJJ from the repository.

## Prerequisites

- Rust 1.80+
- JJ (Jujutsu)
- Moon (optional but recommended)

## Quick Build

```bash
git clone https://github.com/lprior-repo/zjj.git
cd zjj
moon run :build
```

## Without Moon

```bash
cargo build --release
```

Binary in `target/release/zjj`.

## Development Build

```bash
# Fast build (unoptimized)
cargo build

# Binary: target/debug/zjj
```

## Running Tests

```bash
moon run :test
# or
cargo test
```

## Installing

```bash
# With Moon
moon run :install

# Manual
cp target/release/zjj ~/.local/bin/
```

## Dependencies

Key crates:
- `tokio` - Async runtime
- `rusqlite` - SQLite
- `serde` - Serialization
- `clap` - CLI parsing

## Build Features

```bash
# Default build
cargo build --release

# With all features
cargo build --release --all-features
```

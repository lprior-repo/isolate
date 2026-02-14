# Testing

Run ZJJ's test suite.

## Quick Test

```bash
moon run :test
```

## Test Structure

```
tests/
├── integration/     # Integration tests
├── unit/           # Unit tests
└── e2e/            # End-to-end tests
```

## Running Tests

All tests:
```bash
cargo test
```

Specific test:
```bash
cargo test test_name
```

With output:
```bash
cargo test -- --nocapture
```

## Test Categories

**Unit tests:**
```bash
cargo test --lib
```

**Integration:**
```bash
cargo test --test integration
```

## Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let ws = Workspace::new("test");
        assert_eq!(ws.name(), "test");
    }
}
```

## CI Tests

Full suite (as CI runs):
```bash
moon run :ci
```

Includes:
- Formatting check
- Clippy lints
- Tests
- Build

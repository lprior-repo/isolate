# Contributing

Contribute to ZJJ development.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a workspace:
```bash
zjj add my-feature
zjj focus my-feature
```

## Development Workflow

```bash
# Make changes
vim src/main.rs

# Test
moon run :test

# Check formatting
moon run :fmt-fix

# Commit
jj commit -m "Add feature"

# Land
zjj done --push
```

## Code Standards

- Follow Rust conventions
- Pass `cargo clippy`
- Pass `cargo fmt`
- Add tests for new features
- Update documentation

## Pull Request Process

1. Create workspace for PR
2. Make changes
3. Run full test suite
4. Push to your fork
5. Open PR from workspace

## Reporting Issues

Include:
- ZJJ version (`zjj --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior

## Code of Conduct

Be respectful. Focus on the code.

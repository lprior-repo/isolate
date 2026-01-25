# ZJJ - JJ Workspace + Zellij Session Manager

ZJJ is a powerful tool that combines [JJ (Just Join)](https://github.com/martinvonz/jj) version control with [Zellij](https://zellij.dev/) terminal sessions for an enhanced development workflow.

## Features

- **JJ Integration**: Seamlessly manage JJ workspaces
- **Zellij Sessions**: Create and manage Zellij tabs for each session
- **Session Management**: Add, remove, list, and focus sessions
- **JSON Output Support**: All commands support JSON output for scripting
- **Error Handling**: Comprehensive error handling with user-friendly messages

## Full Moon CI/CD Setup

This project implements a comprehensive CI/CD pipeline that includes:

### Continuous Integration
- Automated testing on every push and pull request
- Code quality checks with Clippy
- Formatting validation with rustfmt
- Security audit with cargo-audit

### Automated Testing
- Unit tests for all modules
- Integration tests for session lifecycle
- Error handling verification
- JSON output format validation

### Release Pipeline
- Automated build and packaging
- Cross-platform release artifacts
- GitHub release automation
- Coverage reporting (optional)

## Getting Started

### Prerequisites

- Rust 1.80 or later
- Cargo
- Zellij (for session management)
- SQLite (for database operations)

### Installation

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Build the project
cargo build --release

# Install the binary
cargo install --path crates/zjj
```

### Usage

```bash
# Initialize ZJJ in a JJ repository
zjj init

# Create a new session
zjj add my-session

# List all sessions
zjj list

# Focus on a session
zjj focus my-session

# Remove a session
zjj remove my-session
```

## Development

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test file
cargo test -p zjj test_session_lifecycle

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --workspace
```

### Code Quality

```bash
# Check code formatting
cargo fmt --check

# Run clippy lints
cargo clippy --workspace

# Security audit
cargo audit
```

## CI/CD Configuration

The project includes a complete GitHub Actions workflow in `.github/workflows/full-moon-cicd.yml` that:

1. **Tests**: Runs all tests on Ubuntu with caching for speed
2. **Lints**: Executes Clippy for code quality
3. **Formats**: Checks Rust formatting compliance
4. **Audits**: Performs security audits
5. **Releases**: Builds and publishes release artifacts

## Contributing

Contributions are welcome! Please follow the existing code style and submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.
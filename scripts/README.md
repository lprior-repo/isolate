# ZJJ Development Setup Scripts

This directory contains automation scripts for setting up and maintaining your ZJJ development environment.

## dev-setup.sh

Automated development environment setup for new contributors.

### Quick Start

```bash
# Interactive setup (prompts for confirmation)
./scripts/dev-setup.sh

# Non-interactive setup (auto-accept all prompts)
./scripts/dev-setup.sh --yes

# Check prerequisites only
./scripts/dev-setup.sh --check
```

### What It Does

1. **Checks Prerequisites**
   - Rust 1.80+
   - Moon (build tool)
   - JJ (Jujutsu version control)
   - Zellij (terminal multiplexer, optional)

2. **Installs Missing Dependencies** (with permission)
   - Offers to install Rust via rustup
   - Offers to install Moon via official installer
   - Offers to install JJ via platform-specific installer
   - Offers to install Zellij via cargo/brew

3. **Sets Up Development Database**
   - Creates `.beads/` directory
   - Prepares for automatic database creation

4. **Runs Initial Build and Tests**
   - Quick check: `moon run :quick` (format + type check)
   - Full build: `moon run :build`
   - Full test suite: `moon run :test`

5. **Prints Next Steps**
   - Development workflow commands
   - Common tasks reference
   - Links to documentation

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Prerequisite check failed |
| 2 | Installation failed |
| 3 | Build failed |
| 4 | Tests failed |
| 5 | Database setup failed |

### Features

- **Color-coded output** for easy reading
- **Non-interactive mode** for automation (`--yes`)
- **Check-only mode** for CI/CD (`--check`)
- **Platform detection** for Linux/macOS
- **Graceful failure** with clear error messages
- **Version checking** for Rust minimum version

### Example Output

```
================================================================================
ZJJ Development Environment Setup
================================================================================

Checking Prerequisites
================================================================================
ℹ Found Rust version: 1.93.1
✓ Rust 1.80+ installed
ℹ Found Moon version: moon 2.0.1
✓ Moon installed
ℹ Found JJ version: jj 0.38.0
✓ JJ installed
ℹ Found Zellij version: zellij 0.43.1
✓ Zellij installed
✓ All required prerequisites installed

Setting Up Development Database
================================================================================
ℹ Database will be created automatically on first use
✓ Database setup complete

Running Quick Check (fmt + check)
================================================================================
ℹ Running: moon run :quick
✓ Quick check passed

Running Initial Build
================================================================================
ℹ Running: moon run :build
✓ Build completed successfully

Running Initial Tests
================================================================================
ℹ Running: moon run :test
✓ All tests passed

================================================================================
Setup Complete!
================================================================================

Your ZJJ development environment is ready. Here's what to do next:
...
```

## Manual Setup

If you prefer to set up manually or the script fails, see [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed instructions.

## Troubleshooting

### Moon not found after installation

The Moon installer may require a shell restart:

```bash
# Source your shell profile
source ~/.bashrc   # or ~/.zshrc

# Verify installation
moon --version
```

### Permission denied errors

Some installations may require sudo:

```bash
# The script will handle this automatically for JJ
# If you encounter permission errors elsewhere, ensure you have
# write permissions for the project directory
```

### Rust version too old

Update Rust via rustup:

```bash
rustup update stable
rustup default stable
```

### Build failures

Ensure all dependencies are installed:

```bash
./scripts/dev-setup.sh --check
```

Then try a clean build:

```bash
moon run :check    # Type check only
moon run :build    # Full build
```

## Contributing

When adding new scripts to this directory:

1. **Make them executable**: `chmod +x scripts/your-script.sh`
2. **Add --help flag**: Follow the pattern in `dev-setup.sh`
3. **Use set -euo pipefail**: For robust error handling
4. **Document here**: Add a section to this README

## Related Documentation

- [CONTRIBUTING.md](../CONTRIBUTING.md) - Full development guide
- [AGENTS.md](../AGENTS.md) - Agent workflow and rules
- [README.md](../README.md) - Project overview

# ZJJ Installation Guide

Complete installation guide for ZJJ (JJ + Zellij session manager).

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation Methods](#installation-methods)
  - [Method 1: Cargo Install (Recommended)](#method-1-cargo-install-recommended)
  - [Method 2: Pre-built Binary](#method-2-pre-built-binary)
  - [Method 3: Build from Source](#method-3-build-from-source)
- [Verification](#verification)
- [Post-Installation Setup](#post-installation-setup)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

## Prerequisites

ZJJ requires three external tools to function. Install these first:

### 1. Jujutsu (JJ)

**What it is**: Next-generation version control system that ZJJ uses for workspace management.

**Installation**:

```bash
# macOS (Homebrew)
brew install jj

# Arch Linux
pacman -S jujutsu

# Cargo (all platforms)
cargo install --locked jj-cli

# From source
cargo install --git https://github.com/martinvonz/jj.git --locked --bin jj jj-cli
```

**Verify**:
```bash
jj --version
# Should show: jj 0.x.x or higher
```

**Documentation**: https://github.com/martinvonz/jj

### 2. Zellij

**What it is**: Terminal multiplexer that ZJJ uses for session management.

**Installation**:

```bash
# macOS (Homebrew)
brew install zellij

# Arch Linux
pacman -S zellij

# Cargo (all platforms)
cargo install --locked zellij

# Ubuntu/Debian (manual binary install)
wget https://github.com/zellij-org/zellij/releases/latest/download/zellij-x86_64-unknown-linux-musl.tar.gz
tar -xvf zellij-x86_64-unknown-linux-musl.tar.gz
sudo mv zellij /usr/local/bin/
```

**Verify**:
```bash
zellij --version
# Should show: zellij 0.x.x or higher
```

**Documentation**: https://zellij.dev

### 3. Beads

**What it is**: Issue tracking system that ZJJ integrates with for development workflow.

**Installation**:

```bash
# Cargo (recommended)
cargo install beads

# From source
git clone https://github.com/beadsr/beads.git
cd beads
cargo install --path .
```

**Verify**:
```bash
bd --version
# Should show beads version
```

**Documentation**: https://github.com/beadsr/beads

### System Requirements

- **OS**: Linux, macOS, or Windows (WSL2 recommended)
- **Rust**: 1.80 or higher (for building from source or using cargo install)
- **Architecture**: x86_64 or aarch64

## Installation Methods

### Method 1: Cargo Install (Recommended)

This is the simplest method if you have Rust installed.

**Step 1**: Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Step 2**: Install ZJJ
```bash
cargo install zjj
```

**Step 3**: Verify installation
```bash
jjz --version
```

**Advantages**:
- Simple one-command installation
- Automatic updates available via `cargo install --force zjj`
- Platform-independent (works on all supported architectures)

**Disadvantages**:
- Requires Rust toolchain
- Compilation time (1-3 minutes)

### Method 2: Pre-built Binary

Download pre-compiled binaries from GitHub releases.

**Step 1**: Download the appropriate binary

```bash
# Linux x86_64
wget https://github.com/lprior-repo/zjj/releases/latest/download/jjz-linux-x86_64
chmod +x jjz-linux-x86_64
sudo mv jjz-linux-x86_64 /usr/local/bin/jjz

# Linux aarch64
wget https://github.com/lprior-repo/zjj/releases/latest/download/jjz-linux-aarch64
chmod +x jjz-linux-aarch64
sudo mv jjz-linux-aarch64 /usr/local/bin/jjz

# macOS x86_64 (Intel)
wget https://github.com/lprior-repo/zjj/releases/latest/download/jjz-macos-x86_64
chmod +x jjz-macos-x86_64
sudo mv jjz-macos-x86_64 /usr/local/bin/jjz

# macOS aarch64 (Apple Silicon)
wget https://github.com/lprior-repo/zjj/releases/latest/download/jjz-macos-aarch64
chmod +x jjz-macos-aarch64
sudo mv jjz-macos-aarch64 /usr/local/bin/jjz
```

**Step 2**: Verify installation
```bash
jjz --version
```

**Advantages**:
- No compilation required (instant installation)
- No Rust toolchain needed
- Smaller download size

**Disadvantages**:
- Platform-specific binaries
- Manual update process

### Method 3: Build from Source

For developers or users wanting the latest unreleased features.

**Step 1**: Clone the repository
```bash
git clone https://github.com/lprior-repo/zjj.git
cd zjj
```

**Step 2**: Install Moon (build system)
```bash
# macOS/Linux
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Or via Cargo
cargo install moon

# Or via proto (recommended)
proto install moon
```

**Step 3**: Build the project
```bash
# Quick build (development)
moon run :build

# Optimized build (release)
moon run :build-release
```

**Step 4**: Install the binary
```bash
# Install to ~/.cargo/bin
cargo install --path crates/zjj

# Or copy manually
sudo cp target/release/jjz /usr/local/bin/
```

**Step 5**: Verify installation
```bash
jjz --version
```

**Advantages**:
- Latest features and bug fixes
- Full control over build options
- Ability to modify and contribute

**Disadvantages**:
- Requires Rust toolchain and Moon
- Longer installation process
- Requires understanding of build system

## Verification

After installation, verify ZJJ and all prerequisites are working:

```bash
# 1. Check ZJJ binary
jjz --version

# 2. Check prerequisites
jj --version
zellij --version
bd --version

# 3. Check help output
jjz --help
```

Expected output from `jjz --help`:
```
ZJJ - JJ workspace + Zellij session manager

Usage: jjz <COMMAND>

Commands:
  init    Initialize jjz in a JJ repository
  add     Create session with JJ workspace + Zellij tab
  list    Show all sessions
  remove  Cleanup session and workspace
  focus   Switch to session's Zellij tab
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Post-Installation Setup

### 1. Initialize ZJJ in Your Repository

Navigate to a JJ repository and initialize ZJJ:

```bash
cd /path/to/your/jj/repo
jjz init
```

This creates:
- `.jjz/config.toml` - ZJJ configuration
- `.jjz/sessions.db` - SQLite database for session state

### 2. Configure ZJJ (Optional)

Edit `.jjz/config.toml` to customize behavior:

```toml
[core]
# Sync strategy when switching sessions
sync_strategy = "rebase"  # Options: "rebase", "merge", "none"

# Zellij tab prefix
tab_prefix = "jjz:"

[beads]
# Beads integration settings
db_path = ".beads/beads.db"
auto_link = true
```

### 3. Create Your First Session

```bash
# Create a session named "feature-x"
jjz add feature-x

# This creates:
# - JJ workspace: workspace_feature-x
# - Zellij tab: jjz:feature-x
# - Session entry in database
```

### 4. Verify Session Creation

```bash
# List all sessions
jjz list

# Check JJ workspaces
jj workspace list

# Check Zellij tabs (inside Zellij)
# Press Ctrl+T then 'Tab' to see tab list
```

## Troubleshooting

### Issue: `jjz: command not found`

**Cause**: Binary not in PATH.

**Solutions**:

1. **If installed via cargo**:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   export PATH="$HOME/.cargo/bin:$PATH"
   source ~/.bashrc  # or ~/.zshrc
   ```

2. **If installed manually**:
   ```bash
   # Ensure binary is in /usr/local/bin or another PATH directory
   echo $PATH
   which jjz
   ```

3. **Verify installation location**:
   ```bash
   # Find the binary
   find ~ -name jjz 2>/dev/null
   ```

### Issue: `jj: command not found`

**Cause**: Jujutsu not installed or not in PATH.

**Solution**:
```bash
# Install JJ (see Prerequisites section)
cargo install --locked jj-cli

# Verify
jj --version
```

### Issue: `zellij: command not found`

**Cause**: Zellij not installed or not in PATH.

**Solution**:
```bash
# Install Zellij (see Prerequisites section)
cargo install --locked zellij

# Verify
zellij --version
```

### Issue: `bd: command not found`

**Cause**: Beads not installed or not in PATH.

**Solution**:
```bash
# Install Beads (see Prerequisites section)
cargo install beads

# Verify
bd --version
```

### Issue: `error: failed to initialize jjz: not a jj repository`

**Cause**: Running `jjz init` outside a JJ repository.

**Solution**:
```bash
# Initialize JJ repository first
jj git init
# Or clone an existing JJ repo
jj git clone <repo-url>

# Then initialize ZJJ
jjz init
```

### Issue: `error: workspace already exists`

**Cause**: Attempting to create a session with a name that conflicts with an existing JJ workspace.

**Solution**:
```bash
# List existing workspaces
jj workspace list

# Choose a different session name
jjz add different-name

# Or remove the conflicting workspace
jj workspace forget workspace_old-name
```

### Issue: `error: not running inside Zellij`

**Cause**: Some ZJJ commands (like `focus`) require running inside a Zellij session.

**Solution**:
```bash
# Start Zellij first
zellij

# Then run ZJJ commands inside Zellij
jjz focus feature-x
```

### Issue: Build fails with `error: linker 'cc' not found`

**Cause**: Missing C compiler (required for some dependencies like rusqlite).

**Solution**:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora/RHEL
sudo dnf install gcc

# macOS (install Xcode Command Line Tools)
xcode-select --install

# Arch Linux
sudo pacman -S base-devel
```

### Issue: Compilation takes too long

**Cause**: Building from source compiles many dependencies.

**Solution**:
```bash
# Use pre-built binaries instead (Method 2)
# Or use cargo install with multiple cores
cargo install zjj -j $(nproc)

# Or use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
cargo install zjj
```

### Issue: `error: database is locked`

**Cause**: Multiple ZJJ processes accessing `.jjz/sessions.db` simultaneously.

**Solution**:
```bash
# Wait for other processes to complete
# Or check for hung processes
ps aux | grep jjz

# If necessary, kill hung processes
pkill jjz

# Database will unlock automatically
```

### Issue: Tab switching doesn't work

**Cause**: Zellij keybindings conflict or ZJJ not running inside Zellij.

**Solution**:
```bash
# 1. Ensure you're inside Zellij
zellij

# 2. Use jjz focus inside Zellij
jjz focus session-name

# 3. Manually switch using Zellij keybindings
# Press: Ctrl+T, then type tab name: jjz:session-name
```

### Issue: Permission denied when creating session

**Cause**: Insufficient permissions in repository directory.

**Solution**:
```bash
# Check directory permissions
ls -la .jjz/

# Fix permissions
chmod 755 .jjz/
chmod 644 .jjz/config.toml
chmod 644 .jjz/sessions.db
```

### Getting Help

If you encounter issues not covered here:

1. **Check logs**:
   ```bash
   # Enable debug logging
   RUST_LOG=debug jjz <command>
   ```

2. **File an issue**: https://github.com/lprior-repo/zjj/issues
   - Include `jjz --version`
   - Include `jj --version`, `zellij --version`, `bd --version`
   - Include full error message
   - Include OS and architecture

3. **Check documentation**:
   - `docs/00_START_HERE.md` - Quick start guide
   - `docs/11_ARCHITECTURE.md` - ZJJ architecture
   - `docs/INDEX.md` - Full documentation index

## Uninstallation

For complete uninstall instructions including cleanup of all files, sessions, and state, see:

**[Complete Uninstall Guide](docs/15_UNINSTALL.md)**

### Quick Uninstall

Remove the binary only (preserves data):

```bash
# If installed via cargo
cargo uninstall zjj

# If installed manually
sudo rm /usr/local/bin/jjz
```

For complete cleanup including:
- Session removal
- Database cleanup
- JJ workspace cleanup
- Global configuration removal
- Dependency removal

See the [Complete Uninstall Guide](docs/15_UNINSTALL.md).

## Next Steps

After successful installation:

1. **Read the quick start**: `docs/00_START_HERE.md`
2. **Understand the architecture**: `docs/11_ARCHITECTURE.md`
3. **Learn the workflow**: `docs/03_WORKFLOW.md`
4. **Create your first session**: `jjz add my-feature`
5. **Integrate with your workflow**: See `docs/08_BEADS.md` for Beads integration

## Version Information

- **Current Version**: 0.1.0
- **Minimum Rust Version**: 1.80
- **License**: MIT
- **Repository**: https://github.com/lprior-repo/zjj

## Platform Support Matrix

| Platform | Architecture | Cargo Install | Pre-built Binary | Build from Source |
|----------|--------------|---------------|------------------|-------------------|
| Linux    | x86_64       | ✓             | ✓                | ✓                 |
| Linux    | aarch64      | ✓             | ✓                | ✓                 |
| macOS    | x86_64       | ✓             | ✓                | ✓                 |
| macOS    | aarch64      | ✓             | ✓                | ✓                 |
| Windows  | WSL2         | ✓             | ✗                | ✓                 |

## Support

- **Issues**: https://github.com/lprior-repo/zjj/issues
- **Documentation**: `docs/INDEX.md`
- **Contributing**: See `CONTRIBUTING.md` (if available)

---

**Happy coding with ZJJ!**

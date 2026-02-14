# Installation

This guide covers installing ZJJ and its prerequisites.

---

## Prerequisites

Before installing ZJJ, you need:

### Required Tools

| Tool | Purpose | Installation Link |
|------|---------|------------------|
| **JJ (Jujutsu)** | Version control | [Install JJ](https://github.com/martinvonz/jj#installation) |
| **Zellij** | Terminal multiplexer | [Install Zellij](https://zellij.dev/download) |
| **Rust** 1.80+ | Build toolchain | [Install Rust](https://rustup.rs/) |

### Optional Tools

| Tool | Purpose | Installation Link |
|------|---------|------------------|
| **Moon** | Build system (for development) | [Install Moon](https://moonrepo.dev/docs/install) |
| **bazel-remote** | Local caching (for development) | See setup below |

<div class="info">
💡 <strong>Tip</strong>: If you're just using ZJJ, you only need JJ, Zellij, and Rust. Moon and bazel-remote are only needed if you're building from source or contributing.
</div>

---

## Installation Options

### Option 1: From Source (Recommended)

This gives you the latest features and allows you to contribute.

```bash
# Clone the repository
git clone https://github.com/lprior-repo/zjj.git
cd zjj

# Install Moon (if not already installed)
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Build with Moon
moon run :build

# Install the binary
moon run :install

# Or copy manually
sudo cp target/release/zjj /usr/local/bin/

# Verify installation
zjj --version
```

### Option 2: From crates.io

```bash
cargo install zjj
```

<div class="warning">
⚠️ <strong>Warning</strong>: The crates.io version may lag behind the latest GitHub release. Building from source is recommended for the best experience.
</div>

---

## Verifying Installation

After installation, verify everything works:

```bash
# Check ZJJ version
zjj --version

# Check JJ is installed
jj --version

# Check Zellij is installed
zellij --version

# Run system health check
zjj doctor
```

---

## Installing Prerequisites

### Installing JJ (Jujutsu)

**macOS:**
```bash
brew install jj
```

**Linux:**
```bash
# Arch Linux
yay -S jujutsu

# Ubuntu/Debian (via cargo)
cargo install --locked jj-cli
```

### Installing Zellij

**macOS:**
```bash
brew install zellij
```

**Linux:**
```bash
# Via cargo
cargo install --locked zellij

# Or via package manager
sudo pacman -S zellij  # Arch Linux
```

### Installing Rust

Visit [https://rustup.rs/](https://rustup.rs/) and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Post-Installation Setup

### Initialize ZJJ in a Repository

```bash
# Navigate to a JJ repository
cd /path/to/your/repo

# If not a JJ repo yet
jj init

# Initialize ZJJ
zjj init
```

---

## Next Steps

- **[Quick Start Guide](./quickstart.md)** - Get running in 5 minutes
- **[Core Concepts](./concepts.md)** - Understand sessions and queues
- **[User Guide](./guide/workspaces.md)** - Learn all the features

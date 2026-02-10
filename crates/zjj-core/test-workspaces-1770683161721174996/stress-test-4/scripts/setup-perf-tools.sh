#!/usr/bin/env bash
# Install performance tools for faster Rust builds and tests

set -euo pipefail

echo "🚀 Setting up performance tools for zjj..."
echo

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "⚠️  Warning: Some tools (mold) are Linux-only"
fi

# Install sccache (compilation cache)
echo "📦 Installing sccache (shared compilation cache)..."
if ! command -v sccache &> /dev/null; then
    cargo install sccache --locked
    echo "✅ sccache installed"
else
    echo "✅ sccache already installed"
fi

# Install mold linker (Linux only - much faster linking)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "📦 Installing mold linker (fast linking)..."
    if ! command -v mold &> /dev/null; then
        # Check if we can use package manager
        if command -v pacman &> /dev/null; then
            echo "Installing via pacman..."
            sudo pacman -S --noconfirm mold
        elif command -v apt &> /dev/null; then
            echo "Installing via apt..."
            sudo apt install -y mold
        else
            echo "⚠️  Please install mold manually: https://github.com/rui314/mold"
            echo "   (Skipping mold installation)"
        fi

        if command -v mold &> /dev/null; then
            echo "✅ mold installed"
        fi
    else
        echo "✅ mold already installed"
    fi
fi

# Verify cargo-nextest is installed
echo "📦 Verifying cargo-nextest..."
if ! command -v cargo-nextest &> /dev/null; then
    echo "Installing cargo-nextest (faster test runner)..."
    cargo install cargo-nextest --locked
    echo "✅ cargo-nextest installed"
else
    echo "✅ cargo-nextest already installed"
fi

echo
echo "🔧 Configuring .cargo/config.toml..."

# Enable sccache in .cargo/config.toml
CONFIG_FILE=".cargo/config.toml"
if command -v sccache &> /dev/null; then
    if ! grep -q "rustc-wrapper.*sccache" "$CONFIG_FILE"; then
        echo "Enabling sccache in $CONFIG_FILE..."
        # Uncomment sccache line
        sed -i 's|^# rustc-wrapper = "sccache"|rustc-wrapper = "sccache"|' "$CONFIG_FILE"
        # If that didn't work, add it manually
        if ! grep -q "rustc-wrapper.*sccache" "$CONFIG_FILE"; then
            cat >> "$CONFIG_FILE" << 'EOF'

# Added by setup-perf-tools.sh
[build]
rustc-wrapper = "sccache"
EOF
        fi
        echo "✅ sccache enabled"
    else
        echo "✅ sccache already enabled"
    fi
fi

# Enable mold linker (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]] && command -v mold &> /dev/null; then
    if ! grep -q "link-arg=-fuse-ld=mold" "$CONFIG_FILE"; then
        echo "Enabling mold linker in $CONFIG_FILE..."
        # Uncomment mold lines
        sed -i 's|^# \[target.x86_64-unknown-linux-gnu\]|[target.x86_64-unknown-linux-gnu]|' "$CONFIG_FILE"
        sed -i 's|^# linker = "clang"|linker = "clang"|' "$CONFIG_FILE"
        sed -i 's|^# rustflags = \["-C", "link-arg=-fuse-ld=mold"\]|rustflags = ["-C", "link-arg=-fuse-ld=mold"]|' "$CONFIG_FILE"
        echo "✅ mold linker enabled"
    else
        echo "✅ mold linker already enabled"
    fi
fi

echo
echo "✨ Performance tools setup complete!"
echo
echo "Expected speedups:"
echo "  • sccache: 50-90% faster rebuilds (shared cache)"
echo "  • mold: 2-5x faster linking (Linux)"
echo "  • cargo-nextest: 2-4x faster test execution"
echo "  • moon caching: Skip unchanged tasks entirely"
echo
echo "Run 'sccache --show-stats' to see cache statistics"
echo "Run 'moon run :ci' to test the full optimized pipeline"

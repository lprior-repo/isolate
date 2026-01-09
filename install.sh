#!/usr/bin/env bash
# Quick install script for jjz development

set -e

echo "Building jjz in release mode..."
cargo build --release

echo "Installing to ~/.local/bin/jjz..."
mkdir -p ~/.local/bin
cp target/release/jjz ~/.local/bin/jjz
chmod +x ~/.local/bin/jjz

echo "✓ Installed jjz to ~/.local/bin/jjz"
echo ""
echo "You can now run 'jjz' from anywhere!"
jjz --version

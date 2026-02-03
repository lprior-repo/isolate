#!/usr/bin/env bash
# Quick install script for zjj development

set -e

echo "Building zjj in release mode..."
cargo build --release

echo "Installing to ~/.local/bin/zjj..."
mkdir -p ~/.local/bin
cp target/release/zjj ~/.local/bin/zjj
chmod +x ~/.local/bin/zjj

echo "✓ Installed zjj to ~/.local/bin/zjj"
echo ""
echo "You can now run 'zjj' from anywhere!"
zjj --version

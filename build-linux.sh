#!/bin/bash
# Quick build script for Linux

set -e

export PATH="$HOME/.cargo/bin:$PATH"

echo "=== TrackPersonalInsights Build Script ==="
echo "Building release binary for Linux..."

cd "$(dirname "$0")"

# Clean previous builds
cargo clean --release

# Build
cargo build --release

# Copy to releases folder
mkdir -p releases
cp target/release/TrackPersonalInsights releases/TrackPersonalInsights-linux-x86_64
chmod +x releases/TrackPersonalInsights-linux-x86_64

echo ""
echo "✅ Build complete!"
echo "Binary: releases/TrackPersonalInsights-linux-x86_64"
ls -lh releases/TrackPersonalInsights-linux-x86_64

echo ""
echo "To run: ./releases/TrackPersonalInsights-linux-x86_64"
echo ""
echo "📚 Optional: Install system dictionary for better spell checking:"
echo "  Ubuntu/Debian: sudo apt install wamerican"
echo "  Arch Linux:    sudo pacman -S words"

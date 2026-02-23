#!/bin/bash
# Cross-platform build script

set -e

echo "Building Patent Hub..."

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    CYGWIN*)    MACHINE=Windows;;
    MINGW*)     MACHINE=Windows;;
    MSYS*)      MACHINE=Windows;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

echo "Detected OS: ${MACHINE}"

# Build
cargo build --release

# Copy files
mkdir -p dist
cp target/release/patent-hub* dist/ 2>/dev/null || true
cp -r templates dist/
cp -r static dist/
cp .env.example dist/

echo "Build complete! Files in dist/"
echo "Run: cd dist && ./patent-hub"

#!/bin/bash
set -e

# loar local build and packaging script
echo "=== Building loar for Release ==="

# Go to script directory
cd "$(dirname "$0")"

# Build release
cargo build --release

# Determine target architecture/OS
OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH_TYPE=$(uname -m)

ASSET_NAME="loar-local-${OS_TYPE}-${ARCH_TYPE}.tar.gz"

echo "=== Packaging Artifacts ==="
cd target/release
tar -czf "../../${ASSET_NAME}" loar

echo "=== Build and Packaging Complete! ==="
echo "Artifact created: ${ASSET_NAME}"

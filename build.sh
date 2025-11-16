#!/bin/bash
# Build script for Kasate extension

set -e

echo "Building Kasate extension..."

# Check if cargo-pgrx is installed
if ! command -v cargo-pgrx &> /dev/null; then
    echo "cargo-pgrx not found. Installing..."
    cargo install --locked cargo-pgrx
fi

# Build the extension
echo "Building for PostgreSQL 16..."
cargo pgrx build --pg16

echo "Build complete!"
echo ""
echo "To install the extension, run:"
echo "  cargo pgrx install --pg16"
echo ""
echo "To run tests, run:"
echo "  cargo test"
echo "  cargo pgrx test pg16"

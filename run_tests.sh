#!/bin/bash
# Test script for Kasate extension

set -e

echo "Running Kasate tests..."
echo ""

# Run Rust unit tests
echo "=== Running Rust unit tests ==="
cargo test --lib
echo ""

# Run PGRX integration tests
echo "=== Running PGRX tests ==="
cargo pgrx test pg16
echo ""

echo "All tests completed!"
echo ""
echo "To run integration tests with a running PostgreSQL instance:"
echo "  psql -d your_database -f test_integration.sql"
echo ""
echo "To try the examples:"
echo "  psql -d your_database -f examples/basic_usage.sql"

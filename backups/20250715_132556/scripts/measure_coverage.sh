#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "🔍 Measuring test coverage..."

if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

mkdir -p coverage-report

cd controller
cargo tarpaulin --features "mock-kubernetes test-integration test-isolated" --out Json --output-dir ../coverage-report
cargo tarpaulin --features "mock-kubernetes test-integration test-isolated" --out Html --output-dir ../coverage-report

COVERAGE=$(grep -o '"line_rate":[0-9.]*' ../coverage-report/tarpaulin-report.json | grep -o '[0-9.]*')
TARGET=0.8

echo "Current test coverage: ${COVERAGE}"
echo "Target test coverage: ${TARGET}"

if (( $(echo "$COVERAGE < $TARGET" | bc -l) )); then
    echo "❌ Test coverage (${COVERAGE}) is below target (${TARGET})"
    echo "Please add more tests to increase coverage"
    exit 1
else
    echo "✅ Test coverage (${COVERAGE}) meets or exceeds target (${TARGET})"
fi

echo "Coverage report generated at coverage-report/"

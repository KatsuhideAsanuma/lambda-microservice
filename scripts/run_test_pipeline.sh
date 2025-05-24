#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "🧪 Starting gradual test pipeline"

echo "Step 1: Running unit tests..."
cd controller
cargo test --lib
if [ $? -ne 0 ]; then
    echo "❌ Unit tests failed"
    exit 1
fi
echo "✅ Unit tests passed"

echo "Step 2: Running isolated component tests..."
cd ..
./scripts/run_isolated_tests.sh
if [ $? -ne 0 ]; then
    echo "❌ Isolated component tests failed"
    exit 1
fi
echo "✅ Isolated component tests passed"

echo "Step 3: Running integration tests..."
./scripts/run_integration_tests.sh
if [ $? -ne 0 ]; then
    echo "❌ Integration tests failed"
    exit 1
fi
echo "✅ Integration tests passed"

if [ -f "./scripts/test_e2e.sh" ]; then
    echo "Step 4: Running end-to-end tests..."
    ./scripts/test_e2e.sh
    if [ $? -ne 0 ]; then
        echo "❌ End-to-end tests failed"
        exit 1
    fi
    echo "✅ End-to-end tests passed"
else
    echo "Step 4: Skipping end-to-end tests (script not found)"
fi

echo "Step 5: Measuring test coverage..."
./scripts/measure_coverage.sh
if [ $? -ne 0 ]; then
    echo "❌ Test coverage is below target"
    exit 1
fi
echo "✅ Test coverage meets or exceeds target"

echo "🎉 All tests passed successfully!"

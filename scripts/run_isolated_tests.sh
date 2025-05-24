#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Setting up isolated test environment..."
TEST_ENV_DIR=$(mktemp -d)
export TEST_ENV_DIR

mkdir -p $TEST_ENV_DIR/fixtures
cp -r test/fixtures/* $TEST_ENV_DIR/fixtures/ 2>/dev/null || true

echo "Starting mock services..."
./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh -f docker-compose.test.yml up -d

echo "Waiting for services to be ready..."
sleep 5

echo "Running isolated tests..."
cd controller
cargo test --features test-isolated

RESULT=$?

echo "Cleaning up test environment..."
./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh -f docker-compose.test.yml down
rm -rf $TEST_ENV_DIR

if [ $RESULT -eq 0 ]; then
    echo "✅ All isolated tests passed!"
else
    echo "❌ Some isolated tests failed!"
    exit 1
fi

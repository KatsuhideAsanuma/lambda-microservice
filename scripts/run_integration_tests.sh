set -e

cd "$(dirname "$0")/.."

if ! docker-compose ps | grep -q "controller"; then
    echo "Starting services..."
    docker-compose up -d
    
    echo "Waiting for services to be ready..."
    sleep 30
fi

echo "Running integration tests..."
cd controller
cargo test --features test-integration -- --ignored -v

if [ $? -eq 0 ]; then
    echo "✅ All integration tests passed!"
else
    echo "❌ Some integration tests failed!"
    exit 1
fi

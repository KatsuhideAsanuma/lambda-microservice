set -e

cd "$(dirname "$0")/.."

if ! ./scripts/docker_compose_compat.sh ps | grep -q "controller"; then
    echo "Starting services..."
    ./scripts/set_docker_env.sh ./scripts/docker_compose_compat.sh up -d
    
    echo "Waiting for services to be ready..."
    sleep 30
fi

echo "Running integration tests..."
./scripts/init_meta_schema.sh

echo "Running tests locally with connection to Docker postgres..."
cd controller
export TEST_DATABASE_URL="postgres://postgres:postgres@localhost:5432/lambda_microservice"
export RUST_BACKTRACE=1
export RUST_LOG=debug
cargo test --features test-integration -- --ignored

if [ $? -eq 0 ]; then
    echo "✅ All integration tests passed!"
else
    echo "❌ Some integration tests failed!"
    exit 1
fi

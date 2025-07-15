#!/bin/bash

# Docker環境でのテスト実行スクリプト
# Rust Nightly環境でのテストを実行

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🐳 Docker環境でのテスト実行${NC}"
echo "=================================="

# Docker環境でのテスト実行
run_docker_tests() {
    echo -e "${YELLOW}📋 Docker環境でテストを実行中...${NC}"
    
    # Rust Nightly環境でのテスト実行
    docker run --rm \
        -v "$PROJECT_ROOT/controller:/app" \
        -w /app \
        rustlang/rust:nightly-slim \
        bash -c "
            apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler
            cargo test --lib --no-default-features
        "
}

# 統合テスト用のDocker Compose環境でのテスト
run_integration_tests() {
    echo -e "${YELLOW}🔗 統合テスト環境を準備中...${NC}"
    
    # Docker Compose環境でのテスト実行
    docker run --rm \
        -v "$PROJECT_ROOT:/workspace" \
        -w /workspace/controller \
        --network host \
        rustlang/rust:nightly-slim \
        bash -c "
            apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler
            export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/lambda_microservice'
            export REDIS_URL='redis://localhost:6379'
            cargo test --features test-integration -- --ignored
        "
}

# パフォーマンステスト用のDocker環境
run_performance_tests() {
    echo -e "${YELLOW}⚡ パフォーマンステストを実行中...${NC}"
    
    # パフォーマンステスト用のコンテナ実行
    docker run --rm \
        -v "$PROJECT_ROOT:/workspace" \
        -w /workspace \
        --network host \
        alpine:latest \
        sh -c "
            apk add --no-cache curl jq bc
            ./scripts/performance_test_simple.sh
        "
}

# メイン実行
case "${1:-all}" in
    "unit")
        run_docker_tests
        ;;
    "integration")
        run_integration_tests
        ;;
    "performance")
        run_performance_tests
        ;;
    "all")
        echo -e "${BLUE}🚀 全テストを実行します${NC}"
        run_docker_tests
        echo ""
        run_integration_tests
        echo ""
        run_performance_tests
        ;;
    *)
        echo "使用方法: $0 [unit|integration|performance|all]"
        exit 1
        ;;
esac

echo -e "${GREEN}✅ テスト実行完了${NC}"

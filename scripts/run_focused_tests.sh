#!/bin/bash
set -e

cd "$(dirname "$0")/.."

MODULE=$1
if [ -z "$MODULE" ]; then
    echo "使用方法: $0 <module_name>"
    echo "例: $0 function"
    exit 1
fi

echo "🔍 ${MODULE} モジュールの単体テストを実行中..."

cd controller
rm -rf target/debug/deps/lambda_microservice_controller-*

if [ "$MODULE" = "main" ]; then
    RUST_BACKTRACE=1 cargo test --test main_tests -- --nocapture
elif [ "$MODULE" = "openfaas" ]; then
    RUST_BACKTRACE=1 cargo test --lib -- openfaas::tests::tests:: --nocapture
    RUST_BACKTRACE=1 cargo test --test openfaas_tests -- --nocapture
else
    RUST_BACKTRACE=1 cargo test --lib -- ${MODULE}::tests:: --nocapture
    if [ -f "tests/${MODULE}_tests.rs" ]; then
        RUST_BACKTRACE=1 cargo test --test ${MODULE}_tests -- --nocapture
    fi
fi

echo "✅ ${MODULE} モジュールのテスト完了"

#!/bin/bash
set -e

cd "$(dirname "$0")/.."

MODULE=$1
if [ -z "$MODULE" ]; then
    echo "使用方法: $0 <module_name>"
    echo "例: $0 function"
    exit 1
fi

echo "🔍 ${MODULE} モジュールのテストカバレッジを測定中..."

cd controller
rm -rf target/debug/deps/lambda_microservice_controller-*

if [ "$MODULE" = "openfaas" ]; then
    RUST_BACKTRACE=1 cargo tarpaulin --features test-integration --lib --out Html --output-dir ../coverage-report/${MODULE} -- ${MODULE}::tests
    RUST_BACKTRACE=1 cargo tarpaulin --features test-integration --test openfaas_tests --out Html --output-dir ../coverage-report/${MODULE}-tests
elif [ "$MODULE" = "main" ]; then
    RUST_BACKTRACE=1 cargo tarpaulin --features "mock-kubernetes test-integration" --test main_tests --out Html --output-dir ../coverage-report/${MODULE}
    RUST_BACKTRACE=1 cargo tarpaulin --features "mock-kubernetes test-integration" --test lib_main_tests --out Html --output-dir ../coverage-report/${MODULE}-lib
elif [ "$MODULE" = "kubernetes" ]; then
    RUST_BACKTRACE=1 cargo tarpaulin --features mock-kubernetes --lib --out Html --output-dir ../coverage-report/${MODULE} -- ${MODULE}::tests
    RUST_BACKTRACE=1 cargo tarpaulin --features mock-kubernetes --test kubernetes_tests --out Html --output-dir ../coverage-report/${MODULE}-tests
else
    RUST_BACKTRACE=1 cargo tarpaulin --lib --out Html --output-dir ../coverage-report/${MODULE} -- ${MODULE}::tests
    if [ -f "tests/${MODULE}_tests.rs" ]; then
        RUST_BACKTRACE=1 cargo tarpaulin --test ${MODULE}_tests --out Html --output-dir ../coverage-report/${MODULE}-tests
    fi
fi

echo "✅ ${MODULE} モジュールのテストカバレッジ測定完了"
echo "レポートは coverage-report/${MODULE}/tarpaulin-report.html に保存されました"

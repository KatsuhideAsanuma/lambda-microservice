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
elif [ "$MODULE" = "main" ]; then
    RUST_BACKTRACE=1 cargo tarpaulin --test main_tests --out Html --output-dir ../coverage-report/${MODULE}
else
    RUST_BACKTRACE=1 cargo tarpaulin --lib --out Html --output-dir ../coverage-report/${MODULE} -- ${MODULE}::tests
fi

echo "✅ ${MODULE} モジュールのテストカバレッジ測定完了"
echo "レポートは coverage-report/${MODULE}/tarpaulin-report.html に保存されました"

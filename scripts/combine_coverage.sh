#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "🔍 全モジュールの統合テストカバレッジを測定中..."

mkdir -p coverage-report/combined

MODULES=("function" "cache" "database" "openfaas" "main" "kubernetes")

for MODULE in "${MODULES[@]}"; do
  echo "テスト実行: ${MODULE}"
  ./scripts/run_focused_tests.sh ${MODULE} || echo "警告: ${MODULE}のテスト実行に失敗しました"
done

cd controller
RUST_BACKTRACE=1 cargo tarpaulin --features "test-integration mock-kubernetes" --lib --tests --out Html --output-dir ../coverage-report/combined

echo "✅ 統合テストカバレッジ測定完了"
echo "レポートは coverage-report/combined/tarpaulin-report.html に保存されました"

COVERAGE=$(grep -o "[0-9]\+\.[0-9]\+% coverage" ../coverage-report/combined/tarpaulin-report.html | grep -o "[0-9]\+\.[0-9]\+")
echo "現在のカバレッジ率: ${COVERAGE}%"

if (( $(echo "${COVERAGE} >= 50" | bc -l) )); then
  echo "🎉 目標カバレッジ(50%)を達成しました！"
else
  echo "⚠️ 目標カバレッジ(50%)に達していません。現在: ${COVERAGE}%"
fi

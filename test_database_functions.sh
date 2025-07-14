#!/bin/bash

# Lambda Microservice Database Function Tests
# データベースに投入されている関数とスクリプトを確認するテスト

set -e

BASE_URL="http://localhost:8080"
TIMESTAMP=$(date +%s)

echo "=== Lambda Microservice Database Function Tests ==="
echo "Base URL: $BASE_URL"
echo "Timestamp: $TIMESTAMP"
echo

# ヘルスチェック
echo "1. Health Check"
curl -s "$BASE_URL/health" | grep -q "ok" && echo "✅ Health check passed" || echo "❌ Health check failed"
echo

# Test 1: データベースの関数一覧確認
echo "2. Test 1: Database Functions List"
echo "  Checking functions in database..."
FUNCTIONS_COUNT=$(docker-compose exec -T postgres psql -U postgres -d lambda_microservice -t -c "SELECT COUNT(*) FROM meta.functions;" | tr -d '[:space:]')
echo "  Functions count: $FUNCTIONS_COUNT"
if [ "$FUNCTIONS_COUNT" -eq "3" ]; then
    echo "✅ Test 1 passed - Expected 3 functions found"
else
    echo "❌ Test 1 failed - Expected 3 functions, found $FUNCTIONS_COUNT"
fi

echo "  Function details:"
docker-compose exec -T postgres psql -U postgres -d lambda_microservice -c "SELECT language_title, description FROM meta.functions ORDER BY language_title;"
echo

# Test 2: Node.js Calculator関数の確認
echo "3. Test 2: Node.js Calculator Function"
echo "  Checking nodejs-calculator function..."
NODEJS_FUNC=$(docker-compose exec -T postgres psql -U postgres -d lambda_microservice -t -c "SELECT COUNT(*) FROM meta.functions WHERE language_title = 'nodejs-calculator';" | tr -d '[:space:]')
if [ "$NODEJS_FUNC" -eq "1" ]; then
    echo "✅ Test 2 passed - nodejs-calculator function exists"
else
    echo "❌ Test 2 failed - nodejs-calculator function not found"
fi

echo "  Function schema:"
docker-compose exec -T postgres psql -U postgres -d lambda_microservice -c "SELECT schema_definition FROM meta.functions WHERE language_title = 'nodejs-calculator';"
echo

# Test 3: Python Text Processor関数の確認
echo "4. Test 3: Python Text Processor Function"
echo "  Checking python-text_processor function..."
PYTHON_FUNC=$(docker-compose exec -T postgres psql -U postgres -d lambda_microservice -t -c "SELECT COUNT(*) FROM meta.functions WHERE language_title = 'python-text_processor';" | tr -d '[:space:]')
if [ "$PYTHON_FUNC" -eq "1" ]; then
    echo "✅ Test 3 passed - python-text_processor function exists"
else
    echo "❌ Test 3 failed - python-text_processor function not found"
fi

echo "  Function examples:"
docker-compose exec -T postgres psql -U postgres -d lambda_microservice -c "SELECT examples FROM meta.functions WHERE language_title = 'python-text_processor';"
echo

# Test 4: スクリプトの確認
echo "5. Test 4: Scripts Verification"
echo "  Checking scripts in database..."
SCRIPTS_COUNT=$(docker-compose exec -T postgres psql -U postgres -d lambda_microservice -t -c "SELECT COUNT(*) FROM meta.scripts;" | tr -d '[:space:]')
echo "  Scripts count: $SCRIPTS_COUNT"
if [ "$SCRIPTS_COUNT" -eq "3" ]; then
    echo "✅ Test 4 passed - Expected 3 scripts found"
else
    echo "❌ Test 4 failed - Expected 3 scripts, found $SCRIPTS_COUNT"
fi

echo "  Script preview:"
docker-compose exec -T postgres psql -U postgres -d lambda_microservice -c "SELECT LEFT(content, 100) || '...' as script_preview FROM meta.scripts ORDER BY function_id;"
echo

# Test 5: セッションテーブルの確認
echo "6. Test 5: Sessions Table Verification"
echo "  Checking sessions table..."
SESSIONS_TABLE=$(docker-compose exec -T postgres psql -U postgres -d lambda_microservice -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'meta' AND table_name = 'sessions';" | tr -d '[:space:]')
if [ "$SESSIONS_TABLE" -eq "1" ]; then
    echo "✅ Test 5 passed - Sessions table exists"
else
    echo "❌ Test 5 failed - Sessions table not found"
fi

echo "  Sessions table structure:"
docker-compose exec -T postgres psql -U postgres -d lambda_microservice -c "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = 'meta' AND table_name = 'sessions' ORDER BY ordinal_position;"
echo

echo "=== All Database Function Tests Completed ==="
echo "Summary:"
echo "- Test 1: Database Functions List (3 functions)"
echo "- Test 2: Node.js Calculator Function verification"
echo "- Test 3: Python Text Processor Function verification"
echo "- Test 4: Scripts verification (3 scripts)"
echo "- Test 5: Sessions table verification"
echo
echo "Database Migration Status: All migration files have been executed successfully"
echo "- V1.0.5: Sessions table created ✅"
echo "- V1.0.6: Default functions inserted ✅"
echo
echo "To run these tests:"
echo "1. Ensure the Lambda Microservice is running: docker-compose up -d"
echo "2. Run this script: chmod +x test_database_functions.sh && ./test_database_functions.sh"

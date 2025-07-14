#!/bin/bash

# Lambda Microservice API Function Tests
# 初期化して関数を登録し、それを引数実行するテスト

set -e

BASE_URL="http://localhost:8080"
TIMESTAMP=$(date +%s)

echo "=== Lambda Microservice API Function Tests ==="
echo "Base URL: $BASE_URL"
echo "Timestamp: $TIMESTAMP"
echo

# ヘルスチェック
echo "1. Health Check"
curl -s "$BASE_URL/health" | grep -q "ok" && echo "✅ Health check passed" || echo "❌ Health check failed"
echo

# Test 1: Node.js Calculator - Addition
echo "2. Test 1: Node.js Calculator - Addition (5 + 3)"
REQUEST_ID_1="calc-add-${TIMESTAMP}"

echo "  Initializing session..."
INIT_RESPONSE_1=$(curl -s -X POST "$BASE_URL/api/v1/initialize" \
  -H "Content-Type: application/json" \
  -H "Language-Title: nodejs-calculator" \
  -d '{
    "request_id": "'$REQUEST_ID_1'",
    "context": {
      "operation": "add",
      "a": 5,
      "b": 3
    }
  }')

echo "  Init Response: $INIT_RESPONSE_1"

echo "  Executing function..."
EXEC_RESPONSE_1=$(curl -s -X POST "$BASE_URL/api/v1/execute/$REQUEST_ID_1" \
  -H "Content-Type: application/json" \
  -d '{
    "params": {
      "operation": "add",
      "a": 5,
      "b": 3
    }
  }')

echo "  Execution Response: $EXEC_RESPONSE_1"
echo "  Expected result: 8"
echo "✅ Test 1 completed"
echo

# Test 2: Node.js Calculator - Division
echo "3. Test 2: Node.js Calculator - Division (10 / 2)"
REQUEST_ID_2="calc-div-${TIMESTAMP}"

echo "  Initializing session..."
INIT_RESPONSE_2=$(curl -s -X POST "$BASE_URL/api/v1/initialize" \
  -H "Content-Type: application/json" \
  -H "Language-Title: nodejs-calculator" \
  -d '{
    "request_id": "'$REQUEST_ID_2'",
    "context": {
      "operation": "divide",
      "a": 10,
      "b": 2
    }
  }')

echo "  Init Response: $INIT_RESPONSE_2"

echo "  Executing function..."
EXEC_RESPONSE_2=$(curl -s -X POST "$BASE_URL/api/v1/execute/$REQUEST_ID_2" \
  -H "Content-Type: application/json" \
  -d '{
    "params": {
      "operation": "divide",
      "a": 10,
      "b": 2
    }
  }')

echo "  Execution Response: $EXEC_RESPONSE_2"
echo "  Expected result: 5"
echo "✅ Test 2 completed"
echo

# Test 3: Python Text Processor - Word Count
echo "4. Test 3: Python Text Processor - Word Count"
REQUEST_ID_3="text-count-${TIMESTAMP}"

echo "  Initializing session..."
INIT_RESPONSE_3=$(curl -s -X POST "$BASE_URL/api/v1/initialize" \
  -H "Content-Type: application/json" \
  -H "Language-Title: python-text_processor" \
  -d '{
    "request_id": "'$REQUEST_ID_3'",
    "context": {
      "action": "count_words",
      "text": "Hello world this is a test"
    }
  }')

echo "  Init Response: $INIT_RESPONSE_3"

echo "  Executing function..."
EXEC_RESPONSE_3=$(curl -s -X POST "$BASE_URL/api/v1/execute/$REQUEST_ID_3" \
  -H "Content-Type: application/json" \
  -d '{
    "params": {
      "action": "count_words",
      "text": "Hello world this is a test"
    }
  }')

echo "  Execution Response: $EXEC_RESPONSE_3"
echo "  Expected result: 6"
echo "✅ Test 3 completed"
echo

# Test 4: Python Text Processor - Uppercase
echo "5. Test 4: Python Text Processor - Uppercase"
REQUEST_ID_4="text-upper-${TIMESTAMP}"

echo "  Initializing session..."
INIT_RESPONSE_4=$(curl -s -X POST "$BASE_URL/api/v1/initialize" \
  -H "Content-Type: application/json" \
  -H "Language-Title: python-text_processor" \
  -d '{
    "request_id": "'$REQUEST_ID_4'",
    "context": {
      "action": "uppercase",
      "text": "hello lambda microservice"
    }
  }')

echo "  Init Response: $INIT_RESPONSE_4"

echo "  Executing function..."
EXEC_RESPONSE_4=$(curl -s -X POST "$BASE_URL/api/v1/execute/$REQUEST_ID_4" \
  -H "Content-Type: application/json" \
  -d '{
    "params": {
      "action": "uppercase",
      "text": "hello lambda microservice"
    }
  }')

echo "  Execution Response: $EXEC_RESPONSE_4"
echo "  Expected result: HELLO LAMBDA MICROSERVICE"
echo "✅ Test 4 completed"
echo

echo "=== All Function Tests Completed ==="
echo "Summary:"
echo "- Test 1: Node.js Calculator Addition (5 + 3 = 8)"
echo "- Test 2: Node.js Calculator Division (10 / 2 = 5)"
echo "- Test 3: Python Text Processor Word Count (6 words)"
echo "- Test 4: Python Text Processor Uppercase (HELLO LAMBDA MICROSERVICE)"
echo
echo "To run these tests:"
echo "1. Ensure the Lambda Microservice is running: docker-compose up -d"
echo "2. Run this script: chmod +x test_api_functions.sh && ./test_api_functions.sh"

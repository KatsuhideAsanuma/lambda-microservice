#!/bin/bash

echo "=== Simple API Test ==="
echo "Testing initialize endpoint..."

# Test initialize endpoint
response=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST \
  -H "Content-Type: application/json" \
  -H "Language-Title: nodejs-calculator" \
  -d '{
    "context": {"env": "test"},
    "script_content": "module.exports = async (event) => { return { result: event.params.a + event.params.b }; }"
  }' \
  http://localhost:8080/api/v1/initialize)

echo "Response:"
echo "$response"
echo ""

# Extract HTTP code
http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
response_body=$(echo "$response" | sed '/HTTP_CODE:/d')

echo "HTTP Code: $http_code"
echo "Response Body: $response_body"

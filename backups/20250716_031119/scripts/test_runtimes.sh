#!/bin/bash
set -e

CONTROLLER_URL=${CONTROLLER_URL:-"http://localhost:8080"}
NODEJS_URL=${NODEJS_URL:-"http://localhost:8081"}
PYTHON_URL=${PYTHON_URL:-"http://localhost:8082"}
RUST_URL=${RUST_URL:-"http://localhost:8083"}

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing Lambda Microservice Runtime Containers${NC}"

echo -e "\n${YELLOW}Testing health endpoints:${NC}"

echo -n "Controller health check: "
if curl -s "${CONTROLLER_URL}/health" | grep -q "ok"; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -n "Node.js runtime health check: "
if curl -s "${NODEJS_URL}/health" | grep -q "ok"; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -n "Python runtime health check: "
if curl -s "${PYTHON_URL}/health" | grep -q "ok"; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -n "Rust runtime health check: "
if curl -s "${RUST_URL}/health" | grep -q "ok"; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -e "\n${YELLOW}Initializing a session:${NC}"
REQUEST_ID=$(uuidgen)
echo "Request ID: ${REQUEST_ID}"

INIT_RESPONSE=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/initialize" \
    -H "Content-Type: application/json" \
    -H "Language-Title: nodejs-calculator" \
    -d "{
        \"context\": {\"env\": \"test\"},
        \"script_content\": \"const operations = {
            add: (a, b) => a + b,
            subtract: (a, b) => a - b,
            multiply: (a, b) => a * b,
            divide: (a, b) => b !== 0 ? a / b : \\\"Error: Division by zero\\\"
        };
        
        const { operation, a, b } = event.params;
        
        if (!operations[operation]) {
            return { error: \\\"Invalid operation\\\" };
        }
        
        return { result: operations[operation](a, b) };\"
    }")

echo "Initialization response:"
echo "${INIT_RESPONSE}" | jq . || echo "${INIT_RESPONSE}"

EXTRACTED_REQUEST_ID=$(echo "${INIT_RESPONSE}" | jq -r '.request_id' 2>/dev/null || echo "${REQUEST_ID}")

echo -e "\n${YELLOW}Executing the function:${NC}"
EXEC_RESPONSE=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute/${EXTRACTED_REQUEST_ID}" \
    -H "Content-Type: application/json" \
    -d "{
        \"params\": {
            \"operation\": \"add\",
            \"a\": 5,
            \"b\": 3
        }
    }")

echo "Execution response:"
echo "${EXEC_RESPONSE}" | jq . || echo "${EXEC_RESPONSE}"

echo -e "\n${YELLOW}Checking database for logs:${NC}"
echo "To check logs in the database, run:"
echo "docker-compose exec postgres psql -U postgres -d lambda_microservice -c \"SELECT * FROM public.request_logs ORDER BY timestamp DESC LIMIT 5;\""
echo "docker-compose exec postgres psql -U postgres -d lambda_microservice -c \"SELECT * FROM public.error_logs ORDER BY timestamp DESC LIMIT 5;\""

echo -e "\n${GREEN}Testing completed!${NC}"

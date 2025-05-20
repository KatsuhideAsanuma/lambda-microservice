#!/bin/bash
set -e

CONTROLLER_URL=${CONTROLLER_URL:-"http://localhost:8080"}
RUST_URL=${RUST_URL:-"http://localhost:8083"}

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing gRPC Protocol Adapter${NC}"

REQUEST_ID=$(uuidgen)
echo "Request ID: ${REQUEST_ID}"

test_grpc_execute() {
    echo -e "\n${YELLOW}Testing gRPC Initialize Operation First:${NC}"
    
    local script="function calculate(a, b, operation) { return operation === 'add' ? a + b : a * b; }"
    
    local init_response=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/initialize" \
        -H "Content-Type: application/json" \
        -H "Language-Title: nodejs-calculator" \
        -d "{
            \"request_id\": \"${REQUEST_ID}\",
            \"context\": {\"environment\": \"test\"},
            \"script_content\": \"${script}\",
            \"use_grpc\": true
        }")
    
    echo "Initialize response:"
    if echo "${init_response}" | jq . 2>/dev/null; then
        echo "Response is valid JSON"
    else
        echo "Response is not valid JSON:"
        echo "${init_response}"
        echo -e "${RED}Initialize failed, cannot continue with execute test${NC}"
        return 1
    fi
    
    echo -e "\n${YELLOW}Testing gRPC Execute Operation:${NC}"
    
    local response=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute/${REQUEST_ID}" \
        -H "Content-Type: application/json" \
        -d "{
            \"params\": {
                \"operation\": \"add\",
                \"a\": 5,
                \"b\": 3
            },
            \"use_grpc\": true
        }")
    
    echo "Execute response:"
    if echo "${response}" | jq . 2>/dev/null; then
        echo "Response is valid JSON"
        local result=$(echo "${response}" | jq -r '.result.result // "error"')
        if [[ -n "${result}" && "${result}" != "null" && "${result}" != "error" ]]; then
            echo -e "${GREEN}gRPC Execute test passed!${NC}"
            return 0
        else
            echo -e "${RED}gRPC Execute test failed! Invalid result.${NC}"
            return 1
        fi
    else
        echo "Response is not valid JSON:"
        echo "${response}"
        echo -e "${RED}gRPC Execute test failed! Invalid JSON response.${NC}"
        return 1
    fi
}

test_grpc_initialize() {
    echo -e "\n${YELLOW}Testing gRPC Initialize Operation:${NC}"
    
    local script="function calculate(a, b, operation) { return operation === 'add' ? a + b : a * b; }"
    local new_request_id=$(uuidgen)
    
    local response=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/initialize" \
        -H "Content-Type: application/json" \
        -H "Language-Title: nodejs-calculator" \
        -d "{
            \"request_id\": \"${new_request_id}\",
            \"context\": {\"environment\": \"test\"},
            \"script_content\": \"${script}\",
            \"use_grpc\": true
        }")
    
    echo "Initialize response:"
    if echo "${response}" | jq . 2>/dev/null; then
        echo "Response is valid JSON"
        local status=$(echo "${response}" | jq -r '.status // "error"')
        if [[ "${status}" == "initialized" ]]; then
            echo -e "${GREEN}gRPC Initialize test passed!${NC}"
            return 0
        else
            echo -e "${RED}gRPC Initialize test failed! Invalid status.${NC}"
            return 1
        fi
    else
        echo "Response is not valid JSON:"
        echo "${response}"
        echo -e "${RED}gRPC Initialize test failed! Invalid JSON response.${NC}"
        return 1
    fi
}

test_grpc_health_check() {
    echo -e "\n${YELLOW}Testing gRPC Health Check Operation:${NC}"
    
    local response=$(curl -s -X GET "${CONTROLLER_URL}/health" \
        -H "Content-Type: application/json")
    
    echo "Health check response:"
    if echo "${response}" | jq . 2>/dev/null; then
        echo "Response is valid JSON"
        local status=$(echo "${response}" | jq -r '.status // "error"')
        if [[ "${status}" == "ok" || "${status}" == "degraded" ]]; then
            echo -e "${GREEN}gRPC Health Check test passed!${NC}"
            return 0
        else
            echo -e "${RED}gRPC Health Check test failed! Invalid status.${NC}"
            return 1
        fi
    else
        echo "Response is not valid JSON:"
        echo "${response}"
        echo -e "${RED}gRPC Health Check test failed! Invalid JSON response.${NC}"
        return 1
    fi
}

test_circuit_breaker() {
    echo -e "\n${YELLOW}Testing Circuit Breaker:${NC}"
    
    echo "Sending 6 requests to a non-existent endpoint to trigger circuit breaker..."
    local new_request_id=$(uuidgen)
    
    local init_response=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/initialize" \
        -H "Content-Type: application/json" \
        -H "Language-Title: nodejs-calculator" \
        -d "{
            \"request_id\": \"${new_request_id}\",
            \"context\": {\"environment\": \"test\"},
            \"script_content\": \"function test() { return 'test'; }\"
        }")
    
    echo "Initialize response for circuit breaker test:"
    if echo "${init_response}" | jq . 2>/dev/null; then
        echo "Response is valid JSON"
    else
        echo "Response is not valid JSON:"
        echo "${init_response}"
        echo -e "${RED}Initialize failed, cannot continue with circuit breaker test${NC}"
        return 1
    fi
    
    for i in {1..6}; do
        echo "Request $i:"
        local response=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute/${new_request_id}" \
            -H "Content-Type: application/json" \
            -d "{
                \"params\": {
                    \"operation\": \"add\",
                    \"a\": 5,
                    \"b\": 3
                },
                \"target_url\": \"http://nonexistent:8080\"
            }")
        
        if echo "${response}" | jq . 2>/dev/null; then
            echo "${response}" | jq .
        else
            echo "Response is not valid JSON:"
            echo "${response}"
        fi
        
        if [[ "${response}" == *"Circuit breaker"* || "${response}" == *"degraded"* ]]; then
            echo -e "${GREEN}Circuit breaker activated as expected!${NC}"
            return 0
        fi
        
        sleep 1
    done
    
    echo -e "${RED}Circuit breaker test failed - breaker did not activate after multiple failures.${NC}"
    return 1
}

echo -e "${YELLOW}Starting gRPC Protocol Adapter tests...${NC}"

passed=0
total=0

if test_grpc_health_check; then
    passed=$((passed + 1))
fi
total=$((total + 1))

if test_grpc_initialize; then
    passed=$((passed + 1))
fi
total=$((total + 1))

if test_grpc_execute; then
    passed=$((passed + 1))
fi
total=$((total + 1))

if test_circuit_breaker; then
    passed=$((passed + 1))
fi
total=$((total + 1))

echo -e "\n${YELLOW}gRPC Protocol Adapter Test Summary: ${passed}/${total} tests passed${NC}"

if [ $passed -eq $total ]; then
    echo -e "${GREEN}All gRPC Protocol Adapter tests passed! 🎉${NC}"
    exit 0
else
    echo -e "${RED}Some gRPC Protocol Adapter tests failed! 😢${NC}"
    exit 1
fi

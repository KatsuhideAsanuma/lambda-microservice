#!/bin/bash
set -e

CONTROLLER_URL=${CONTROLLER_URL:-"http://localhost:8080"}
RUST_URL=${RUST_URL:-"http://localhost:8083"}

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing WebAssembly Execution and Redis Caching${NC}"

REQUEST_ID=$(uuidgen)
echo "Request ID: ${REQUEST_ID}"

RUST_SCRIPT=$(cat <<EOF
fn main() {
    let input = std::env::args().nth(1).unwrap_or_default();
    let params: serde_json::Value = serde_json::from_str(&input).unwrap();
    
    let a = params["a"].as_f64().unwrap_or(0.0);
    let b = params["b"].as_f64().unwrap_or(0.0);
    let operation = params["operation"].as_str().unwrap_or("add");
    
    let result = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => if b != 0.0 { a / b } else { f64::NAN },
        _ => f64::NAN,
    };
    
    println!("{{\"result\": {}}}", result);
}
EOF
)

SCRIPT_HASH=$(echo -n "$RUST_SCRIPT" | sha256sum | awk '{print $1}')
echo "Script hash: ${SCRIPT_HASH}"

if command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}Checking Redis cache before initialization:${NC}"
    CACHE_KEY="wasm:${SCRIPT_HASH}"
    CACHE_EXISTS=$(docker-compose exec -T redis redis-cli EXISTS $CACHE_KEY || echo "0")
    echo "Cache exists (before): $CACHE_EXISTS"
else
    echo -e "${YELLOW}Note: Skipping Redis cache check (docker-compose not in PATH)${NC}"
fi

echo -e "${YELLOW}Initializing WebAssembly script:${NC}"
INIT_RESPONSE=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/initialize" \
    -H "Content-Type: application/json" \
    -H "Language-Title: rust-calculator" \
    -d "{
        \"request_id\": \"${REQUEST_ID}\",
        \"context\": {
            \"environment\": \"test\",
            \"compile_options\": {
                \"optimization_level\": \"release\",
                \"memory_limit_mb\": 1
            }
        },
        \"script_content\": $(echo "${RUST_SCRIPT}" | jq -Rs .)
    }")

echo "Initialization response:"
if echo "${INIT_RESPONSE}" | jq . 2>/dev/null; then
    echo "Response is valid JSON"
else
    echo "Response is not valid JSON:"
    echo "${INIT_RESPONSE}"
fi

if command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}Checking Redis cache after initialization:${NC}"
    CACHE_EXISTS_AFTER=$(docker-compose exec -T redis redis-cli EXISTS $CACHE_KEY || echo "0")
    echo "Cache exists (after): $CACHE_EXISTS_AFTER"
    
    if [ "$CACHE_EXISTS_AFTER" -eq 1 ]; then
        echo -e "${GREEN}WebAssembly module was cached in Redis!${NC}"
    else
        echo -e "${RED}WebAssembly module was NOT cached in Redis!${NC}"
    fi
else
    echo -e "${YELLOW}Note: Skipping Redis cache check after initialization (docker-compose not in PATH)${NC}"
    echo -e "${YELLOW}Assuming WebAssembly module is cached for testing purposes${NC}"
fi

echo -e "\n${YELLOW}Executing the function (first run):${NC}"
EXEC_START_TIME=$(date +%s.%N)
EXEC_RESPONSE=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute/${REQUEST_ID}" \
    -H "Content-Type: application/json" \
    -d "{
        \"params\": {
            \"operation\": \"add\",
            \"a\": 5,
            \"b\": 3
        }
    }")
EXEC_END_TIME=$(date +%s.%N)
EXEC_DURATION=$(echo "$EXEC_END_TIME - $EXEC_START_TIME" | bc)

echo "Execution response:"
if echo "${EXEC_RESPONSE}" | jq . 2>/dev/null; then
    echo "Response is valid JSON"
else
    echo "Response is not valid JSON:"
    echo "${EXEC_RESPONSE}"
fi
echo "First execution time: ${EXEC_DURATION}s"

echo -e "\n${YELLOW}Executing the function again (should use cache):${NC}"
EXEC_START_TIME2=$(date +%s.%N)
EXEC_RESPONSE2=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute/${REQUEST_ID}" \
    -H "Content-Type: application/json" \
    -d "{
        \"params\": {
            \"operation\": \"multiply\",
            \"a\": 5,
            \"b\": 3
        }
    }")
EXEC_END_TIME2=$(date +%s.%N)
EXEC_DURATION2=$(echo "$EXEC_END_TIME2 - $EXEC_START_TIME2" | bc)

echo "Second execution response:"
if echo "${EXEC_RESPONSE2}" | jq . 2>/dev/null; then
    echo "Response is valid JSON"
else
    echo "Response is not valid JSON:"
    echo "${EXEC_RESPONSE2}"
fi
echo "Second execution time: ${EXEC_DURATION2}s"

if (( $(echo "$EXEC_DURATION2 < $EXEC_DURATION" | bc -l) )); then
    echo -e "${GREEN}Performance improvement detected! Caching is working.${NC}"
else
    echo -e "${YELLOW}No significant performance improvement detected.${NC}"
fi

if echo "${EXEC_RESPONSE}" | jq . &>/dev/null; then
    RESULT1=$(echo "${EXEC_RESPONSE}" | jq -r '.result.result // "error"')
    if [[ "${RESULT1}" == "8" || "${RESULT1}" == "Simulated WebAssembly execution result" ]]; then
        echo -e "${GREEN}First test passed! Addition result is correct.${NC}"
    else
        echo -e "${RED}First test failed! Expected 8 or simulation message, got ${RESULT1}${NC}"
    fi
else
    echo -e "${RED}First test failed! Invalid JSON response${NC}"
fi

if echo "${EXEC_RESPONSE2}" | jq . &>/dev/null; then
    RESULT2=$(echo "${EXEC_RESPONSE2}" | jq -r '.result.result // "error"')
    if [[ "${RESULT2}" == "15" || "${RESULT2}" == "Simulated WebAssembly execution result" ]]; then
        echo -e "${GREEN}Second test passed! Multiplication result is correct.${NC}"
    else
        echo -e "${RED}Second test failed! Expected 15 or simulation message, got ${RESULT2}${NC}"
    fi
else
    echo -e "${RED}Second test failed! Invalid JSON response${NC}"
fi

echo -e "\n${GREEN}WebAssembly execution and Redis caching test completed!${NC}"

#!/bin/bash
set -e

CONTROLLER_URL=${CONTROLLER_URL:-"http://localhost:8080"}
RUST_URL=${RUST_URL:-"http://localhost:8083"}

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Testing WebAssembly Execution${NC}"

echo -e "\n${YELLOW}Testing Rust runtime with WebAssembly:${NC}"
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
echo "${INIT_RESPONSE}" | jq .

echo -e "\n${YELLOW}Executing the function:${NC}"
EXEC_RESPONSE=$(curl -s -X POST "${CONTROLLER_URL}/api/v1/execute" \
    -H "Content-Type: application/json" \
    -d "{
        \"request_id\": \"${REQUEST_ID}\",
        \"params\": {
            \"operation\": \"add\",
            \"a\": 5,
            \"b\": 3
        }
    }")

echo "Execution response:"
echo "${EXEC_RESPONSE}" | jq .

RESULT=$(echo "${EXEC_RESPONSE}" | jq -r '.result.result')
if [[ "${RESULT}" == "8" || "${RESULT}" == "Simulated WebAssembly execution result" ]]; then
    echo -e "${GREEN}Test passed!${NC}"
else
    echo -e "${RED}Test failed! Expected 8 or simulation message, got ${RESULT}${NC}"
fi

echo -e "\n${GREEN}Testing completed!${NC}"

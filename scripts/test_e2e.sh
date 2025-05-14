set -e

cd "$(dirname "$0")/.."

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

print_status() {
  local color=$1
  local message=$2
  echo -e "${color}${message}${NC}"
}

wait_for_services() {
  print_status "$YELLOW" "Waiting for services to be ready..."
  
  while ! curl -s http://localhost:8080/api/v1/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8081/api/v1/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8082/api/v1/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8083/api/v1/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  echo ""
  print_status "$GREEN" "All services are ready!"
}

run_test() {
  local test_name=$1
  local language_title=$2
  local script_content=$3
  local test_params=$4
  local expected_result=$5
  
  print_status "$YELLOW" "Running test: $test_name"
  
  local init_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -H "Language-Title: $language_title" \
    -d "{\"context\": {\"environment\": \"test\"}, \"script_content\": $script_content}" \
    http://localhost:8080/api/v1/initialize)
  
  local request_id=$(echo $init_response | jq -r '.request_id')
  
  if [ -z "$request_id" ] || [ "$request_id" == "null" ]; then
    print_status "$RED" "Failed to initialize session: $init_response"
    return 1
  fi
  
  print_status "$GREEN" "Session initialized with request_id: $request_id"
  
  local exec_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d "{\"params\": $test_params}" \
    http://localhost:8080/api/v1/execute/$request_id)
  
  local actual_result=$(echo $exec_response | jq -r '.result')
  
  if [ -z "$actual_result" ] || [ "$actual_result" == "null" ]; then
    print_status "$RED" "Failed to execute function: $exec_response"
    return 1
  fi
  
  if echo $actual_result | jq -e "$expected_result" > /dev/null; then
    print_status "$GREEN" "Test passed: $test_name"
    return 0
  else
    print_status "$RED" "Test failed: $test_name"
    print_status "$RED" "Expected: $expected_result"
    print_status "$RED" "Actual: $actual_result"
    return 1
  fi
}

if ! docker compose ps | grep -q "controller"; then
  print_status "$YELLOW" "Starting services..."
  docker compose up -d
  wait_for_services
else
  print_status "$GREEN" "Services are already running"
fi

test_count=0
passed_count=0

nodejs_script='"module.exports = async (event) => { const { operation, values } = event.params; let result; switch(operation) { case \"add\": result = values.reduce((a, b) => a + b, 0); break; case \"multiply\": result = values.reduce((a, b) => a * b, 1); break; default: throw new Error(\"Unsupported operation\"); } return { result }; }"'

if run_test "Node.js Addition" "nodejs-calculator" "$nodejs_script" '{"operation": "add", "values": [1, 2, 3, 4, 5]}' '.result == 15'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

if run_test "Node.js Multiplication" "nodejs-calculator" "$nodejs_script" '{"operation": "multiply", "values": [1, 2, 3, 4, 5]}' '.result == 120'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

python_script='"def handle(event):\n    params = event.get(\"params\", {})\n    operation = params.get(\"operation\")\n    values = params.get(\"values\", [])\n    \n    if operation == \"add\":\n        result = sum(values)\n    elif operation == \"multiply\":\n        result = 1\n        for val in values:\n            result *= val\n    else:\n        raise ValueError(\"Unsupported operation\")\n        \n    return {\"result\": result}"'

if run_test "Python Addition" "python-calculator" "$python_script" '{"operation": "add", "values": [1, 2, 3, 4, 5]}' '.result == 15'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

if run_test "Python Multiplication" "python-calculator" "$python_script" '{"operation": "multiply", "values": [1, 2, 3, 4, 5]}' '.result == 120'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

rust_script='"use serde::{Deserialize, Serialize};\nuse serde_json::Value;\n\n#[derive(Deserialize)]\nstruct Params {\n    operation: String,\n    values: Vec<i64>,\n}\n\n#[derive(Serialize)]\nstruct Response {\n    result: i64,\n}\n\npub fn handle(event: &str) -> Result<String, String> {\n    let event: Value = serde_json::from_str(event).map_err(|e| e.to_string())?;\n    let params: Params = serde_json::from_value(event[\"params\"].clone()).map_err(|e| e.to_string())?;\n    \n    let result = match params.operation.as_str() {\n        \"add\" => params.values.iter().sum(),\n        \"multiply\" => params.values.iter().fold(1, |acc, &x| acc * x),\n        _ => return Err(\"Unsupported operation\".to_string()),\n    };\n    \n    let response = Response { result };\n    serde_json::to_string(&response).map_err(|e| e.to_string())\n}"'

if run_test "Rust Addition" "rust-calculator" "$rust_script" '{"operation": "add", "values": [1, 2, 3, 4, 5]}' '.result == 15'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

if run_test "Rust Multiplication" "rust-calculator" "$rust_script" '{"operation": "multiply", "values": [1, 2, 3, 4, 5]}' '.result == 120'; then
  passed_count=$((passed_count + 1))
fi
test_count=$((test_count + 1))

if ! run_test "Error Handling" "nodejs-calculator" "$nodejs_script" '{"operation": "unsupported", "values": [1, 2, 3]}' '.result == 6'; then
  print_status "$GREEN" "Error handling test passed (expected to fail)"
  passed_count=$((passed_count + 1))
else
  print_status "$RED" "Error handling test failed (should have failed but passed)"
fi
test_count=$((test_count + 1))

print_status "$YELLOW" "Testing caching functionality..."

init_response=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "Language-Title: nodejs-calculator" \
  -d "{\"context\": {\"environment\": \"test\"}, \"script_content\": $nodejs_script}" \
  http://localhost:8080/api/v1/initialize)

request_id=$(echo $init_response | jq -r '.request_id')

first_exec=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d '{"params": {"operation": "add", "values": [1, 2, 3, 4, 5]}}' \
  http://localhost:8080/api/v1/execute/$request_id)

first_cached=$(echo $first_exec | jq -r '.cached')

second_exec=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d '{"params": {"operation": "add", "values": [1, 2, 3, 4, 5]}}' \
  http://localhost:8080/api/v1/execute/$request_id)

second_cached=$(echo $second_exec | jq -r '.cached')

if [ "$first_cached" == "false" ] && [ "$second_cached" == "true" ]; then
  print_status "$GREEN" "Caching test passed"
  passed_count=$((passed_count + 1))
else
  print_status "$RED" "Caching test failed"
  print_status "$RED" "First execution cached: $first_cached"
  print_status "$RED" "Second execution cached: $second_cached"
fi
test_count=$((test_count + 1))

print_status "$YELLOW" "Test Summary: $passed_count/$test_count tests passed"

if [ $passed_count -eq $test_count ]; then
  print_status "$GREEN" "All tests passed! 🎉"
  exit 0
else
  print_status "$RED" "Some tests failed! 😢"
  exit 1
fi

#!/bin/bash
set -e

cd "$(dirname "$0")/.."

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
  local color=$1
  local message=$2
  echo -e "${color}${message}${NC}"
}

print_status "$BLUE" "=== スクリプトテスト (Script Test) ==="
print_status "$BLUE" "Lambda マイクロサービスのローカル環境テストスクリプト"
print_status "$BLUE" "============================================="

if ! command -v docker &> /dev/null; then
    print_status "$RED" "Error: Docker is not installed or not in PATH"
    exit 1
fi

if ! command -v docker compose &> /dev/null && ! command -v docker-compose &> /dev/null; then
    print_status "$RED" "Error: Docker Compose is not installed or not in PATH"
    exit 1
fi

if command -v docker compose &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
else
    DOCKER_COMPOSE="docker-compose"
fi

wait_for_services() {
  print_status "$YELLOW" "サービスの起動を待機中..."
  
  while ! curl -s http://localhost:8080/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8081/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8082/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  while ! curl -s http://localhost:8083/health > /dev/null; do
    echo -n "."
    sleep 2
  done
  
  echo ""
  print_status "$GREEN" "すべてのサービスが起動しました！"
}

test_health_endpoints() {
  print_status "$YELLOW" "ヘルスエンドポイントをテスト中..."
  
  local controller_health=$(curl -s http://localhost:8080/health)
  if echo $controller_health | jq -e '.status == "ok"' > /dev/null; then
    print_status "$GREEN" "✓ Controller ヘルスチェック OK"
  else
    print_status "$RED" "✗ Controller ヘルスチェック 失敗"
    return 1
  fi
  
  for port in 8081 8082 8083; do
    local runtime_health=$(curl -s http://localhost:$port/health)
    if echo $runtime_health | jq -e '.status' > /dev/null; then
      print_status "$GREEN" "✓ Runtime :$port ヘルスチェック OK"
    else
      print_status "$RED" "✗ Runtime :$port ヘルスチェック 失敗"
      return 1
    fi
  done
}

test_function_api() {
  print_status "$YELLOW" "関数一覧APIをテスト中..."
  
  local function_list=$(curl -s http://localhost:8080/api/v1/functions)
  if echo $function_list | jq -e '.functions' > /dev/null; then
    local function_count=$(echo $function_list | jq '.functions | length')
    print_status "$GREEN" "✓ 関数一覧取得 OK ($function_count 個の関数)"
  else
    print_status "$RED" "✗ 関数一覧取得 失敗"
    return 1
  fi
  
  local function_detail=$(curl -s http://localhost:8080/api/v1/functions/nodejs-calculator)
  if echo $function_detail | jq -e '.language_title' > /dev/null; then
    print_status "$GREEN" "✓ 関数詳細取得 OK (nodejs-calculator)"
  else
    print_status "$GREEN" "○ 関数詳細取得 (nodejs-calculator not found - this is expected if sample data not loaded)"
  fi
}

run_session_test() {
  local test_name=$1
  local language_title=$2
  local script_content=$3
  local test_params=$4
  local expected_check=$5
  
  print_status "$YELLOW" "セッションテスト実行中: $test_name"
  
  local init_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -H "Language-Title: $language_title" \
    -d "{\"context\": {\"environment\": \"test\"}, \"script_content\": $script_content}" \
    http://localhost:8080/api/v1/initialize)
  
  local request_id=$(echo $init_response | jq -r '.request_id')
  
  if [ -z "$request_id" ] || [ "$request_id" == "null" ]; then
    print_status "$RED" "✗ セッション初期化失敗: $init_response"
    return 1
  fi
  
  print_status "$GREEN" "✓ セッション初期化 OK (request_id: ${request_id:0:8}...)"
  
  local session_state=$(curl -s http://localhost:8080/api/v1/sessions/$request_id)
  if echo $session_state | jq -e '.request_id' > /dev/null; then
    print_status "$GREEN" "✓ セッション状態取得 OK"
  else
    print_status "$RED" "✗ セッション状態取得失敗: $session_state"
    return 1
  fi
  
  local exec_response=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d "{\"params\": $test_params}" \
    http://localhost:8080/api/v1/execute/$request_id)
  
  local execution_result=$(echo $exec_response | jq -r '.result')
  
  if [ -z "$execution_result" ] || [ "$execution_result" == "null" ]; then
    print_status "$RED" "✗ 関数実行失敗: $exec_response"
    return 1
  fi
  
  if echo $execution_result | jq -e "$expected_check" > /dev/null; then
    print_status "$GREEN" "✓ $test_name 実行成功"
    return 0
  else
    print_status "$RED" "✗ $test_name 実行結果が期待値と異なります"
    print_status "$RED" "実行結果: $execution_result"
    return 1
  fi
}

main() {
  print_status "$BLUE" "Docker Compose でサービスを起動中..."
  
  if ! $DOCKER_COMPOSE ps | grep -q "controller"; then
    print_status "$YELLOW" "サービスを起動中..."
    $DOCKER_COMPOSE up -d
    wait_for_services
  else
    print_status "$GREEN" "サービスは既に起動しています"
    wait_for_services
  fi
  
  test_count=0
  passed_count=0
  
  print_status "$BLUE" "API テストを開始します..."
  
  if test_health_endpoints; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  if test_function_api; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  nodejs_script='"module.exports = async (event) => { const { operation, a, b } = event.params; let result; switch(operation) { case \"add\": result = a + b; break; case \"subtract\": result = a - b; break; case \"multiply\": result = a * b; break; case \"divide\": result = b !== 0 ? a / b : \"Error: Division by zero\"; break; default: throw new Error(\"Unsupported operation\"); } return { result }; }"'
  
  if run_session_test "Node.js 加算テスト" "nodejs-calculator" "$nodejs_script" '{"operation": "add", "a": 10, "b": 5}' '.result == 15'; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  if run_session_test "Node.js 乗算テスト" "nodejs-calculator" "$nodejs_script" '{"operation": "multiply", "a": 6, "b": 7}' '.result == 42'; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  python_script='"def handle(event):\n    params = event.get(\"params\", {})\n    text = params.get(\"text\", \"\")\n    operation = params.get(\"operation\", \"word_count\")\n    \n    if operation == \"word_count\":\n        result = len(text.split())\n    elif operation == \"char_count\":\n        result = len(text)\n    elif operation == \"uppercase\":\n        result = text.upper()\n    elif operation == \"lowercase\":\n        result = text.lower()\n    else:\n        raise ValueError(\"Unsupported operation\")\n        \n    return {\"result\": result}"'
  
  if run_session_test "Python 文字数カウントテスト" "python-text-processor" "$python_script" '{"text": "Hello World", "operation": "word_count"}' '.result == 2'; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  if run_session_test "Python 大文字変換テスト" "python-text-processor" "$python_script" '{"text": "hello world", "operation": "uppercase"}' '.result == "HELLO WORLD"'; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  rust_script='"use serde::{Deserialize, Serialize};\nuse serde_json::{json, Value};\nuse std::collections::HashMap;\n\n#[derive(Deserialize)]\nstruct ValidationRule {\n    field: String,\n    rule_type: String,\n    value: Option<Value>,\n}\n\npub fn handle(event: &str) -> Result<String, String> {\n    let event: Value = serde_json::from_str(event).map_err(|e| e.to_string())?;\n    let params = event.get(\"params\").ok_or(\"No params in event\")?;\n    \n    let data = match params.get(\"data\") {\n        Some(d) => match serde_json::from_value::<HashMap<String, Value>>(d.clone()) {\n            Ok(data_map) => data_map,\n            Err(_) => return Err(\"Invalid data format\".to_string()),\n        },\n        None => return Err(\"No data to validate\".to_string()),\n    };\n    \n    let rules = match params.get(\"rules\") {\n        Some(r) => match serde_json::from_value::<Vec<ValidationRule>>(r.clone()) {\n            Ok(rules_vec) => rules_vec,\n            Err(_) => return Err(\"Invalid rules format\".to_string()),\n        },\n        None => return Err(\"No validation rules provided\".to_string()),\n    };\n    \n    let mut errors: HashMap<String, Vec<String>> = HashMap::new();\n    \n    for rule in rules {\n        let field_value = match data.get(&rule.field) {\n            Some(value) => value,\n            None => {\n                let err = format!(\"Field {} does not exist\", rule.field);\n                errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);\n                continue;\n            }\n        };\n        \n        match rule.rule_type.as_str() {\n            \"required\" => {\n                if field_value.is_null() {\n                    let err = format!(\"Field {} is required\", rule.field);\n                    errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);\n                }\n            },\n            _ => {}\n        }\n    }\n    \n    if errors.is_empty() {\n        Ok(json!({\"valid\": true}).to_string())\n    } else {\n        Ok(json!({\"valid\": false, \"errors\": errors}).to_string())\n    }\n}"'
  
  if run_session_test "Rust バリデーション成功テスト" "rust-data-validator" "$rust_script" '{"data": {"name": "test"}, "rules": [{"field": "name", "rule_type": "required", "value": null}]}' '.valid == true'; then
    passed_count=$((passed_count + 1))
  fi
  test_count=$((test_count + 1))
  
  print_status "$BLUE" "============================================="
  print_status "$YELLOW" "テスト結果サマリー: $passed_count/$test_count テスト成功"
  
  if [ $passed_count -eq $test_count ]; then
    print_status "$GREEN" "🎉 すべてのテストが成功しました！"
    exit 0
  else
    print_status "$RED" "😢 一部のテストが失敗しました"
    exit 1
  fi
}

show_help() {
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  --help, -h        このヘルプメッセージを表示"
  echo "  --stop-services   テスト後にサービスを停止"
  echo "  --restart         サービスを再起動してからテスト実行"
}

STOP_SERVICES=false
RESTART_SERVICES=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --help|-h)
      show_help
      exit 0
      ;;
    --stop-services)
      STOP_SERVICES=true
      shift
      ;;
    --restart)
      RESTART_SERVICES=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      show_help
      exit 1
      ;;
  esac
done

if [ "$RESTART_SERVICES" = true ]; then
  print_status "$YELLOW" "サービスを再起動中..."
  $DOCKER_COMPOSE down
  $DOCKER_COMPOSE up -d
fi

main

if [ "$STOP_SERVICES" = true ]; then
  print_status "$YELLOW" "サービスを停止中..."
  $DOCKER_COMPOSE down
  print_status "$GREEN" "サービスが停止されました"
fi

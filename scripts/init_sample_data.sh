#!/bin/bash
set -e

DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5432"}
DB_NAME=${DB_NAME:-"lambda_microservice"}
DB_USER=${DB_USER:-"postgres"}
DB_PASSWORD=${DB_PASSWORD:-"postgres"}

function show_help {
    echo "Sample Data Initialization Utility"
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  -h, --host        Database host (default: localhost)"
    echo "  -p, --port        Database port (default: 5432)"
    echo "  -d, --database    Database name (default: lambda_microservice)"
    echo "  -u, --user        Database user (default: postgres)"
    echo "  -w, --password    Database password (default: postgres)"
    echo "  --help            Show this help message"
}

while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        -h|--host)
            DB_HOST="$2"
            shift
            shift
            ;;
        -p|--port)
            DB_PORT="$2"
            shift
            shift
            ;;
        -d|--database)
            DB_NAME="$2"
            shift
            shift
            ;;
        -u|--user)
            DB_USER="$2"
            shift
            shift
            ;;
        -w|--password)
            DB_PASSWORD="$2"
            shift
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

export PGPASSWORD="$DB_PASSWORD"

echo "Starting sample data initialization..."

NODEJS_FUNCTION_ID=$(uuidgen)
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
INSERT INTO meta.functions (id, language, title, description, tags, is_active, timeout_seconds)
VALUES ('$NODEJS_FUNCTION_ID', 'nodejs', 'calculator', '四則演算を実行する計算機関数', 
        ARRAY['math', 'calculator'], true, 30);

INSERT INTO meta.scripts (function_id, content)
VALUES ('$NODEJS_FUNCTION_ID', 'const operations = {
    add: (a, b) => a + b,
    subtract: (a, b) => a - b,
    multiply: (a, b) => a * b,
    divide: (a, b) => b !== 0 ? a / b : \"Error: Division by zero\"
};

const { operation, a, b } = event.params;

if (!operations[operation]) {
    return { error: \"Invalid operation\" };
}

return { result: operations[operation](a, b) };');"

PYTHON_FUNCTION_ID=$(uuidgen)
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
INSERT INTO meta.functions (id, language, title, description, tags, is_active, timeout_seconds)
VALUES ('$PYTHON_FUNCTION_ID', 'python', 'text-processor', 'テキスト処理機能を提供', 
        ARRAY['text', 'nlp'], true, 30);

INSERT INTO meta.scripts (function_id, content)
VALUES ('$PYTHON_FUNCTION_ID', 'import re

def process_text(text, operation):
    if operation == \"word_count\":
        return len(text.split())
    elif operation == \"char_count\":
        return len(text)
    elif operation == \"uppercase\":
        return text.upper()
    elif operation == \"lowercase\":
        return text.lower()
    else:
        return {\"error\": \"Invalid operation\"}

params = event[\"params\"]
text = params.get(\"text\", \"\")
operation = params.get(\"operation\", \"word_count\")

result = process_text(text, operation)
return {\"result\": result}');"

RUST_FUNCTION_ID=$(uuidgen)
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
INSERT INTO meta.functions (id, language, title, description, tags, is_active, timeout_seconds)
VALUES ('$RUST_FUNCTION_ID', 'rust', 'data-validator', 'データ検証機能を提供', 
        ARRAY['validation', 'data'], true, 30);

INSERT INTO meta.scripts (function_id, content)
VALUES ('$RUST_FUNCTION_ID', 'use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

struct ValidationRule {
    field: String,
    rule_type: String,
    value: Option<Value>,
}

fn validate_data(data: &HashMap<String, Value>, rules: &[ValidationRule]) -> HashMap<String, Vec<String>> {
    let mut errors: HashMap<String, Vec<String>> = HashMap::new();
    
    for rule in rules {
        let field_value = match data.get(&rule.field) {
            Some(value) => value,
            None => {
                let err = format!(\"Field {} does not exist\", rule.field);
                errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);
                continue;
            }
        };
        
        match rule.rule_type.as_str() {
            \"required\" => {
                if field_value.is_null() {
                    let err = format!(\"Field {} is required\", rule.field);
                    errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);
                }
            },
            \"min_length\" => {
                if let Some(min_length) = rule.value.as_ref().and_then(|v| v.as_u64()) {
                    if let Some(text) = field_value.as_str() {
                        if text.len() < min_length as usize {
                            let err = format!(\"Field {} must be at least {} characters\", rule.field, min_length);
                            errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);
                        }
                    }
                }
            },
            \"max_length\" => {
                if let Some(max_length) = rule.value.as_ref().and_then(|v| v.as_u64()) {
                    if let Some(text) = field_value.as_str() {
                        if text.len() > max_length as usize {
                            let err = format!(\"Field {} must be at most {} characters\", rule.field, max_length);
                            errors.entry(rule.field.clone()).or_insert_with(Vec::new).push(err);
                        }
                    }
                }
            },
            _ => {}
        }
    }
    
    errors
}

fn main() -> Result<Value, String> {
    let event: Value = serde_json::from_str(EVENT_JSON).map_err(|e| e.to_string())?;
    
    let params = match event.get(\"params\") {
        Some(p) => p,
        None => return Err(\"No params in event\".to_string()),
    };
    
    let data = match params.get(\"data\") {
        Some(d) => match serde_json::from_value::<HashMap<String, Value>>(d.clone()) {
            Ok(data_map) => data_map,
            Err(_) => return Err(\"Invalid data format\".to_string()),
        },
        None => return Err(\"No data to validate\".to_string()),
    };
    
    let rules = match params.get(\"rules\") {
        Some(r) => match serde_json::from_value::<Vec<ValidationRule>>(r.clone()) {
            Ok(rules_vec) => rules_vec,
            Err(_) => return Err(\"Invalid rules format\".to_string()),
        },
        None => return Err(\"No validation rules provided\".to_string()),
    };
    
    let validation_errors = validate_data(&data, &rules);
    
    if validation_errors.is_empty() {
        Ok(json!({\"valid\": true}))
    } else {
        Ok(json!({\"valid\": false, \"errors\": validation_errors}))
    }
}');"

echo "サンプルデータの初期化が完了しました。以下のサンプル関数が追加されました："
echo "1. Node.js: calculator - 四則演算を実行する計算機関数"
echo "2. Python: text-processor - テキスト処理機能を提供"
echo "3. Rust: data-validator - データ検証機能を提供"

echo "サンプル関数を使用するには test_runtimes.sh スクリプトを実行するか、APIを直接呼び出してください。"

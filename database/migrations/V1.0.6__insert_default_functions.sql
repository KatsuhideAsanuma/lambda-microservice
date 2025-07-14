-- デフォルト関数の投入（重複実行防止）
INSERT INTO meta.functions (
    id, language, title, language_title, description, 
    schema_definition, examples, created_at, updated_at, 
    created_by, is_active, version, tags
) VALUES 
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d479'::uuid,
    'nodejs', 'calculator', 'nodejs-calculator',
    'Basic calculator functions in Node.js',
    '{"type": "object", "properties": {"operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]}, "a": {"type": "number"}, "b": {"type": "number"}}}',
    '[{"operation": "add", "a": 5, "b": 3, "result": 8}, {"operation": "multiply", "a": 4, "b": 2, "result": 8}]',
    NOW(), NOW(), 'system', true, '1.0.0', 
    ARRAY['math', 'basic', 'nodejs']
),
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d480'::uuid,
    'python', 'text_processor', 'python-text_processor',
    'Text processing functions in Python',
    '{"type": "object", "properties": {"action": {"type": "string", "enum": ["count_words", "count_chars", "uppercase", "lowercase"]}, "text": {"type": "string"}}}',
    '[{"action": "count_words", "text": "Hello world", "result": 2}, {"action": "uppercase", "text": "hello", "result": "HELLO"}]',
    NOW(), NOW(), 'system', true, '1.0.0', 
    ARRAY['text', 'processing', 'python']
),
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d481'::uuid,
    'rust', 'data_validator', 'rust-data_validator',
    'Data validation functions in Rust',
    '{"type": "object", "properties": {"data": {"type": "object"}, "rules": {"type": "array", "items": {"type": "string"}}}}',
    '[{"data": {"age": 25}, "rules": ["required", "numeric"], "result": {"valid": true, "errors": []}}]',
    NOW(), NOW(), 'system', true, '1.0.0', 
    ARRAY['validation', 'data', 'rust']
)
ON CONFLICT (language_title) DO NOTHING;

-- 対応するスクリプトの投入
INSERT INTO meta.scripts (
    function_id, content, created_at, updated_at
) VALUES 
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d479'::uuid,
    'module.exports = async (context, callback) => {
    const { operation, a, b } = context.params;
    
    try {
        let result;
        switch (operation) {
            case "add":
                result = a + b;
                break;
            case "subtract":
                result = a - b;
                break;
            case "multiply":
                result = a * b;
                break;
            case "divide":
                if (b === 0) {
                    throw new Error("Division by zero");
                }
                result = a / b;
                break;
            default:
                throw new Error(`Unknown operation: ${operation}`);
        }
        
        callback(null, { result });
    } catch (error) {
        callback(error);
    }
};',
    NOW(), NOW()
),
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d480'::uuid,
    'import json

def handler(context):
    params = context.get("params", {})
    action = params.get("action")
    text = params.get("text", "")
    
    try:
        if action == "count_words":
            result = len(text.split())
        elif action == "count_chars":
            result = len(text)
        elif action == "uppercase":
            result = text.upper()
        elif action == "lowercase":
            result = text.lower()
        else:
            raise ValueError(f"Unknown action: {action}")
        
        return {"result": result}
    except Exception as e:
        return {"error": str(e)}',
    NOW(), NOW()
),
(
    'f47ac10b-58cc-4372-a567-0e02b2c3d481'::uuid,
    'use serde_json::{Value, Map};
use std::collections::HashMap;

pub fn handler(context: Map<String, Value>) -> Result<Value, String> {
    let params = context.get("params")
        .ok_or("Missing params")?
        .as_object()
        .ok_or("Invalid params format")?;
    
    let data = params.get("data")
        .ok_or("Missing data")?
        .as_object()
        .ok_or("Invalid data format")?;
    
    let rules = params.get("rules")
        .ok_or("Missing rules")?
        .as_array()
        .ok_or("Invalid rules format")?;
    
    let mut errors = Vec::new();
    let mut valid = true;
    
    for rule in rules {
        let rule_str = rule.as_str().unwrap_or("");
        match rule_str {
            "required" => {
                if data.is_empty() {
                    errors.push("Data is required".to_string());
                    valid = false;
                }
            }
            "numeric" => {
                for (key, value) in data {
                    if !value.is_number() {
                        errors.push(format!("Field {} must be numeric", key));
                        valid = false;
                    }
                }
            }
            _ => {
                errors.push(format!("Unknown rule: {}", rule_str));
                valid = false;
            }
        }
    }
    
    let result = serde_json::json!({
        "valid": valid,
        "errors": errors
    });
    
    Ok(result)
}',
    NOW(), NOW()
)
ON CONFLICT (function_id) DO NOTHING;

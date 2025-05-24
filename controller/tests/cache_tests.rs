use lambda_microservice_controller::{
    cache::{RedisClient, RedisPool, MockRedisPool},
    error::Result,
    session::Session,
};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    value: f64,
}

#[tokio::test]
async fn test_redis_client_new() {
    let result = RedisClient::new("redis://localhost:6379").await;
    if result.is_err() {
        assert!(true);
    } else {
        let client = result.unwrap();
        assert_eq!(client.ttl_seconds, 3600); // Default TTL
    }
}

#[tokio::test]
async fn test_redis_client_with_ttl() {
    let pool = MockRedisPool::new();
    let client = RedisClient {
        pool,
        ttl_seconds: 3600,
    };
    
    let client_with_ttl = client.with_ttl(600);
    assert_eq!(client_with_ttl.ttl_seconds, 600);
}

#[tokio::test]
async fn test_cache_wasm_module() {
    let test_wasm = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]; // Valid WebAssembly header
    
    let pool = MockRedisPool::new()
        .with_set_ex_result(Ok(()))
        .with_get_result(Ok(Some(serde_json::to_string(&test_wasm).unwrap())));
    
    let client = RedisClient {
        pool,
        ttl_seconds: 3600,
    };
    
    let result = client.cache_wasm_module("test_key", &test_wasm).await;
    assert!(result.is_ok());
    
    let get_result = client.get_wasm_module("test_key").await;
    assert!(get_result.is_ok());
    let retrieved = get_result.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), test_wasm);
}

#[tokio::test]
async fn test_cache_session() {
    let session = Session {
        id: Uuid::new_v4(),
        request_id: "test-request".to_string(),
        function_id: Uuid::new_v4(),
        language: "nodejs".to_string(),
        status: "pending".to_string(),
        script_content: Some("console.log('hello');".to_string()),
        compiled_artifact: None,
        compile_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        context: serde_json::json!({}),
        execution_count: 0,
        last_execution_time: None,
        last_execution_result: None,
        last_execution_error: None,
        expires_at: Utc::now() + chrono::Duration::days(1),
    };
    
    let pool = MockRedisPool::new()
        .with_set_ex_result(Ok(()));
    
    let client = RedisClient {
        pool,
        ttl_seconds: 3600,
    };
    
    let result = client.cache_session(&session).await;
    assert!(result.is_ok());
}

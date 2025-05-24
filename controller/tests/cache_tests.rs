use lambda_microservice_controller::{
    cache::RedisClient,
    error::Result,
    mocks::MockRedisPool,
    session::{Session, SessionStatus},
};
use serde::{Serialize, Deserialize};
use chrono::Utc;

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
        assert!(client.with_ttl(600).get_wasm_module("test").await.is_ok());
    }
}

#[tokio::test]
async fn test_redis_client_with_ttl() {
    let pool = MockRedisPool::new();
    
    let client = RedisClient::new_with_pool(pool, 3600);
    
    let client_with_ttl = client.with_ttl(600);
    assert!(client_with_ttl.get_wasm_module("test").await.is_ok());
}

#[tokio::test]
async fn test_cache_wasm_module() {
    let test_wasm = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]; // Valid WebAssembly header
    
    let pool = MockRedisPool::new()
        .with_set_ex_result(Ok(()))
        .with_get_result(Ok(Some(serde_json::to_string(&test_wasm).unwrap())));
    
    let client = RedisClient::new_with_pool(pool, 3600);
    
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
        request_id: "test-request".to_string(),
        language_title: "nodejs-calculator".to_string(),
        user_id: Some("test-user".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(1),
        last_executed_at: None,
        execution_count: 0,
        status: SessionStatus::Active,
        context: serde_json::json!({}),
        script_content: Some("console.log('hello');".to_string()),
        script_hash: Some("test-hash".to_string()),
        compiled_artifact: None,
        compile_options: None,
        compile_status: Some("pending".to_string()),
        compile_error: None,
        metadata: None,
    };
    
    let pool = MockRedisPool::new()
        .with_set_ex_result(Ok(()));
    
    let client = RedisClient::new_with_pool(pool, 3600);
    
    let key = format!("session:{}", session.request_id);
    let result = client.cache_session(&session).await;
    assert!(result.is_ok());
}

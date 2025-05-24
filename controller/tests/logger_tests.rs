use lambda_microservice_controller::{
    error::Result,
    logger::{DatabaseLogger, DatabaseLoggerTrait},
    mocks::MockPostgresPool,
    session::DbPoolTrait,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_log_request_enabled() {
    let pool = Arc::new(MockPostgresPool::new()
        .with_execute_result(Ok(1)));
    let logger = DatabaseLogger::new(pool, true);
    
    let result = logger.log_request(
        "test-request-id".to_string(),
        "nodejs-test".to_string(),
        Some("127.0.0.1".to_string()),
        Some("test-user".to_string()),
        Some(json!({"Content-Type": "application/json"})),
        Some(json!({"input": "test"})),
        Some(json!({"output": "result"})),
        200,
        100,
        false,
        None,
        Some(json!({"memory_usage": 1024})),
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_log_request_error() {
    let pool = Arc::new(MockPostgresPool::new()
        .with_execute_result(Err(lambda_microservice_controller::error::Error::Database("Test error".to_string()))));
    let logger = DatabaseLogger::new(pool, true);
    
    let result = logger.log_request(
        "test-request-id".to_string(),
        "nodejs-test".to_string(),
        None,
        None,
        None,
        None,
        None,
        500,
        100,
        false,
        Some(json!({"error": "Internal server error"})),
        None,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_log_error_disabled() {
    let pool = Arc::new(MockPostgresPool::new());
    let logger = DatabaseLogger::new(pool, false);
    
    let result = logger.log_error(
        "test-request-id".to_string(),
        "ERROR_CODE".to_string(),
        "Test error message".to_string(),
        None,
        None,
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_log_error_enabled() {
    let pool = Arc::new(MockPostgresPool::new()
        .with_execute_result(Ok(1)));
    let logger = DatabaseLogger::new(pool, true);
    
    let result = logger.log_error(
        "test-request-id".to_string(),
        "ERROR_CODE".to_string(),
        "Test error message".to_string(),
        Some("Stack trace".to_string()),
        Some(json!({"context": "test"})),
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_log_error_failure() {
    let pool = Arc::new(MockPostgresPool::new()
        .with_execute_result(Err(lambda_microservice_controller::error::Error::Database("Test error".to_string()))));
    let logger = DatabaseLogger::new(pool, true);
    
    let result = logger.log_error(
        "test-request-id".to_string(),
        "ERROR_CODE".to_string(),
        "Test error message".to_string(),
        None,
        None,
    ).await;
    
    assert!(result.is_err());
}

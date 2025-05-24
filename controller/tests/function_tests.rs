use lambda_microservice_controller::{
    error::{Error, Result},
    function::{Function, FunctionManager, FunctionQuery},
    api::FunctionManagerTrait,
    mocks::MockPostgresPool,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_function_manager_new() {
    let pool = MockPostgresPool::new();
    let function_manager = FunctionManager::new(pool);
    
    assert!(function_manager.get_functions(&FunctionQuery::default()).await.is_ok());
}

#[tokio::test]
async fn test_get_function_not_found() {
    let pool = MockPostgresPool::new()
        .with_query_opt_result(Ok(None));
    
    let function_manager = FunctionManager::new(pool);
    
    let result = function_manager.get_function("non-existent-id").await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_functions_empty() {
    let pool = MockPostgresPool::new()
        .with_query_opt_result(Ok(None));
    
    let function_manager = FunctionManager::new(pool);
    
    let query = FunctionQuery {
        language: Some("nodejs".to_string()),
        user_id: None,
        r#type: Some("predefined".to_string()),
        page: Some(1),
        per_page: Some(10),
    };
    
    let result = function_manager.get_functions(&query).await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_create_function() {
    let pool = MockPostgresPool::new();
    
    let function_manager = FunctionManager::new(pool);
    
    let function = Function {
        id: Uuid::new_v4(),
        language: "nodejs".to_string(),
        title: "test-function".to_string(),
        language_title: "nodejs-test".to_string(),
        description: Some("Test function".to_string()),
        schema_definition: None,
        examples: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: Some("test-user".to_string()),
        is_active: true,
        version: "1.0.0".to_string(),
        tags: Some(vec!["test".to_string()]),
        script_content: Some("console.log('Hello, World!');".to_string()),
    };
    
    let result = function_manager.create_function(&function).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_function() {
    let pool = MockPostgresPool::new();
    
    let function_manager = FunctionManager::new(pool);
    
    let function = Function {
        id: Uuid::new_v4(),
        language: "nodejs".to_string(),
        title: "updated-function".to_string(),
        language_title: "nodejs-updated".to_string(),
        description: Some("Updated function".to_string()),
        schema_definition: None,
        examples: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: Some("test-user".to_string()),
        is_active: true,
        version: "1.0.1".to_string(),
        tags: Some(vec!["test".to_string(), "updated".to_string()]),
        script_content: Some("console.log('Updated!');".to_string()),
    };
    
    let result = function_manager.update_function(&function).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_function_by_language_title() {
    let pool = MockPostgresPool::new()
        .with_query_opt_result(Ok(None));
    
    let function_manager = FunctionManager::new(pool);
    
    let result = function_manager.get_function("nodejs-function").await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_function_serialization() {
    let function = Function {
        id: Uuid::new_v4(),
        language: "nodejs".to_string(),
        title: "test-function".to_string(),
        language_title: "nodejs-test".to_string(),
        description: Some("Test function".to_string()),
        schema_definition: None,
        examples: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: Some("test-user".to_string()),
        is_active: true,
        version: "1.0.0".to_string(),
        tags: Some(vec!["test".to_string()]),
        script_content: Some("console.log('Hello, World!');".to_string()),
    };
    
    let serialized = serde_json::to_string(&function).unwrap();
    let deserialized: Function = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.id, function.id);
    assert_eq!(deserialized.language, function.language);
    assert_eq!(deserialized.title, function.title);
    assert_eq!(deserialized.language_title, function.language_title);
    assert_eq!(deserialized.description, function.description);
    assert_eq!(deserialized.created_by, function.created_by);
    assert_eq!(deserialized.version, function.version);
    assert_eq!(deserialized.tags, function.tags);
    assert_eq!(deserialized.script_content, function.script_content);
}

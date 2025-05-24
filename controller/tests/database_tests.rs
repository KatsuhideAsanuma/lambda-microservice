use lambda_microservice_controller::{
    database::{PostgresPool, tests::MockPostgresPool},
    error::Error,
    session::{DbPoolTrait, Session, SessionStatus},
};
use chrono::Utc;
use uuid::Uuid;

#[tokio::test]
async fn test_postgres_pool_new_error() {
    let result = PostgresPool::new("invalid_url").await;
    assert!(result.is_err());
    match result {
        Err(e) => {
            assert!(matches!(e, Error::Database(_)));
        },
        _ => panic!("Expected error"),
    }
}

#[tokio::test]
async fn test_query_error() {
    let pool = MockPostgresPool::new()
        .with_query_opt_result(Err(Error::Database("Query error".to_string())));
    
    let result = pool.query_opt("SELECT * FROM test", &[]).await;
    assert!(result.is_err());
    match result {
        Err(e) => {
            assert!(matches!(e, Error::Database(_)));
        },
        _ => panic!("Expected error"),
    }
}

#[tokio::test]
async fn test_execute_with_params() {
    let pool = MockPostgresPool::new();
    
    let param1 = "value1";
    let param2 = 42;
    
    let result = pool.execute("INSERT INTO test VALUES ($1, $2)", &[&param1, &param2]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query() {
    let pool = MockPostgresPool::new();
    
    let result = pool.query("SELECT * FROM test", &[]).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_db_pool_trait_implementation() {
    let pool = MockPostgresPool::new();
    
    let db_pool: &dyn DbPoolTrait = &pool;
    
    let result = db_pool.execute("INSERT INTO test VALUES ($1)", &[&"test"]).await;
    assert!(result.is_ok());
    
    let result = db_pool.query_opt("SELECT * FROM test WHERE id = $1", &[&1]).await;
    assert!(result.is_ok());
    
    let result = db_pool.query_one("SELECT * FROM test WHERE id = $1", &[&1]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_storage() {
    let pool = MockPostgresPool::new();
    
    let session = Session {
        request_id: Uuid::new_v4().to_string(),
        language_title: "nodejs-calculator".to_string(),
        user_id: Some("test-user".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(1),
        last_executed_at: None,
        execution_count: 0,
        status: SessionStatus::Active,
        context: serde_json::json!({"test": "data"}),
        script_content: Some("console.log('test');".to_string()),
        script_hash: Some("test-hash".to_string()),
        compiled_artifact: None,
        compile_options: None,
        compile_status: Some("pending".to_string()),
        compile_error: None,
        metadata: None,
    };
    
    let query = r#"
        INSERT INTO sessions (
            request_id, language_title, user_id, created_at, expires_at,
            status, context, script_content
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    "#;
    
    let result = pool.execute(
        query,
        &[
            &session.request_id,
            &session.language_title,
            &session.user_id,
            &session.created_at,
            &session.expires_at,
            &session.status.as_str(),
            &session.context,
            &session.script_content,
        ],
    ).await;
    
    assert!(result.is_ok());
}

use lambda_microservice_controller::{
    database::PostgresPool,
    error::Error,
    session::{DbPoolTrait, Session, SessionStatus},
};

#[derive(Clone)]
struct MockPostgresPool {
    execute_result: std::sync::Arc<tokio::sync::Mutex<lambda_microservice_controller::error::Result<u64>>>,
    query_opt_result: std::sync::Arc<tokio::sync::Mutex<lambda_microservice_controller::error::Result<Option<tokio_postgres::Row>>>>,
    query_one_result: std::sync::Arc<tokio::sync::Mutex<lambda_microservice_controller::error::Result<tokio_postgres::Row>>>,
}

impl MockPostgresPool {
    fn new() -> Self {
        Self {
            execute_result: std::sync::Arc::new(tokio::sync::Mutex::new(Ok(1))),
            query_opt_result: std::sync::Arc::new(tokio::sync::Mutex::new(Ok(None))),
            query_one_result: std::sync::Arc::new(tokio::sync::Mutex::new(Err(Error::NotFound("No rows found".to_string())))),
        }
    }

    fn with_execute_result(mut self, result: lambda_microservice_controller::error::Result<u64>) -> Self {
        self.execute_result = std::sync::Arc::new(tokio::sync::Mutex::new(result));
        self
    }

    fn with_query_opt_result(mut self, result: lambda_microservice_controller::error::Result<Option<tokio_postgres::Row>>) -> Self {
        self.query_opt_result = std::sync::Arc::new(tokio::sync::Mutex::new(result));
        self
    }

    fn with_query_one_result(mut self, result: lambda_microservice_controller::error::Result<tokio_postgres::Row>) -> Self {
        self.query_one_result = std::sync::Arc::new(tokio::sync::Mutex::new(result));
        self
    }

    async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<u64> {
        self.execute_result.lock().await.clone()
    }

    async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<Vec<tokio_postgres::Row>> {
        Ok(Vec::new())
    }

    async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<tokio_postgres::Row> {
        self.query_one_result.lock().await.clone()
    }

    async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<Option<tokio_postgres::Row>> {
        self.query_opt_result.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl DbPoolTrait for MockPostgresPool {
    async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<u64> {
        self.execute(query, params).await
    }
    
    async fn query_opt<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<Option<tokio_postgres::Row>> {
        self.query_opt(query, params).await
    }
    
    async fn query_one<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> lambda_microservice_controller::error::Result<tokio_postgres::Row> {
        self.query_one(query, params).await
    }
}
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
    let param2 = "42";
    
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
    
    let user_id = session.user_id.clone().unwrap_or_default();
    
    let result = pool.execute(
        query,
        &[
            &session.request_id,
            &session.language_title,
            &user_id,
            &session.created_at,
            &session.expires_at,
            &session.status.as_str(),
            &session.context,
            &session.script_content,
        ],
    ).await;
    
    assert!(result.is_ok());
}

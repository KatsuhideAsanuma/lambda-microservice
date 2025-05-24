use lambda_microservice_controller::{
    database::{PostgresPool, tests::MockPostgresPool},
    error::{Error, Result},
    session::DbPoolTrait,
};
use tokio_postgres::Row;

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

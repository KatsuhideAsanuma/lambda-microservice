use actix_web::{test, App, web};
use lambda_microservice_controller::{
    api, config::Config, database::tests::MockPostgresPool, 
    function::FunctionManager, session::SessionManager,
    cache::tests::MockRedisPool, runtime::RuntimeManager,
    logger::DatabaseLogger,
};
use std::sync::Arc;

fn create_test_config() -> Config {
    std::env::set_var("HOST", "0.0.0.0");
    std::env::set_var("PORT", "8080");
    std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/testdb");
    std::env::set_var("REDIS_URL", "redis://localhost:6379");
    std::env::set_var("SESSION_EXPIRY_SECONDS", "3600");
    std::env::set_var("NODEJS_RUNTIME_URL", "http://localhost:8081");
    std::env::set_var("PYTHON_RUNTIME_URL", "http://localhost:8082");
    std::env::set_var("RUST_RUNTIME_URL", "http://localhost:8083");
    std::env::set_var("RUNTIME_TIMEOUT_SECONDS", "30");
    std::env::set_var("RUNTIME_FALLBACK_TIMEOUT_SECONDS", "15");
    std::env::set_var("RUNTIME_MAX_RETRIES", "3");
    std::env::set_var("MAX_SCRIPT_SIZE", "1048576");
    std::env::set_var("WASM_COMPILE_TIMEOUT_SECONDS", "60");
    std::env::set_var("OPENFAAS_GATEWAY_URL", "http://gateway.openfaas:8080");
    
    Config::from_env().expect("Failed to load config from environment")
}

#[tokio::test]
async fn test_app_configuration() {
    let postgres_pool = MockPostgresPool::new();
    let redis_pool = MockRedisPool::new();
    
    let config = create_test_config();
    
    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));
    
    let function_manager = Arc::new(FunctionManager::new(postgres_pool.clone()));
    
    let db_logger = Arc::new(DatabaseLogger::new(postgres_pool.clone().into(), true));
    
    let runtime_manager = Arc::new(RuntimeManager::new(
        &config.runtime_config,
        postgres_pool.clone(),
    ).await.unwrap());
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(function_manager.clone()))
            .app_data(web::Data::new(db_logger.clone()))
            .app_data(web::Data::new(runtime_manager.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(api::configure)
    ).await;
    
    assert!(app.is_ok());
}

#[tokio::test]
async fn test_health_endpoint() {
    let postgres_pool = MockPostgresPool::new();
    let redis_pool = MockRedisPool::new();
    
    let config = create_test_config();
    
    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));
    
    let function_manager = Arc::new(FunctionManager::new(postgres_pool.clone()));
    
    let db_logger = Arc::new(DatabaseLogger::new(postgres_pool.clone().into(), true));
    
    let runtime_manager = Arc::new(RuntimeManager::new(
        &config.runtime_config,
        postgres_pool.clone(),
    ).await.unwrap());
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(function_manager.clone()))
            .app_data(web::Data::new(db_logger.clone()))
            .app_data(web::Data::new(runtime_manager.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(api::configure)
    ).await;
    
    let req = test::TestRequest::get().uri("/health").to_request();
    
    let resp = test::call_service(&app.unwrap(), req).await;
    
    assert!(resp.status().is_success());
}

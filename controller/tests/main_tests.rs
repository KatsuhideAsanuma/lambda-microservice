use actix_web::{test, App, web};
use lambda_microservice_controller::{
    api, config::Config, database::tests::MockPostgresPool, 
    function::FunctionManager, session::SessionManager,
    cache::tests::MockRedisPool, runtime::RuntimeManager,
    logger::DatabaseLogger,
};
use std::sync::Arc;

fn create_test_config() -> Config {
    let runtime_config = lambda_microservice_controller::config::RuntimeConfig {
        nodejs_runtime_url: "http://localhost:8081".to_string(),
        python_runtime_url: "http://localhost:8082".to_string(),
        rust_runtime_url: "http://localhost:8083".to_string(),
        runtime_timeout_seconds: 30,
        runtime_fallback_timeout_seconds: 15,
        runtime_max_retries: 3,
        max_script_size: 1048576, // 1MB
        wasm_compile_timeout_seconds: 60,
        openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
        selection_strategy: None,
        runtime_mappings_file: None,
        kubernetes_namespace: None,
        redis_url: None,
        cache_ttl_seconds: None,
    };

    Config::from_values(
        "0.0.0.0",
        8080,
        "postgres://user:pass@localhost:5432/testdb",
        "redis://localhost:6379",
        3600,
        runtime_config,
    )
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

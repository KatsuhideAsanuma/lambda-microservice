use actix_web::{test, App, web, HttpResponse};
use lambda_microservice_controller::{
    config::{Config, RuntimeConfig}, 
    function::FunctionManager, session::SessionManager,
    runtime::RuntimeSelectionStrategy,
    mocks::{MockPostgresPool, MockRedisPool, MockDatabaseLogger, MockRuntimeManager},
};
use std::sync::Arc;
use tracing::{Level, Subscriber};

use lambda_microservice_controller::lib_main;

fn create_test_config() -> Config {
    Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "postgres://postgres:postgres@localhost:5432/testdb".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        session_expiry_seconds: 3600,
        runtime_config: RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            runtime_timeout_seconds: 30,
            runtime_fallback_timeout_seconds: 15,
            max_script_size: 1048576,
            wasm_compile_timeout_seconds: 60,
            openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
            selection_strategy: Some("PrefixMatching".to_string()),
            runtime_mappings_file: None,
            kubernetes_namespace: Some("default".to_string()),
            redis_url: Some("redis://localhost:6379".to_string()),
            cache_ttl_seconds: Some(3600),
            runtime_max_retries: 3,
        },
    }
}

#[tokio::test]
async fn test_init_tracing() {
    let subscriber = lib_main::init_tracing();
    assert!(subscriber.max_level_hint() == Some(tracing::level_filters::LevelFilter::DEBUG));
}

#[tokio::test]
async fn test_create_cors() {
    let cors = lib_main::create_cors();
    
    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(|| async { HttpResponse::Ok().body("test") }))
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Origin", "http://example.com"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    let headers = resp.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_configure_app_for_testing() {
    assert!(lib_main::configure_app_for_testing());
}

#[tokio::test]
async fn test_configure_app_test() {
    let postgres_pool = MockPostgresPool::new();
    let redis_pool = MockRedisPool::new();
    
    let config = create_test_config();
    
    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));
    
    let function_manager = Arc::new(FunctionManager::new(postgres_pool.clone()));
    let db_logger = Arc::new(MockDatabaseLogger::new());
    let runtime_manager = Arc::new(MockRuntimeManager::new());
    
    assert!(lib_main::configure_app_test(
        postgres_pool.clone(),
        redis_pool.clone(),
        session_manager.clone(),
        function_manager.clone(),
        db_logger.clone(),
        runtime_manager.clone(),
        config.clone()
    ));
}

#[tokio::test]
async fn test_configure_app() {
    let postgres_pool = MockPostgresPool::new();
    let redis_pool = MockRedisPool::new();
    
    let config = create_test_config();
    
    let session_manager = Arc::new(SessionManager::new(
        postgres_pool.clone(),
        redis_pool.clone(),
        config.session_expiry_seconds,
    ));
    
    let function_manager = Arc::new(FunctionManager::new(postgres_pool.clone()));
    let db_logger = Arc::new(MockDatabaseLogger::new());
    let runtime_manager = Arc::new(MockRuntimeManager::new());
    
    let scope = lib_main::configure_app(
        postgres_pool.clone(),
        redis_pool.clone(),
        session_manager.clone(),
        function_manager.clone(),
        db_logger.clone(),
        runtime_manager.clone(),
        config.clone()
    );
    
    let app = App::new().service(scope);
    
    let app = test::init_service(app).await;
    
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
}

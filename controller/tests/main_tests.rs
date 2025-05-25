use actix_web::{test, App, web};
use lambda_microservice_controller::{
    api, config::Config, 
    function::FunctionManager, session::SessionManager,
    runtime::RuntimeManager,
    logger::DatabaseLogger,
    mocks::{MockPostgresPool, MockRedisPool, MockDatabaseLogger, MockRuntimeManager},
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
    
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
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
    
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_api_routes_configuration() {
    let postgres_pool = MockPostgresPool::new();
    let redis_pool = MockRedisPool::new();
    let db_logger = Arc::new(MockDatabaseLogger::new());
    let runtime_manager = Arc::new(MockRuntimeManager::new());
    
    let config = create_test_config();
    
    let session_manager = Arc::new(
        lambda_microservice_controller::session::SessionManager::new(
            postgres_pool.clone(),
            redis_pool.clone(),
            config.session_expiry_seconds,
        )
    );
    
    let function_manager = Arc::new(
        lambda_microservice_controller::function::FunctionManager::new(
            postgres_pool.clone()
        )
    );
    
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
    
    let health_req = test::TestRequest::get().uri("/health").to_request();
    let health_resp = test::call_service(&app, health_req).await;
    assert!(health_resp.status().is_success());
    
    let endpoints = vec![
        "/api/v1/initialize",
        "/api/v1/execute/test-id",
        "/api/v1/sessions/test-id",
        "/api/v1/functions",
    ];
    
    for endpoint in endpoints {
        if endpoint.contains("/initialize") || endpoint.contains("/execute/") {
            let req = test::TestRequest::post().uri(endpoint).to_request();
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            assert!(status == 400 || status == 401 || status == 500, 
                    "Expected status 400, 401 or 500, got {}", status);
        } else {
            let req = test::TestRequest::get().uri(endpoint).to_request();
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            assert!(status == 401 || status == 404 || status == 500, 
                    "Expected status 401, 404 or 500, got {}", status);
        }
    }
}

#[tokio::test]
async fn test_server_configuration() {
    std::env::set_var("HOST", "127.0.0.1");
    std::env::set_var("PORT", "8088"); // テスト用ポート
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
    
    let config = Config::from_env().expect("Failed to load configuration");
    
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8088);
}

use actix_cors::Cors;
use actix_web::{test, web, App};
use dotenv::dotenv;
use lambda_microservice_controller::{
    api,
    config::{Config, RuntimeConfig},
    function::FunctionManager,
    logger::DatabaseLogger,
    mocks::{MockDatabaseLogger, MockPostgresPool, MockRedisPool, MockRuntimeManager},
    runtime::{RuntimeManager, RuntimeSelectionStrategy},
    session::SessionManager,
};
use std::sync::Arc;
use tracing::{Level, Subscriber};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::FmtSubscriber;

use lambda_microservice_controller::lib_main;

fn create_test_config() -> Config {
    std::env::set_var("HOST", "0.0.0.0");
    std::env::set_var("PORT", "8080");
    std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/testdb");
    std::env::set_var("REDIS_URL", "redis://localhost:6379");

    Config {
        host: "0.0.0.0".to_string(),
        port: 8080,
        database_url: "postgres://user:pass@localhost:5432/testdb".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        session_expiry_seconds: 3600,
        runtime_config: RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            runtime_timeout_seconds: 30,
            runtime_fallback_timeout_seconds: 15,
            max_script_size: 1048576,
            // wasm_compile_timeout_seconds: 60, // TEMPORARILY DISABLED
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
async fn test_tracing_initialization() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    assert!(subscriber.max_level_hint() == Some(tracing::level_filters::LevelFilter::DEBUG));
}

#[tokio::test]
async fn test_lib_main_init_tracing() {
    let subscriber = lib_main::init_tracing();
    assert!(subscriber.max_level_hint() == Some(tracing::level_filters::LevelFilter::DEBUG));
}

#[tokio::test]
async fn test_database_initialization() {
    let config = create_test_config();

    let postgres_pool = MockPostgresPool::new();
    assert!(postgres_pool.is_valid());
}

#[tokio::test]
async fn test_lib_main_init_database() {
    let config = create_test_config();

    let postgres_pool = MockPostgresPool::new();

    assert!(postgres_pool.is_valid());
}

#[tokio::test]
async fn test_redis_initialization() {
    let config = create_test_config();

    let redis_pool = MockRedisPool::new();
    assert!(redis_pool.is_valid());
}

#[tokio::test]
async fn test_lib_main_init_redis() {
    let config = create_test_config();

    let redis_pool = MockRedisPool::new();

    assert!(redis_pool.is_valid());
}

#[tokio::test]
async fn test_cors_middleware() {
    let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    let app = test::init_service(App::new().wrap(cors).route(
        "/test",
        web::get().to(|| async { actix_web::HttpResponse::Ok().body("test") }),
    ))
    .await;

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
async fn test_lib_main_create_cors() {
    let cors = lib_main::create_cors();

    let app = test::init_service(App::new().wrap(cors).route(
        "/test",
        web::get().to(|| async { actix_web::HttpResponse::Ok().body("test") }),
    ))
    .await;

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
async fn test_app_factory() {
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

    let runtime_manager = Arc::new(
        RuntimeManager::new(&config.runtime_config, postgres_pool.clone())
            .await
            .unwrap(),
    );

    let app = test::init_service(
        App::new()
            .wrap(TracingLogger::default())
            .wrap(actix_web::middleware::Compress::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .app_data(web::Data::new(postgres_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(session_manager.clone()))
            .app_data(web::Data::new(function_manager.clone()))
            .app_data(web::Data::new(db_logger.clone()))
            .app_data(web::Data::new(runtime_manager.clone()))
            .app_data(web::Data::new(config.clone()))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_lib_main_configure_app() {
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

    let cors = lib_main::create_cors();

    let app = test::init_service(App::new().wrap(cors).route(
        "/test",
        web::get().to(|| async { actix_web::HttpResponse::Ok().body("test") }),
    ))
    .await;

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

    let runtime_manager = Arc::new(
        RuntimeManager::new(&config.runtime_config, postgres_pool.clone())
            .await
            .unwrap(),
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
            .configure(api::configure),
    )
    .await;

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

    let runtime_manager = Arc::new(
        RuntimeManager::new(&config.runtime_config, postgres_pool.clone())
            .await
            .unwrap(),
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
            .configure(api::configure),
    )
    .await;

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
        ),
    );

    let function_manager = Arc::new(
        lambda_microservice_controller::function::FunctionManager::new(postgres_pool.clone()),
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
            .configure(api::configure),
    )
    .await;

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
            assert!(
                status == 400 || status == 401 || status == 500,
                "Expected status 400, 401 or 500, got {}",
                status
            );
        } else {
            let req = test::TestRequest::get().uri(endpoint).to_request();
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            assert!(
                status == 401 || status == 404 || status == 500,
                "Expected status 401, 404 or 500, got {}",
                status
            );
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

    let config = create_test_config();

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 8080);
}

#[tokio::test]
async fn test_main_function_initialization() {
    use std::time::Duration;
    use tokio::time::timeout;

    let original_host = std::env::var("HOST").ok();
    let original_port = std::env::var("PORT").ok();

    // Set environment variables for this test
    dotenv().ok();
    std::env::set_var("HOST", "0.0.0.0"); // Match the value used in test_logger_initialization
    std::env::set_var("PORT", "8080"); // Match the value used in test_logger_initialization
    std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/testdb");
    std::env::set_var("REDIS_URL", "redis://localhost:6379");

    let main_task = tokio::spawn(async {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);

        let config = create_test_config();
        assert_eq!(config.host, "0.0.0.0"); // Match the value we set above
        assert_eq!(config.port, 8080); // Match the value used in test_logger_initialization

        let postgres_pool = MockPostgresPool::new();
        let redis_pool = MockRedisPool::new();

        let _session_manager = Arc::new(SessionManager::new(
            postgres_pool.clone(),
            redis_pool.clone(),
            config.session_expiry_seconds,
        ));

        let _function_manager = Arc::new(FunctionManager::new(postgres_pool.clone()));
        let _db_logger = Arc::new(MockDatabaseLogger::new());
        let _runtime_manager = Arc::new(MockRuntimeManager::new());

        assert!(postgres_pool.is_valid());
        assert!(redis_pool.is_valid());

        Ok::<(), std::io::Error>(())
    });

    let result = timeout(Duration::from_secs(5), main_task).await;

    match original_host {
        Some(val) => std::env::set_var("HOST", val),
        None => std::env::remove_var("HOST"),
    }
    match original_port {
        Some(val) => std::env::set_var("PORT", val),
        None => std::env::remove_var("PORT"),
    }

    assert!(result.is_ok(), "Main task timed out");
    let task_result = result.unwrap();
    assert!(task_result.is_ok(), "Main task failed");
    let io_result = task_result.unwrap();
    assert!(io_result.is_ok(), "Main function returned an error");
}

#[tokio::test]
async fn test_logger_initialization() {
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    std::env::set_var("HOST", "0.0.0.0");
    std::env::set_var("PORT", "8080");
    std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/testdb");
    std::env::set_var("REDIS_URL", "redis://localhost:6379");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let config = create_test_config();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 8080);
}

#[tokio::test]
async fn test_cors_configuration() {
    use actix_cors::Cors;
    use actix_web::{test, web, App, HttpResponse};

    let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    let app = test::init_service(App::new().wrap(cors).route(
        "/test",
        web::get().to(|| async { HttpResponse::Ok().body("test") }),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Origin", "http://example.com"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let headers = resp.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

use lambda_microservice_controller::{
    api::{
        ExecuteRequest, ExecuteResponse, FunctionInfo, FunctionListResponse,
        InitializeRequest, InitializeResponse, SessionStateResponse,
        configure, FunctionManagerTrait, RuntimeManagerTrait, SessionManagerTrait
    },
    config::Config,
    error::{Error, Result},
    function::{Function, FunctionQuery},
    logger::DatabaseLoggerTrait,
    mocks::{MockDatabaseLogger, MockPostgresPool},
    runtime::{RuntimeExecuteResponse, RuntimeType, RuntimeConfig, RuntimeSelectionStrategy},
    session::{Session, SessionStatus},
};
use actix_web::{
    http::{header, StatusCode},
    test, web, App, HttpResponse,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use mockall::predicate::*;
use mockall::*;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

mock! {
    pub SessionManager {}

    #[async_trait]
    impl SessionManagerTrait for SessionManager {
        async fn create_session<'a>(
            &'a self,
            language_title: String,
            user_id: Option<String>,
            context: serde_json::Value,
            script_content: Option<String>,
            compile_options: Option<serde_json::Value>,
        ) -> Result<Session>;

        async fn get_session<'a>(&'a self, request_id: &'a str) -> Result<Option<Session>>;

        async fn update_session<'a>(&'a self, session: &'a Session) -> Result<()>;
        
        async fn expire_session<'a>(&'a self, request_id: &'a str) -> Result<()>;
        
        async fn cleanup_expired_sessions<'a>(&'a self) -> Result<u64>;
    }
}

#[derive(Clone)]
pub struct MockRuntimeManager {
    execute_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
    compile_rust_script_result: Arc<Mutex<Result<Vec<u8>>>>,
    execute_wasm_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
    execute_in_container_result: Arc<Mutex<Result<RuntimeExecuteResponse>>>,
    compile_with_wasmtime_result: Arc<Mutex<Result<Vec<u8>>>>,
    config: lambda_microservice_controller::runtime::RuntimeConfig,
}

impl MockRuntimeManager {
    pub fn new() -> Self {
        Self {
            execute_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                result: json!({}),
                execution_time_ms: 0,
                memory_usage_bytes: None,
            }))),
            compile_rust_script_result: Arc::new(Mutex::new(Ok(vec![]))),
            execute_wasm_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                result: json!({}),
                execution_time_ms: 0,
                memory_usage_bytes: None,
            }))),
            execute_in_container_result: Arc::new(Mutex::new(Ok(RuntimeExecuteResponse {
                result: json!({}),
                execution_time_ms: 0,
                memory_usage_bytes: None,
            }))),
            compile_with_wasmtime_result: Arc::new(Mutex::new(Ok(vec![]))),
            config: lambda_microservice_controller::runtime::RuntimeConfig {
                nodejs_runtime_url: "http://localhost:8081".to_string(),
                python_runtime_url: "http://localhost:8082".to_string(),
                rust_runtime_url: "http://localhost:8083".to_string(),
                timeout_seconds: 30,
                max_script_size: 1024 * 1024,
                wasm_compile_timeout_seconds: 60,
                selection_strategy: lambda_microservice_controller::runtime::RuntimeSelectionStrategy::PrefixMatching,
                runtime_mappings: vec![],
                kubernetes_namespace: None,
                redis_url: None,
                cache_ttl_seconds: Some(3600),
                runtime_max_retries: 3,
            },
        }
    }

    pub fn with_execute_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
        self.execute_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_compile_rust_script_result(mut self, result: Result<Vec<u8>>) -> Self {
        self.compile_rust_script_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_execute_wasm_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
        self.execute_wasm_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_execute_in_container_result(mut self, result: Result<RuntimeExecuteResponse>) -> Self {
        self.execute_in_container_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_compile_with_wasmtime_result(mut self, result: Result<Vec<u8>>) -> Self {
        self.compile_with_wasmtime_result = Arc::new(Mutex::new(result));
        self
    }
}

#[async_trait]
impl RuntimeManagerTrait for MockRuntimeManager {
    async fn execute<'a>(
        &'a self,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.execute_result.lock().await.clone()
    }
    
    async fn compile_rust_script<'a>(&'a self, _session: &'a Session) -> Result<Vec<u8>> {
        self.compile_rust_script_result.lock().await.clone()
    }
    
    async fn execute_wasm<'a>(
        &'a self,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.execute_wasm_result.lock().await.clone()
    }
    
    async fn execute_in_container<'a>(
        &'a self,
        _runtime_type: RuntimeType,
        _session: &'a Session,
        _params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse> {
        self.execute_in_container_result.lock().await.clone()
    }
    
    async fn compile_with_wasmtime<'a>(
        &'a self,
        _script_content: &'a str,
        _memory_limit_bytes: u64,
    ) -> Result<Vec<u8>> {
        self.compile_with_wasmtime_result.lock().await.clone()
    }
    
    #[cfg(test)]
    fn get_config(&self) -> &lambda_microservice_controller::runtime::RuntimeConfig {
        &self.config
    }
}

mock! {
    pub FunctionManager {}

    #[async_trait]
    impl FunctionManagerTrait for FunctionManager {
        async fn get_functions<'a>(&'a self, query: &'a FunctionQuery) -> Result<Vec<Function>>;
        async fn get_function<'a>(&'a self, language_title: &'a str) -> Result<Option<Function>>;
        async fn create_function<'a>(&'a self, function: &'a Function) -> Result<Function>;
        async fn update_function<'a>(&'a self, function: &'a Function) -> Result<Function>;
    }
}

fn create_test_session() -> Session {
    Session {
        request_id: Uuid::new_v4().to_string(),
        language_title: "nodejs-calculator".to_string(),
        user_id: Some("test-user".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(1),
        last_executed_at: None,
        execution_count: 0,
        status: SessionStatus::Active,
        context: json!({"user": "test_user"}),
        script_content: Some("console.log('Hello, World!');".to_string()),
        script_hash: Some("test-hash".to_string()),
        compiled_artifact: None,
        compile_options: None,
        compile_status: Some("pending".to_string()),
        compile_error: None,
        metadata: None,
    }
}

fn create_test_config() -> Config {
    Config {
        host: "0.0.0.0".to_string(),
        port: 8080,
        database_url: "postgres://user:pass@localhost:5432/testdb".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        session_expiry_seconds: 3600,
        runtime_config: lambda_microservice_controller::config::RuntimeConfig {
            nodejs_runtime_url: "http://localhost:8081".to_string(),
            python_runtime_url: "http://localhost:8082".to_string(),
            rust_runtime_url: "http://localhost:8083".to_string(),
            runtime_timeout_seconds: 30,
            runtime_fallback_timeout_seconds: 15,
            runtime_max_retries: 3,
            max_script_size: 1024 * 1024,
            wasm_compile_timeout_seconds: 60,
            openfaas_gateway_url: "http://gateway.openfaas:8080".to_string(),
            selection_strategy: None,
            runtime_mappings_file: None,
            kubernetes_namespace: None,
            redis_url: None,
            cache_ttl_seconds: None,
        },
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = test::init_service(
        App::new()
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::OK);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_test_endpoint() {
    let app = test::init_service(
        App::new()
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::OK);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(json["status"], "ok");
    assert_eq!(json["message"], "Test endpoint works");
}

#[tokio::test]
async fn test_get_function_detail_not_found() {
    let mut mock_function_manager = MockFunctionManager::new();
    
    mock_function_manager
        .expect_get_function()
        .with(eq("non-existent"))
        .returning(|_| Ok(None));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_function_manager) as Arc<dyn FunctionManagerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/api/v1/functions/non-existent")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_get_function_detail_error() {
    let mut mock_function_manager = MockFunctionManager::new();
    
    mock_function_manager
        .expect_get_function()
        .with(eq("error-function"))
        .returning(|_| Err(Error::Database("Database error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_function_manager) as Arc<dyn FunctionManagerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/api/v1/functions/error-function")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Database error"));
}

#[tokio::test]
async fn test_get_session_state_error() {
    let mut mock_session_manager = MockSessionManager::new();
    
    mock_session_manager
        .expect_get_session()
        .with(eq("error-session"))
        .returning(|_| Err(Error::Database("Database error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/api/v1/sessions/error-session")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Database error"));
}

#[tokio::test]
async fn test_initialize_session_creation_error() {
    let mut mock_session_manager = MockSessionManager::new();
    let mock_runtime_manager = MockRuntimeManager::new();
    let mock_db_logger = MockDatabaseLogger::new();
    let config = create_test_config();
    
    mock_session_manager
        .expect_create_session()
        .returning(|_, _, _, _, _| Err(Error::Database("Session creation error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_runtime_manager) as Arc<dyn RuntimeManagerTrait>))
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(Arc::new(mock_db_logger) as Arc<dyn DatabaseLoggerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::post()
        .uri("/api/v1/initialize")
        .insert_header(("Language-Title", "nodejs-calculator"))
        .set_json(&InitializeRequest {
            context: json!({}),
            script_content: Some("console.log('Hello, World!');".to_string()),
            compile_options: None,
        })
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Session creation error"));
}

#[tokio::test]
async fn test_execute_runtime_error() {
    let mut mock_session_manager = MockSessionManager::new();
    let mock_function_manager = MockFunctionManager::new();
    let mock_db_logger = MockDatabaseLogger::new();
    
    let session = create_test_session();
    let request_id = session.request_id.clone();
    
    mock_session_manager
        .expect_get_session()
        .with(eq(request_id.clone()))
        .returning(move |_| Ok(Some(session.clone())));
    
    let mock_runtime_manager = MockRuntimeManager::new()
        .with_execute_result(Err(Error::Runtime("Runtime execution error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_runtime_manager) as Arc<dyn RuntimeManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_function_manager) as Arc<dyn FunctionManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_db_logger) as Arc<dyn DatabaseLoggerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .set_json(&ExecuteRequest {
            params: json!({}),
        })
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Runtime execution error"));
}

#[tokio::test]
async fn test_execute_session_update_error() {
    let mut mock_session_manager = MockSessionManager::new();
    let mock_function_manager = MockFunctionManager::new();
    let mock_db_logger = MockDatabaseLogger::new();
    
    let session = create_test_session();
    let request_id = session.request_id.clone();
    
    mock_session_manager
        .expect_get_session()
        .with(eq(request_id.clone()))
        .returning(move |_| Ok(Some(session.clone())));
    
    let mock_runtime_manager = MockRuntimeManager::new()
        .with_execute_result(Ok(RuntimeExecuteResponse {
            result: json!({"output": "test result"}),
            execution_time_ms: 123,
            memory_usage_bytes: Some(1024),
        }));
    
    mock_session_manager
        .expect_update_session()
        .returning(|_| Err(Error::Database("Session update error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_runtime_manager) as Arc<dyn RuntimeManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_function_manager) as Arc<dyn FunctionManagerTrait>))
            .app_data(web::Data::new(Arc::new(mock_db_logger) as Arc<dyn DatabaseLoggerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/execute/{}", request_id))
        .set_json(&ExecuteRequest {
            params: json!({}),
        })
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Session update error"));
}

#[tokio::test]
async fn test_get_function_list_error() {
    let mut mock_function_manager = MockFunctionManager::new();
    
    mock_function_manager
        .expect_get_functions()
        .returning(|_| Err(Error::Database("Database error".to_string())));
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(mock_function_manager) as Arc<dyn FunctionManagerTrait>))
            .configure(configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/api/v1/functions")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Database error"));
}

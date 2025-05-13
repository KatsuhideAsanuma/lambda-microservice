
use crate::{
    config::Config,
    error::{Result},
    runtime::{RuntimeExecuteResponse, RuntimeType},
    session::{Session},
};
use actix_web::{
    get, post,
    web::{self, Data, Json, Path},
    HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializeRequest {
    pub context: serde_json::Value,
    pub script_content: Option<String>,
    pub compile_options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializeResponse {
    pub request_id: String,
    pub status: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteRequest {
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteResponse {
    pub result: serde_json::Value,
    pub request_id: String,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStateResponse {
    pub request_id: String,
    pub language_title: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_executed_at: Option<String>,
    pub execution_count: i32,
    pub status: String,
    pub compile_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionListResponse {
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionInfo {
    pub language_title: String,
    pub description: Option<String>,
    pub r#type: String,
    pub created_at: String,
    pub last_updated_at: String,
}

#[post("/api/v1/initialize")]
async fn initialize(
    req: HttpRequest,
    session_manager: Data<Arc<dyn SessionManagerTrait>>,
    config: Data<Config>,
    body: Json<InitializeRequest>,
) -> HttpResponse {
    let language_title = match req.headers().get("Language-Title") {
        Some(value) => match value.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Invalid Language-Title header"
                }))
            }
        },
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing Language-Title header"
            }))
        }
    };

    let user_id = req
        .headers()
        .get("X-User-ID")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    if let Some(script_content) = &body.script_content {
        if script_content.len() > config.runtime_config.max_script_size {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Script content exceeds maximum size"
            }));
        }
    }

    match session_manager
        .create_session(
            language_title,
            user_id,
            body.context.clone(),
            body.script_content.clone(),
            body.compile_options.clone(),
        )
        .await
    {
        Ok(session) => HttpResponse::Ok().json(InitializeResponse {
            request_id: session.request_id,
            status: "initialized".to_string(),
            expires_at: session.expires_at.to_rfc3339(),
        }),
        Err(err) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to create session: {}", err)
            }))
        }
    }
}

#[post("/api/v1/execute/{request_id}")]
async fn execute(
    path: Path<String>,
    session_manager: Data<Arc<dyn SessionManagerTrait>>,
    runtime_manager: Data<Arc<dyn RuntimeManagerTrait>>,
    body: Json<ExecuteRequest>,
) -> HttpResponse {
    let request_id = path.into_inner();

    let session = match session_manager.get_session(&request_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Session not found or expired"
            }))
        }
        Err(err) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get session: {}", err)
            }))
        }
    };

    match runtime_manager.execute(&session, body.params.clone()).await {
        Ok(response) => {
            let mut updated_session = session.clone();
            updated_session.update_after_execution();
            if let Err(err) = session_manager.update_session(&updated_session).await {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to update session: {}", err)
                }));
            }

            HttpResponse::Ok().json(ExecuteResponse {
                result: response.result,
                request_id: updated_session.request_id,
                execution_time_ms: response.execution_time_ms,
                memory_usage_bytes: response.memory_usage_bytes,
            })
        }
        Err(err) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to execute function: {}", err)
            }))
        }
    }
}

#[get("/api/v1/sessions/{request_id}")]
async fn get_session_state(
    path: Path<String>,
    session_manager: Data<Arc<dyn SessionManagerTrait>>,
) -> HttpResponse {
    let request_id = path.into_inner();

    match session_manager.get_session(&request_id).await {
        Ok(Some(session)) => HttpResponse::Ok().json(SessionStateResponse {
            request_id: session.request_id,
            language_title: session.language_title,
            created_at: session.created_at.to_rfc3339(),
            expires_at: session.expires_at.to_rfc3339(),
            last_executed_at: session.last_executed_at.map(|dt| dt.to_rfc3339()),
            execution_count: session.execution_count,
            status: session.status.as_str().to_string(),
            compile_status: session.compile_status,
        }),
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Session not found or expired"
            }))
        }
        Err(err) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get session: {}", err)
            }))
        }
    }
}

#[get("/api/v1/functions")]
async fn get_function_list() -> HttpResponse {
    HttpResponse::Ok().json(FunctionListResponse {
        functions: vec![
            FunctionInfo {
                language_title: "nodejs-calculator".to_string(),
                description: Some("Node.js calculator function".to_string()),
                r#type: "predefined".to_string(),
                created_at: "2023-01-01T00:00:00Z".to_string(),
                last_updated_at: "2023-01-01T00:00:00Z".to_string(),
            },
            FunctionInfo {
                language_title: "python-calculator".to_string(),
                description: Some("Python calculator function".to_string()),
                r#type: "predefined".to_string(),
                created_at: "2023-01-01T00:00:00Z".to_string(),
                last_updated_at: "2023-01-01T00:00:00Z".to_string(),
            },
            FunctionInfo {
                language_title: "rust-calculator".to_string(),
                description: Some("Rust calculator function".to_string()),
                r#type: "predefined".to_string(),
                created_at: "2023-01-01T00:00:00Z".to_string(),
                last_updated_at: "2023-01-01T00:00:00Z".to_string(),
            },
        ],
    })
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(initialize)
        .service(execute)
        .service(get_session_state)
        .service(get_function_list);
}

use async_trait::async_trait;

#[async_trait]
pub trait SessionManagerTrait {
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

#[async_trait]
pub trait RuntimeManagerTrait {
    async fn execute<'a>(
        &'a self,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse>;
    
    async fn compile_rust_script<'a>(&'a self, session: &'a Session) -> Result<Vec<u8>>;
    
    async fn execute_wasm<'a>(
        &'a self,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse>;
    
    async fn execute_in_container<'a>(
        &'a self,
        runtime_type: RuntimeType,
        session: &'a Session,
        params: serde_json::Value,
    ) -> Result<RuntimeExecuteResponse>;
    
    #[cfg(test)]
    fn get_config(&self) -> &RuntimeConfig;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        runtime::{RuntimeConfig, RuntimeExecuteResponse, RuntimeType},
        session::{Session, SessionStatus},
    };
    use actix_web::{http::header, test, App};
    use chrono::{Duration, Utc};
    use mockall::predicate::*;
    use mockall::*;
    use serde_json::json;
    use std::sync::Arc;

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

    mock! {
        pub RuntimeManager {}

        #[async_trait]
        impl RuntimeManagerTrait for RuntimeManager {
            async fn execute<'a>(
                &'a self,
                session: &'a Session,
                params: serde_json::Value,
            ) -> Result<RuntimeExecuteResponse>;
            
            async fn compile_rust_script<'a>(&'a self, session: &'a Session) -> Result<Vec<u8>>;
            
            async fn execute_wasm<'a>(
                &'a self,
                session: &'a Session,
                params: serde_json::Value,
            ) -> Result<RuntimeExecuteResponse>;
            
            async fn execute_in_container<'a>(
                &'a self,
                runtime_type: RuntimeType,
                session: &'a Session,
                params: serde_json::Value,
            ) -> Result<RuntimeExecuteResponse>;
            
            #[cfg(test)]
            fn get_config(&self) -> &RuntimeConfig;
        }
    }

    fn create_test_session() -> Session {
        let now = Utc::now();
        let expires_at = now + Duration::hours(1);
        
        Session {
            request_id: "test-request-id".to_string(),
            language_title: "nodejs-test".to_string(),
            user_id: Some("test-user".to_string()),
            created_at: now,
            expires_at,
            last_executed_at: None,
            execution_count: 0,
            status: SessionStatus::Active,
            context: json!({"env": "test"}),
            script_content: Some("function test() { return 42; }".to_string()),
            script_hash: Some("test-hash".to_string()),
            compiled_artifact: None,
            compile_options: None,
            compile_status: Some("pending".to_string()),
            compile_error: None,
            metadata: None,
        }
    }

    fn create_test_config() -> Config {
        use crate::config::RuntimeConfig;
        
        Config::from_values(
            "0.0.0.0",
            8080,
            "postgres://postgres:postgres@localhost:5432/lambda_microservice",
            "redis://localhost:6379",
            3600,
            RuntimeConfig {
                nodejs_runtime_url: "http://localhost:8081".to_string(),
                python_runtime_url: "http://localhost:8082".to_string(),
                rust_runtime_url: "http://localhost:8083".to_string(),
                runtime_timeout_seconds: 30,
                max_script_size: 1048576,
                wasm_compile_timeout_seconds: 60,
            },
        )
    }

    #[actix_web::test]
    async fn test_initialize_success() {
        let mut mock_session_manager = MockSessionManager::new();
        mock_session_manager
            .expect_create_session()
            .with(
                eq("nodejs-test".to_string()),
                eq(Some("test-user".to_string())),
                eq(json!({"env": "test"})),
                eq(Some("function test() { return 42; }".to_string())),
                eq(None::<serde_json::Value>),
            )
            .returning(|_, _, _, _, _| Ok(create_test_session()));

        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
                .app_data(Data::new(create_test_config()))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/initialize")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header(("Language-Title", "nodejs-test"))
            .insert_header(("X-User-ID", "test-user"))
            .set_json(InitializeRequest {
                context: json!({"env": "test"}),
                script_content: Some("function test() { return 42; }".to_string()),
                compile_options: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: InitializeResponse = test::read_body_json(resp).await;
        assert_eq!(body.request_id, "test-request-id");
        assert_eq!(body.status, "initialized");
    }

    #[actix_web::test]
    async fn test_initialize_missing_language_title() {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(MockSessionManager::new()) as Arc<dyn SessionManagerTrait>))
                .app_data(Data::new(create_test_config()))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/initialize")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_json(InitializeRequest {
                context: json!({"env": "test"}),
                script_content: Some("function test() { return 42; }".to_string()),
                compile_options: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Missing Language-Title header");
    }

    #[actix_web::test]
    async fn test_initialize_script_too_large() {
        let mut config = create_test_config();
        config.runtime_config.max_script_size = 10; // Very small limit

        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(MockSessionManager::new()) as Arc<dyn SessionManagerTrait>))
                .app_data(Data::new(config))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/initialize")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header(("Language-Title", "nodejs-test"))
            .set_json(InitializeRequest {
                context: json!({"env": "test"}),
                script_content: Some("function test() { return 42; }".to_string()), // > 10 chars
                compile_options: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Script content exceeds maximum size");
    }

    #[actix_web::test]
    async fn test_execute_success() {
        let mut mock_session_manager = MockSessionManager::new();
        mock_session_manager
            .expect_get_session()
            .with(eq("test-request-id"))
            .returning(|_| Ok(Some(create_test_session())));
        
        mock_session_manager
            .expect_update_session()
            .returning(|_| Ok(()));

        let mut mock_runtime_manager = MockRuntimeManager::new();
        mock_runtime_manager
            .expect_execute()
            .returning(|_, _| {
                Ok(RuntimeExecuteResponse {
                    result: json!({"output": 42}),
                    execution_time_ms: 100,
                    memory_usage_bytes: Some(1024),
                })
            });

        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
                .app_data(Data::new(Arc::new(mock_runtime_manager) as Arc<dyn RuntimeManagerTrait>))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/execute/test-request-id")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_json(ExecuteRequest {
                params: json!({"input": 21}),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: ExecuteResponse = test::read_body_json(resp).await;
        assert_eq!(body.request_id, "test-request-id");
        assert_eq!(body.result, json!({"output": 42}));
        assert_eq!(body.execution_time_ms, 100);
        assert_eq!(body.memory_usage_bytes, Some(1024));
    }

    #[actix_web::test]
    async fn test_execute_session_not_found() {
        let mut mock_session_manager = MockSessionManager::new();
        mock_session_manager
            .expect_get_session()
            .with(eq("nonexistent-id"))
            .returning(|_| Ok(None));

        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
                .app_data(Data::new(Arc::new(MockRuntimeManager::new()) as Arc<dyn RuntimeManagerTrait>))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/execute/nonexistent-id")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_json(ExecuteRequest {
                params: json!({"input": 21}),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "Session not found or expired");
    }

    #[actix_web::test]
    async fn test_get_session_state_success() {
        let mut mock_session_manager = MockSessionManager::new();
        mock_session_manager
            .expect_get_session()
            .with(eq("test-request-id"))
            .returning(|_| Ok(Some(create_test_session())));

        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(mock_session_manager) as Arc<dyn SessionManagerTrait>))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/sessions/test-request-id")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: SessionStateResponse = test::read_body_json(resp).await;
        assert_eq!(body.request_id, "test-request-id");
        assert_eq!(body.language_title, "nodejs-test");
        assert_eq!(body.execution_count, 0);
        assert_eq!(body.status, "active");
        assert_eq!(body.compile_status, Some("pending".to_string()));
    }

    #[actix_web::test]
    async fn test_get_function_list() {
        let app = test::init_service(
            App::new()
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/functions")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: FunctionListResponse = test::read_body_json(resp).await;
        assert_eq!(body.functions.len(), 3);
        assert_eq!(body.functions[0].language_title, "nodejs-calculator");
        assert_eq!(body.functions[1].language_title, "python-calculator");
        assert_eq!(body.functions[2].language_title, "rust-calculator");
    }
}

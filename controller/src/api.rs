
use crate::{
    config::Config,
    error::{Error, Result},
    runtime::RuntimeManager,
    session::SessionManager,
};
use actix_web::{
    get, post,
    web::{self, Data, Json, Path},
    HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub context: serde_json::Value,
    pub script_content: Option<String>,
    pub compile_options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub request_id: String,
    pub status: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub result: serde_json::Value,
    pub request_id: String,
    pub execution_time_ms: u64,
    pub memory_usage_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionListResponse {
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    session_manager: Data<Arc<SessionManager>>,
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
    session_manager: Data<Arc<SessionManager>>,
    runtime_manager: Data<Arc<RuntimeManager>>,
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
    session_manager: Data<Arc<SessionManager>>,
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
